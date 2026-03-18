use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use crate::ParseError;

use super::plural::PluralProfile;

/// File and line information for an extracted message origin.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CatalogOrigin {
    /// Path-like source identifier where the message came from.
    pub file: String,
    /// One-based line number when the extractor provided one.
    pub line: Option<u32>,
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

/// Extra translator-facing metadata preserved on a catalog message.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CatalogMessageExtra {
    /// Translator comments that were attached to the original PO item.
    pub translator_comments: Vec<String>,
    /// PO flags such as `fuzzy`.
    pub flags: Vec<String>,
}

/// Public message representation returned by [`super::parse_catalog`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogMessage {
    /// Source message identifier.
    pub msgid: String,
    /// Optional gettext message context.
    pub msgctxt: Option<String>,
    /// Public translation representation.
    pub translation: TranslationShape,
    /// Extracted comments preserved from the source catalog.
    pub comments: Vec<String>,
    /// Source origins preserved from PO references.
    pub origin: Vec<CatalogOrigin>,
    /// Whether the message is marked obsolete.
    pub obsolete: bool,
    /// Optional additional translator-facing PO metadata.
    pub extra: Option<CatalogMessageExtra>,
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
                let fallback = profile.source_locale_translation(source);
                for (category, source_value) in fallback {
                    let should_fill = effective.get(&category).is_none_or(String::is_empty);
                    if should_fill {
                        effective.insert(category, source_value);
                    }
                }
                EffectiveTranslation::Plural(effective)
            }
        }
    }
}

/// Stable lookup key for catalog messages.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

/// Parsed catalog plus diagnostics and normalized headers.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedParsedCatalog {
    pub(super) catalog: ParsedCatalog,
    pub(super) key_index: BTreeMap<CatalogMessageKey, usize>,
}

