//! Internal catalog pipeline for the public `ferrocat-po` catalog API.
//!
//! This module owns the higher-level workflow around PO parsing, extracted-message
//! normalization, merge semantics, and export back to PO. The byte-oriented parser
//! and serializer hot paths stay elsewhere; this layer is where we preserve
//! catalog semantics and diagnostics.

use std::collections::BTreeMap;
use std::fs;

use crate::diagnostic_codes;

use super::export::export_catalog_content;
use super::file_io::atomic_write;
use super::helpers::{
    dedupe_origins, dedupe_placeholders, dedupe_strings, merge_placeholders, merge_unique_origins,
    merge_unique_strings,
};
use super::mt::{
    MachineTranslationMetadata, PO_MACHINE_TRANSLATION_KEY, parse_po_machine_translation_metadata,
};
use super::ndjson::parse_catalog_to_internal_ndjson;
use super::plural::{PluralProfile, derive_plural_variable, expected_gettext_nplurals_for_locale};
use super::{
    ApiError, CatalogMessage, CatalogMessageExtra, CatalogOrigin, CatalogSemantics, CatalogStats,
    CatalogStorageFormat, CatalogUpdateInput, CatalogUpdateResult, Diagnostic, DiagnosticSeverity,
    ExtractedMessage, ObsoleteStrategy, OrderBy, ParseCatalogOptions, ParsedCatalog,
    PluralEncoding, PluralSource, TranslationShape, UpdateCatalogFileOptions, UpdateCatalogOptions,
};
use crate::{MsgStr, PoFile, PoItem, parse_po};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct Catalog {
    pub(super) locale: Option<String>,
    pub(super) headers: BTreeMap<String, String>,
    pub(super) file_comments: Vec<String>,
    pub(super) file_extracted_comments: Vec<String>,
    pub(super) messages: Vec<CanonicalMessage>,
    pub(super) diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CanonicalMessage {
    pub(super) msgid: String,
    pub(super) msgctxt: Option<String>,
    pub(super) translation: CanonicalTranslation,
    pub(super) comments: Vec<String>,
    pub(super) origins: Vec<CatalogOrigin>,
    pub(super) placeholders: BTreeMap<String, Vec<String>>,
    pub(super) obsolete: bool,
    pub(super) machine_translation: Option<MachineTranslationMetadata>,
    pub(super) translator_comments: Vec<String>,
    pub(super) flags: Vec<String>,
}

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
    origins: Vec<CatalogOrigin>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MergeCatalogContext<'a> {
    locale: Option<&'a str>,
    source_locale: &'a str,
    semantics: CatalogSemantics,
    overwrite_source_translations: bool,
    obsolete_strategy: ObsoleteStrategy,
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
/// let result = update_catalog(UpdateCatalogOptions {
///     locale: Some("de"),
///     input: CatalogUpdateInput::SourceFirst(vec![SourceExtractedMessage {
///         msgid: "Checkout".to_owned(),
///         ..SourceExtractedMessage::default()
///     }]),
///     ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
/// })?;
///
/// assert!(result.created);
/// assert!(result.content.contains("msgid \"Checkout\""));
/// # Ok::<(), ferrocat_po::ApiError>(())
/// ```
#[expect(
    clippy::needless_pass_by_value,
    reason = "Public API takes owned option structs so callers can build and move them ergonomically."
)]
pub fn update_catalog(options: UpdateCatalogOptions<'_>) -> Result<CatalogUpdateResult, ApiError> {
    super::validate_source_locale(options.source_locale)?;

    let created = options.existing.is_none();
    let original = options.existing.unwrap_or("");
    let existing = match options.existing {
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
    let mut diagnostics = existing.diagnostics.clone();
    let normalized = normalize_update_input(&options.input)?;
    let merge_context = MergeCatalogContext {
        locale: locale.as_deref(),
        source_locale: options.source_locale,
        semantics: options.mode.semantics(),
        overwrite_source_translations: options.overwrite_source_translations,
        obsolete_strategy: options.obsolete_strategy,
    };
    let (mut merged, stats) =
        merge_catalogs(existing, &normalized, merge_context, &mut diagnostics);
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
/// let catalog = parse_catalog(ParseCatalogOptions {
///     locale: Some("de"),
///     ..ParseCatalogOptions::new("msgid \"Checkout\"\nmsgstr \"Zur Kasse\"\n", "en")
/// })?;
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
fn normalize_update_input(input: &CatalogUpdateInput) -> Result<Vec<NormalizedMessage>, ApiError> {
    let mut index = BTreeMap::<(String, Option<String>), usize>::new();
    let mut normalized = Vec::<NormalizedMessage>::new();

    match input {
        CatalogUpdateInput::Structured(extracted) => {
            for message in extracted {
                let (msgid, msgctxt, kind, comments, origins, placeholders) = match message {
                    ExtractedMessage::Singular(message) => (
                        message.msgid.clone(),
                        message.msgctxt.clone(),
                        NormalizedKind::Singular,
                        message.comments.clone(),
                        message.origin.clone(),
                        message.placeholders.clone(),
                    ),
                    ExtractedMessage::Plural(message) => (
                        message.msgid.clone(),
                        message.msgctxt.clone(),
                        NormalizedKind::Plural {
                            source: message.source.clone(),
                            variable: None,
                        },
                        message.comments.clone(),
                        message.origin.clone(),
                        message.placeholders.clone(),
                    ),
                };

                push_normalized_message(
                    &mut index,
                    &mut normalized,
                    NormalizedMessage {
                        msgid,
                        msgctxt,
                        kind,
                        comments: dedupe_strings(comments),
                        origins: dedupe_origins(origins),
                        placeholders: dedupe_placeholders(placeholders),
                    },
                )?;
            }
        }
        CatalogUpdateInput::SourceFirst(messages) => {
            for message in messages {
                push_normalized_message(
                    &mut index,
                    &mut normalized,
                    NormalizedMessage {
                        msgid: message.msgid.clone(),
                        msgctxt: message.msgctxt.clone(),
                        kind: NormalizedKind::Singular,
                        comments: dedupe_strings(message.comments.clone()),
                        origins: dedupe_origins(message.origin.clone()),
                        placeholders: dedupe_placeholders(message.placeholders.clone()),
                    },
                )?;
            }
        }
    }

    Ok(normalized)
}

/// Inserts one normalized message, merging duplicate extractor entries that
/// refer to the same gettext identity.
///
/// Duplicate singular/plural shape mismatches remain a hard error because they
/// would otherwise make the final catalog shape ambiguous.
fn push_normalized_message(
    index: &mut BTreeMap<(String, Option<String>), usize>,
    normalized: &mut Vec<NormalizedMessage>,
    message: NormalizedMessage,
) -> Result<(), ApiError> {
    let msgid = message.msgid.clone();
    let msgctxt = message.msgctxt.clone();
    if msgid.is_empty() {
        return Err(ApiError::InvalidArguments(
            "extracted msgid must not be empty".to_owned(),
        ));
    }

    let key = (msgid.clone(), msgctxt);
    if let Some(existing_index) = index.get(&key).copied() {
        let existing = &mut normalized[existing_index];
        if existing.kind != message.kind {
            return Err(ApiError::Conflict(format!(
                "conflicting duplicate extracted message for msgid {msgid:?}"
            )));
        }
        merge_unique_strings(&mut existing.comments, message.comments);
        merge_unique_origins(&mut existing.origins, message.origins);
        merge_placeholders(&mut existing.placeholders, message.placeholders);
    } else {
        index.insert(key, normalized.len());
        normalized.push(message);
    }

    Ok(())
}

/// Applies extracted messages onto an existing canonical catalog and records the
/// coarse-grained update counters used by the high-level API.
fn merge_catalogs(
    existing: Catalog,
    normalized: &[NormalizedMessage],
    context: MergeCatalogContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> (Catalog, CatalogStats) {
    let is_source_locale = context
        .locale
        .is_none_or(|value| value == context.source_locale);
    let mut stats = CatalogStats::default();

    let mut existing_index = BTreeMap::<(String, Option<String>), usize>::new();
    for (index, message) in existing.messages.iter().enumerate() {
        existing_index.insert((message.msgid.clone(), message.msgctxt.clone()), index);
    }

    let mut matched = vec![false; existing.messages.len()];
    let mut messages = Vec::with_capacity(normalized.len() + existing.messages.len());

    // `locale` and `semantics` are constant for the whole merge, so build the
    // plural profile once instead of per plural message.
    let plural_profile = if context.semantics == CatalogSemantics::GettextCompat {
        PluralProfile::for_gettext_locale(context.locale)
    } else {
        PluralProfile::for_locale(context.locale)
    };

    for next in normalized {
        let key = (next.msgid.clone(), next.msgctxt.clone());
        let previous = if let Some(&index) = existing_index.get(&key) {
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

    for (index, message) in existing.messages.into_iter().enumerate() {
        if matched[index] {
            continue;
        }
        match context.obsolete_strategy {
            ObsoleteStrategy::Delete => {
                stats.obsolete_removed += 1;
            }
            ObsoleteStrategy::Mark => {
                let mut message = message;
                if !message.obsolete {
                    message.obsolete = true;
                    stats.obsolete_marked += 1;
                }
                messages.push(message);
            }
            ObsoleteStrategy::Keep => {
                let mut message = message;
                message.obsolete = false;
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

/// Resolves the final canonical message for one gettext identity.
///
/// This is the central place where source-locale overwrite rules, plural
/// variable inference, and locale-aware plural category materialization meet.
fn merge_message(
    previous: Option<&CanonicalMessage>,
    next: &NormalizedMessage,
    is_source_locale: bool,
    plural_profile: &PluralProfile,
    overwrite_source_translations: bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> CanonicalMessage {
    let translation = match (&next.kind, previous) {
        (NormalizedKind::Singular, Some(previous))
            if matches!(previous.translation, CanonicalTranslation::Singular { .. })
                && !(is_source_locale && overwrite_source_translations) =>
        {
            previous.translation.clone()
        }
        (NormalizedKind::Singular, _) => CanonicalTranslation::Singular {
            value: if is_source_locale {
                next.msgid.clone()
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
            source: source.clone(),
            translation_by_category: plural_profile
                .materialize_translation(translation_by_category),
            variable: variable
                .as_deref()
                .map_or_else(|| previous_variable.clone(), str::to_owned),
        },
        (NormalizedKind::Plural { source, variable }, previous) => {
            let variable = variable
                .clone()
                .or_else(|| previous.and_then(extract_plural_variable))
                .or_else(|| derive_plural_variable(&next.placeholders))
                .unwrap_or_else(|| {
                    diagnostics.push(
                        Diagnostic::new(
                            DiagnosticSeverity::Warning,
                            diagnostic_codes::plural::ASSUMED_VARIABLE,
                            "Unable to determine plural placeholder name, assuming \"count\".",
                        )
                        .with_identity(&next.msgid, next.msgctxt.as_deref()),
                    );
                    "count".to_owned()
                });

            CanonicalTranslation::Plural {
                source: source.clone(),
                translation_by_category: if is_source_locale {
                    plural_profile.source_locale_translation(source)
                } else {
                    plural_profile.empty_translation()
                },
                variable,
            }
        }
    };

    let (machine_translation, translator_comments, flags, obsolete) = previous.map_or_else(
        || (None, Vec::new(), Vec::new(), false),
        |message| {
            (
                message.machine_translation.clone(),
                message.translator_comments.clone(),
                message.flags.clone(),
                false,
            )
        },
    );

    CanonicalMessage {
        msgid: next.msgid.clone(),
        msgctxt: next.msgctxt.clone(),
        translation,
        comments: next.comments.clone(),
        origins: next.origins.clone(),
        placeholders: next.placeholders.clone(),
        obsolete,
        machine_translation,
        translator_comments,
        flags,
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
        OrderBy::Origin => messages.sort_by(|left, right| {
            first_origin_sort_key(&left.origins)
                .cmp(&first_origin_sort_key(&right.origins))
                .then_with(|| left.msgid.cmp(&right.msgid))
                .then_with(|| left.msgctxt.cmp(&right.msgctxt))
        }),
    }
}

fn first_origin_sort_key(origins: &[CatalogOrigin]) -> (String, Option<u32>) {
    origins.first().map_or_else(
        || (String::new(), None),
        |origin| (origin.file.clone(), origin.line),
    )
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
        CatalogStorageFormat::Ndjson => {
            if options
                .render
                .custom_header_attributes
                .is_some_and(|headers| !headers.is_empty())
            {
                return Err(ApiError::Unsupported(
                    "custom_header_attributes are not supported for NDJSON catalogs".to_owned(),
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
        CatalogStorageFormat::Ndjson => parse_catalog_to_internal_ndjson(
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
    let (comments, placeholders) = split_placeholder_comments(item.extracted_comments);
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
                .zip(
                    item.msgstr
                        .iter()
                        .map(String::as_str)
                        .chain(std::iter::repeat("")),
                )
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
        obsolete: item.obsolete,
        machine_translation: import_machine_translation_metadata(&item.metadata)?,
        translator_comments: item.comments,
        flags: item.flags,
    })
}

fn import_machine_translation_metadata(
    metadata: &[(String, String)],
) -> Result<Option<MachineTranslationMetadata>, ApiError> {
    let mut value = None;
    for (key, next_value) in metadata {
        if key != PO_MACHINE_TRANSLATION_KEY {
            continue;
        }
        if value.replace(next_value).is_some() {
            return Err(ApiError::InvalidArguments(
                "duplicate machine translation metadata for PO item".to_owned(),
            ));
        }
    }
    value
        .map(|value| parse_po_machine_translation_metadata(value))
        .transpose()
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

/// Parses the internal placeholder comment format emitted by `append_placeholder_comments`.
fn parse_placeholder_comment(comment: &str) -> Option<(String, String)> {
    let rest = comment.strip_prefix("placeholder {")?;
    let end = rest.find("}: ")?;
    Some((rest[..end].to_owned(), rest[end + 3..].to_owned()))
}

/// Parses a gettext reference while tolerating plain paths and `path:line`.
/// Splits a `file:line` reference into a [`CatalogOrigin`], reusing the owned
/// reference buffer for the file part instead of allocating a fresh string.
fn parse_origin_owned(mut reference: String) -> CatalogOrigin {
    if let Some((file, line)) = reference.rsplit_once(':')
        && line.chars().all(|ch| ch.is_ascii_digit())
    {
        let parsed_line = line.parse::<u32>().ok();
        let file_len = file.len();
        reference.truncate(file_len);
        return CatalogOrigin {
            file: reference,
            line: parsed_line,
        };
    }

    CatalogOrigin {
        file: reference,
        line: None,
    }
}

/// Moves the first available translation string out of a [`MsgStr`], matching
/// the `first_str().unwrap_or_default()` semantics without copying.
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
        machine_translation: message.machine_translation,
        extra: Some(CatalogMessageExtra {
            translator_comments: message.translator_comments,
            flags: message.flags,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_origin_owned, take_first_msgstr};
    use crate::MsgStr;

    #[test]
    fn parse_origin_owned_handles_file_line_and_bare_references() {
        let with_line = parse_origin_owned("src/app.rs:42".to_owned());
        assert_eq!(with_line.file, "src/app.rs");
        assert_eq!(with_line.line, Some(42));

        // No colon and a non-numeric line suffix both fall back to a line-less
        // origin that reuses the original buffer verbatim.
        let bare = parse_origin_owned("README".to_owned());
        assert_eq!(bare.file, "README");
        assert_eq!(bare.line, None);

        let non_numeric = parse_origin_owned("a:b:rev".to_owned());
        assert_eq!(non_numeric.file, "a:b:rev");
        assert_eq!(non_numeric.line, None);
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
}
