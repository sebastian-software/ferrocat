//! Catalog combine workflow for the public catalog API.
//!
//! This module keeps N-way catalog overlay logic separate from update/import
//! and export mechanics while still sharing the canonical catalog model.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::mem;
use std::path::{Path, PathBuf};

use crate::SerializeOptions;
use crate::diagnostic_codes;

use super::catalog::{
    CanonicalMessage, CanonicalTranslation, Catalog, apply_header_defaults, sort_messages,
};
use super::export::export_catalog_content;
use super::file_io::atomic_write;
use super::helpers::{merge_placeholders, merge_unique_origins, merge_unique_strings};
use super::{
    ApiError, CatalogCombineResult, CatalogCombineSelection, CatalogCombineStats,
    CatalogConflictStrategy, CatalogFileCombineResult, CatalogFileFormat, CatalogMode,
    CatalogStorageFormat, CatalogUpdateInput, CombineCatalogFilesOptions, CombineCatalogOptions,
    Diagnostic, DiagnosticSeverity, ObsoleteInfo, ObsoleteStrategy, OrderBy,
    PlaceholderCommentMode, RenderOptions, UpdateCatalogOptions,
};

#[derive(Debug, Clone, PartialEq)]
struct CombineEntry {
    message: CanonicalMessage,
    definitions: usize,
    label_indexes: Vec<usize>,
}

#[derive(Debug, Clone, Copy)]
struct CombineConfig<'a> {
    locale: Option<&'a str>,
    source_locale: &'a str,
    mode: CatalogMode,
    conflict_strategy: CatalogConflictStrategy,
    selection: CatalogCombineSelection,
    order_by: OrderBy,
    include_origins: bool,
    include_obsolete: bool,
    po_serialize: &'a SerializeOptions,
}

impl<'a> CombineConfig<'a> {
    fn from_options(options: &'a CombineCatalogOptions<'a>) -> Self {
        Self {
            locale: options.locale,
            source_locale: options.source_locale,
            mode: options.mode,
            conflict_strategy: options.conflict_strategy,
            selection: options.selection,
            order_by: options.order_by,
            include_origins: options.include_origins,
            include_obsolete: options.include_obsolete,
            po_serialize: &options.po_serialize,
        }
    }

