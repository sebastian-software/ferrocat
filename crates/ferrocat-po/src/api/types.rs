use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::{ParseError, PoVec};

use super::mt::MachineMetadata;
use super::plural::PluralProfile;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Source origin metadata for an extracted message.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CatalogOrigin {
    /// Path-like source file identifier where the message came from.
    ///
    /// Ferrocat intentionally tracks no line number: line numbers shift on every
    /// edit above a message and add diff and merge churn without identifying
    /// anything the `(msgid, msgctxt)` key does not already.
    pub file: String,
    /// Optional stable scope within the file, such as the enclosing component,
    /// function, class, route handler, or similar named authoring unit.
    ///
    /// Unlike a line number it survives edits, so it adds context for
    /// translators and tools without churn. It is metadata, not message
    /// identity, and it is not a replacement for gettext context / `msgctxt`.
    /// Producers (e.g. extractors) fill it with values like `CheckoutButton`,
    /// `formatInvoiceStatus`, or `SettingsPage`; serialized with the file as
    /// `file#scope`.
    pub scope: Option<String>,
}

/// Structured singular message input used by catalog update operations.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtractedSingularMessage {
    /// Source message identifier.
    pub msgid: String,
    /// Optional gettext message context.
    pub msgctxt: Option<String>,
    /// Extracted comments that should become translator-facing guidance.
    pub comments: Vec<String>,
    /// Source locations collected by the extractor.
    pub origin: Vec<CatalogOrigin>,
    /// Placeholder hints keyed by placeholder name.
    pub placeholders: BTreeMap<String, Vec<String>>,
}

/// Source-side plural forms for structured catalog messages.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct PluralSource {
    /// Singular source form, when one exists separately from `other`.
    pub one: Option<String>,
    /// Required plural catch-all source form.
    pub other: String,
}

/// Structured plural message input used by catalog update operations.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtractedPluralMessage {
    /// Stable source identifier for the message family.
    pub msgid: String,
    /// Optional gettext message context.
    pub msgctxt: Option<String>,
    /// Structured source-side plural forms.
    pub source: PluralSource,
    /// Extracted comments that should become translator-facing guidance.
    pub comments: Vec<String>,
    /// Source locations collected by the extractor.
    pub origin: Vec<CatalogOrigin>,
    /// Placeholder hints keyed by placeholder name.
    pub placeholders: BTreeMap<String, Vec<String>>,
}

/// Structured extractor input accepted by [`super::update_catalog`] and [`super::update_catalog_file`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractedMessage {
    /// Message that has a single source/translation value.
    Singular(ExtractedSingularMessage),
    /// Message that carries structured plural source forms.
    Plural(ExtractedPluralMessage),
}

/// Source-first extractor input that lets `ferrocat` infer plural structure.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceExtractedMessage {
    /// Source message text used both as identifier and source value.
    pub msgid: String,
    /// Optional gettext message context.
    pub msgctxt: Option<String>,
    /// Extracted comments that should become translator-facing guidance.
    pub comments: Vec<String>,
    /// Source locations collected by the extractor.
    pub origin: Vec<CatalogOrigin>,
    /// Placeholder hints keyed by placeholder name.
    pub placeholders: BTreeMap<String, Vec<String>>,
}

/// Input payload accepted by catalog update operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogUpdateInput {
    /// Pre-projected singular/plural messages.
    Structured(Vec<ExtractedMessage>),
    /// Source-first messages that let `ferrocat` infer plural structure.
    SourceFirst(Vec<SourceExtractedMessage>),
}

impl Default for CatalogUpdateInput {
    fn default() -> Self {
        Self::Structured(Vec::new())
    }
}

impl From<Vec<ExtractedMessage>> for CatalogUpdateInput {
    fn from(value: Vec<ExtractedMessage>) -> Self {
        Self::Structured(value)
    }
}

impl From<Vec<SourceExtractedMessage>> for CatalogUpdateInput {
    fn from(value: Vec<SourceExtractedMessage>) -> Self {
        Self::SourceFirst(value)
    }
}

/// Public translation shape returned from parsed catalogs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "snake_case"))]
pub enum TranslationShape {
    /// Message represented by a single string value.
    Singular {
        /// The current translation value.
        value: String,
    },
    /// Message represented by structured plural categories.
    Plural {
        /// Source-side plural forms.
        source: PluralSource,
        /// Translation values keyed by plural category.
        translation: BTreeMap<String, String>,
        /// Variable name used when re-synthesizing ICU plural strings.
        variable: String,
    },
}

/// Borrowed view over a message translation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectiveTranslationRef<'a> {
    /// Singular translation borrowed from the parsed catalog.
    Singular(&'a str),
    /// Plural translation borrowed from the parsed catalog.
    Plural(&'a BTreeMap<String, String>),
}

/// Owned translation value materialized from a parsed catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectiveTranslation {
    /// Singular translation value.
    Singular(String),
    /// Plural translation values keyed by category.
    Plural(BTreeMap<String, String>),
}

/// Public message representation returned by [`super::parse_catalog`].
///
/// Not `Eq`: AI confidence is a float ([`MachineMetadata`]).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CatalogMessage {
    /// Source message identifier.
    pub msgid: String,
    /// Optional gettext message context.
    pub msgctxt: Option<String>,
    /// Public translation representation.
    pub translation: TranslationShape,
    /// Free-text notes for translators (developer- or translator-provided).
    pub comments: Vec<String>,
    /// Source origins preserved from PO references.
    pub origin: PoVec<CatalogOrigin>,
    /// Obsolete state: `None` when active, `Some` when the message is obsolete
    /// (with an optional write-once `since` date, see [`ObsoleteInfo`]).
    pub obsolete: Option<ObsoleteInfo>,
    /// Optional metadata when the current value is machine-managed (see
    /// [`MachineMetadata`]).
    pub machine: Option<MachineMetadata>,
}

/// Obsolete-entry payload. Presence on [`CatalogMessage::obsolete`] marks the
/// entry obsolete; `since` records when it became obsolete so hosts can
/// age-cleanup with [`ObsoleteStrategy::DropObsoleteBefore`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ObsoleteInfo {
    /// Write-once ISO-8601 date the entry became obsolete, set from the host's
    /// injected clock. `None` when the obsolescence date is unknown.
    pub since: Option<String>,
}

impl CatalogMessage {
    /// Returns the lookup key for this message.
    #[must_use]
    pub fn key(&self) -> CatalogMessageKey {
        CatalogMessageKey {
            msgid: self.msgid.clone(),
            msgctxt: self.msgctxt.clone(),
        }
    }

