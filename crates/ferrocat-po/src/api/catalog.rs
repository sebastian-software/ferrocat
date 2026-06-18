//! Internal catalog pipeline for the public `ferrocat-po` catalog API.
//!
//! This module owns the higher-level workflow around PO parsing, extracted-message
//! normalization, merge semantics, and export back to PO. The byte-oriented parser
//! and serializer hot paths stay elsewhere; this layer is where we preserve
//! catalog semantics and diagnostics.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use crate::diagnostic_codes;

use super::file_io::atomic_write;
use super::helpers::{
    dedupe_origins, dedupe_placeholders, dedupe_strings, merge_placeholders, merge_unique_origins,
    merge_unique_strings,
};
use super::mt::{
    MachineTranslationMetadata, PO_MACHINE_TRANSLATION_KEY, format_po_machine_translation_metadata,
    machine_translation_hash, parse_po_machine_translation_metadata,
    validate_machine_translation_metadata,
};
use super::ndjson::{parse_catalog_to_internal_ndjson, stringify_catalog_ndjson};
use super::plural::{
    PluralProfile, derive_plural_variable, expected_gettext_nplurals_for_locale,
    synthesize_icu_plural,
};
use super::{
    ApiError, CatalogCombineResult, CatalogCombineStats, CatalogConflictStrategy, CatalogMessage,
    CatalogMessageExtra, CatalogOrigin, CatalogSemantics, CatalogStats, CatalogStorageFormat,
    CatalogUpdateInput, CatalogUpdateResult, CombineCatalogOptions, Diagnostic, DiagnosticSeverity,
    EffectiveTranslationRef, ExtractedMessage, ObsoleteStrategy, OrderBy, ParseCatalogOptions,
    ParsedCatalog, PlaceholderCommentMode, PluralEncoding, PluralSource, TranslationShape,
    UpdateCatalogFileOptions, UpdateCatalogOptions,
};
use crate::{Header, MsgStr, PoFile, PoItem, SerializeOptions, parse_po, stringify_po};

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct CombineEntry {
    message: CanonicalMessage,
    definitions: usize,
    labels: Vec<String>,
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
///     CatalogUpdateInput, SourceExtractedMessage, UpdateCatalogOptions, update_catalog,
/// };
///
/// let result = update_catalog(UpdateCatalogOptions {
///     source_locale: "en",
///     locale: Some("de"),
///     input: CatalogUpdateInput::SourceFirst(vec![SourceExtractedMessage {
///         msgid: "Checkout".to_owned(),
///         ..SourceExtractedMessage::default()
///     }]),
///     ..UpdateCatalogOptions::default()
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
    super::validate_catalog_semantics(
        options.semantics,
        options.storage_format,
        options.plural_encoding,
    )?;

    let created = options.existing.is_none();
    let original = options.existing.unwrap_or("");
    let existing = match options.existing {
        Some(content) if !content.is_empty() => parse_catalog_to_internal(
            content,
            options.locale,
            options.source_locale,
            options.semantics,
            options.plural_encoding,
            false,
            options.storage_format,
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
        semantics: options.semantics,
        overwrite_source_translations: options.overwrite_source_translations,
        obsolete_strategy: options.obsolete_strategy,
    };
    let (mut merged, stats) =
        merge_catalogs(existing, &normalized, merge_context, &mut diagnostics);
    merged.locale.clone_from(&locale);
    apply_storage_defaults(&mut merged, &options, locale.as_deref(), &mut diagnostics)?;
    sort_messages(&mut merged.messages, options.order_by);
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
    super::validate_source_locale(options.source_locale)?;
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

    let result = update_catalog(UpdateCatalogOptions {
        locale: options.locale,
        source_locale: options.source_locale,
        input: options.input,
        existing: existing.as_deref(),
        storage_format: options.storage_format,
        semantics: options.semantics,
        plural_encoding: options.plural_encoding,
        obsolete_strategy: options.obsolete_strategy,
        overwrite_source_translations: options.overwrite_source_translations,
        order_by: options.order_by,
        include_origins: options.include_origins,
        include_line_numbers: options.include_line_numbers,
        print_placeholders_in_comments: options.print_placeholders_in_comments,
        custom_header_attributes: options.custom_header_attributes,
    })?;

    if result.created || result.updated {
        atomic_write(options.target_path, &result.content)?;
    }

    Ok(result)
}

/// Combines multiple catalogs into one deterministic catalog.
///
/// This is Ferrocat's Rust-native N-way catalog combine API. It covers the
/// useful `msgcat`/`msgmerge`-shaped workflow without reproducing GNU gettext's
/// conflict-marker output: identical `msgid`/`msgctxt` definitions are merged,
/// metadata is cumulated, and non-empty translation conflicts are resolved or
/// rejected according to [`CatalogConflictStrategy`].
///
/// # Errors
///
/// Returns [`ApiError`] when required options are missing, an input cannot be
/// parsed, a singular/plural shape conflict is encountered, or the selected
/// conflict strategy rejects differing translations.
///
/// # Examples
///
/// ```rust
/// use ferrocat_po::{CatalogCombineInput, CombineCatalogOptions, combine_catalogs};
///
/// let base = "msgid \"Checkout\"\nmsgstr \"Zur Kasse\"\n";
/// let template = "msgid \"Checkout\"\nmsgstr \"\"\n\nmsgid \"Cart\"\nmsgstr \"\"\n";
/// let inputs = [
///     CatalogCombineInput::labeled(base, "de.po"),
///     CatalogCombineInput::labeled(template, "messages.pot"),
/// ];
///
/// let combined = combine_catalogs(CombineCatalogOptions::new(&inputs, "en"))?;
/// assert!(combined.content.contains("msgstr \"Zur Kasse\""));
/// assert!(combined.content.contains("msgid \"Cart\""));
/// # Ok::<(), ferrocat_po::ApiError>(())
/// ```
#[expect(
    clippy::needless_pass_by_value,
    reason = "Public API takes owned option structs so callers can build and move them ergonomically."
)]
pub fn combine_catalogs(
    options: CombineCatalogOptions<'_>,
) -> Result<CatalogCombineResult, ApiError> {
    super::validate_source_locale(options.source_locale)?;
    super::validate_catalog_semantics(
        options.semantics,
        options.storage_format,
        options.plural_encoding,
    )?;
    if options.inputs.is_empty() {
        return Err(ApiError::InvalidArguments(
            "inputs must not be empty".to_owned(),
        ));
    }

    let mut diagnostics = Vec::new();
    let mut headers = BTreeMap::new();
    let mut file_comments = Vec::new();
    let mut file_extracted_comments = Vec::new();
    let mut inferred_locale = options.locale.map(str::to_owned);
    let mut entries = Vec::<CombineEntry>::new();
    let mut index = BTreeMap::<(String, Option<String>), usize>::new();
    let mut stats = CatalogCombineStats {
        inputs: options.inputs.len(),
        ..CatalogCombineStats::default()
    };

    for (input_index, input) in options.inputs.iter().enumerate() {
        let catalog = parse_catalog_to_internal(
            input.content,
            options.locale,
            options.source_locale,
            options.semantics,
            options.plural_encoding,
            false,
            options.storage_format,
        )?;

        if input_index == 0 {
            headers = catalog.headers.clone();
            file_comments = catalog.file_comments.clone();
            file_extracted_comments = catalog.file_extracted_comments.clone();
            if inferred_locale.is_none() {
                inferred_locale.clone_from(&catalog.locale);
            }
        }
        diagnostics.extend(catalog.diagnostics);

        let label = input_label(input.label, input_index);
        for message in catalog.messages {
            if message.obsolete && !options.include_obsolete {
                continue;
            }
            stats.definitions += 1;
            push_combine_message(
                &mut entries,
                &mut index,
                message,
                label.clone(),
                options.conflict_strategy,
                &mut stats,
                &mut diagnostics,
            )?;
        }
    }

    let mut messages = Vec::with_capacity(entries.len());
    for entry in entries {
        if options.selection.includes(entry.definitions) {
            messages.push(entry.message);
        } else {
            stats.skipped += 1;
        }
    }
    stats.selected = messages.len();
    stats.total = messages.len();

    let mut combined = Catalog {
        locale: inferred_locale.clone(),
        headers,
        file_comments,
        file_extracted_comments,
        messages,
        diagnostics: Vec::new(),
    };

    apply_storage_defaults_for_combine(
        &mut combined,
        &options,
        inferred_locale.as_deref(),
        &mut diagnostics,
    )?;
    sort_messages(&mut combined.messages, options.order_by);
    let content =
        export_catalog_content_for_combine(&combined, &options, inferred_locale.as_deref())?;

    Ok(CatalogCombineResult {
        content,
        stats,
        diagnostics,
    })
}