impl NormalizedParsedCatalog {
    /// Builds the lookup index once and rejects duplicate gettext identities up front.
    pub(super) fn new(catalog: ParsedCatalog) -> Result<Self, ApiError> {
        let mut key_index = BTreeMap::new();
        for (index, message) in catalog.messages.iter().enumerate() {
            let key = message.key();
            if key_index.insert(key.clone(), index).is_some() {
                return Err(ApiError::Conflict(format!(
                    "duplicate parsed catalog message for msgid {:?} and context {:?}",
                    key.msgid, key.msgctxt
                )));
            }
        }
        Ok(Self { catalog, key_index })
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

    /// Returns `true` if a message for `key` exists.
    #[must_use]
    pub fn contains_key(&self, key: &CatalogMessageKey) -> bool {
        self.key_index.contains_key(key)
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

    /// Returns the effective translation and fills empty source-locale values
    /// from the source text when appropriate.
    #[must_use]
    pub fn effective_translation_with_source_fallback(
        &self,
        key: &CatalogMessageKey,
        source_locale: &str,
    ) -> Option<EffectiveTranslation> {
        let message = self.get(key)?;
        if self
            .catalog
            .locale
            .as_deref()
            .is_none_or(|locale| locale == source_locale)
        {
            Some(message.source_fallback_translation(self.catalog.locale.as_deref()))
        } else {
            Some(message.effective_translation_owned())
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CatalogStorageFormat {
    /// Read and write classic gettext PO catalogs.
    #[default]
    Po,
    /// Read and write Ferrocat's NDJSON catalog format with a small frontmatter header.
    Ndjson,
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

/// Strategy used for messages that disappear from the extracted input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ObsoleteStrategy {
    /// Mark missing messages obsolete and keep them in the file.
    #[default]
    Mark,
    /// Remove missing messages entirely.
    Delete,
    /// Keep missing messages as active entries.
    Keep,
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
    /// Storage format used when reading existing content and rendering the result.
    pub storage_format: CatalogStorageFormat,
    /// High-level semantics used when parsing, merging, and rendering the catalog.
    pub semantics: CatalogSemantics,
    /// Target plural representation for the rendered PO file.
    pub plural_encoding: PluralEncoding,
    /// Strategy for messages absent from the extracted input.
    pub obsolete_strategy: ObsoleteStrategy,
    /// Whether source-locale translations should be refreshed from the extracted source strings.
    pub overwrite_source_translations: bool,
    /// Sort order for the final rendered catalog.
    pub order_by: OrderBy,
    /// Whether source origins should be rendered as references.
    pub include_origins: bool,
    /// Whether rendered references should include line numbers.
    pub include_line_numbers: bool,
    /// Controls emission of placeholder comments.
    pub print_placeholders_in_comments: PlaceholderCommentMode,
    /// Optional additional header attributes to inject or override.
    pub custom_header_attributes: Option<&'a BTreeMap<String, String>>,
}

impl Default for UpdateCatalogOptions<'_> {
    fn default() -> Self {
        Self {
            locale: None,
            source_locale: "",
            input: CatalogUpdateInput::default(),
            existing: None,
            storage_format: CatalogStorageFormat::Po,
            semantics: CatalogSemantics::IcuNative,
            plural_encoding: PluralEncoding::Icu,
            obsolete_strategy: ObsoleteStrategy::Mark,
            overwrite_source_translations: false,
            order_by: OrderBy::Msgid,
            include_origins: true,
            include_line_numbers: true,
            print_placeholders_in_comments: PlaceholderCommentMode::Enabled { limit: 3 },
            custom_header_attributes: None,
        }
    }
}

/// Options for updating a catalog file on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCatalogFileOptions<'a> {
    /// Path to the catalog file that should be read and conditionally written.
    pub target_path: &'a Path,
    /// Locale of the catalog being updated. When `None`, Ferrocat infers it from the existing file.
    pub locale: Option<&'a str>,
    /// Source locale used for source-side semantics and fallback handling.
    pub source_locale: &'a str,
    /// Extracted messages to merge into the catalog.
    pub input: CatalogUpdateInput,
    /// Storage format used when reading and writing the file content.
    pub storage_format: CatalogStorageFormat,
    /// High-level semantics used when parsing, merging, and rendering the catalog.
    pub semantics: CatalogSemantics,
    /// Target plural representation for the rendered PO file.
    pub plural_encoding: PluralEncoding,
    /// Strategy for messages absent from the extracted input.
    pub obsolete_strategy: ObsoleteStrategy,
    /// Whether source-locale translations should be refreshed from the extracted source strings.
    pub overwrite_source_translations: bool,
    /// Sort order for the final rendered catalog.
    pub order_by: OrderBy,
    /// Whether source origins should be rendered as references.
    pub include_origins: bool,
    /// Whether rendered references should include line numbers.
    pub include_line_numbers: bool,
    /// Controls emission of placeholder comments.
    pub print_placeholders_in_comments: PlaceholderCommentMode,
    /// Optional additional header attributes to inject or override.
    pub custom_header_attributes: Option<&'a BTreeMap<String, String>>,
}

impl Default for UpdateCatalogFileOptions<'_> {
    fn default() -> Self {
        Self {
            target_path: Path::new(""),
            locale: None,
            source_locale: "",
            input: CatalogUpdateInput::default(),
            storage_format: CatalogStorageFormat::Po,
            semantics: CatalogSemantics::IcuNative,
            plural_encoding: PluralEncoding::Icu,
            obsolete_strategy: ObsoleteStrategy::Mark,
            overwrite_source_translations: false,
            order_by: OrderBy::Msgid,
            include_origins: true,
            include_line_numbers: true,
            print_placeholders_in_comments: PlaceholderCommentMode::Enabled { limit: 3 },
            custom_header_attributes: None,
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
    /// Storage format used when parsing the content.
    pub storage_format: CatalogStorageFormat,
    /// High-level semantics used when interpreting catalog content.
    pub semantics: CatalogSemantics,
    /// Target plural interpretation for the resulting catalog view.
    pub plural_encoding: PluralEncoding,
    /// Whether unsupported ICU plural projection cases should become hard errors.
    pub strict: bool,
}

impl Default for ParseCatalogOptions<'_> {
    fn default() -> Self {
        Self {
            content: "",
            locale: None,
            source_locale: "",
            storage_format: CatalogStorageFormat::Po,
            semantics: CatalogSemantics::IcuNative,
            plural_encoding: PluralEncoding::Icu,
            strict: false,
        }
    }
}

/// Error returned by catalog parsing and update APIs.
#[derive(Debug)]
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

impl std::error::Error for ApiError {}

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
    use std::io;
    use std::path::Path;

    use super::{
        ApiError, CatalogMessage, CatalogMessageExtra, CatalogMessageKey, CatalogSemantics,
        CatalogStorageFormat, CatalogUpdateInput, Diagnostic, DiagnosticSeverity,
        EffectiveTranslation, EffectiveTranslationRef, NormalizedParsedCatalog, ObsoleteStrategy,
        OrderBy, ParseCatalogOptions, ParsedCatalog, PlaceholderCommentMode, PluralEncoding,
        PluralSource, TranslationShape, UpdateCatalogFileOptions, UpdateCatalogOptions,
    };

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
            origin: Vec::new(),
            obsolete: false,
            extra: Some(CatalogMessageExtra {
                translator_comments: vec!["Imperative".to_owned()],
                flags: vec!["fuzzy".to_owned()],
            }),
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
            origin: Vec::new(),
            obsolete: false,
            extra: None,
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
            messages: vec![CatalogMessage {
                msgid: "Hello".to_owned(),
                msgctxt: None,
                translation: TranslationShape::Singular {
                    value: String::new(),
                },
                comments: Vec::new(),
                origin: Vec::new(),
                obsolete: false,
                extra: None,
            }],
            diagnostics: Vec::new(),
        };

        let normalized = NormalizedParsedCatalog::new(parsed.clone()).expect("normalized");
        let key = CatalogMessageKey::new("Hello", None);

        assert_eq!(normalized.message_count(), 1);
        assert!(normalized.contains_key(&key));
        assert_eq!(
            normalized.parsed_catalog().semantics,
            CatalogSemantics::IcuNative
        );
        assert!(normalized.get(&key).is_some());
        assert_eq!(
            normalized.effective_translation_with_source_fallback(&key, "en"),
            Some(EffectiveTranslation::Singular("Hello".to_owned()))
        );
        assert_eq!(normalized.into_parsed_catalog(), parsed);
    }