    /// Returns the effective translation without source-locale fallback.
    #[must_use]
    pub fn effective_translation(&self) -> EffectiveTranslationRef<'_> {
        match &self.translation {
            TranslationShape::Singular { value } => EffectiveTranslationRef::Singular(value),
            TranslationShape::Plural { translation, .. } => {
                EffectiveTranslationRef::Plural(translation)
            }
        }
    }

    pub(super) fn effective_translation_owned(&self) -> EffectiveTranslation {
        match &self.translation {
            TranslationShape::Singular { value } => EffectiveTranslation::Singular(value.clone()),
            TranslationShape::Plural { translation, .. } => {
                EffectiveTranslation::Plural(translation.clone())
            }
        }
    }

    /// Applies the source-locale fallback semantics used by compilation and
    /// runtime artifact generation.
    ///
    /// Singular messages fall back to `msgid` when empty. Plural messages keep
    /// their category shape and only fill categories that are missing or empty.
    pub(super) fn source_fallback_translation(&self, locale: Option<&str>) -> EffectiveTranslation {
        match &self.translation {
            TranslationShape::Singular { value } => {
                if value.is_empty() {
                    EffectiveTranslation::Singular(self.msgid.clone())
                } else {
                    EffectiveTranslation::Singular(value.clone())
                }
            }
            TranslationShape::Plural {
                source,
                translation,
                ..
            } => {
                let profile = PluralProfile::for_locale(locale);
                let mut effective = profile.materialize_translation(translation);
                for category in profile.categories() {
                    let should_fill = effective.get(category).is_none_or(String::is_empty);
                    if should_fill {
                        effective.insert(
                            category.clone(),
                            profile.source_locale_value(category, source),
                        );
                    }
                }
                EffectiveTranslation::Plural(effective)
            }
        }
    }
}

/// Stable lookup key for catalog messages.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CatalogMessageKey {
    /// Source message identifier.
    pub msgid: String,
    /// Optional gettext message context.
    pub msgctxt: Option<String>,
}

impl CatalogMessageKey {
    /// Creates a message key from `msgid` and optional context.
    #[must_use]
    pub fn new(msgid: impl Into<String>, msgctxt: Option<String>) -> Self {
        Self {
            msgid: msgid.into(),
            msgctxt,
        }
    }
}

/// Severity level attached to a [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum DiagnosticSeverity {
    /// Informational message that does not indicate a problem.
    Info,
    /// Non-fatal condition that may require user attention.
    Warning,
    /// Serious condition associated with invalid input or unsupported output.
    Error,
}

/// Non-fatal issue collected while parsing or updating catalogs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Diagnostic {
    /// Severity level for the diagnostic.
    pub severity: DiagnosticSeverity,
    /// Stable machine-readable code for the diagnostic.
    pub code: String,
    /// Human-readable explanation of the condition.
    pub message: String,
    /// Source `msgid`, when the diagnostic can be tied to one message.
    pub msgid: Option<String>,
    /// Source `msgctxt`, when the diagnostic can be tied to one message.
    pub msgctxt: Option<String>,
}

impl Diagnostic {
    pub(super) fn new(
        severity: DiagnosticSeverity,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
            msgid: None,
            msgctxt: None,
        }
    }

    pub(super) fn with_identity(mut self, msgid: &str, msgctxt: Option<&str>) -> Self {
        self.msgid = Some(msgid.to_owned());
        self.msgctxt = msgctxt.map(str::to_owned);
        self
    }
}

/// Basic counters describing an update operation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CatalogStats {
    /// Total messages in the final catalog.
    pub total: usize,
    /// Messages added during the update.
    pub added: usize,
    /// Existing messages whose rendered representation changed.
    pub changed: usize,
    /// Existing messages preserved without changes.
    pub unchanged: usize,
    /// Messages newly marked obsolete.
    pub obsolete_marked: usize,
    /// Messages removed because the obsolete strategy deleted them.
    pub obsolete_removed: usize,
}

/// Result returned by catalog update operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogUpdateResult {
    /// Final PO content after applying the update.
    pub content: String,
    /// Whether the update created a new catalog from scratch.
    pub created: bool,
    /// Whether the final content differs from the original input.
    pub updated: bool,
    /// Summary counters for the operation.
    pub stats: CatalogStats,
    /// Non-fatal diagnostics collected during processing.
    pub diagnostics: Vec<Diagnostic>,
}

/// One catalog input passed to [`super::combine_catalogs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogCombineInput<'a> {
    /// Catalog content to parse and include in the combine operation.
    pub content: &'a str,
    /// Optional human-readable label used in diagnostics.
    pub label: Option<&'a str>,
}

impl<'a> CatalogCombineInput<'a> {
    /// Creates a combine input without a diagnostic label.
    #[must_use]
    pub const fn new(content: &'a str) -> Self {
        Self {
            content,
            label: None,
        }
    }

    /// Creates a combine input with a diagnostic label.
    #[must_use]
    pub const fn labeled(content: &'a str, label: &'a str) -> Self {
        Self {
            content,
            label: Some(label),
        }
    }
}

/// Strategy used when multiple catalogs define conflicting translations for one identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CatalogConflictStrategy {
    /// Keep the first non-empty translation encountered for each `msgid`/`msgctxt`.
    #[default]
    UseFirst,
    /// Replace the current non-empty translation with the latest non-empty definition.
    UseLast,
    /// Return an error when two non-empty translations differ.
    Error,
}

/// Selection rule used after definitions from all inputs have been counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CatalogCombineSelection {
    /// Keep every message identity.
    #[default]
    All,
    /// Keep identities with more than the provided number of definitions.
    MoreThan(usize),
    /// Keep identities with less than the provided number of definitions.
    LessThan(usize),
    /// Keep identities defined only once.
    Unique,
}

impl CatalogCombineSelection {
    pub(super) const fn includes(self, definitions: usize) -> bool {
        match self {
            Self::All => true,
            Self::MoreThan(limit) => definitions > limit,
            Self::LessThan(limit) => definitions < limit,
            Self::Unique => definitions < 2,
        }
    }
}

/// Basic counters describing a catalog combine operation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CatalogCombineStats {
    /// Number of input catalogs parsed.
    pub inputs: usize,
    /// Total message definitions considered after obsolete filtering.
    pub definitions: usize,
    /// Message identities written to the final catalog.
    pub selected: usize,
    /// Message identities removed by the selection rule.
    pub skipped: usize,
    /// Translation conflicts resolved according to the selected strategy.
    pub conflicts_resolved: usize,
    /// Total messages in the final catalog.
    pub total: usize,
}

/// Result returned by catalog combine operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogCombineResult {
    /// Final catalog content after combining the inputs.
    pub content: String,
    /// Summary counters for the operation.
    pub stats: CatalogCombineStats,
    /// Non-fatal diagnostics collected during processing.
    pub diagnostics: Vec<Diagnostic>,
}

/// File format used by disk-based catalog combine operations.
///
/// This enum is non-exhaustive because Ferrocat can add additional catalog
/// file formats over time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum CatalogFileFormat {
    /// Classic gettext PO catalog files, including gettext template (`.pot`) files.
    #[default]
    Po,
    /// Ferrocat Catalog Lines (`.fcl`) files.
    Fcl,
}

