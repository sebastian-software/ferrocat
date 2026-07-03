//! Internal catalog pipeline for the public `ferrocat-po` catalog API.
//!
//! This module owns the higher-level workflow around PO parsing, extracted-message
//! normalization, merge semantics, and export back to PO. The byte-oriented parser
//! and serializer hot paths stay elsewhere; this layer is where we preserve
//! catalog semantics and diagnostics.

use std::fs;
use std::{collections::BTreeMap, mem};

use rustc_hash::FxHashMap;

use crate::diagnostic_codes;

use super::export::export_catalog_content;
use super::file_io::atomic_write;
use super::helpers::{
    dedupe_origins, dedupe_placeholders, dedupe_strings, merge_placeholders, merge_unique_origins,
    merge_unique_strings,
};
use super::mt::{
    MachineMetadata, PO_AI_KEY, PO_LOCK_KEY, parse_ai_descriptor, validate_machine_metadata,
};
use super::plural::{PluralProfile, derive_plural_variable, expected_gettext_nplurals_for_locale};
use super::{
    ApiError, CatalogMessage, CatalogOrigin, CatalogSemantics, CatalogStats, CatalogStorageFormat,
    CatalogUpdateInput, CatalogUpdateResult, Diagnostic, DiagnosticSeverity, ExtractedMessage,
    ObsoleteInfo, ObsoleteStrategy, OrderBy, ParseCatalogOptions, ParsedCatalog, PluralEncoding,
    PluralSource, TranslationShape, UpdateCatalogFileOptions, UpdateCatalogOptions,
};
use crate::{MsgStr, PoFile, PoItem, PoVec, parse_po};

#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct Catalog {
    pub(super) locale: Option<String>,
    pub(super) headers: BTreeMap<String, String>,
    pub(super) file_comments: Vec<String>,
    pub(super) file_extracted_comments: Vec<String>,
    pub(super) messages: Vec<CanonicalMessage>,
    pub(super) diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CanonicalMessage {
    pub(super) msgid: String,
    pub(super) msgctxt: Option<String>,
    pub(super) translation: CanonicalTranslation,
    pub(super) comments: Vec<String>,
    pub(super) origins: PoVec<CatalogOrigin>,
    pub(super) placeholders: BTreeMap<String, Vec<String>>,
    pub(super) obsolete: Option<ObsoleteInfo>,
    pub(super) machine: Option<MachineMetadata>,
}

