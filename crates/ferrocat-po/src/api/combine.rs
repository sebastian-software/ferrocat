//! Catalog combine workflow for the public catalog API.
//!
//! This module keeps N-way catalog overlay logic separate from update/import
//! and export mechanics while still sharing the canonical catalog model.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostic_codes;

use super::catalog::{
    CanonicalMessage, CanonicalTranslation, Catalog, apply_header_defaults, sort_messages,
};
use super::export::export_catalog_content;
use super::file_io::atomic_write;
use super::helpers::{merge_placeholders, merge_unique_origins, merge_unique_strings};
use super::{
    ApiError, CatalogCombineInput, CatalogCombineResult, CatalogCombineStats,
    CatalogConflictStrategy, CatalogFileCombineResult, CatalogFileFormat, CatalogMode,
    CatalogStorageFormat, CatalogUpdateInput, CombineCatalogFilesOptions, CombineCatalogOptions,
    Diagnostic, DiagnosticSeverity, ObsoleteStrategy, PlaceholderCommentMode, RenderOptions,
    UpdateCatalogOptions,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct CombineEntry {
    message: CanonicalMessage,
    definitions: usize,
    labels: Vec<String>,
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
        let catalog = super::catalog::parse_catalog_to_internal(
            input.content,
            options.locale,
            options.source_locale,
            options.mode.semantics(),
            options.mode.plural_encoding(),
            false,
            options.mode.storage_format(),
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

/// Combines catalog files and atomically replaces the requested output path.
///
/// This is the disk-based counterpart to [`combine_catalogs`]. Ferrocat reads
/// each input path in precedence order, infers or applies a single file format,
/// delegates merge semantics to [`combine_catalogs`], and writes the output only
/// after validation, parsing, and combining all succeed.
///
/// # Errors
///
/// Returns [`ApiError`] when required options are missing, a file format cannot
/// be inferred, inferred input/output formats disagree, a file cannot be read or
/// written, or the underlying combine operation fails.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Public API takes owned option structs so callers can build and move them ergonomically."
)]
pub fn combine_catalog_files(
    options: CombineCatalogFilesOptions<'_>,
) -> Result<CatalogFileCombineResult, ApiError> {
    super::validate_source_locale(options.source_locale)?;
    if options.input_paths.is_empty() {
        return Err(ApiError::InvalidArguments(
            "input_paths must not be empty".to_owned(),
        ));
    }

    let format = match options.format {
        Some(format) => format,
        None => infer_catalog_file_format(options.input_paths, options.output_path)?,
    };
    let mode = catalog_mode_for_file_format(format, options.mode)?;
    let contents = read_catalog_files(options.input_paths)?;
    let labels = options
        .input_paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    let inputs = contents
        .iter()
        .zip(labels.iter())
        .map(|(content, label)| CatalogCombineInput::labeled(content.as_str(), label.as_str()))
        .collect::<Vec<_>>();

    let result = combine_catalogs(CombineCatalogOptions {
        inputs: &inputs,
        locale: options.locale,
        source_locale: options.source_locale,
        mode,
        conflict_strategy: options.conflict_strategy,
        selection: options.selection,
        order_by: options.order_by,
        include_origins: options.include_origins,
        include_line_numbers: options.include_line_numbers,
        include_obsolete: options.include_obsolete,
    })?;

    atomic_write(options.output_path, &result.content)?;

    Ok(CatalogFileCombineResult {
        output_path: options.output_path.to_path_buf(),
        format,
        stats: result.stats,
        diagnostics: result.diagnostics,
    })
}

fn read_catalog_files(input_paths: &[PathBuf]) -> Result<Vec<String>, ApiError> {
    input_paths
        .iter()
        .map(|path| fs::read_to_string(path).map_err(|error| ApiError::io_with_path(path, error)))
        .collect()
}

fn infer_catalog_file_format(
    input_paths: &[PathBuf],
    output_path: &Path,
) -> Result<CatalogFileFormat, ApiError> {
    let output_format = CatalogFileFormat::infer_from_path(output_path)?;
    for path in input_paths {
        let input_format = CatalogFileFormat::infer_from_path(path)?;
        if input_format != output_format {
            return Err(ApiError::InvalidArguments(format!(
                "catalog input `{}` uses {:?}, but output `{}` uses {:?}",
                path.display(),
                input_format,
                output_path.display(),
                output_format
            )));
        }
    }
    Ok(output_format)
}

fn catalog_mode_for_file_format(
    format: CatalogFileFormat,
    mode: Option<CatalogMode>,
) -> Result<CatalogMode, ApiError> {
    let Some(mode) = mode else {
        return Ok(format.default_mode());
    };
    if mode.storage_format() == format.default_mode().storage_format() {
        Ok(mode)
    } else {
        Err(ApiError::InvalidArguments(format!(
            "catalog mode {:?} does not match file format {:?}",
            mode, format
        )))
    }
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
    match options.mode.storage_format() {
        CatalogStorageFormat::Po => {
            apply_header_defaults(
                &mut catalog.headers,
                locale,
                options.mode.semantics(),
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
        mode: options.mode,
        obsolete_strategy: ObsoleteStrategy::Keep,
        render: RenderOptions {
            order_by: options.order_by,
            include_origins: options.include_origins,
            include_line_numbers: options.include_line_numbers,
            print_placeholders_in_comments: PlaceholderCommentMode::default(),
            custom_header_attributes: None,
        },
        ..UpdateCatalogOptions::new(options.source_locale, CatalogUpdateInput::default())
    };
    let mut diagnostics = Vec::new();
    export_catalog_content(catalog, &export_options, locale, &mut diagnostics)
}