impl CatalogFileFormat {
    /// Infers a catalog file format from a path extension.
    ///
    /// Supported path suffixes are `.po`, `.pot`, and `.fcl`.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Unsupported`] when the path suffix does not map to a
    /// supported catalog file format.
    pub fn infer_from_path(path: &Path) -> Result<Self, ApiError> {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        if name.ends_with(".po") || name.ends_with(".pot") {
            return Ok(Self::Po);
        }
        if name.ends_with(".fcl") {
            return Ok(Self::Fcl);
        }

        Err(ApiError::Unsupported(format!(
            "could not infer catalog file format from `{}`; expected .po, .pot, or .fcl",
            path.display()
        )))
    }

    pub(super) const fn default_mode(self) -> CatalogMode {
        match self {
            Self::Po => CatalogMode::IcuPo,
            Self::Fcl => CatalogMode::IcuFcl,
        }
    }
}

/// Options for combining catalog files on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombineCatalogFilesOptions<'a> {
    /// Input catalog paths in precedence order.
    pub input_paths: &'a [PathBuf],
    /// Output catalog path to atomically replace after a successful combine.
    pub output_path: &'a Path,
    /// Optional explicit file format. When `None`, Ferrocat infers it from the
    /// input and output paths and requires all inferred formats to match.
    pub format: Option<CatalogFileFormat>,
    /// Optional high-level catalog mode. When `None`, Ferrocat chooses
    /// `CatalogMode::IcuPo` for PO files and `CatalogMode::IcuFcl` for FCL files.
    pub mode: Option<CatalogMode>,
    /// Locale of the combined catalog. When `None`, Ferrocat uses the first input locale if present.
    pub locale: Option<&'a str>,
    /// Source locale used for source-side semantics and validation.
    pub source_locale: &'a str,
    /// Strategy for resolving conflicting non-empty translations.
    /// Empty template translations never clear non-empty values.
    pub conflict_strategy: CatalogConflictStrategy,
    /// Message identity selection rule applied after all inputs are read.
    pub selection: CatalogCombineSelection,
    /// Sort order for the final rendered catalog.
    pub order_by: OrderBy,
    /// Whether source origins should be rendered as references.
    pub include_origins: bool,
    /// Whether obsolete definitions should participate in the combine operation.
    pub include_obsolete: bool,
}

impl<'a> CombineCatalogFilesOptions<'a> {
    /// Creates file combine options with required fields set.
    ///
    /// Optional fields use the same defaults as [`CombineCatalogOptions`].
    #[must_use]
    pub fn new(input_paths: &'a [PathBuf], output_path: &'a Path, source_locale: &'a str) -> Self {
        Self {
            input_paths,
            output_path,
            format: None,
            mode: None,
            locale: None,
            source_locale,
            conflict_strategy: CatalogConflictStrategy::UseFirst,
            selection: CatalogCombineSelection::All,
            order_by: OrderBy::Msgid,
            include_origins: true,
            include_obsolete: false,
        }
    }
}

/// Result returned by catalog file combine operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogFileCombineResult {
    /// Output path replaced by the operation.
    pub output_path: PathBuf,
    /// File format used for reading inputs and writing the output.
    pub format: CatalogFileFormat,
    /// Summary counters for the operation.
    pub stats: CatalogCombineStats,
    /// Non-fatal diagnostics collected during processing.
    pub diagnostics: Vec<Diagnostic>,
}

/// Parsed catalog plus diagnostics and normalized headers.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCatalog {
    /// Declared or overridden catalog locale.
    pub locale: Option<String>,
    /// High-level semantics used to parse the catalog.
    pub semantics: CatalogSemantics,
    /// Normalized header map keyed by header name.
    pub headers: BTreeMap<String, String>,
    /// Parsed catalog messages in source order.
    pub messages: Vec<CatalogMessage>,
    /// Non-fatal diagnostics collected while parsing.
    pub diagnostics: Vec<Diagnostic>,
}

impl ParsedCatalog {
    /// Builds a lookup-oriented view that rejects duplicate message keys.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Conflict`] when the parsed catalog contains
    /// duplicate `msgid`/`msgctxt` pairs.
    pub fn into_normalized_view(self) -> Result<NormalizedParsedCatalog, ApiError> {
        NormalizedParsedCatalog::new(self)
    }
}

/// Parsed catalog with fast key-based lookup helpers.
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedParsedCatalog {
    pub(super) catalog: ParsedCatalog,
    pub(super) key_index: BTreeMap<CatalogMessageKey, usize>,
    msgid_index: BTreeMap<String, Vec<usize>>,
}

impl NormalizedParsedCatalog {
    /// Builds the lookup index once and rejects duplicate gettext identities up front.
    pub(super) fn new(catalog: ParsedCatalog) -> Result<Self, ApiError> {
        let mut key_index = BTreeMap::new();
        let mut msgid_index = BTreeMap::<String, Vec<usize>>::new();
        for (index, message) in catalog.messages.iter().enumerate() {
            let key = message.key();
            if key_index.insert(key.clone(), index).is_some() {
                return Err(ApiError::Conflict(format!(
                    "duplicate parsed catalog message for msgid {:?} and context {:?}",
                    key.msgid, key.msgctxt
                )));
            }
            msgid_index.entry(key.msgid).or_default().push(index);
        }
        Ok(Self {
            catalog,
            key_index,
            msgid_index,
        })
    }

    /// Returns the underlying parsed catalog.
    #[must_use]
    pub const fn parsed_catalog(&self) -> &ParsedCatalog {
        &self.catalog
    }

    /// Consumes the normalized view and returns the underlying parsed catalog.
    #[must_use]
    pub fn into_parsed_catalog(self) -> ParsedCatalog {
        self.catalog
    }

    /// Returns a message by key.
    #[must_use]
    pub fn get(&self, key: &CatalogMessageKey) -> Option<&CatalogMessage> {
        self.key_index
            .get(key)
            .map(|index| &self.catalog.messages[*index])
    }

    /// Returns a message by borrowed `msgid` and optional context parts.
    ///
    /// This avoids constructing an owned [`CatalogMessageKey`] when callers
    /// already have borrowed source identity fields.
    #[must_use]
    pub fn get_by_parts(&self, msgid: &str, msgctxt: Option<&str>) -> Option<&CatalogMessage> {
        self.msgid_index.get(msgid)?.iter().find_map(|index| {
            let message = &self.catalog.messages[*index];
            (message.msgctxt.as_deref() == msgctxt).then_some(message)
        })
    }

    /// Returns `true` if a message for `key` exists.
    #[must_use]
    pub fn contains_key(&self, key: &CatalogMessageKey) -> bool {
        self.key_index.contains_key(key)
    }

    /// Returns `true` if a message exists for borrowed `msgid` and context parts.
    #[must_use]
    pub fn contains_parts(&self, msgid: &str, msgctxt: Option<&str>) -> bool {
        self.get_by_parts(msgid, msgctxt).is_some()
    }

    /// Returns the number of indexed messages.
    #[must_use]
    pub fn message_count(&self) -> usize {
        self.catalog.messages.len()
    }