/// PO metadata key carrying the obsolete-since date (see [`ObsoleteInfo`]).
pub(super) const PO_OBSOLETE_SINCE_KEY: &str = "obsolete-since";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CanonicalTranslation {
    Singular {
        value: String,
    },
    Plural {
        source: PluralSource,
        translation_by_category: BTreeMap<String, String>,
        variable: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedMessage {
    msgid: String,
    msgctxt: Option<String>,
    kind: NormalizedKind,
    comments: Vec<String>,
    origins: PoVec<CatalogOrigin>,
    placeholders: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NormalizedKind {
    Singular,
    Plural {
        source: PluralSource,
        variable: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct ParsedPluralFormsHeader {
    raw: Option<String>,
    nplurals: Option<usize>,
    plural: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MergeCatalogContext<'a> {
    locale: Option<&'a str>,
    source_locale: &'a str,
    semantics: CatalogSemantics,
    overwrite_source_translations: bool,
    obsolete_strategy: ObsoleteStrategy,
    now: Option<&'a str>,
}

/// Merges extracted messages into an existing catalog and returns updated catalog content.
///
/// # Errors
///
/// Returns [`ApiError`] when the source locale is missing, the existing catalog
/// cannot be parsed, or the requested storage format cannot be rendered safely.
///
/// # Examples
///
/// ```rust
/// use ferrocat_po::{
///     CatalogMode, CatalogUpdateInput, SourceExtractedMessage, UpdateCatalogOptions, update_catalog,
/// };
///
/// let result = update_catalog(
///     UpdateCatalogOptions::new(
///         "en",
///         CatalogUpdateInput::SourceFirst(vec![SourceExtractedMessage {
///             msgid: "Checkout".to_owned(),
///             ..SourceExtractedMessage::default()
///         }]),
///     )
///     .with_locale("de"),
/// )?;
///
/// assert!(result.created);
/// assert!(result.content.contains("msgid \"Checkout\""));
/// # Ok::<(), ferrocat_po::ApiError>(())
/// ```
pub fn update_catalog(
    mut options: UpdateCatalogOptions<'_>,
) -> Result<CatalogUpdateResult, ApiError> {
    super::validate_source_locale(options.source_locale)?;

    let input = mem::take(&mut options.input);
    let created = options.existing.is_none();
    let original = options.existing.unwrap_or("");
    let mut existing = match options.existing {
        Some(content) if !content.is_empty() => parse_catalog_to_internal(
            content,
            options.locale,
            options.source_locale,
            options.mode.semantics(),
            options.mode.plural_encoding(),
            false,
            options.mode.storage_format(),
        )?,
        Some(_) | None => Catalog {
            locale: options.locale.map(str::to_owned),
            headers: BTreeMap::new(),
            file_comments: Vec::new(),
            file_extracted_comments: Vec::new(),
            messages: Vec::new(),
            diagnostics: Vec::new(),
        },
    };

    let locale = options
        .locale
        .map(str::to_owned)
        .or_else(|| existing.locale.clone())
        .or_else(|| existing.headers.get("Language").cloned());
    let mut diagnostics = mem::take(&mut existing.diagnostics);
    let normalized = normalize_update_input(input)?;
    let merge_context = MergeCatalogContext {
        locale: locale.as_deref(),
        source_locale: options.source_locale,
        semantics: options.mode.semantics(),
        overwrite_source_translations: options.overwrite_source_translations,
        obsolete_strategy: options.obsolete_strategy.clone(),
        now: options.now,
    };
    let (mut merged, stats) =
        merge_catalogs(existing, normalized, &merge_context, &mut diagnostics);
    merged.locale.clone_from(&locale);
    apply_storage_defaults(&mut merged, &options, locale.as_deref(), &mut diagnostics)?;
    sort_messages(&mut merged.messages, options.render.order_by);
    let content = export_catalog_content(&merged, &options, locale.as_deref(), &mut diagnostics)?;

    Ok(CatalogUpdateResult {
        updated: content != original,
        content,
        created,
        stats,
        diagnostics,
    })
}

/// Updates a catalog on disk and only writes the file when the rendered
/// output changes.
///
/// # Errors
///
/// Returns [`ApiError`] when the input is invalid, when the existing file
/// cannot be read or parsed, or when the updated content cannot be written.
pub fn update_catalog_file(
    options: UpdateCatalogFileOptions<'_>,
) -> Result<CatalogUpdateResult, ApiError> {
    super::validate_source_locale(options.options.source_locale)?;
    if options.target_path.as_os_str().is_empty() {
        return Err(ApiError::InvalidArguments(
            "target_path must not be empty".to_owned(),
        ));
    }

    let existing = match fs::read_to_string(options.target_path) {
        Ok(content) => Some(content),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(ApiError::io_with_path(options.target_path, error)),
    };

    let mut update_options = options.options;
    update_options.existing = existing.as_deref();
    let result = update_catalog(update_options)?;

    if result.created || result.updated {
        atomic_write(options.target_path, &result.content)?;
    }

    Ok(result)
}

/// Parses catalog content into the higher-level representation used by
/// `ferrocat`'s catalog APIs.
///
/// # Errors
///
/// Returns [`ApiError`] when the catalog content cannot be parsed, the source
/// locale is missing, or strict ICU projection fails.
///
/// # Examples
///
/// ```rust
/// use ferrocat_po::{ParseCatalogOptions, parse_catalog};
///
/// let catalog = parse_catalog(
///     ParseCatalogOptions::new("msgid \"Checkout\"\nmsgstr \"Zur Kasse\"\n", "en")
///         .with_locale("de"),
/// )?;
///
/// assert_eq!(catalog.locale.as_deref(), Some("de"));
/// assert_eq!(catalog.messages.len(), 1);
/// # Ok::<(), ferrocat_po::ApiError>(())
/// ```
#[expect(
    clippy::needless_pass_by_value,
    reason = "Public API takes owned option structs so callers can build and move them ergonomically."
)]
pub fn parse_catalog(options: ParseCatalogOptions<'_>) -> Result<ParsedCatalog, ApiError> {
    super::validate_source_locale(options.source_locale)?;
    let catalog = parse_catalog_to_internal(
        options.content,
        options.locale,
        options.source_locale,
        options.mode.semantics(),
        options.mode.plural_encoding(),
        options.strict,
        options.mode.storage_format(),
    )?;
    let messages = catalog
        .messages
        .into_iter()
        .map(public_message_from_canonical)
        .collect();

    Ok(ParsedCatalog {
        locale: catalog.locale,
        semantics: options.mode.semantics(),
        headers: catalog.headers,
        messages,
        diagnostics: catalog.diagnostics,
    })
}

/// Collapses the accepted extractor input shapes into one merge-oriented form.
///
/// The result keeps only the fields that matter for catalog identity and merge
/// semantics, while also projecting source-first ICU plurals into the same
/// structured plural representation used by `CatalogUpdateInput::Structured`.
fn normalize_update_input(input: CatalogUpdateInput) -> Result<Vec<NormalizedMessage>, ApiError> {
    let mut messages = Vec::<NormalizedMessage>::new();

    match input {
        CatalogUpdateInput::Structured(extracted) => {
            for message in extracted {
                let (msgid, msgctxt, kind, comments, origins, placeholders) = match message {
                    ExtractedMessage::Singular(message) => (
                        message.msgid,
                        message.msgctxt,
                        NormalizedKind::Singular,
                        message.comments,
                        message.origin,
                        message.placeholders,
                    ),
                    ExtractedMessage::Plural(message) => (
                        message.msgid,
                        message.msgctxt,
                        NormalizedKind::Plural {
                            source: message.source,
                            variable: None,
                        },
                        message.comments,
                        message.origin,
                        message.placeholders,
                    ),
                };

                messages.push(NormalizedMessage {
                    msgid,
                    msgctxt,
                    kind,
                    comments: dedupe_strings(comments),
                    origins: dedupe_origins(origins),
                    placeholders: dedupe_placeholders(placeholders),
                });
            }
        }
        CatalogUpdateInput::SourceFirst(source_messages) => {
            for message in source_messages {
                messages.push(NormalizedMessage {
                    msgid: message.msgid,
                    msgctxt: message.msgctxt,
                    kind: NormalizedKind::Singular,
                    comments: dedupe_strings(message.comments),
                    origins: dedupe_origins(message.origin),
                    placeholders: dedupe_placeholders(message.placeholders),
                });
            }
        }
    }

    merge_normalized_messages(messages)
}

/// Merges duplicate extractor entries that refer to the same gettext identity.
///
/// Duplicate singular/plural shape mismatches remain a hard error because they
/// would otherwise make the final catalog shape ambiguous.
fn merge_normalized_messages(
    messages: Vec<NormalizedMessage>,
) -> Result<Vec<NormalizedMessage>, ApiError> {
    let mut index = FxHashMap::<(&str, Option<&str>), usize>::with_capacity_and_hasher(
        messages.len(),
        Default::default(),
    );
    let mut selected_indexes = Vec::with_capacity(messages.len());
    let mut duplicate_sources = vec![None; messages.len()];

    for (message_index, message) in messages.iter().enumerate() {
        if message.msgid.is_empty() {
            return Err(ApiError::InvalidArguments(
                "extracted msgid must not be empty".to_owned(),
            ));
        }

        let key = (message.msgid.as_str(), message.msgctxt.as_deref());
        if let Some(existing_index) = index.get(&key).copied() {
            let existing = &messages[existing_index];
            if existing.kind != message.kind {
                return Err(ApiError::Conflict(format!(
                    "conflicting duplicate extracted message for msgid {:?}",
                    message.msgid
                )));
            }
            duplicate_sources[message_index] = Some(existing_index);
        } else {
            index.insert(key, message_index);
            selected_indexes.push(message_index);
        }
    }

    let mut output_index_by_input = vec![None; messages.len()];
    let mut out = Vec::with_capacity(selected_indexes.len());
    let mut selected_indexes = selected_indexes.into_iter();
    let mut next_selected = selected_indexes.next();

    for (message_index, message) in messages.into_iter().enumerate() {
        if next_selected == Some(message_index) {
            output_index_by_input[message_index] = Some(out.len());
            out.push(message);
            next_selected = selected_indexes.next();
            continue;
        }

        if let Some(existing_input_index) = duplicate_sources[message_index] {
            let existing_output_index = output_index_by_input[existing_input_index]
                .expect("duplicate source should already be emitted");
            let existing = &mut out[existing_output_index];
            merge_unique_strings(&mut existing.comments, message.comments);
            merge_unique_origins(&mut existing.origins, message.origins);
            merge_placeholders(&mut existing.placeholders, message.placeholders);
        }
    }

    Ok(out)
}

/// Applies extracted messages onto an existing canonical catalog and records the
/// coarse-grained update counters used by the high-level API.
fn merge_catalogs(
    existing: Catalog,
    normalized: Vec<NormalizedMessage>,
    context: &MergeCatalogContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> (Catalog, CatalogStats) {
    let is_source_locale = context
        .locale
        .is_none_or(|value| value == context.source_locale);
    let mut stats = CatalogStats::default();

    let mut matched = vec![false; existing.messages.len()];
    let mut messages = Vec::with_capacity(normalized.len() + existing.messages.len());

    // `locale` and `semantics` are constant for the whole merge, so build the
    // plural profile once instead of per plural message.
    let plural_profile = if context.semantics == CatalogSemantics::GettextCompat {
        PluralProfile::for_gettext_locale(context.locale)
    } else {
        PluralProfile::for_locale(context.locale)
    };

    {
        let mut existing_index = FxHashMap::<(&str, Option<&str>), usize>::with_capacity_and_hasher(
            existing.messages.len(),
            Default::default(),
        );
        for (index, message) in existing.messages.iter().enumerate() {
            existing_index.insert((message.msgid.as_str(), message.msgctxt.as_deref()), index);
        }

        for next in normalized {
            let previous = if let Some(&index) =
                existing_index.get(&(next.msgid.as_str(), next.msgctxt.as_deref()))
            {
                matched[index] = true;
                Some(&existing.messages[index])
            } else {
                None
            };
            let merged = merge_message(
                previous,
                next,
                is_source_locale,
                &plural_profile,
                context.overwrite_source_translations,
                diagnostics,
            );
            if previous.is_none() {
                stats.added += 1;
            } else if previous == Some(&merged) {
                stats.unchanged += 1;
            } else {
                stats.changed += 1;
            }
            messages.push(merged);
        }
    }

    for (index, message) in existing.messages.into_iter().enumerate() {
        if matched[index] {
            continue;
        }
        match &context.obsolete_strategy {
            ObsoleteStrategy::Delete => {
                stats.obsolete_removed += 1;
            }
            ObsoleteStrategy::Mark => {
                let mut message = message;
                mark_obsolete(&mut message, context.now, &mut stats);
                messages.push(message);
            }
            ObsoleteStrategy::Keep => {
                let mut message = message;
                message.obsolete = None;
                messages.push(message);
            }
            ObsoleteStrategy::DropObsoleteBefore(cutoff) => {
                let mut message = message;
                mark_obsolete(&mut message, context.now, &mut stats);
                // Age-based GC: drop dated entries older than the cutoff; keep
                // undated ones since their age is unknown.
                if let Some(ObsoleteInfo { since: Some(since) }) = &message.obsolete
                    && since.as_str() < cutoff.as_str()
                {
                    stats.obsolete_removed += 1;
                    continue;
                }
                messages.push(message);
            }
        }
    }

    stats.total = messages.len();
    (
        Catalog {
            locale: existing.locale,
            headers: existing.headers,
            file_comments: existing.file_comments,
            file_extracted_comments: existing.file_extracted_comments,
            messages,
            diagnostics: existing.diagnostics,
        },
        stats,
    )
}

/// Marks a newly-missing message obsolete, stamping the host-provided `now` as
/// the write-once `since` date. Already-obsolete messages keep their date.
fn mark_obsolete(message: &mut CanonicalMessage, now: Option<&str>, stats: &mut CatalogStats) {
    if message.obsolete.is_none() {
        message.obsolete = Some(ObsoleteInfo {
            since: now.map(str::to_owned),
        });
        stats.obsolete_marked += 1;
    }
}

/// Resolves the final canonical message for one gettext identity.
///
/// This is the central place where source-locale overwrite rules, plural
/// variable inference, and locale-aware plural category materialization meet.
fn merge_message(
    previous: Option<&CanonicalMessage>,
    next: NormalizedMessage,
    is_source_locale: bool,
    plural_profile: &PluralProfile,
    overwrite_source_translations: bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> CanonicalMessage {
    let NormalizedMessage {
        msgid,
        msgctxt,
        kind,
        comments,
        origins,
        placeholders,
    } = next;

    let translation = match (kind, previous) {
        (NormalizedKind::Singular, Some(previous))
            if matches!(previous.translation, CanonicalTranslation::Singular { .. })
                && !(is_source_locale && overwrite_source_translations) =>
        {
            previous.translation.clone()
        }
        (NormalizedKind::Singular, _) => CanonicalTranslation::Singular {
            value: if is_source_locale {
                msgid.clone()
            } else {
                String::new()
            },
        },
        // Reuse an existing plural translation when the source plural is
        // unchanged and we're not overwriting source-locale text. Binding the
        // inner `Plural` here means a non-plural previous simply falls through
        // to the rebuild arm below.
        (
            NormalizedKind::Plural { source, variable },
            Some(CanonicalMessage {
                translation:
                    CanonicalTranslation::Plural {
                        translation_by_category,
                        variable: previous_variable,
                        ..
                    },
                ..
            }),
        ) if !(is_source_locale && overwrite_source_translations) => CanonicalTranslation::Plural {
            source,
            translation_by_category: plural_profile
                .materialize_translation(translation_by_category),
            variable: variable.unwrap_or_else(|| previous_variable.clone()),
        },
        (NormalizedKind::Plural { source, variable }, previous) => {
            let variable = variable
                .or_else(|| previous.and_then(extract_plural_variable))
                .or_else(|| derive_plural_variable(&placeholders))
                .unwrap_or_else(|| {
                    diagnostics.push(
                        Diagnostic::new(
                            DiagnosticSeverity::Warning,
                            diagnostic_codes::plural::ASSUMED_VARIABLE,
                            "Unable to determine plural placeholder name, assuming \"count\".",
                        )
                        .with_identity(&msgid, msgctxt.as_deref()),
                    );
                    "count".to_owned()
                });

            let translation_by_category = if is_source_locale {
                plural_profile.source_locale_translation(&source)
            } else {
                plural_profile.empty_translation()
            };

            CanonicalTranslation::Plural {
                source,
                translation_by_category,
                variable,
            }
        }
    };

    let (machine, obsolete) =
        previous.map_or_else(|| (None, None), |message| (message.machine.clone(), None));

    CanonicalMessage {
        msgid,
        msgctxt,
        translation,
        comments,
        origins,
        placeholders,
        obsolete,
        machine,
    }
}

fn extract_plural_variable(message: &CanonicalMessage) -> Option<String> {
    match &message.translation {
        CanonicalTranslation::Plural { variable, .. } => Some(variable.clone()),
        CanonicalTranslation::Singular { .. } => None,
    }
}

/// Fills in the standard catalog headers and only synthesizes `Plural-Forms`
/// when we have a conservative, locale-safe default.
pub(super) fn apply_header_defaults(
    headers: &mut BTreeMap<String, String>,
    locale: Option<&str>,
    semantics: CatalogSemantics,
    diagnostics: &mut Vec<Diagnostic>,
    custom: &BTreeMap<String, String>,
) {
    headers
        .entry("MIME-Version".to_owned())
        .or_insert_with(|| "1.0".to_owned());
    headers
        .entry("Content-Type".to_owned())
        .or_insert_with(|| "text/plain; charset=utf-8".to_owned());
    headers
        .entry("Content-Transfer-Encoding".to_owned())
        .or_insert_with(|| "8bit".to_owned());
    headers
        .entry("X-Generator".to_owned())
        .or_insert_with(|| "ferrocat".to_owned());
    if let Some(locale) = locale {
        headers.insert("Language".to_owned(), locale.to_owned());
    }
    if semantics == CatalogSemantics::GettextCompat && !custom.contains_key("Plural-Forms") {
        let profile = PluralProfile::for_gettext_locale(locale);
        let parsed_header = parse_plural_forms_from_headers(headers);
        match (parsed_header.raw.as_deref(), profile.gettext_header()) {
            (None, Some(header)) => {
                headers.insert("Plural-Forms".to_owned(), header);
            }
            (None, None) => diagnostics.push(Diagnostic::new(
                DiagnosticSeverity::Info,
                diagnostic_codes::plural::MISSING_PLURAL_FORMS_HEADER,
                "No safe default Plural-Forms header is known for this locale; keeping the header unset.",
            )),
            (Some(_), Some(header))
                if parsed_header.nplurals == Some(profile.nplurals())
                    && parsed_header.plural.is_none() =>
            {
                headers.insert("Plural-Forms".to_owned(), header);
                diagnostics.push(Diagnostic::new(
                    DiagnosticSeverity::Info,
                    diagnostic_codes::plural::COMPLETED_PLURAL_FORMS_HEADER,
                    "Plural-Forms header was missing the plural expression and has been completed using a safe locale default.",
                ));
            }
            _ => {}
        }
    }
    for (key, value) in custom {
        headers.insert(key.clone(), value.clone());
    }
}

pub(super) fn sort_messages(messages: &mut [CanonicalMessage], order_by: OrderBy) {
    match order_by {
        OrderBy::Msgid => messages.sort_by(|left, right| {
            left.msgid
                .cmp(&right.msgid)
                .then_with(|| left.msgctxt.cmp(&right.msgctxt))
                .then_with(|| left.obsolete.cmp(&right.obsolete))
        }),
        OrderBy::Origin => messages.sort_unstable_by(|left, right| {
            first_origin_sort_key(&left.origins)
                .cmp(first_origin_sort_key(&right.origins))
                .then_with(|| left.msgid.cmp(&right.msgid))
                .then_with(|| left.msgctxt.cmp(&right.msgctxt))
                .then_with(|| left.obsolete.cmp(&right.obsolete))
        }),
    }
}

fn first_origin_sort_key(origins: &[CatalogOrigin]) -> &str {
    origins.first().map_or("", |origin| origin.file.as_str())
}

fn apply_storage_defaults(
    catalog: &mut Catalog,
    options: &UpdateCatalogOptions<'_>,
    locale: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), ApiError> {
    match options.mode.storage_format() {
        CatalogStorageFormat::Po => {
            let empty_custom_headers = BTreeMap::new();
            apply_header_defaults(
                &mut catalog.headers,
                locale,
                options.mode.semantics(),
                diagnostics,
                options
                    .render
                    .custom_header_attributes
                    .unwrap_or(&empty_custom_headers),
            );
            Ok(())
        }
        CatalogStorageFormat::Fcl => {
            if options
                .render
                .custom_header_attributes
                .is_some_and(|headers| !headers.is_empty())
            {
                return Err(ApiError::Unsupported(
                    "custom_header_attributes are not supported for FCL catalogs".to_owned(),
                ));
            }
            catalog.headers.clear();
            Ok(())
        }
    }
}

/// Parses catalog text into the canonical internal catalog representation used by
/// both `parse_catalog` and `update_catalog`.
///
/// Keeping this internal representation stable lets the public APIs share one
/// import path before they diverge into normalized lookup or update/export work.
pub(super) fn parse_catalog_to_internal(
    content: &str,
    locale_override: Option<&str>,
    source_locale: &str,
    semantics: CatalogSemantics,
    plural_encoding: PluralEncoding,
    strict: bool,
    storage_format: CatalogStorageFormat,
) -> Result<Catalog, ApiError> {
    match storage_format {
        CatalogStorageFormat::Po => parse_catalog_to_internal_po(
            content,
            locale_override,
            semantics,
            plural_encoding,
            strict,
        ),
        CatalogStorageFormat::Fcl => super::fcl::parse_catalog_to_internal_fcl(
            content,
            locale_override,
            source_locale,
            semantics,
            strict,
        ),
    }
}

fn parse_catalog_to_internal_po(
    content: &str,
    locale_override: Option<&str>,
    semantics: CatalogSemantics,
    _plural_encoding: PluralEncoding,
    strict: bool,
) -> Result<Catalog, ApiError> {
    let PoFile {
        headers: po_headers,
        items: po_items,
        comments: po_comments,
        extracted_comments: po_extracted_comments,
    } = parse_po(content)?;
    let headers = po_headers
        .into_iter()
        .map(|header| (header.key, header.value))
        .collect::<BTreeMap<_, _>>();
    let locale = locale_override
        .map(str::to_owned)
        .or_else(|| headers.get("Language").cloned());
    let plural_forms = parse_plural_forms_from_headers(&headers);
    let nplurals = plural_forms.nplurals;
    let mut diagnostics = Vec::new();
    validate_plural_forms_header(
        locale.as_deref(),
        &plural_forms,
        semantics,
        &mut diagnostics,
    );
    let mut messages = Vec::with_capacity(po_items.len());

    for item in po_items {
        let mut conversion_diagnostics = Vec::new();
        let message = import_message_from_po(
            item,
            locale.as_deref(),
            nplurals,
            semantics,
            strict,
            &mut conversion_diagnostics,
        )?;
        diagnostics.extend(conversion_diagnostics);
        messages.push(message);
    }

    Ok(Catalog {
        locale,
        headers,
        file_comments: po_comments,
        file_extracted_comments: po_extracted_comments,
        messages,
        diagnostics,
    })
}

/// Converts one parsed `PoItem` into the canonical internal message form.
///
/// The branching is intentionally centralized here so that gettext plural slot
/// import, ICU projection, and all associated diagnostics stay in one semantic
/// decision point.
fn import_message_from_po(
    item: PoItem,
    locale: Option<&str>,
    nplurals: Option<usize>,
    semantics: CatalogSemantics,
    _strict: bool,
    _diagnostics: &mut Vec<Diagnostic>,
) -> Result<CanonicalMessage, ApiError> {
    // Extracted (`#.`) and translator (`#`) comments collapse into one notes list.
    let (mut comments, placeholders) =
        split_placeholder_comments(item.extracted_comments.into_vec());
    comments.extend(item.comments.into_vec());
    let origins = item
        .references
        .into_iter()
        .map(parse_origin_owned)
        .collect();

    let translation = if let Some(msgid_plural) = &item.msgid_plural {
        if semantics == CatalogSemantics::IcuNative {
            return Err(ApiError::Unsupported(
                "classic gettext plural requires compat mode".to_owned(),
            ));
        }
        let plural_profile =
            PluralProfile::for_gettext_slots(locale, nplurals.or(Some(item.msgstr.len())));
        CanonicalTranslation::Plural {
            source: PluralSource {
                one: Some(item.msgid.clone()),
                other: msgid_plural.clone(),
            },
            translation_by_category: plural_profile
                .categories()
                .iter()
                .zip(item.msgstr.iter().chain(std::iter::repeat("")))
                .map(|(category, value)| (category.clone(), value.to_owned()))
                .collect(),
            variable: "count".to_owned(),
        }
    } else {
        if semantics == CatalogSemantics::IcuNative && matches!(item.msgstr, MsgStr::Plural(_)) {
            return Err(ApiError::Unsupported(
                "classic gettext plural requires compat mode".to_owned(),
            ));
        }
        CanonicalTranslation::Singular {
            value: take_first_msgstr(item.msgstr),
        }
    };

    Ok(CanonicalMessage {
        msgid: item.msgid,
        msgctxt: item.msgctxt,
        translation,
        comments,
        origins,
        placeholders,
        obsolete: import_obsolete(item.obsolete, &item.metadata),
        machine: import_machine_metadata(&item.metadata)?,
    })
}

/// Builds the obsolete state from the low-level obsolete flag, reading the
/// optional `#@ obsolete-since:` date from the entry's metadata.
fn import_obsolete(obsolete: bool, metadata: &[(String, String)]) -> Option<ObsoleteInfo> {
    if !obsolete {
        return None;
    }
    let since = metadata
        .iter()
        .find(|(key, _)| key == PO_OBSOLETE_SINCE_KEY)
        .map(|(_, value)| value.clone());
    Some(ObsoleteInfo { since })
}

fn import_machine_metadata(
    metadata: &[(String, String)],
) -> Result<Option<MachineMetadata>, ApiError> {
    let mut lock = None;
    let mut ai = None;
    for (key, value) in metadata {
        if key == PO_LOCK_KEY {
            if lock.replace(value).is_some() {
                return Err(ApiError::InvalidArguments(
                    "duplicate `lock` metadata for PO item".to_owned(),
                ));
            }
        } else if key == PO_AI_KEY && ai.replace(value).is_some() {
            return Err(ApiError::InvalidArguments(
                "duplicate `ai` metadata for PO item".to_owned(),
            ));
        }
    }

    let Some(lock) = lock else {
        if ai.is_some() {
            return Err(ApiError::InvalidArguments(
                "PO `ai` metadata requires a `lock`".to_owned(),
            ));
        }
        return Ok(None);
    };

    let metadata = MachineMetadata {
        lock: lock.clone(),
        ai: ai.map(|value| parse_ai_descriptor(value)),
    };
    validate_machine_metadata(&metadata)?;
    Ok(Some(metadata))
}

/// Splits extractor-style placeholder comments back out of the generic
/// extracted-comment list during PO import.
pub(super) fn split_placeholder_comments(
    extracted_comments: Vec<String>,
) -> (Vec<String>, BTreeMap<String, Vec<String>>) {
    let mut comments = Vec::new();
    let mut placeholders = BTreeMap::<String, Vec<String>>::new();

    for comment in extracted_comments {
        if let Some((name, value)) = parse_placeholder_comment(&comment) {
            placeholders.entry(name).or_default().push(value);
        } else {
            comments.push(comment);
        }
    }

    (comments, dedupe_placeholders(placeholders))
}

/// Parses the internal placeholder comment format emitted by the export helpers.
fn parse_placeholder_comment(comment: &str) -> Option<(String, String)> {
    let rest = comment.strip_prefix("placeholder {")?;
    let end = rest.find("}: ")?;
    Some((rest[..end].to_owned(), rest[end + 3..].to_owned()))
}

/// Parses a reference into a [`CatalogOrigin`]. A trailing `#scope` anchor (the
/// stable enclosing component or function) becomes [`CatalogOrigin::scope`], and
/// a legacy trailing `:line` (as emitted by gettext tools) is stripped so line
/// numbers never enter the catalog model and cannot round-trip into output.
pub(super) fn parse_origin_owned(mut reference: String) -> CatalogOrigin {
    // Anchor first: split on the last `#`, so a `#` earlier in the path stays
    // with the file.
    let scope = match reference.rsplit_once('#') {
        Some((file, scope)) if !scope.is_empty() => {
            let scope = scope.to_owned();
            let file_len = file.len();
            reference.truncate(file_len);
            Some(scope)
        }
        _ => None,
    };

    if let Some((file, line)) = reference.rsplit_once(':')
        && !line.is_empty()
        && line.bytes().all(|byte| byte.is_ascii_digit())
    {
        let file_len = file.len();
        reference.truncate(file_len);
    }

    CatalogOrigin {
        file: reference,
        scope,
    }
}

/// Serializes a [`CatalogOrigin`] to its wire form, shared by PO references and
/// FCL `r=` tags: `file` or `file#scope`.
pub(super) fn format_origin(origin: &CatalogOrigin) -> String {
    match &origin.scope {
        Some(scope) => format!("{}#{scope}", origin.file),
        None => origin.file.clone(),
    }
}

/// Moves the first available translation string out of a [`MsgStr`], matching
/// the `first().unwrap_or_default()` semantics without copying.
fn take_first_msgstr(msgstr: MsgStr) -> String {
    match msgstr {
        MsgStr::Singular(value) => value,
        MsgStr::Plural(values) => values.into_iter().next().unwrap_or_default(),
        MsgStr::None => String::new(),
    }
}

/// Extracts the small `Plural-Forms` subset that Ferrocat needs for diagnostics
/// and gettext-slot interpretation.
fn parse_plural_forms_from_headers(headers: &BTreeMap<String, String>) -> ParsedPluralFormsHeader {
    let Some(plural_forms) = headers.get("Plural-Forms") else {
        return ParsedPluralFormsHeader::default();
    };

    let mut parsed = ParsedPluralFormsHeader {
        raw: Some(plural_forms.clone()),
        ..ParsedPluralFormsHeader::default()
    };
    for part in plural_forms.split(';') {
        let trimmed = part.trim();
        if let Some(value) = trimmed.strip_prefix("nplurals=") {
            parsed.nplurals = value.trim().parse().ok();
        } else if let Some(value) = trimmed.strip_prefix("plural=") {
            let value = value.trim();
            if !value.is_empty() {
                parsed.plural = Some(value.to_owned());
            }
        }
    }

    parsed
}

/// Validates only the invariants that materially affect Ferrocat's plural
/// interpretation, keeping the diagnostics focused on actionable mismatches.
fn validate_plural_forms_header(
    locale: Option<&str>,
    plural_forms: &ParsedPluralFormsHeader,
    semantics: CatalogSemantics,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if semantics != CatalogSemantics::GettextCompat {
        return;
    }

    if let Some(nplurals) = plural_forms.nplurals {
        match expected_gettext_nplurals_for_locale(locale) {
            Some(expected) if nplurals != expected => diagnostics.push(Diagnostic::new(
                DiagnosticSeverity::Warning,
                diagnostic_codes::plural::NPLURALS_LOCALE_MISMATCH,
                format!(
                    "Plural-Forms declares nplurals={nplurals}, but locale-derived categories expect {expected}."
                ),
            )),
            _ => {}
        }
    } else if plural_forms.plural.is_some() {
        diagnostics.push(Diagnostic::new(
            DiagnosticSeverity::Warning,
            diagnostic_codes::parse::INVALID_PLURAL_FORMS_HEADER,
            "Plural-Forms header contains a plural expression but no parseable nplurals value.",
        ));
    }

    if plural_forms.nplurals.is_some() && plural_forms.plural.is_none() {
        diagnostics.push(Diagnostic::new(
            DiagnosticSeverity::Info,
            diagnostic_codes::plural::MISSING_PLURAL_EXPRESSION,
            "Plural-Forms header declares nplurals but omits the plural expression.",
        ));
    }
}

/// Rebuilds the public `CatalogMessage` shape from the canonical internal form.
pub(super) fn public_message_from_canonical(message: CanonicalMessage) -> CatalogMessage {
    let translation = match message.translation {
        CanonicalTranslation::Singular { value } => TranslationShape::Singular { value },
        CanonicalTranslation::Plural {
            source,
            translation_by_category,
            variable,
            ..
        } => TranslationShape::Plural {
            source,
            translation: translation_by_category,
            variable,
        },
    };

    CatalogMessage {
        msgid: message.msgid,
        msgctxt: message.msgctxt,
        translation,
        comments: message.comments,
        origin: message.origins,
        obsolete: message.obsolete,
        machine: message.machine,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CanonicalMessage, CanonicalTranslation, first_origin_sort_key, parse_origin_owned,
        sort_messages, take_first_msgstr,
    };
    use crate::api::{CatalogOrigin, OrderBy};
    use crate::{MsgStr, PoVec};

    #[test]
    fn parse_origin_owned_strips_line_numbers_and_keeps_file() {
        // A trailing numeric `:line` is dropped so line numbers never enter the
        // catalog model or round-trip back into rendered references.
        assert_eq!(
            parse_origin_owned("src/app.rs:42".to_owned()).file,
            "src/app.rs"
        );

        // No colon, and non-numeric or empty suffixes, keep the buffer verbatim.
        assert_eq!(parse_origin_owned("README".to_owned()).file, "README");
        assert_eq!(parse_origin_owned("a:b:rev".to_owned()).file, "a:b:rev");
    }

    #[test]
    fn take_first_msgstr_moves_first_available_value() {
        assert_eq!(take_first_msgstr(MsgStr::Singular("one".to_owned())), "one");
        assert_eq!(
            take_first_msgstr(MsgStr::Plural(vec!["a".to_owned(), "b".to_owned()])),
            "a"
        );
        assert_eq!(take_first_msgstr(MsgStr::Plural(Vec::new())), "");
        assert_eq!(take_first_msgstr(MsgStr::None), "");
    }

    #[test]
    fn first_origin_sort_key_borrows_origin_file() {
        let origins = PoVec::from(vec![CatalogOrigin {
            file: "src/app.rs".to_owned(),
            scope: None,
        }]);

        assert_eq!(first_origin_sort_key(&origins), "src/app.rs");
    }

    #[test]
    fn sort_messages_by_origin_uses_msgid_and_context_tiebreakers() {
        let mut messages = vec![
            test_message("src/app.rs", "beta", Some("dialog")),
            test_message("src/app.rs", "alpha", Some("dialog")),
            test_message("src/app.rs", "alpha", None),
            test_message("src/lib.rs", "alpha", None),
        ];

        sort_messages(&mut messages, OrderBy::Origin);

        assert_eq!(
            messages
                .iter()
                .map(|message| {
                    (
                        message.origins[0].file.as_str(),
                        message.msgid.as_str(),
                        message.msgctxt.as_deref(),
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                ("src/app.rs", "alpha", None),
                ("src/app.rs", "alpha", Some("dialog")),
                ("src/app.rs", "beta", Some("dialog")),
                ("src/lib.rs", "alpha", None)
            ]
        );
    }

    fn test_message(file: &str, msgid: &str, msgctxt: Option<&str>) -> CanonicalMessage {
        CanonicalMessage {
            msgid: msgid.to_owned(),
            msgctxt: msgctxt.map(str::to_owned),
            translation: CanonicalTranslation::Singular {
                value: String::new(),
            },
            comments: Vec::new(),
            origins: PoVec::from(vec![CatalogOrigin {
                file: file.to_owned(),
                scope: None,
            }]),
            placeholders: Default::default(),
            obsolete: None,
            machine: None,
        }
    }
}