fn input_label(label: Option<&str>, index: usize) -> String {
    label.map_or_else(|| format!("input {}", index + 1), str::to_owned)
}

fn push_combine_message(
    entries: &mut Vec<CombineEntry>,
    index: &mut BTreeMap<(String, Option<String>), usize>,
    message: CanonicalMessage,
    label: String,
    conflict_strategy: CatalogConflictStrategy,
    stats: &mut CatalogCombineStats,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), ApiError> {
    let key = (message.msgid.clone(), message.msgctxt.clone());
    let Some(existing_index) = index.get(&key).copied() else {
        index.insert(key, entries.len());
        entries.push(CombineEntry {
            message,
            definitions: 1,
            labels: vec![label],
        });
        return Ok(());
    };

    let entry = &mut entries[existing_index];
    entry.definitions += 1;
    entry.labels.push(label);
    merge_combine_message(entry, message, conflict_strategy, stats, diagnostics)
}

fn merge_combine_message(
    entry: &mut CombineEntry,
    message: CanonicalMessage,
    conflict_strategy: CatalogConflictStrategy,
    stats: &mut CatalogCombineStats,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), ApiError> {
    if translation_shape(&entry.message.translation) != translation_shape(&message.translation) {
        return Err(ApiError::Conflict(format!(
            "conflicting catalog message shape for msgid {:?} and context {:?}",
            entry.message.msgid, entry.message.msgctxt
        )));
    }

    let has_conflict =
        has_meaningful_translation_conflict(&entry.message.translation, &message.translation);
    if has_conflict {
        match conflict_strategy {
            CatalogConflictStrategy::Error => {
                return Err(ApiError::Conflict(format!(
                    "conflicting catalog translation for msgid {:?} and context {:?}",
                    entry.message.msgid, entry.message.msgctxt
                )));
            }
            CatalogConflictStrategy::UseFirst | CatalogConflictStrategy::UseLast => {
                stats.conflicts_resolved += 1;
                diagnostics.push(
                    Diagnostic::new(
                        DiagnosticSeverity::Warning,
                        diagnostic_codes::combine::CONFLICT_RESOLVED,
                        format!(
                            "Resolved conflicting translations for definitions from {} using {:?}.",
                            entry.labels.join(", "),
                            conflict_strategy
                        ),
                    )
                    .with_identity(&entry.message.msgid, entry.message.msgctxt.as_deref()),
                );
            }
        }
    }

    if conflict_strategy == CatalogConflictStrategy::UseLast {
        entry.message.translation = message.translation.clone();
        entry.message.machine_translation = message.machine_translation.clone();
    }

    merge_combine_metadata(&mut entry.message, message);
    Ok(())
}