    /// Iterates over all indexed messages in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&CatalogMessageKey, &CatalogMessage)> + '_ {
        self.key_index
            .iter()
            .map(|(key, index)| (key, &self.catalog.messages[*index]))
    }

    /// Returns the effective translation for `key`, if present.
    pub fn effective_translation(
        &self,
        key: &CatalogMessageKey,
    ) -> Option<EffectiveTranslationRef<'_>> {
        self.get(key).map(CatalogMessage::effective_translation)
    }

    /// Returns the effective translation for borrowed `msgid` and context parts.
    pub fn effective_translation_by_parts(
        &self,
        msgid: &str,
        msgctxt: Option<&str>,
    ) -> Option<EffectiveTranslationRef<'_>> {
        self.get_by_parts(msgid, msgctxt)
            .map(CatalogMessage::effective_translation)
    }

    /// Returns the effective translation and fills empty source-locale values
    /// from the source text when appropriate.
    #[must_use]
    pub fn effective_translation_with_source_fallback(
        &self,
        key: &CatalogMessageKey,
        source_locale: &str,
    ) -> Option<EffectiveTranslation> {
        let message = self.get(key)?;
        Some(self.effective_translation_for_message(message, source_locale))
    }

    /// Returns the effective translation for borrowed identity parts and fills
    /// empty source-locale values from the source text when appropriate.
    #[must_use]
    pub fn effective_translation_with_source_fallback_by_parts(
        &self,
        msgid: &str,
        msgctxt: Option<&str>,
        source_locale: &str,
    ) -> Option<EffectiveTranslation> {
        let message = self.get_by_parts(msgid, msgctxt)?;
        Some(self.effective_translation_for_message(message, source_locale))
    }

    fn effective_translation_for_message(
        &self,
        message: &CatalogMessage,
        source_locale: &str,
    ) -> EffectiveTranslation {
        if self
            .catalog
            .locale
            .as_deref()
            .is_none_or(|locale| locale == source_locale)
        {
            message.source_fallback_translation(self.catalog.locale.as_deref())
        } else {
            message.effective_translation_owned()
        }
    }
}

/// Encoding used for plural messages in PO files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PluralEncoding {
    /// Keep plural messages in Ferrocat's structured ICU-oriented representation.
    #[default]
    Icu,
    /// Materialize plural messages as classic gettext `msgid_plural` plus `msgstr[n]`.
    Gettext,
}

/// Storage format used by the high-level catalog API.
///
/// This enum is non-exhaustive because Ferrocat can add additional catalog
/// storage formats over time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum CatalogStorageFormat {
    /// Read and write classic gettext PO catalogs.
    #[default]
    Po,
    /// Read and write Ferrocat Catalog Lines (`.fcl`): a line-oriented,
    /// git-merge-optimized, machine-owned catalog format.
    Fcl,
}

/// High-level semantics used by the catalog API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CatalogSemantics {
    /// ICU-native semantics with raw ICU/text messages as the primary representation.
    #[default]
    IcuNative,
    /// Classic gettext plural semantics used for PO compatibility workflows.
    GettextCompat,
}

/// ICU parser behavior used by catalog audit and runtime artifact validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum IcuSyntaxPolicy {
    /// Parse ICU MessageFormat v1 with Ferrocat's strict apostrophe rules.
    #[default]
    Strict,
    /// Treat ordinary literal apostrophes as runtime-valid text before parsing.
    ///
    /// Use this when a downstream runtime accepts messages such as `you're`
    /// and `We've got {count, plural, one {...} other {...}}` without requiring
    /// translators to double every literal apostrophe.
    /// Callers that rely on ICU apostrophe quoting should keep [`Self::Strict`].
    RuntimeLiteralApostrophes,
}

/// Valid high-level catalog mode combinations.
///
/// This type groups the storage format, semantic model, and plural encoding
/// choices that must be kept in sync for catalog parse, update, and combine
/// operations. It is non-exhaustive so future supported catalog modes can be
/// added without breaking downstream matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum CatalogMode {
    /// Gettext PO storage with ICU-native message semantics.
    #[default]
    IcuPo,
    /// Gettext PO storage with classic gettext plural semantics.
    GettextPo,
    /// FCL (Ferrocat Catalog Lines) storage with ICU-native message semantics.
    IcuFcl,
}

impl CatalogMode {
    /// Returns the storage format implied by this mode.
    #[must_use]
    pub const fn storage_format(self) -> CatalogStorageFormat {
        match self {
            Self::IcuPo | Self::GettextPo => CatalogStorageFormat::Po,
            Self::IcuFcl => CatalogStorageFormat::Fcl,
        }
    }

    /// Returns the catalog semantics implied by this mode.
    #[must_use]
    pub const fn semantics(self) -> CatalogSemantics {
        match self {
            Self::IcuPo | Self::IcuFcl => CatalogSemantics::IcuNative,
            Self::GettextPo => CatalogSemantics::GettextCompat,
        }
    }

    /// Returns the plural encoding implied by this mode.
    #[must_use]
    pub const fn plural_encoding(self) -> PluralEncoding {
        match self {
            Self::IcuPo | Self::IcuFcl => PluralEncoding::Icu,
            Self::GettextPo => PluralEncoding::Gettext,
        }
    }

    /// Returns the matching catalog mode for explicit parts.
    #[must_use]
    pub const fn from_parts(
        storage_format: CatalogStorageFormat,
        semantics: CatalogSemantics,
        plural_encoding: PluralEncoding,
    ) -> Option<Self> {
        match (storage_format, semantics, plural_encoding) {
            (CatalogStorageFormat::Po, CatalogSemantics::IcuNative, PluralEncoding::Icu) => {
                Some(Self::IcuPo)
            }
            (CatalogStorageFormat::Fcl, CatalogSemantics::IcuNative, PluralEncoding::Icu) => {
                Some(Self::IcuFcl)
            }
            (
                CatalogStorageFormat::Po,
                CatalogSemantics::GettextCompat,
                PluralEncoding::Gettext,
            ) => Some(Self::GettextPo),
            _ => None,
        }
    }
}

/// Strategy used for messages that disappear from the extracted input.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ObsoleteStrategy {
    /// Mark missing messages obsolete and keep them in the file.
    #[default]
    Mark,
    /// Remove missing messages entirely.
    Delete,
    /// Keep missing messages as active entries.
    Keep,
    /// Mark missing messages obsolete (like [`ObsoleteStrategy::Mark`]) and
    /// additionally drop any obsolete entry whose `since` date predates the given
    /// ISO-8601 cutoff. Undated obsolete entries are kept. The host computes the
    /// cutoff (e.g. today minus 90 days); ISO dates compare lexicographically, so
    /// no date arithmetic happens inside Ferrocat. See
    /// [ADR 0025](https://ferrocat.dev/architecture/adr/0025-obsolete-age-and-cleanup).
    DropObsoleteBefore(String),
}

/// Sort order used when writing output catalogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrderBy {
    /// Sort by `msgid` then context.
    #[default]
    Msgid,
    /// Sort by the first source origin, then by message identity.
    Origin,
}

/// Controls whether placeholder hints are emitted as extracted comments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaceholderCommentMode {
    /// Do not emit placeholder comments.
    Disabled,
    /// Emit up to `limit` placeholder comments per placeholder name.
    Enabled {
        /// Maximum number of values rendered per placeholder name.
        limit: usize,
    },
}