    fn from_file_options(options: &'a CombineCatalogFilesOptions<'a>, mode: CatalogMode) -> Self {
        Self {
            locale: options.locale,
            source_locale: options.source_locale,
            mode,
            conflict_strategy: options.conflict_strategy,
            selection: options.selection,
            order_by: options.order_by,
            include_origins: options.include_origins,
            include_obsolete: options.include_obsolete,
            po_serialize: &options.po_serialize,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct CombineState {
    diagnostics: Vec<Diagnostic>,
    headers: BTreeMap<String, String>,
    file_comments: Vec<String>,
    file_extracted_comments: Vec<String>,
    inferred_locale: Option<String>,
    entries: Vec<CombineEntry>,
    index: BTreeMap<String, BTreeMap<Option<String>, usize>>,
    stats: CatalogCombineStats,
    labels: Vec<String>,
}

impl CombineState {
    fn new(input_count: usize, locale: Option<&str>) -> Self {
        Self {
            diagnostics: Vec::new(),
            headers: BTreeMap::new(),
            file_comments: Vec::new(),
            file_extracted_comments: Vec::new(),
            inferred_locale: locale.map(str::to_owned),
            entries: Vec::new(),
            index: BTreeMap::new(),
            stats: CatalogCombineStats {
                inputs: input_count,
                ..CatalogCombineStats::default()
            },
            labels: Vec::with_capacity(input_count),
        }
    }

    fn push_catalog(
        &mut self,
        catalog: Catalog,
        input_index: usize,
        label: String,
        config: CombineConfig<'_>,
    ) -> Result<(), ApiError> {
        let Catalog {
            locale,
            headers,
            file_comments,
            file_extracted_comments,
            messages,
            diagnostics,
        } = catalog;

        if input_index == 0 {
            self.headers = headers;
            self.file_comments = file_comments;
            self.file_extracted_comments = file_extracted_comments;
            if self.inferred_locale.is_none() {
                self.inferred_locale = locale;
            }
        }
        self.diagnostics.extend(diagnostics);

        let label_index = self.labels.len();
        self.labels.push(label);
        for message in messages {
            if message.obsolete.is_some() && !config.include_obsolete {
                continue;
            }
            self.stats.definitions += 1;
            self.push_message(message, label_index, config.conflict_strategy)?;
        }

        Ok(())
    }

    fn push_message(
        &mut self,
        message: CanonicalMessage,
        label_index: usize,
        conflict_strategy: CatalogConflictStrategy,
    ) -> Result<(), ApiError> {
        let Some(existing_index) = self.message_index(&message) else {
            self.insert_message_index(&message);
            self.entries.push(CombineEntry {
                message,
                definitions: 1,
                label_indexes: vec![label_index],
            });
            return Ok(());
        };

        let entry = &mut self.entries[existing_index];
        entry.definitions += 1;
        entry.label_indexes.push(label_index);
        merge_combine_message(
            entry,
            message,
            &self.labels,
            conflict_strategy,
            &mut self.stats,
            &mut self.diagnostics,
        )
    }

    fn message_index(&self, message: &CanonicalMessage) -> Option<usize> {
        self.index
            .get(message.msgid.as_str())
            .and_then(|by_context| by_context.get(&message.msgctxt))
            .copied()
    }

    fn insert_message_index(&mut self, message: &CanonicalMessage) {
        let entry_index = self.entries.len();
        if let Some(by_context) = self.index.get_mut(message.msgid.as_str()) {
            by_context.insert(message.msgctxt.clone(), entry_index);
        } else {
            self.index.insert(
                message.msgid.clone(),
                BTreeMap::from([(message.msgctxt.clone(), entry_index)]),
            );
        }
    }

    fn finish(mut self, config: CombineConfig<'_>) -> Result<CatalogCombineResult, ApiError> {
        let mut messages = Vec::with_capacity(self.entries.len());
        for entry in self.entries {
            if config.selection.includes(entry.definitions) {
                messages.push(entry.message);
            } else {
                self.stats.skipped += 1;
            }
        }
        self.stats.selected = messages.len();
        self.stats.total = messages.len();

        let mut combined = Catalog {
            locale: self.inferred_locale.clone(),
            headers: self.headers,
            file_comments: self.file_comments,
            file_extracted_comments: self.file_extracted_comments,
            messages,
            diagnostics: Vec::new(),
        };

        apply_storage_defaults_for_combine(
            &mut combined,
            config,
            self.inferred_locale.as_deref(),
            &mut self.diagnostics,
        )?;
        sort_messages(&mut combined.messages, config.order_by);
        let content =
            export_catalog_content_for_combine(&combined, config, self.inferred_locale.as_deref())?;

        Ok(CatalogCombineResult {
            content,
            stats: self.stats,
            diagnostics: self.diagnostics,
        })
    }
}

/// Combines multiple catalogs into one deterministic catalog.
///
/// This is Ferrocat's Rust-native N-way catalog combine API. It covers the
/// useful `msgcat`/`msgmerge`-shaped workflow without reproducing GNU gettext's
/// conflict-marker output: identical `msgid`/`msgctxt` definitions are merged,
/// metadata is cumulated, empty template translations do not replace non-empty
/// translations, and non-empty translation conflicts are resolved or rejected
/// according to [`CatalogConflictStrategy`].
///
/// Translator-owned metadata (translator comments and per-entry flags) is the
/// exception to cumulation: it is carried from the definition that wins the
/// entry value rather than unioned across inputs.
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

    let config = CombineConfig::from_options(&options);
    let mut state = CombineState::new(options.inputs.len(), options.locale);

    for (input_index, input) in options.inputs.iter().enumerate() {
        let catalog = parse_combine_catalog(input.content, config)?;
        state.push_catalog(
            catalog,
            input_index,
            input_label(input.label, input_index),
            config,
        )?;
    }

    state.finish(config)
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
    let config = CombineConfig::from_file_options(&options, mode);
    let mut state = CombineState::new(options.input_paths.len(), options.locale);
    for (input_index, path) in options.input_paths.iter().enumerate() {
        let content =
            fs::read_to_string(path).map_err(|error| ApiError::io_with_path(path, error))?;
        let catalog = parse_combine_catalog(&content, config)?;
        state.push_catalog(catalog, input_index, path.display().to_string(), config)?;
    }

    let result = state.finish(config)?;
    atomic_write(options.output_path, &result.content)?;

    Ok(CatalogFileCombineResult {
        output_path: options.output_path.to_path_buf(),
        format,
        stats: result.stats,
        diagnostics: result.diagnostics,
    })
}

fn parse_combine_catalog(content: &str, config: CombineConfig<'_>) -> Result<Catalog, ApiError> {
    super::catalog::parse_catalog_to_internal(
        content,
        config.locale,
        config.source_locale,
        config.mode.semantics(),
        config.mode.plural_encoding(),
        false,
        config.mode.storage_format(),
    )
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

fn merge_combine_message(
    entry: &mut CombineEntry,
    mut message: CanonicalMessage,
    labels: &[String],
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
                            format_label_list(&entry.label_indexes, labels),
                            conflict_strategy
                        ),
                    )
                    .with_identity(&entry.message.msgid, entry.message.msgctxt.as_deref()),
                );
            }
        }
    }

    let translation_merge = merge_combine_translation(
        &mut entry.message.translation,
        &message.translation,
        conflict_strategy,
    );
    if translation_merge.changed {
        entry.message.machine = translation_merge
            .matches_source
            .then(|| message.machine.clone())
            .flatten();
    } else if translation_merge.matches_source && entry.message.machine.is_none() {
        entry.message.machine = message.machine.clone();
    }

    // Opaque translator metadata belongs to whichever definition ends up owning
    // the entry value, and moves as one block. Unioning it across inputs would
    // invent comments and flags no single catalog ever declared.
    if translation_merge.changed
        || (entry.message.translator_comments.is_empty() && entry.message.flags.is_empty())
    {
        entry.message.translator_comments = mem::take(&mut message.translator_comments);
        entry.message.flags = mem::take(&mut message.flags);
    }

    merge_combine_metadata(&mut entry.message, message);
    Ok(())
}