fn merge_combine_metadata(target: &mut CanonicalMessage, source: CanonicalMessage) {
    merge_unique_strings(&mut target.comments, source.comments);
    merge_unique_origins(&mut target.origins, source.origins);
    merge_placeholders(&mut target.placeholders, source.placeholders);
    merge_unique_strings(&mut target.translator_comments, source.translator_comments);
    merge_unique_strings(&mut target.flags, source.flags);
    if target.machine_translation.is_none() {
        target.machine_translation = source.machine_translation;
    }
    target.obsolete = target.obsolete && source.obsolete;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TranslationKind {
    Singular,
    Plural,
}

fn translation_shape(translation: &CanonicalTranslation) -> TranslationKind {
    match translation {
        CanonicalTranslation::Singular { .. } => TranslationKind::Singular,
        CanonicalTranslation::Plural { .. } => TranslationKind::Plural,
    }
}

fn has_meaningful_translation_conflict(
    left: &CanonicalTranslation,
    right: &CanonicalTranslation,
) -> bool {
    match (left, right) {
        (
            CanonicalTranslation::Singular { value: left },
            CanonicalTranslation::Singular { value: right },
        ) => !left.is_empty() && !right.is_empty() && left != right,
        (
            CanonicalTranslation::Plural {
                translation_by_category: left,
                ..
            },
            CanonicalTranslation::Plural {
                translation_by_category: right,
                ..
            },
        ) => {
            let categories = left.keys().chain(right.keys()).collect::<BTreeSet<_>>();
            categories.into_iter().any(|category| {
                let left = left.get(category).map(String::as_str).unwrap_or_default();
                let right = right.get(category).map(String::as_str).unwrap_or_default();
                !left.is_empty() && !right.is_empty() && left != right
            })
        }
        _ => false,
    }
}

fn apply_storage_defaults_for_combine(
    catalog: &mut Catalog,
    options: &CombineCatalogOptions<'_>,
    locale: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), ApiError> {
    match options.storage_format {
        CatalogStorageFormat::Po => {
            apply_header_defaults(
                &mut catalog.headers,
                locale,
                options.semantics,
                diagnostics,
                &BTreeMap::new(),
            );
            Ok(())
        }
        CatalogStorageFormat::Ndjson => {
            catalog.headers.clear();
            Ok(())
        }
    }
}

fn export_catalog_content_for_combine(
    catalog: &Catalog,
    options: &CombineCatalogOptions<'_>,
    locale: Option<&str>,
) -> Result<String, ApiError> {
    let export_options = UpdateCatalogOptions {
        locale: options.locale,
        source_locale: options.source_locale,
        input: CatalogUpdateInput::default(),
        existing: None,
        storage_format: options.storage_format,
        semantics: options.semantics,
        plural_encoding: options.plural_encoding,
        obsolete_strategy: ObsoleteStrategy::Keep,
        overwrite_source_translations: false,
        order_by: options.order_by,
        include_origins: options.include_origins,
        include_line_numbers: options.include_line_numbers,
        print_placeholders_in_comments: PlaceholderCommentMode::default(),
        custom_header_attributes: None,
    };
    let mut diagnostics = Vec::new();
    export_catalog_content(catalog, &export_options, locale, &mut diagnostics)
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
///     content: "msgid \"Checkout\"\nmsgstr \"Zur Kasse\"\n",
///     locale: Some("de"),
///     source_locale: "en",
///     ..ParseCatalogOptions::default()
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
    super::validate_catalog_semantics(
        options.semantics,
        options.storage_format,
        options.plural_encoding,
    )?;
    let catalog = parse_catalog_to_internal(
        options.content,
        options.locale,
        options.source_locale,
        options.semantics,
        options.plural_encoding,
        options.strict,
        options.storage_format,
    )?;
    let messages = catalog
        .messages
        .into_iter()
        .map(public_message_from_canonical)
        .collect();

    Ok(ParsedCatalog {
        locale: catalog.locale,
        semantics: options.semantics,
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

    for next in normalized {
        let key = (next.msgid.clone(), next.msgctxt.clone());
        let previous = existing_index.get(&key).copied().map(|index| {
            matched[index] = true;
            existing.messages[index].clone()
        });
        let merged = merge_message(
            previous.as_ref(),
            next,
            is_source_locale,
            context.locale,
            context.semantics,
            context.overwrite_source_translations,
            diagnostics,
        );
        if previous.is_none() {
            stats.added += 1;
        } else if previous.as_ref() == Some(&merged) {
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
    locale: Option<&str>,
    semantics: CatalogSemantics,
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
        (NormalizedKind::Plural { source, variable }, previous) => {
            let plural_profile = if semantics == CatalogSemantics::GettextCompat {
                PluralProfile::for_gettext_locale(locale)
            } else {
                PluralProfile::for_locale(locale)
            };

            match previous {
                Some(previous)
                    if matches!(previous.translation, CanonicalTranslation::Plural { .. })
                        && !(is_source_locale && overwrite_source_translations) =>
                {
                    match &previous.translation {
                        CanonicalTranslation::Plural {
                            translation_by_category,
                            variable: previous_variable,
                            ..
                        } => CanonicalTranslation::Plural {
                            source: source.clone(),
                            translation_by_category: plural_profile
                                .materialize_translation(translation_by_category),
                            variable: variable
                                .as_deref()
                                .map_or_else(|| previous_variable.clone(), str::to_owned),
                        },
                        CanonicalTranslation::Singular { .. } => unreachable!(),
                    }
                }
                _ => {
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
fn apply_header_defaults(
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

fn sort_messages(messages: &mut [CanonicalMessage], order_by: OrderBy) {
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
    match options.storage_format {
        CatalogStorageFormat::Po => {
            let empty_custom_headers = BTreeMap::new();
            apply_header_defaults(
                &mut catalog.headers,
                locale,
                options.semantics,
                diagnostics,
                options
                    .custom_header_attributes
                    .unwrap_or(&empty_custom_headers),
            );
            Ok(())
        }
        CatalogStorageFormat::Ndjson => {
            if options
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

fn export_catalog_content(
    catalog: &Catalog,
    options: &UpdateCatalogOptions<'_>,
    locale: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<String, ApiError> {
    match options.storage_format {
        CatalogStorageFormat::Po => {
            let file = export_catalog_to_po(catalog, options, locale, diagnostics)?;
            Ok(stringify_po(&file, &SerializeOptions::default()))
        }
        CatalogStorageFormat::Ndjson => Ok(stringify_catalog_ndjson(
            catalog,
            locale,
            options.source_locale,
            &options.print_placeholders_in_comments,
        )),
    }
}

/// Converts the canonical in-memory catalog back into a `PoFile` while keeping
/// file-level comments and header order normalized.
fn export_catalog_to_po(
    catalog: &Catalog,
    options: &UpdateCatalogOptions<'_>,
    locale: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<PoFile, ApiError> {
    let mut file = PoFile {
        comments: catalog.file_comments.clone(),
        extracted_comments: catalog.file_extracted_comments.clone(),
        headers: catalog
            .headers
            .iter()
            .map(|(key, value)| Header {
                key: key.clone(),
                value: value.clone(),
            })
            .collect(),
        items: Vec::with_capacity(catalog.messages.len()),
    };

    for message in &catalog.messages {
        file.items
            .push(export_message_to_po(message, options, locale, diagnostics)?);
    }

    Ok(file)
}

/// Renders one canonical message into the chosen PO representation.
///
/// Singular messages are straightforward, while plural messages either stay as
/// a synthesized ICU string or are lowered into gettext slots depending on the
/// caller-selected `PluralEncoding`.
fn export_message_to_po(
    message: &CanonicalMessage,
    options: &UpdateCatalogOptions<'_>,
    locale: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<PoItem, ApiError> {
    match &message.translation {
        CanonicalTranslation::Singular { value } => {
            let mut item = base_po_item(message, options, 1);
            item.msgid.clone_from(&message.msgid);
            item.msgstr = MsgStr::from(value.clone());
            Ok(item)
        }
        CanonicalTranslation::Plural {
            source,
            translation_by_category,
            variable,
        } => {
            if options.semantics == CatalogSemantics::IcuNative {
                let mut item = base_po_item(message, options, 1);
                item.msgid = synthesize_icu_plural(variable, &plural_source_branches(source));
                item.msgstr =
                    MsgStr::from(synthesize_icu_plural(variable, translation_by_category));
                return Ok(item);
            }

            let plural_profile =
                PluralProfile::for_gettext_translation(locale, translation_by_category);
            let nplurals = plural_profile
                .nplurals()
                .max(translation_by_category.len().max(1));
            let mut item = base_po_item(message, options, nplurals);

            if !translation_by_category.contains_key("other") {
                diagnostics.push(
                    Diagnostic::new(
                        DiagnosticSeverity::Error,
                        diagnostic_codes::plural::UNSUPPORTED_GETTEXT_EXPORT,
                        "Plural translation is missing the required \"other\" category.",
                    )
                    .with_identity(&message.msgid, message.msgctxt.as_deref()),
                );
                return Err(ApiError::Unsupported(
                    "plural translation is missing the required \"other\" category".to_owned(),
                ));
            }
            item.msgid = source.one.clone().unwrap_or_else(|| source.other.clone());
            item.msgid_plural = Some(source.other.clone());
            item.msgstr = MsgStr::from(plural_profile.gettext_values(translation_by_category));
            item.nplurals = plural_profile.nplurals();

            Ok(item)
        }
    }
}

/// Builds the common `PoItem` shell shared by singular and plural export.
fn base_po_item(
    message: &CanonicalMessage,
    options: &UpdateCatalogOptions<'_>,
    nplurals: usize,
) -> PoItem {
    let mut item = PoItem::new(nplurals);
    item.msgctxt.clone_from(&message.msgctxt);
    item.comments.clone_from(&message.translator_comments);
    item.flags.clone_from(&message.flags);
    item.obsolete = message.obsolete;
    item.extracted_comments.clone_from(&message.comments);
    append_placeholder_comments(
        &mut item.extracted_comments,
        &message.placeholders,
        &options.print_placeholders_in_comments,
    );
    if let Some(metadata) = valid_machine_translation_metadata(message) {
        item.metadata.push((
            PO_MACHINE_TRANSLATION_KEY.to_owned(),
            format_po_machine_translation_metadata(metadata),
        ));
    }
    item.references = if options.include_origins {
        message
            .origins
            .iter()
            .map(|origin| {
                if options.include_line_numbers {
                    origin.line.map_or_else(
                        || origin.file.clone(),
                        |line| format!("{}:{line}", origin.file),
                    )
                } else {
                    origin.file.clone()
                }
            })
            .collect()
    } else {
        Vec::new()
    };
    item
}

fn valid_machine_translation_metadata(
    message: &CanonicalMessage,
) -> Option<&MachineTranslationMetadata> {
    let metadata = message.machine_translation.as_ref()?;
    if validate_machine_translation_metadata(metadata).is_err() {
        return None;
    }
    (metadata.hash == machine_translation_hash(canonical_translation_ref(&message.translation)))
        .then_some(metadata)
}

fn canonical_translation_ref(translation: &CanonicalTranslation) -> EffectiveTranslationRef<'_> {
    match translation {
        CanonicalTranslation::Singular { value } => EffectiveTranslationRef::Singular(value),
        CanonicalTranslation::Plural {
            translation_by_category,
            ..
        } => EffectiveTranslationRef::Plural(translation_by_category),
    }
}

/// Builds the minimal category map needed to re-synthesize a source ICU plural.
pub(super) fn plural_source_branches(source: &PluralSource) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    if let Some(one) = &source.one {
        map.insert("one".to_owned(), one.clone());
    }
    map.insert("other".to_owned(), source.other.clone());
    map
}

/// Emits extracted placeholder comments only for numeric placeholders, which
/// mirrors how gettext tools commonly surface ordered placeholder hints.
pub(super) fn append_placeholder_comments(
    comments: &mut Vec<String>,
    placeholders: &BTreeMap<String, Vec<String>>,
    mode: &PlaceholderCommentMode,
) {
    let limit = match mode {
        PlaceholderCommentMode::Disabled => return,
        PlaceholderCommentMode::Enabled { limit } => *limit,
    };

    let mut seen = comments.iter().cloned().collect::<BTreeSet<String>>();

    for (name, values) in placeholders {
        if !name.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        for value in values.iter().take(limit) {
            let comment = format!(
                "placeholder {{{name}}}: {}",
                normalize_placeholder_value(value)
            );
            if seen.insert(comment.clone()) {
                comments.push(comment);
            }
        }
    }
}

fn normalize_placeholder_value(value: &str) -> String {
    value.replace('\n', " ")
}

/// Parses catalog text into the canonical internal catalog representation used by
/// both `parse_catalog` and `update_catalog`.
///
/// Keeping this internal representation stable lets the public APIs share one
/// import path before they diverge into normalized lookup or update/export work.
fn parse_catalog_to_internal(
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
    let file = parse_po(content)?;
    let headers = file
        .headers
        .iter()
        .map(|header| (header.key.clone(), header.value.clone()))
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
    let mut messages = Vec::with_capacity(file.items.len());

    for item in file.items {
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
        file_comments: file.comments,
        file_extracted_comments: file.extracted_comments,
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
        .iter()
        .map(|reference| parse_origin(reference))
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
            value: item.msgstr.first_str().unwrap_or_default().to_owned(),
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
fn parse_origin(reference: &str) -> CatalogOrigin {
    match reference.rsplit_once(':') {
        Some((file, line)) if line.chars().all(|ch| ch.is_ascii_digit()) => CatalogOrigin {
            file: file.to_owned(),
            line: line.parse::<u32>().ok(),
        },
        _ => CatalogOrigin {
            file: reference.to_owned(),
            line: None,
        },
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