impl Default for PlaceholderCommentMode {
    fn default() -> Self {
        Self::Enabled { limit: 3 }
    }
}

/// Shared rendering options for catalog serialization.
///
/// These fields control how a catalog is sorted and which optional reference and
/// placeholder details are written. Some storage formats still impose their own
/// invariants: FCL always renders in canonical `(id, ctxt)` order, while origin
/// and placeholder detail flags apply across supported catalog storage formats.
///
/// References render the source file only; Ferrocat does not track or emit line
/// numbers (see [`CatalogOrigin`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderOptions<'a> {
    /// Sort order for the final rendered catalog.
    pub order_by: OrderBy,
    /// Whether source origins should be rendered as references.
    pub include_origins: bool,
    /// Controls emission of placeholder comments.
    pub print_placeholders_in_comments: PlaceholderCommentMode,
    /// Optional additional header attributes to inject or override.
    pub custom_header_attributes: Option<&'a BTreeMap<String, String>>,
}

impl Default for RenderOptions<'_> {
    fn default() -> Self {
        Self {
            order_by: OrderBy::Msgid,
            include_origins: true,
            print_placeholders_in_comments: PlaceholderCommentMode::Enabled { limit: 3 },
            custom_header_attributes: None,
        }
    }
}

impl<'a> RenderOptions<'a> {
    /// Returns options that render messages with the given sort order.
    #[must_use]
    pub fn with_order_by(mut self, order_by: OrderBy) -> Self {
        self.order_by = order_by;
        self
    }

    /// Returns options that enable or disable rendered source origins.
    #[must_use]
    pub fn with_include_origins(mut self, include_origins: bool) -> Self {
        self.include_origins = include_origins;
        self
    }

    /// Returns options that use the given placeholder comment mode.
    #[must_use]
    pub fn with_placeholder_comments(
        mut self,
        print_placeholders_in_comments: PlaceholderCommentMode,
    ) -> Self {
        self.print_placeholders_in_comments = print_placeholders_in_comments;
        self
    }

    /// Returns options that inject or override the given header attributes.
    #[must_use]
    pub fn with_custom_header_attributes(
        mut self,
        custom_header_attributes: &'a BTreeMap<String, String>,
    ) -> Self {
        self.custom_header_attributes = Some(custom_header_attributes);
        self
    }
}

/// Options for in-memory catalog updates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCatalogOptions<'a> {
    /// Locale of the catalog being updated. When `None`, Ferrocat infers it from the existing file.
    pub locale: Option<&'a str>,
    /// Source locale used for source-side semantics and fallback handling.
    pub source_locale: &'a str,
    /// Extracted messages to merge into the catalog.
    pub input: CatalogUpdateInput,
    /// Existing catalog content, when updating an in-memory catalog.
    pub existing: Option<&'a str>,
    /// High-level catalog mode used when parsing, merging, and rendering the catalog.
    pub mode: CatalogMode,
    /// Strategy for messages absent from the extracted input.
    pub obsolete_strategy: ObsoleteStrategy,
    /// Whether source-locale translations should be refreshed from the extracted source strings.
    pub overwrite_source_translations: bool,
    /// Optional host-provided clock as an ISO-8601 date. When set, entries newly
    /// transitioning to obsolete are stamped with this `since` date (write-once).
    /// Ferrocat never reads a clock itself, so updates stay deterministic given
    /// their inputs. See [ADR 0025](https://ferrocat.dev/architecture/adr/0025-obsolete-age-and-cleanup).
    pub now: Option<&'a str>,
    /// Shared serialization options for the rendered catalog.
    pub render: RenderOptions<'a>,
}

impl<'a> UpdateCatalogOptions<'a> {
    /// Creates in-memory update options with required fields set.
    ///
    /// Optional fields default to an inferred locale, ICU-native PO mode,
    /// marking missing messages obsolete, preserving source-locale translations,
    /// no host clock, and [`RenderOptions::default`].
    #[must_use]
    pub fn new(source_locale: &'a str, input: impl Into<CatalogUpdateInput>) -> Self {
        Self {
            locale: None,
            source_locale,
            input: input.into(),
            existing: None,
            mode: CatalogMode::default(),
            obsolete_strategy: ObsoleteStrategy::Mark,
            overwrite_source_translations: false,
            now: None,
            render: RenderOptions::default(),
        }
    }

    /// Returns options that use the given catalog locale.
    #[must_use]
    pub fn with_locale(mut self, locale: &'a str) -> Self {
        self.locale = Some(locale);
        self
    }

    /// Returns options that update the given existing catalog content.
    #[must_use]
    pub fn with_existing(mut self, existing: &'a str) -> Self {
        self.existing = Some(existing);
        self
    }

    /// Returns options that parse, merge, and render with the given catalog mode.
    #[must_use]
    pub fn with_mode(mut self, mode: CatalogMode) -> Self {
        self.mode = mode;
        self
    }

    /// Returns options that render the updated catalog with the given options.
    #[must_use]
    pub fn with_render(mut self, render: RenderOptions<'a>) -> Self {
        self.render = render;
        self
    }

    /// Returns options that handle missing extracted messages with the given strategy.
    #[must_use]
    pub fn with_obsolete_strategy(mut self, obsolete_strategy: ObsoleteStrategy) -> Self {
        self.obsolete_strategy = obsolete_strategy;
        self
    }

    /// Returns options that enable or disable source-locale translation refreshes.
    #[must_use]
    pub fn with_overwrite_source_translations(
        mut self,
        overwrite_source_translations: bool,
    ) -> Self {
        self.overwrite_source_translations = overwrite_source_translations;
        self
    }

    /// Returns options that stamp newly obsolete entries with the given ISO date.
    #[must_use]
    pub fn with_now(mut self, now: &'a str) -> Self {
        self.now = Some(now);
        self
    }
}

/// Options for updating a catalog file on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCatalogFileOptions<'a> {
    /// Path to the catalog file that should be read and conditionally written.
    pub target_path: &'a Path,
    /// In-memory update options applied to the file content.
    pub options: UpdateCatalogOptions<'a>,
}

impl<'a> UpdateCatalogFileOptions<'a> {
    /// Creates file update options with required fields set.
    ///
    /// Optional fields on the nested update options use
    /// [`UpdateCatalogOptions::new`] defaults.
    #[must_use]
    pub fn new(
        target_path: &'a Path,
        source_locale: &'a str,
        input: impl Into<CatalogUpdateInput>,
    ) -> Self {
        Self {
            target_path,
            options: UpdateCatalogOptions::new(source_locale, input),
        }
    }
}