    #[test]
    fn option_defaults_reflect_native_po_defaults() {
        let update = UpdateCatalogOptions::default();
        assert_eq!(update.storage_format, CatalogStorageFormat::Po);
        assert_eq!(update.semantics, CatalogSemantics::IcuNative);
        assert_eq!(update.plural_encoding, PluralEncoding::Icu);
        assert_eq!(update.obsolete_strategy, ObsoleteStrategy::Mark);
        assert_eq!(update.order_by, OrderBy::Msgid);
        assert!(update.include_origins);
        assert!(update.include_line_numbers);
        assert_eq!(
            update.print_placeholders_in_comments,
            PlaceholderCommentMode::Enabled { limit: 3 }
        );

        let update_file = UpdateCatalogFileOptions::default();
        assert_eq!(update_file.target_path, Path::new(""));
        assert_eq!(update_file.storage_format, CatalogStorageFormat::Po);
        assert_eq!(update_file.semantics, CatalogSemantics::IcuNative);
        assert_eq!(update_file.plural_encoding, PluralEncoding::Icu);

        let parse = ParseCatalogOptions::default();
        assert_eq!(parse.storage_format, CatalogStorageFormat::Po);
        assert_eq!(parse.semantics, CatalogSemantics::IcuNative);
        assert_eq!(parse.plural_encoding, PluralEncoding::Icu);
        assert!(!parse.strict);
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
            ApiError::InvalidArguments("bad input".to_owned()).to_string(),
            "bad input"
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
}