fn format_label_list(label_indexes: &[usize], labels: &[String]) -> String {
    let mut rendered = String::new();
    for (offset, label_index) in label_indexes.iter().enumerate() {
        if offset > 0 {
            rendered.push_str(", ");
        }
        if let Some(label) = labels.get(*label_index) {
            rendered.push_str(label);
        }
    }
    rendered
}

fn merge_combine_metadata(target: &mut CanonicalMessage, source: CanonicalMessage) {
    merge_unique_strings(&mut target.comments, source.comments);
    merge_unique_origins(&mut target.origins, source.origins);
    merge_placeholders(&mut target.placeholders, source.placeholders);
    // Obsolete only if obsolete in both inputs; keep the earliest known `since`.
    target.obsolete = match (target.obsolete.take(), source.obsolete) {
        (Some(a), Some(b)) => Some(combine_obsolete(a, b)),
        _ => None,
    };
}

/// Combines two obsolete states, keeping the earliest known `since` date (ISO
/// dates compare lexicographically).
fn combine_obsolete(a: ObsoleteInfo, b: ObsoleteInfo) -> ObsoleteInfo {
    let since = match (a.since, b.since) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (Some(x), None) | (None, Some(x)) => Some(x),
        (None, None) => None,
    };
    ObsoleteInfo { since }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TranslationKind {
    Singular,
    Plural,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TranslationMergeOutcome {
    changed: bool,
    matches_source: bool,
}

fn translation_shape(translation: &CanonicalTranslation) -> TranslationKind {
    match translation {
        CanonicalTranslation::Singular { .. } => TranslationKind::Singular,
        CanonicalTranslation::Plural { .. } => TranslationKind::Plural,
    }
}

fn merge_combine_translation(
    target: &mut CanonicalTranslation,
    source: &CanonicalTranslation,
    conflict_strategy: CatalogConflictStrategy,
) -> TranslationMergeOutcome {
    match (target, source) {
        (
            CanonicalTranslation::Singular { value: target },
            CanonicalTranslation::Singular { value: source },
        ) => {
            let should_replace = (!source.is_empty() && target.is_empty())
                || (!source.is_empty()
                    && target != source
                    && conflict_strategy == CatalogConflictStrategy::UseLast);
            if should_replace {
                target.clone_from(source);
            }
            TranslationMergeOutcome {
                changed: should_replace,
                matches_source: target == source,
            }
        }
        (
            CanonicalTranslation::Plural {
                translation_by_category: target,
                ..
            },
            CanonicalTranslation::Plural {
                translation_by_category: source,
                ..
            },
        ) => {
            let categories = target
                .keys()
                .chain(source.keys())
                .cloned()
                .collect::<BTreeSet<_>>();
            let mut changed = false;
            for category in categories {
                let source_value = source
                    .get(&category)
                    .map(String::as_str)
                    .unwrap_or_default();
                if source_value.is_empty() {
                    continue;
                }

                let target_value = target
                    .get(&category)
                    .map(String::as_str)
                    .unwrap_or_default();
                let should_replace = target_value.is_empty()
                    || (target_value != source_value
                        && conflict_strategy == CatalogConflictStrategy::UseLast);
                if should_replace {
                    target.insert(category, source_value.to_owned());
                    changed = true;
                }
            }

            TranslationMergeOutcome {
                changed,
                matches_source: target == source,
            }
        }
        _ => TranslationMergeOutcome {
            changed: false,
            matches_source: false,
        },
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
    config: CombineConfig<'_>,
    locale: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), ApiError> {
    match config.mode.storage_format() {
        CatalogStorageFormat::Po => {
            apply_header_defaults(
                &mut catalog.headers,
                locale,
                config.mode.semantics(),
                diagnostics,
                &BTreeMap::new(),
            );
            Ok(())
        }
        CatalogStorageFormat::Fcl => {
            catalog.headers.clear();
            Ok(())
        }
    }
}

fn export_catalog_content_for_combine(
    catalog: &Catalog,
    config: CombineConfig<'_>,
    locale: Option<&str>,
) -> Result<String, ApiError> {
    let export_options = UpdateCatalogOptions {
        locale: config.locale,
        mode: config.mode,
        obsolete_strategy: ObsoleteStrategy::Keep,
        render: RenderOptions {
            order_by: config.order_by,
            include_origins: config.include_origins,
            print_placeholders_in_comments: PlaceholderCommentMode::default(),
            custom_header_attributes: None,
            po_serialize: config.po_serialize.clone(),
        },
        ..UpdateCatalogOptions::new(config.source_locale, CatalogUpdateInput::default())
    };
    let mut diagnostics = Vec::new();
    export_catalog_content(catalog, &export_options, locale, &mut diagnostics)
}