/// Options for combining multiple catalogs into one deterministic catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombineCatalogOptions<'a> {
    /// Input catalogs in precedence order.
    pub inputs: &'a [CatalogCombineInput<'a>],
    /// Locale of the combined catalog. When `None`, Ferrocat uses the first input locale if present.
    pub locale: Option<&'a str>,
    /// Source locale used for source-side semantics and validation.
    pub source_locale: &'a str,
    /// High-level catalog mode used when reading inputs and rendering the result.
    pub mode: CatalogMode,
    /// Strategy for resolving conflicting non-empty translations.
    /// Empty template translations never clear non-empty values.
    pub conflict_strategy: CatalogConflictStrategy,
    /// Message identity selection rule applied after all inputs are read.
    pub selection: CatalogCombineSelection,
    /// Sort order for the final rendered catalog.
    pub order_by: OrderBy,
    /// Whether source origins should be rendered as references.
    pub include_origins: bool,
    /// Whether obsolete definitions should participate in the combine operation.
    pub include_obsolete: bool,
}

impl<'a> CombineCatalogOptions<'a> {
    /// Creates combine options with required fields set.
    ///
    /// Optional fields default to an inferred locale, ICU-native PO mode,
    /// first-definition conflict resolution, all message identities, `msgid`
    /// ordering, rendered origins, and skipped obsolete entries.
    #[must_use]
    pub fn new(inputs: &'a [CatalogCombineInput<'a>], source_locale: &'a str) -> Self {
        Self {
            inputs,
            locale: None,
            source_locale,
            mode: CatalogMode::default(),
            conflict_strategy: CatalogConflictStrategy::UseFirst,
            selection: CatalogCombineSelection::All,
            order_by: OrderBy::Msgid,
            include_origins: true,
            include_obsolete: false,
        }
    }
}

/// Options for parsing a catalog into the higher-level message model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseCatalogOptions<'a> {
    /// Catalog content to parse.
    pub content: &'a str,
    /// Optional explicit locale override.
    pub locale: Option<&'a str>,
    /// Source locale used for source-side semantics and validation.
    pub source_locale: &'a str,
    /// High-level catalog mode used when interpreting catalog content.
    pub mode: CatalogMode,
    /// Whether unsupported ICU plural projection cases should become hard errors.
    pub strict: bool,
}

impl<'a> ParseCatalogOptions<'a> {
    /// Creates parse options with required fields set.
    ///
    /// Optional fields default to an inferred locale, ICU-native PO mode, and
    /// non-strict plural projection.
    #[must_use]
    pub fn new(content: &'a str, source_locale: &'a str) -> Self {
        Self {
            content,
            locale: None,
            source_locale,
            mode: CatalogMode::default(),
            strict: false,
        }
    }

    /// Returns options that use the given explicit catalog locale.
    #[must_use]
    pub fn with_locale(mut self, locale: &'a str) -> Self {
        self.locale = Some(locale);
        self
    }

    /// Returns options that interpret catalog content with the given mode.
    #[must_use]
    pub fn with_mode(mut self, mode: CatalogMode) -> Self {
        self.mode = mode;
        self
    }

    /// Returns options that enable or disable strict plural projection.
    #[must_use]
    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }
}

/// Error returned by catalog parsing and update APIs.
///
/// This enum is non-exhaustive so new API-level failure categories can be added
/// without turning downstream error matches into a breaking change.
#[derive(Debug)]
#[non_exhaustive]
pub enum ApiError {
    /// Underlying PO parse or string-unescape failure.
    Parse(ParseError),
    /// Filesystem failure raised by disk-based helpers.
    Io(std::io::Error),
    /// Caller-supplied arguments were missing, inconsistent, or invalid.
    InvalidArguments(String),
    /// The requested operation encountered conflicting catalog state.
    Conflict(String),
    /// The requested behavior cannot be represented safely.
    Unsupported(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => error.fmt(f),
            Self::Io(error) => error.fmt(f),
            Self::InvalidArguments(message)
            | Self::Conflict(message)
            | Self::Unsupported(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::InvalidArguments(_) | Self::Conflict(_) | Self::Unsupported(_) => None,
        }
    }
}

impl ApiError {
    /// Creates a filesystem error with path context.
    #[must_use]
    pub fn io_with_path(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        let kind = source.kind();
        Self::Io(std::io::Error::new(
            kind,
            PathIoError {
                path: path.into(),
                source,
            },
        ))
    }

    /// Returns the filesystem path associated with this error, when available.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Io(error) => error
                .get_ref()
                .and_then(|source| source.downcast_ref::<PathIoError>())
                .map(|error| error.path.as_path()),
            Self::Parse(_)
            | Self::InvalidArguments(_)
            | Self::Conflict(_)
            | Self::Unsupported(_) => None,
        }
    }
}

#[derive(Debug)]
struct PathIoError {
    path: PathBuf,
    source: std::io::Error,
}

impl fmt::Display for PathIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "I/O error for `{}`: {}",
            self.path.display(),
            self.source
        )
    }
}

impl std::error::Error for PathIoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

impl From<ParseError> for ApiError {
    fn from(value: ParseError) -> Self {
        Self::Parse(value)
    }
}

impl From<std::io::Error> for ApiError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::error::Error;
    use std::io;
    use std::path::{Path, PathBuf};

    use super::{
        ApiError, CatalogCombineInput, CatalogCombineSelection, CatalogConflictStrategy,
        CatalogFileFormat, CatalogMessage, CatalogMessageKey, CatalogMode, CatalogSemantics,
        CatalogStorageFormat, CatalogUpdateInput, CombineCatalogFilesOptions,
        CombineCatalogOptions, Diagnostic, DiagnosticSeverity, EffectiveTranslation,
        EffectiveTranslationRef, NormalizedParsedCatalog, ObsoleteStrategy, OrderBy,
        ParseCatalogOptions, ParsedCatalog, PlaceholderCommentMode, PluralEncoding, PluralSource,
        RenderOptions, TranslationShape, UpdateCatalogFileOptions, UpdateCatalogOptions,
    };
    use crate::ParseError;

    #[test]
    fn catalog_update_input_defaults_and_conversions_use_expected_variants() {
        assert!(matches!(
            CatalogUpdateInput::default(),
            CatalogUpdateInput::Structured(messages) if messages.is_empty()
        ));
        assert!(matches!(
            CatalogUpdateInput::from(Vec::<super::ExtractedMessage>::new()),
            CatalogUpdateInput::Structured(messages) if messages.is_empty()
        ));
        assert!(matches!(
            CatalogUpdateInput::from(Vec::<super::SourceExtractedMessage>::new()),
            CatalogUpdateInput::SourceFirst(messages) if messages.is_empty()
        ));
    }

    #[test]
    fn catalog_message_helpers_cover_key_and_fallback_behavior() {
        let singular = CatalogMessage {
            msgid: "Hello".to_owned(),
            msgctxt: Some("button".to_owned()),
            translation: TranslationShape::Singular {
                value: String::new(),
            },
            comments: vec!["Shown in toolbar".to_owned()],
            origin: crate::PoVec::new(),
            obsolete: None,
            machine: None,
        };

        assert_eq!(
            singular.key(),
            CatalogMessageKey::new("Hello", Some("button".to_owned()))
        );
        assert!(matches!(
            singular.effective_translation(),
            EffectiveTranslationRef::Singular("")
        ));
        assert_eq!(
            singular.source_fallback_translation(Some("en")),
            EffectiveTranslation::Singular("Hello".to_owned())
        );

        let plural = CatalogMessage {
            msgid: "{count, plural, one {# file} other {# files}}".to_owned(),
            msgctxt: None,
            translation: TranslationShape::Plural {
                source: PluralSource {
                    one: Some("{count} file".to_owned()),
                    other: "{count} files".to_owned(),
                },
                translation: BTreeMap::from([
                    ("one".to_owned(), String::new()),
                    ("other".to_owned(), "{count} Dateien".to_owned()),
                ]),
                variable: "count".to_owned(),
            },
            comments: Vec::new(),
            origin: crate::PoVec::new(),
            obsolete: None,
            machine: None,
        };

        assert!(matches!(
            plural.effective_translation(),
            EffectiveTranslationRef::Plural(values)
                if values.get("other") == Some(&"{count} Dateien".to_owned())
        ));
        assert_eq!(
            plural.source_fallback_translation(Some("de")),
            EffectiveTranslation::Plural(BTreeMap::from([
                ("one".to_owned(), "{count} file".to_owned()),
                ("other".to_owned(), "{count} Dateien".to_owned()),
            ]))
        );
    }

    #[test]
    fn normalized_catalog_helpers_expose_lookup_and_source_fallback_views() {
        let parsed = ParsedCatalog {
            locale: Some("en".to_owned()),
            semantics: CatalogSemantics::IcuNative,
            headers: BTreeMap::new(),
            messages: vec![
                CatalogMessage {
                    msgid: "Hello".to_owned(),
                    msgctxt: None,
                    translation: TranslationShape::Singular {
                        value: String::new(),
                    },
                    comments: Vec::new(),
                    origin: crate::PoVec::new(),
                    obsolete: None,
                    machine: None,
                },
                CatalogMessage {
                    msgid: "Hello".to_owned(),
                    msgctxt: Some("button".to_owned()),
                    translation: TranslationShape::Singular {
                        value: "Howdy".to_owned(),
                    },
                    comments: Vec::new(),
                    origin: crate::PoVec::new(),
                    obsolete: None,
                    machine: None,
                },
            ],
            diagnostics: Vec::new(),
        };

        let normalized = NormalizedParsedCatalog::new(parsed.clone()).expect("normalized");
        let key = CatalogMessageKey::new("Hello", None);

        assert_eq!(normalized.message_count(), 2);
        assert!(normalized.contains_key(&key));
        assert!(normalized.contains_parts("Hello", Some("button")));
        assert_eq!(
            normalized.parsed_catalog().semantics,
            CatalogSemantics::IcuNative
        );
        assert!(normalized.get(&key).is_some());
        assert_eq!(
            normalized
                .get_by_parts("Hello", Some("button"))
                .and_then(|message| match message.effective_translation() {
                    EffectiveTranslationRef::Singular(value) => Some(value),
                    EffectiveTranslationRef::Plural(_) => None,
                }),
            Some("Howdy")
        );
        assert!(matches!(
            normalized.effective_translation_by_parts("Hello", None),
            Some(EffectiveTranslationRef::Singular(""))
        ));
        assert_eq!(
            normalized.effective_translation_with_source_fallback(&key, "en"),
            Some(EffectiveTranslation::Singular("Hello".to_owned()))
        );
        assert_eq!(
            normalized.effective_translation_with_source_fallback_by_parts(
                "Hello",
                Some("button"),
                "en"
            ),
            Some(EffectiveTranslation::Singular("Howdy".to_owned()))
        );
        assert_eq!(normalized.into_parsed_catalog(), parsed);
    }

    #[test]
    fn option_defaults_reflect_native_po_defaults() {
        let update = UpdateCatalogOptions::new("en", Vec::<super::ExtractedMessage>::new());
        assert_eq!(update.mode, CatalogMode::IcuPo);
        assert_eq!(update.obsolete_strategy, ObsoleteStrategy::Mark);
        assert_eq!(update.render.order_by, OrderBy::Msgid);
        assert!(update.render.include_origins);
        assert_eq!(
            update.render.print_placeholders_in_comments,
            PlaceholderCommentMode::Enabled { limit: 3 }
        );
        assert_eq!(update.source_locale, "en");
        assert!(matches!(
            update.input,
            CatalogUpdateInput::Structured(messages) if messages.is_empty()
        ));

        let update_file = UpdateCatalogFileOptions::new(
            Path::new("locale/de.po"),
            "en",
            Vec::<super::SourceExtractedMessage>::new(),
        );
        assert_eq!(update_file.target_path, Path::new("locale/de.po"));
        assert_eq!(update_file.options.mode, CatalogMode::IcuPo);
        assert_eq!(update_file.options.source_locale, "en");
        assert!(matches!(
            update_file.options.input,
            CatalogUpdateInput::SourceFirst(messages) if messages.is_empty()
        ));

        let parse = ParseCatalogOptions::new("msgid \"Hello\"\nmsgstr \"Hallo\"\n", "en");
        assert_eq!(parse.content, "msgid \"Hello\"\nmsgstr \"Hallo\"\n");
        assert_eq!(parse.source_locale, "en");
        assert_eq!(parse.mode, CatalogMode::IcuPo);
        assert!(!parse.strict);

        let inputs = [CatalogCombineInput::labeled(
            "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
            "de.po",
        )];
        let combine = CombineCatalogOptions::new(&inputs, "en");
        assert_eq!(combine.inputs, &inputs);
        assert_eq!(combine.source_locale, "en");
        assert_eq!(combine.mode, CatalogMode::IcuPo);
        assert_eq!(combine.conflict_strategy, CatalogConflictStrategy::UseFirst);
        assert_eq!(combine.selection, CatalogCombineSelection::All);
        assert!(combine.include_origins);
        assert!(!combine.include_obsolete);

        let input_paths = [PathBuf::from("locale/de.po")];
        let combine_files =
            CombineCatalogFilesOptions::new(&input_paths, Path::new("locale/merged.po"), "en");
        assert_eq!(combine_files.input_paths, &input_paths);
        assert_eq!(combine_files.output_path, Path::new("locale/merged.po"));
        assert_eq!(combine_files.format, None);
        assert_eq!(combine_files.mode, None);
        assert_eq!(combine_files.source_locale, "en");
        assert_eq!(
            combine_files.conflict_strategy,
            CatalogConflictStrategy::UseFirst
        );
        assert_eq!(combine_files.selection, CatalogCombineSelection::All);
        assert!(combine_files.include_origins);
        assert!(!combine_files.include_obsolete);
    }

    #[test]
    fn catalog_option_builders_set_fields() {
        let headers = BTreeMap::from([("X-Generator".to_owned(), "ferrocat".to_owned())]);
        let render = RenderOptions::default()
            .with_order_by(OrderBy::Origin)
            .with_include_origins(false)
            .with_placeholder_comments(PlaceholderCommentMode::Disabled)
            .with_custom_header_attributes(&headers);

        assert_eq!(render.order_by, OrderBy::Origin);
        assert!(!render.include_origins);
        assert_eq!(
            render.print_placeholders_in_comments,
            PlaceholderCommentMode::Disabled
        );
        assert_eq!(render.custom_header_attributes, Some(&headers));

        let update = UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
            .with_locale("de")
            .with_existing("msgid \"Hello\"\nmsgstr \"Hallo\"\n")
            .with_mode(CatalogMode::GettextPo)
            .with_render(render.clone())
            .with_obsolete_strategy(ObsoleteStrategy::Delete)
            .with_overwrite_source_translations(true)
            .with_now("2026-07-02");

        assert_eq!(update.locale, Some("de"));
        assert_eq!(update.existing, Some("msgid \"Hello\"\nmsgstr \"Hallo\"\n"));
        assert_eq!(update.mode, CatalogMode::GettextPo);
        assert_eq!(update.render, render);
        assert_eq!(update.obsolete_strategy, ObsoleteStrategy::Delete);
        assert!(update.overwrite_source_translations);
        assert_eq!(update.now, Some("2026-07-02"));

        let parse = ParseCatalogOptions::new("content", "en")
            .with_locale("fr")
            .with_mode(CatalogMode::IcuFcl)
            .with_strict(true);

        assert_eq!(parse.locale, Some("fr"));
        assert_eq!(parse.mode, CatalogMode::IcuFcl);
        assert!(parse.strict);
    }

    #[test]
    fn catalog_file_format_infers_supported_suffixes() {
        assert_eq!(
            CatalogFileFormat::infer_from_path(Path::new("locale/de.po")).expect("po"),
            CatalogFileFormat::Po
        );
        assert_eq!(
            CatalogFileFormat::infer_from_path(Path::new("locale/messages.POT")).expect("pot"),
            CatalogFileFormat::Po
        );
        assert_eq!(
            CatalogFileFormat::infer_from_path(Path::new("locale/de.fcl")).expect("fcl"),
            CatalogFileFormat::Fcl
        );
        assert!(matches!(
            CatalogFileFormat::infer_from_path(Path::new("locale/de.txt")),
            Err(ApiError::Unsupported(message)) if message.contains("could not infer")
        ));
    }

    #[test]
    fn catalog_mode_maps_only_supported_catalog_combinations() {
        assert_eq!(CatalogMode::default(), CatalogMode::IcuPo);
        assert_eq!(
            CatalogMode::IcuPo.storage_format(),
            CatalogStorageFormat::Po
        );
        assert_eq!(CatalogMode::IcuPo.semantics(), CatalogSemantics::IcuNative);
        assert_eq!(CatalogMode::IcuPo.plural_encoding(), PluralEncoding::Icu);

        assert_eq!(
            CatalogMode::from_parts(
                CatalogStorageFormat::Fcl,
                CatalogSemantics::IcuNative,
                PluralEncoding::Icu
            ),
            Some(CatalogMode::IcuFcl)
        );
        assert_eq!(
            CatalogMode::from_parts(
                CatalogStorageFormat::Po,
                CatalogSemantics::GettextCompat,
                PluralEncoding::Gettext
            ),
            Some(CatalogMode::GettextPo)
        );
        assert_eq!(
            CatalogMode::from_parts(
                CatalogStorageFormat::Fcl,
                CatalogSemantics::GettextCompat,
                PluralEncoding::Gettext
            ),
            None
        );
        assert_eq!(
            CatalogMode::from_parts(
                CatalogStorageFormat::Po,
                CatalogSemantics::IcuNative,
                PluralEncoding::Icu
            ),
            Some(CatalogMode::IcuPo)
        );
        let update = UpdateCatalogOptions::new("en", Vec::<super::SourceExtractedMessage>::new())
            .with_mode(CatalogMode::GettextPo);
        assert_eq!(update.mode, CatalogMode::GettextPo);
    }

    #[test]
    fn diagnostics_and_api_errors_preserve_human_readable_messages() {
        let diagnostic = Diagnostic::new(DiagnosticSeverity::Warning, "code", "message")
            .with_identity("Hello", Some("button"));
        assert_eq!(diagnostic.severity, DiagnosticSeverity::Warning);
        assert_eq!(diagnostic.code, "code");
        assert_eq!(diagnostic.message, "message");
        assert_eq!(diagnostic.msgid.as_deref(), Some("Hello"));
        assert_eq!(diagnostic.msgctxt.as_deref(), Some("button"));

        let io_error = ApiError::from(io::Error::other("disk"));
        assert_eq!(io_error.to_string(), "disk");
        assert_eq!(
            io_error.source().map(ToString::to_string).as_deref(),
            Some("disk")
        );
        assert_eq!(io_error.path(), None);

        let io_path_error = ApiError::io_with_path(
            Path::new("locale/de.po"),
            io::Error::other("permission denied"),
        );
        assert_eq!(io_path_error.path(), Some(Path::new("locale/de.po")));
        let source = io_path_error.source().expect("io source");
        assert!(source.to_string().contains("locale/de.po"));
        assert_eq!(
            source.source().map(ToString::to_string).as_deref(),
            Some("permission denied")
        );
        assert!(io_path_error.to_string().contains("locale/de.po"));

        let parse_error = ApiError::from(ParseError::new("bad syntax"));
        assert_eq!(
            parse_error.source().map(ToString::to_string).as_deref(),
            Some("bad syntax")
        );
        assert_eq!(
            ApiError::InvalidArguments("bad input".to_owned()).to_string(),
            "bad input"
        );
        assert!(
            ApiError::InvalidArguments("bad input".to_owned())
                .source()
                .is_none()
        );
        assert_eq!(
            ApiError::Conflict("duplicate".to_owned()).to_string(),
            "duplicate"
        );
        assert_eq!(
            ApiError::Unsupported("unsupported".to_owned()).to_string(),
            "unsupported"
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn catalog_message_and_diagnostic_serde_use_stable_public_shapes() {
        let message = CatalogMessage {
            msgid: "Hello".to_owned(),
            msgctxt: Some("button".to_owned()),
            translation: TranslationShape::Singular {
                value: "Hallo".to_owned(),
            },
            comments: vec!["Shown in toolbar".to_owned()],
            origin: crate::PoVec::new(),
            obsolete: None,
            machine: None,
        };
        let message_json =
            serde_json::to_value(&message).expect("catalog message serialization must succeed");
        assert_eq!(message_json["translation"]["kind"], "singular");

        let roundtrip_message: CatalogMessage = serde_json::from_value(message_json)
            .expect("catalog message deserialization must succeed");
        assert_eq!(roundtrip_message, message);

        let diagnostic = Diagnostic::new(
            DiagnosticSeverity::Error,
            "catalog.missing",
            "missing translation",
        )
        .with_identity("Hello", Some("button"));
        let diagnostic_json =
            serde_json::to_value(&diagnostic).expect("diagnostic serialization must succeed");
        assert_eq!(diagnostic_json["severity"], "error");

        let roundtrip_diagnostic: Diagnostic = serde_json::from_value(diagnostic_json)
            .expect("diagnostic deserialization must succeed");
        assert_eq!(roundtrip_diagnostic, diagnostic);
    }
}
