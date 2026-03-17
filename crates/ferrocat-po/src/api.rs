use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::{Header, MsgStr, ParseError, PoFile, PoItem, SerializeOptions, parse_po, stringify_po};
use ferrocat_icu::{IcuMessage, IcuNode, IcuPluralKind, parse_icu};
use icu_locale::Locale;
use icu_plurals::{PluralCategory, PluralRules};
use sha2::{Digest, Sha256};

/// File and line information for an extracted message origin.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CatalogOrigin {
    pub file: String,
    pub line: Option<u32>,
}

/// Structured singular message input used by catalog update operations.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtractedSingularMessage {
    pub msgid: String,
    pub msgctxt: Option<String>,
    pub comments: Vec<String>,
    pub origin: Vec<CatalogOrigin>,
    pub placeholders: BTreeMap<String, Vec<String>>,
}

/// Source-side plural forms for structured catalog messages.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PluralSource {
    pub one: Option<String>,
    pub other: String,
}

/// Structured plural message input used by catalog update operations.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtractedPluralMessage {
    pub msgid: String,
    pub msgctxt: Option<String>,
    pub source: PluralSource,
    pub comments: Vec<String>,
    pub origin: Vec<CatalogOrigin>,
    pub placeholders: BTreeMap<String, Vec<String>>,
}

/// Structured extractor input accepted by [`update_catalog`] and [`update_catalog_file`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractedMessage {
    Singular(ExtractedSingularMessage),
    Plural(ExtractedPluralMessage),
}

/// Source-first extractor input that lets `ferrocat` infer plural structure.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceExtractedMessage {
    pub msgid: String,
    pub msgctxt: Option<String>,
    pub comments: Vec<String>,
    pub origin: Vec<CatalogOrigin>,
    pub placeholders: BTreeMap<String, Vec<String>>,
}

/// Input payload accepted by catalog update operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogUpdateInput {
    Structured(Vec<ExtractedMessage>),
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
    Singular {
        value: String,
    },
    Plural {
        source: PluralSource,
        translation: BTreeMap<String, String>,
    },
}

/// Borrowed view over a message translation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectiveTranslationRef<'a> {
    Singular(&'a str),
    Plural(&'a BTreeMap<String, String>),
}

/// Owned translation value materialized from a parsed catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectiveTranslation {
    Singular(String),
    Plural(BTreeMap<String, String>),
}

/// Translation value stored in a compiled runtime catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompiledTranslation {
    Singular(String),
    Plural(BTreeMap<String, String>),
}

/// Built-in key strategy used when compiling runtime catalogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompiledKeyStrategy {
    /// `ferrocat` v1 key format: SHA-256 over a versioned, length-delimited
    /// `msgctxt`/`msgid` payload, truncated to 64 bits and encoded as unpadded
    /// `Base64URL`.
    #[default]
    FerrocatV1,
}

/// Options controlling runtime catalog compilation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileCatalogOptions {
    /// Built-in strategy used to derive stable runtime keys.
    pub key_strategy: CompiledKeyStrategy,
    /// Whether empty source-locale values should be filled from the source text.
    pub source_fallback: bool,
    /// Source locale used when `source_fallback` is enabled.
    pub source_locale: Option<String>,
}

impl Default for CompileCatalogOptions {
    fn default() -> Self {
        Self {
            key_strategy: CompiledKeyStrategy::FerrocatV1,
            source_fallback: false,
            source_locale: None,
        }
    }
}

/// A compiled runtime message keyed by a derived lookup key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledMessage {
    /// Stable runtime key derived from the source identity.
    pub key: String,
    /// Original gettext identity preserved for diagnostics and tooling.
    pub source_key: CatalogMessageKey,
    /// Materialized translation payload for runtime lookup.
    pub translation: CompiledTranslation,
}

/// Runtime-oriented lookup structure compiled from a normalized catalog.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompiledCatalog {
    entries: BTreeMap<String, CompiledMessage>,
}

impl CompiledCatalog {
    /// Returns the compiled message for `key`, if present.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&CompiledMessage> {
        self.entries.get(key)
    }

    /// Returns the number of compiled entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` when the compiled catalog has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterates over compiled entries in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &CompiledMessage)> + '_ {
        self.entries
            .iter()
            .map(|(key, message)| (key.as_str(), message))
    }
}

/// Extra translator-facing metadata preserved on a catalog message.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CatalogMessageExtra {
    pub translator_comments: Vec<String>,
    pub flags: Vec<String>,
}

/// Public message representation returned by [`parse_catalog`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogMessage {
    pub msgid: String,
    pub msgctxt: Option<String>,
    pub translation: TranslationShape,
    pub comments: Vec<String>,
    pub origin: Vec<CatalogOrigin>,
    pub obsolete: bool,
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

    fn effective_translation_owned(&self) -> EffectiveTranslation {
        match &self.translation {
            TranslationShape::Singular { value } => EffectiveTranslation::Singular(value.clone()),
            TranslationShape::Plural { translation, .. } => {
                EffectiveTranslation::Plural(translation.clone())
            }
        }
    }

    fn source_fallback_translation(&self, locale: Option<&str>) -> EffectiveTranslation {
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
    pub msgid: String,
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
    Info,
    Warning,
    Error,
}

/// Non-fatal issue collected while parsing or updating catalogs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub msgid: Option<String>,
    pub msgctxt: Option<String>,
}

impl Diagnostic {
    fn new(
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

    fn with_identity(mut self, msgid: &str, msgctxt: Option<&str>) -> Self {
        self.msgid = Some(msgid.to_owned());
        self.msgctxt = msgctxt.map(str::to_owned);
        self
    }
}

/// Basic counters describing an update operation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CatalogStats {
    pub total: usize,
    pub added: usize,
    pub changed: usize,
    pub unchanged: usize,
    pub obsolete_marked: usize,
    pub obsolete_removed: usize,
}

/// Result returned by catalog update operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogUpdateResult {
    pub content: String,
    pub created: bool,
    pub updated: bool,
    pub stats: CatalogStats,
    pub diagnostics: Vec<Diagnostic>,
}

/// Parsed catalog plus diagnostics and normalized headers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCatalog {
    pub locale: Option<String>,
    pub headers: BTreeMap<String, String>,
    pub messages: Vec<CatalogMessage>,
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
    catalog: ParsedCatalog,
    key_index: BTreeMap<CatalogMessageKey, usize>,
}

impl NormalizedParsedCatalog {
    fn new(catalog: ParsedCatalog) -> Result<Self, ApiError> {
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

    /// Compiles the normalized catalog into a runtime-oriented lookup map.
    ///
    /// Compiled keys are derived from the canonical gettext identity
    /// (`msgctxt` + `msgid`) using the selected built-in key strategy.
    /// The default configuration keeps translations as-is without filling
    /// missing values from the source text.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::InvalidArguments`] when source fallback is enabled
    /// without a `source_locale`, or [`ApiError::Conflict`] when two source
    /// messages compile to the same derived key.
    ///
    /// ```rust
    /// use ferrocat_po::{CompileCatalogOptions, ParseCatalogOptions, parse_catalog};
    ///
    /// let parsed = parse_catalog(ParseCatalogOptions {
    ///     content: "msgid \"Hello\"\nmsgstr \"Hallo\"\n".to_owned(),
    ///     source_locale: "en".to_owned(),
    ///     locale: Some("de".to_owned()),
    ///     ..ParseCatalogOptions::default()
    /// })?;
    /// let normalized = parsed.into_normalized_view()?;
    /// let compiled = normalized.compile(&CompileCatalogOptions::default())?;
    ///
    /// assert_eq!(compiled.len(), 1);
    /// let (_, message) = compiled.iter().next().expect("compiled message");
    /// assert_eq!(message.source_key.msgid, "Hello");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn compile(&self, options: &CompileCatalogOptions) -> Result<CompiledCatalog, ApiError> {
        self.compile_with_key_generator(options, compiled_key_for)
    }

    fn compile_with_key_generator<F>(
        &self,
        options: &CompileCatalogOptions,
        mut key_generator: F,
    ) -> Result<CompiledCatalog, ApiError>
    where
        F: FnMut(CompiledKeyStrategy, &CatalogMessageKey) -> String,
    {
        let source_locale = if options.source_fallback {
            Some(options.source_locale.as_deref().ok_or_else(|| {
                ApiError::InvalidArguments(
                    "compile_catalog source_fallback requires source_locale".to_owned(),
                )
            })?)
        } else {
            None
        };
        let mut entries = BTreeMap::new();

        for (source_key, message) in self.iter() {
            let translation = source_locale.map_or_else(
                || compiled_translation_from_effective(message.effective_translation_owned()),
                |source_locale| {
                    compiled_translation_from_effective(
                        self.effective_translation_with_source_fallback(source_key, source_locale)
                            .expect("normalized catalog lookup"),
                    )
                },
            );
            let compiled_key = key_generator(options.key_strategy, source_key);
            let compiled_message = CompiledMessage {
                key: compiled_key.clone(),
                source_key: source_key.clone(),
                translation,
            };

            if let Some(existing) = entries.insert(compiled_key.clone(), compiled_message) {
                return Err(ApiError::Conflict(format!(
                    "compiled catalog key collision for {:?} / {:?} and {:?} / {:?} using key {}",
                    existing.source_key.msgctxt,
                    existing.source_key.msgid,
                    source_key.msgctxt,
                    source_key.msgid,
                    compiled_key
                )));
            }
        }

        Ok(CompiledCatalog { entries })
    }
}

/// Encoding used for plural messages in PO files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PluralEncoding {
    #[default]
    Icu,
    Gettext,
}

/// Strategy used for messages that disappear from the extracted input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ObsoleteStrategy {
    #[default]
    Mark,
    Delete,
    Keep,
}

/// Sort order used when writing output catalogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrderBy {
    #[default]
    Msgid,
    Origin,
}

/// Controls whether placeholder hints are emitted as extracted comments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaceholderCommentMode {
    Disabled,
    Enabled { limit: usize },
}

impl Default for PlaceholderCommentMode {
    fn default() -> Self {
        Self::Enabled { limit: 3 }
    }
}

/// Options for in-memory catalog updates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCatalogOptions {
    pub locale: Option<String>,
    pub source_locale: String,
    pub input: CatalogUpdateInput,
    pub existing: Option<String>,
    pub plural_encoding: PluralEncoding,
    pub obsolete_strategy: ObsoleteStrategy,
    pub overwrite_source_translations: bool,
    pub order_by: OrderBy,
    pub include_origins: bool,
    pub include_line_numbers: bool,
    pub print_placeholders_in_comments: PlaceholderCommentMode,
    pub custom_header_attributes: BTreeMap<String, String>,
}

impl Default for UpdateCatalogOptions {
    fn default() -> Self {
        Self {
            locale: None,
            source_locale: String::new(),
            input: CatalogUpdateInput::default(),
            existing: None,
            plural_encoding: PluralEncoding::Icu,
            obsolete_strategy: ObsoleteStrategy::Mark,
            overwrite_source_translations: false,
            order_by: OrderBy::Msgid,
            include_origins: true,
            include_line_numbers: true,
            print_placeholders_in_comments: PlaceholderCommentMode::Enabled { limit: 3 },
            custom_header_attributes: BTreeMap::new(),
        }
    }
}

/// Options for updating a catalog file on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCatalogFileOptions {
    pub target_path: PathBuf,
    pub locale: Option<String>,
    pub source_locale: String,
    pub input: CatalogUpdateInput,
    pub plural_encoding: PluralEncoding,
    pub obsolete_strategy: ObsoleteStrategy,
    pub overwrite_source_translations: bool,
    pub order_by: OrderBy,
    pub include_origins: bool,
    pub include_line_numbers: bool,
    pub print_placeholders_in_comments: PlaceholderCommentMode,
    pub custom_header_attributes: BTreeMap<String, String>,
}

impl Default for UpdateCatalogFileOptions {
    fn default() -> Self {
        Self {
            target_path: PathBuf::new(),
            locale: None,
            source_locale: String::new(),
            input: CatalogUpdateInput::default(),
            plural_encoding: PluralEncoding::Icu,
            obsolete_strategy: ObsoleteStrategy::Mark,
            overwrite_source_translations: false,
            order_by: OrderBy::Msgid,
            include_origins: true,
            include_line_numbers: true,
            print_placeholders_in_comments: PlaceholderCommentMode::Enabled { limit: 3 },
            custom_header_attributes: BTreeMap::new(),
        }
    }
}

/// Options for parsing a PO catalog into the higher-level message model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseCatalogOptions {
    pub content: String,
    pub locale: Option<String>,
    pub source_locale: String,
    pub plural_encoding: PluralEncoding,
    pub strict: bool,
}

impl Default for ParseCatalogOptions {
    fn default() -> Self {
        Self {
            content: String::new(),
            locale: None,
            source_locale: String::new(),
            plural_encoding: PluralEncoding::Icu,
            strict: false,
        }
    }
}

/// Error returned by catalog parsing and update APIs.
#[derive(Debug)]
pub enum ApiError {
    Parse(ParseError),
    Io(std::io::Error),
    InvalidArguments(String),
    Conflict(String),
    Unsupported(String),
}

fn compiled_translation_from_effective(value: EffectiveTranslation) -> CompiledTranslation {
    match value {
        EffectiveTranslation::Singular(value) => CompiledTranslation::Singular(value),
        EffectiveTranslation::Plural(values) => CompiledTranslation::Plural(values),
    }
}

fn compiled_key_for(strategy: CompiledKeyStrategy, key: &CatalogMessageKey) -> String {
    match strategy {
        CompiledKeyStrategy::FerrocatV1 => ferrocat_v1_compiled_key(key),
    }
}

fn ferrocat_v1_compiled_key(key: &CatalogMessageKey) -> String {
    let mut payload = Vec::with_capacity(
        16 + 1 + 4 + key.msgctxt.as_ref().map_or(0, String::len) + 1 + 4 + key.msgid.len(),
    );
    payload.extend_from_slice(b"ferrocat:compile:v1");
    push_compiled_key_component(&mut payload, key.msgctxt.as_deref());
    push_compiled_key_component(&mut payload, Some(key.msgid.as_str()));
    let digest = Sha256::digest(&payload);
    base64_url_no_pad(&digest[..8])
}

fn push_compiled_key_component(out: &mut Vec<u8>, value: Option<&str>) {
    if let Some(value) = value {
        out.push(1);
        let value_len = u32::try_from(value.len()).expect("compiled key component exceeds u32");
        out.extend_from_slice(&value_len.to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    } else {
        out.push(0);
        out.extend_from_slice(&0u32.to_be_bytes());
    }
}

fn base64_url_no_pad(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity((bytes.len() * 4).div_ceil(3));
    let mut index = 0;

    while index + 3 <= bytes.len() {
        let chunk = (u32::from(bytes[index]) << 16)
            | (u32::from(bytes[index + 1]) << 8)
            | u32::from(bytes[index + 2]);
        out.push(ALPHABET[((chunk >> 18) & 0x3f) as usize] as char);
        out.push(ALPHABET[((chunk >> 12) & 0x3f) as usize] as char);
        out.push(ALPHABET[((chunk >> 6) & 0x3f) as usize] as char);
        out.push(ALPHABET[(chunk & 0x3f) as usize] as char);
        index += 3;
    }

    match bytes.len() - index {
        1 => {
            let chunk = u32::from(bytes[index]) << 16;
            out.push(ALPHABET[((chunk >> 18) & 0x3f) as usize] as char);
            out.push(ALPHABET[((chunk >> 12) & 0x3f) as usize] as char);
        }
        2 => {
            let chunk = (u32::from(bytes[index]) << 16) | (u32::from(bytes[index + 1]) << 8);
            out.push(ALPHABET[((chunk >> 18) & 0x3f) as usize] as char);
            out.push(ALPHABET[((chunk >> 12) & 0x3f) as usize] as char);
            out.push(ALPHABET[((chunk >> 6) & 0x3f) as usize] as char);
        }
        _ => {}
    }

    out
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Catalog {
    locale: Option<String>,
    headers: BTreeMap<String, String>,
    file_comments: Vec<String>,
    file_extracted_comments: Vec<String>,
    messages: Vec<CanonicalMessage>,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanonicalMessage {
    msgid: String,
    msgctxt: Option<String>,
    translation: CanonicalTranslation,
    comments: Vec<String>,
    origins: Vec<CatalogOrigin>,
    placeholders: BTreeMap<String, Vec<String>>,
    obsolete: bool,
    translator_comments: Vec<String>,
    flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CanonicalTranslation {
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedIcuPlural {
    variable: String,
    branches: BTreeMap<String, String>,
}

enum IcuPluralProjection {
    NotPlural,
    Projected(ParsedIcuPlural),
    Unsupported(&'static str),
    Malformed,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct ParsedPluralFormsHeader {
    raw: Option<String>,
    nplurals: Option<usize>,
    plural: Option<String>,
}

type PluralCategoryCache = Mutex<HashMap<String, Option<Vec<String>>>>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PluralProfile {
    categories: Vec<String>,
}

impl PluralProfile {
    fn new(locale: Option<&str>, nplurals: Option<usize>) -> Self {
        let categories = locale.and_then(icu_plural_categories_for).map_or_else(
            || fallback_plural_categories(nplurals),
            |locale_categories| {
                if nplurals.is_none() || nplurals == Some(locale_categories.len()) {
                    locale_categories
                } else {
                    fallback_plural_categories(nplurals)
                }
            },
        );

        Self { categories }
    }

    fn for_locale(locale: Option<&str>) -> Self {
        Self::new(locale, None)
    }

    fn for_gettext_slots(locale: Option<&str>, nplurals: Option<usize>) -> Self {
        Self::new(locale, nplurals)
    }

    fn for_translation(
        locale: Option<&str>,
        translation_by_category: &BTreeMap<String, String>,
    ) -> Self {
        Self::new(locale, Some(translation_by_category.len()))
    }

    fn categories(&self) -> &[String] {
        &self.categories
    }

    fn nplurals(&self) -> usize {
        self.categories.len().max(1)
    }

    fn materialize_translation(
        &self,
        translation: &BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        self.categories
            .iter()
            .map(|category| {
                (
                    category.clone(),
                    translation.get(category).cloned().unwrap_or_default(),
                )
            })
            .collect()
    }

    fn source_locale_translation(&self, source: &PluralSource) -> BTreeMap<String, String> {
        let mut translation = BTreeMap::new();
        for category in &self.categories {
            let value = match category.as_str() {
                "one" => source.one.clone().unwrap_or_else(|| source.other.clone()),
                _ => source.other.clone(),
            };
            translation.insert(category.clone(), value);
        }
        translation
    }

    fn empty_translation(&self) -> BTreeMap<String, String> {
        self.categories
            .iter()
            .map(|category| (category.clone(), String::new()))
            .collect()
    }

    fn gettext_values(&self, translation: &BTreeMap<String, String>) -> Vec<String> {
        self.categories
            .iter()
            .map(|category| translation.get(category).cloned().unwrap_or_default())
            .collect()
    }

    fn gettext_header(&self) -> Option<String> {
        match self.nplurals() {
            1 => Some("nplurals=1; plural=0;".to_owned()),
            2 => Some("nplurals=2; plural=(n != 1);".to_owned()),
            _ => None,
        }
    }
}

/// Merges extracted messages into an existing catalog and returns updated PO content.
///
/// # Errors
///
/// Returns [`ApiError`] when the source locale is missing, the existing PO file
/// cannot be parsed, or the requested plural encoding cannot be represented safely.
pub fn update_catalog(options: UpdateCatalogOptions) -> Result<CatalogUpdateResult, ApiError> {
    validate_source_locale(&options.source_locale)?;

    let created = options.existing.is_none();
    let original = options.existing.as_deref().unwrap_or("");
    let existing = match options.existing.as_deref() {
        Some(content) if !content.is_empty() => parse_catalog_to_internal(
            content,
            options.locale.as_deref(),
            options.plural_encoding,
            false,
        )?,
        Some(_) | None => Catalog {
            locale: options.locale.clone(),
            headers: BTreeMap::new(),
            file_comments: Vec::new(),
            file_extracted_comments: Vec::new(),
            messages: Vec::new(),
            diagnostics: Vec::new(),
        },
    };

    let locale = options
        .locale
        .clone()
        .or_else(|| existing.locale.clone())
        .or_else(|| existing.headers.get("Language").cloned());
    let mut diagnostics = existing.diagnostics.clone();
    let normalized = normalize_update_input(&options.input, &mut diagnostics)?;
    let (mut merged, stats) = merge_catalogs(
        existing,
        &normalized,
        locale.as_deref(),
        &options.source_locale,
        options.overwrite_source_translations,
        options.obsolete_strategy,
        &mut diagnostics,
    );
    merged.locale = locale.clone();
    apply_header_defaults(
        &mut merged.headers,
        locale.as_deref(),
        options.plural_encoding,
        &mut diagnostics,
        &options.custom_header_attributes,
    );
    sort_messages(&mut merged.messages, options.order_by);
    let file = export_catalog_to_po(&merged, &options, locale.as_deref(), &mut diagnostics)?;
    let content = stringify_po(&file, &SerializeOptions::default());

    Ok(CatalogUpdateResult {
        updated: content != original,
        content,
        created,
        stats,
        diagnostics,
    })
}

/// Updates a PO catalog on disk and only writes the file when the rendered
/// output changes.
///
/// # Errors
///
/// Returns [`ApiError`] when the input is invalid, when the existing file
/// cannot be read or parsed, or when the updated content cannot be written.
pub fn update_catalog_file(
    options: UpdateCatalogFileOptions,
) -> Result<CatalogUpdateResult, ApiError> {
    validate_source_locale(&options.source_locale)?;
    if options.target_path.as_os_str().is_empty() {
        return Err(ApiError::InvalidArguments(
            "target_path must not be empty".to_owned(),
        ));
    }

    let existing = match fs::read_to_string(&options.target_path) {
        Ok(content) => Some(content),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(ApiError::Io(error)),
    };

    let result = update_catalog(UpdateCatalogOptions {
        locale: options.locale,
        source_locale: options.source_locale,
        input: options.input,
        existing,
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
        atomic_write(&options.target_path, &result.content)?;
    }

    Ok(result)
}

/// Parses PO content into the higher-level catalog representation used by
/// `ferrocat`'s catalog APIs.
///
/// # Errors
///
/// Returns [`ApiError`] when the PO content cannot be parsed, the source
/// locale is missing, or strict ICU projection fails.
pub fn parse_catalog(options: ParseCatalogOptions) -> Result<ParsedCatalog, ApiError> {
    validate_source_locale(&options.source_locale)?;
    let catalog = parse_catalog_to_internal(
        &options.content,
        options.locale.as_deref(),
        options.plural_encoding,
        options.strict,
    )?;
    let messages = catalog
        .messages
        .into_iter()
        .map(public_message_from_canonical)
        .collect();

    Ok(ParsedCatalog {
        locale: catalog.locale,
        headers: catalog.headers,
        messages,
        diagnostics: catalog.diagnostics,
    })
}

fn validate_source_locale(source_locale: &str) -> Result<(), ApiError> {
    if source_locale.trim().is_empty() {
        return Err(ApiError::InvalidArguments(
            "source_locale must not be empty".to_owned(),
        ));
    }
    Ok(())
}

fn normalize_update_input(
    input: &CatalogUpdateInput,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Vec<NormalizedMessage>, ApiError> {
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
                let kind = match project_icu_plural(&message.msgid) {
                    IcuPluralProjection::Projected(projected) => NormalizedKind::Plural {
                        source: PluralSource {
                            one: projected.branches.get("one").cloned(),
                            other: projected
                                .branches
                                .get("other")
                                .cloned()
                                .unwrap_or_else(|| message.msgid.clone()),
                        },
                        variable: Some(projected.variable),
                    },
                    IcuPluralProjection::Unsupported(reason) => {
                        diagnostics.push(
                            Diagnostic::new(
                                DiagnosticSeverity::Warning,
                                "plural.source_first_fallback",
                                format!(
                                    "Could not project source-first ICU plural into catalog plural form: {reason}"
                                ),
                            )
                            .with_identity(&message.msgid, message.msgctxt.as_deref()),
                        );
                        NormalizedKind::Singular
                    }
                    IcuPluralProjection::Malformed => {
                        diagnostics.push(
                            Diagnostic::new(
                                DiagnosticSeverity::Warning,
                                "plural.source_first_fallback",
                                "Could not parse source-first ICU plural safely; keeping the message as singular.",
                            )
                            .with_identity(&message.msgid, message.msgctxt.as_deref()),
                        );
                        NormalizedKind::Singular
                    }
                    IcuPluralProjection::NotPlural => NormalizedKind::Singular,
                };

                push_normalized_message(
                    &mut index,
                    &mut normalized,
                    NormalizedMessage {
                        msgid: message.msgid.clone(),
                        msgctxt: message.msgctxt.clone(),
                        kind,
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

fn merge_catalogs(
    existing: Catalog,
    normalized: &[NormalizedMessage],
    locale: Option<&str>,
    source_locale: &str,
    overwrite_source_translations: bool,
    obsolete_strategy: ObsoleteStrategy,
    diagnostics: &mut Vec<Diagnostic>,
) -> (Catalog, CatalogStats) {
    let is_source_locale = locale.is_none_or(|value| value == source_locale);
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
            locale,
            overwrite_source_translations,
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
        match obsolete_strategy {
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

fn merge_message(
    previous: Option<&CanonicalMessage>,
    next: &NormalizedMessage,
    is_source_locale: bool,
    locale: Option<&str>,
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
            let plural_profile = PluralProfile::for_locale(locale);

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
                                    "plural.assumed_variable",
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

    let (translator_comments, flags, obsolete) = previous.map_or_else(
        || (Vec::new(), Vec::new(), false),
        |message| {
            (
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
        translator_comments,
        flags,
    }
}

fn materialize_plural_categories(
    categories: &[String],
    translation: BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    categories
        .iter()
        .map(|category| {
            (
                category.clone(),
                translation.get(category).cloned().unwrap_or_default(),
            )
        })
        .collect()
}

fn extract_plural_variable(message: &CanonicalMessage) -> Option<String> {
    match &message.translation {
        CanonicalTranslation::Plural { variable, .. } => Some(variable.clone()),
        CanonicalTranslation::Singular { .. } => None,
    }
}

fn apply_header_defaults(
    headers: &mut BTreeMap<String, String>,
    locale: Option<&str>,
    plural_encoding: PluralEncoding,
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
    if plural_encoding == PluralEncoding::Gettext && !custom.contains_key("Plural-Forms") {
        let profile = PluralProfile::for_locale(locale);
        let parsed_header = parse_plural_forms_from_headers(headers);
        match (parsed_header.raw.as_deref(), profile.gettext_header()) {
            (None, Some(header)) => {
                headers.insert("Plural-Forms".to_owned(), header);
            }
            (None, None) => diagnostics.push(Diagnostic::new(
                DiagnosticSeverity::Info,
                "plural.missing_plural_forms_header",
                "No safe default Plural-Forms header is known for this locale; keeping the header unset.",
            )),
            (Some(_), Some(header))
                if parsed_header.nplurals == Some(profile.nplurals())
                    && parsed_header.plural.is_none() =>
            {
                headers.insert("Plural-Forms".to_owned(), header);
                diagnostics.push(Diagnostic::new(
                    DiagnosticSeverity::Info,
                    "plural.completed_plural_forms_header",
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

fn export_catalog_to_po(
    catalog: &Catalog,
    options: &UpdateCatalogOptions,
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

fn export_message_to_po(
    message: &CanonicalMessage,
    options: &UpdateCatalogOptions,
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
            let plural_profile = PluralProfile::for_translation(locale, translation_by_category);
            let nplurals = plural_profile
                .nplurals()
                .max(translation_by_category.len().max(1));
            let mut item = base_po_item(message, options, nplurals);

            match options.plural_encoding {
                PluralEncoding::Icu => {
                    item.msgid = synthesize_icu_plural(variable, &plural_source_branches(source));
                    item.msgstr =
                        MsgStr::from(synthesize_icu_plural(variable, translation_by_category));
                }
                PluralEncoding::Gettext => {
                    if !translation_by_category.contains_key("other") {
                        diagnostics.push(
                            Diagnostic::new(
                                DiagnosticSeverity::Error,
                                "plural.unsupported_gettext_export",
                                "Plural translation is missing the required \"other\" category.",
                            )
                            .with_identity(&message.msgid, message.msgctxt.as_deref()),
                        );
                        return Err(ApiError::Unsupported(
                            "plural translation is missing the required \"other\" category"
                                .to_owned(),
                        ));
                    }
                    item.msgid = source.one.clone().unwrap_or_else(|| source.other.clone());
                    item.msgid_plural = Some(source.other.clone());
                    item.msgstr =
                        MsgStr::from(plural_profile.gettext_values(translation_by_category));
                    item.nplurals = plural_profile.nplurals();
                }
            }

            Ok(item)
        }
    }
}

fn base_po_item(
    message: &CanonicalMessage,
    options: &UpdateCatalogOptions,
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

fn plural_source_branches(source: &PluralSource) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    if let Some(one) = &source.one {
        map.insert("one".to_owned(), one.clone());
    }
    map.insert("other".to_owned(), source.other.clone());
    map
}

fn append_placeholder_comments(
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

fn parse_catalog_to_internal(
    content: &str,
    locale_override: Option<&str>,
    plural_encoding: PluralEncoding,
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
        plural_encoding,
        &mut diagnostics,
    );
    let mut messages = Vec::with_capacity(file.items.len());

    for item in file.items {
        let mut conversion_diagnostics = Vec::new();
        let message = import_message_from_po(
            item,
            locale.as_deref(),
            nplurals,
            plural_encoding,
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

fn import_message_from_po(
    item: PoItem,
    locale: Option<&str>,
    nplurals: Option<usize>,
    plural_encoding: PluralEncoding,
    strict: bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<CanonicalMessage, ApiError> {
    let (comments, placeholders) = split_placeholder_comments(item.extracted_comments);
    let origins = item
        .references
        .iter()
        .map(|reference| parse_origin(reference))
        .collect();

    let translation = if let Some(msgid_plural) = &item.msgid_plural {
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
                .enumerate()
                .map(|(index, category)| {
                    (
                        category.clone(),
                        item.msgstr.iter().nth(index).cloned().unwrap_or_default(),
                    )
                })
                .collect(),
            variable: "count".to_owned(),
        }
    } else {
        let msgstr = item.msgstr.first_str().unwrap_or_default().to_owned();
        if plural_encoding == PluralEncoding::Icu {
            match project_icu_plural(&item.msgid) {
                IcuPluralProjection::Projected(source_plural) => {
                    let translated_projection = project_icu_plural(&msgstr);
                    match translated_projection {
                        IcuPluralProjection::Projected(translated_plural)
                            if translated_plural.variable == source_plural.variable =>
                        {
                            CanonicalTranslation::Plural {
                                source: PluralSource {
                                    one: source_plural.branches.get("one").cloned(),
                                    other: source_plural
                                        .branches
                                        .get("other")
                                        .cloned()
                                        .unwrap_or_else(|| item.msgid.clone()),
                                },
                                translation_by_category: materialize_plural_categories(
                                    &sorted_plural_keys(&translated_plural.branches),
                                    translated_plural.branches,
                                ),
                                variable: source_plural.variable,
                            }
                        }
                        IcuPluralProjection::Projected(_) => {
                            if strict {
                                return Err(ApiError::Unsupported(
                                    "ICU plural source and translation use different variables"
                                        .to_owned(),
                                ));
                            }
                            diagnostics.push(
                            Diagnostic::new(
                                DiagnosticSeverity::Warning,
                                "plural.partial_icu_parse",
                                "Could not safely align ICU plural source and translation; keeping the message as singular.",
                            )
                            .with_identity(&item.msgid, item.msgctxt.as_deref()),
                        );
                            CanonicalTranslation::Singular { value: msgstr }
                        }
                        IcuPluralProjection::Unsupported(_) | IcuPluralProjection::Malformed => {
                            if strict
                                && matches!(translated_projection, IcuPluralProjection::Malformed)
                            {
                                return Err(ApiError::Unsupported(
                                    "ICU plural message could not be parsed in strict mode"
                                        .to_owned(),
                                ));
                            }
                            diagnostics.push(
                            Diagnostic::new(
                                DiagnosticSeverity::Warning,
                                "plural.partial_icu_parse",
                                "Could not fully parse ICU plural translation; keeping the message as singular.",
                            )
                            .with_identity(&item.msgid, item.msgctxt.as_deref()),
                        );
                            CanonicalTranslation::Singular { value: msgstr }
                        }
                        IcuPluralProjection::NotPlural => {
                            diagnostics.push(
                            Diagnostic::new(
                                DiagnosticSeverity::Warning,
                                "plural.partial_icu_parse",
                                "Could not fully parse ICU plural translation; keeping the message as singular.",
                            )
                            .with_identity(&item.msgid, item.msgctxt.as_deref()),
                        );
                            CanonicalTranslation::Singular { value: msgstr }
                        }
                    }
                }
                IcuPluralProjection::Malformed if strict => {
                    return Err(ApiError::Unsupported(
                        "ICU plural parsing failed in strict mode".to_owned(),
                    ));
                }
                IcuPluralProjection::Unsupported(message) => {
                    diagnostics.push(
                        Diagnostic::new(
                            DiagnosticSeverity::Warning,
                            "plural.unsupported_icu_projection",
                            message,
                        )
                        .with_identity(&item.msgid, item.msgctxt.as_deref()),
                    );
                    CanonicalTranslation::Singular { value: msgstr }
                }
                IcuPluralProjection::NotPlural | IcuPluralProjection::Malformed => {
                    CanonicalTranslation::Singular { value: msgstr }
                }
            }
        } else {
            CanonicalTranslation::Singular { value: msgstr }
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
        translator_comments: item.comments,
        flags: item.flags,
    })
}

fn split_placeholder_comments(
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

fn parse_placeholder_comment(comment: &str) -> Option<(String, String)> {
    let rest = comment.strip_prefix("placeholder {")?;
    let end = rest.find("}: ")?;
    Some((rest[..end].to_owned(), rest[end + 3..].to_owned()))
}

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

fn validate_plural_forms_header(
    locale: Option<&str>,
    plural_forms: &ParsedPluralFormsHeader,
    plural_encoding: PluralEncoding,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if plural_encoding != PluralEncoding::Gettext {
        return;
    }

    if let Some(nplurals) = plural_forms.nplurals {
        let profile = PluralProfile::for_locale(locale);
        let expected = profile.nplurals();
        if locale.is_some() && nplurals != expected {
            diagnostics.push(Diagnostic::new(
                DiagnosticSeverity::Warning,
                "plural.nplurals_locale_mismatch",
                format!(
                    "Plural-Forms declares nplurals={nplurals}, but locale-derived categories expect {expected}."
                ),
            ));
        }
    } else if plural_forms.plural.is_some() {
        diagnostics.push(Diagnostic::new(
            DiagnosticSeverity::Warning,
            "parse.invalid_plural_forms_header",
            "Plural-Forms header contains a plural expression but no parseable nplurals value.",
        ));
    }

    if plural_forms.nplurals.is_some() && plural_forms.plural.is_none() {
        diagnostics.push(Diagnostic::new(
            DiagnosticSeverity::Info,
            "plural.missing_plural_expression",
            "Plural-Forms header declares nplurals but omits the plural expression.",
        ));
    }
}

fn public_message_from_canonical(message: CanonicalMessage) -> CatalogMessage {
    let translation = match message.translation {
        CanonicalTranslation::Singular { value } => TranslationShape::Singular { value },
        CanonicalTranslation::Plural {
            source,
            translation_by_category,
            ..
        } => TranslationShape::Plural {
            source,
            translation: translation_by_category,
        },
    };

    CatalogMessage {
        msgid: message.msgid,
        msgctxt: message.msgctxt,
        translation,
        comments: message.comments,
        origin: message.origins,
        obsolete: message.obsolete,
        extra: Some(CatalogMessageExtra {
            translator_comments: message.translator_comments,
            flags: message.flags,
        }),
    }
}

fn icu_plural_categories_for(locale: &str) -> Option<Vec<String>> {
    static CACHE: OnceLock<PluralCategoryCache> = OnceLock::new();

    cached_icu_plural_categories_for(locale, CACHE.get_or_init(|| Mutex::new(HashMap::new())))
}

fn cached_icu_plural_categories_for(
    locale: &str,
    cache: &PluralCategoryCache,
) -> Option<Vec<String>> {
    let normalized = normalize_plural_locale(locale);
    if normalized.is_empty() {
        return None;
    }

    let cached = match cache.lock() {
        Ok(guard) => guard.get(&normalized).cloned(),
        Err(poisoned) => poisoned.into_inner().get(&normalized).cloned(),
    };
    if let Some(cached) = cached {
        return cached;
    }

    let resolved = normalized
        .parse::<Locale>()
        .ok()
        .and_then(|locale| PluralRules::try_new_cardinal(locale.into()).ok())
        .map(|rules| {
            rules
                .categories()
                .map(plural_category_name)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        });

    match cache.lock() {
        Ok(mut guard) => {
            guard.insert(normalized, resolved.clone());
        }
        Err(poisoned) => {
            poisoned.into_inner().insert(normalized, resolved.clone());
        }
    }

    resolved
}

fn normalize_plural_locale(locale: &str) -> String {
    locale.trim().replace('_', "-")
}

const fn plural_category_name(category: PluralCategory) -> &'static str {
    match category {
        PluralCategory::Zero => "zero",
        PluralCategory::One => "one",
        PluralCategory::Two => "two",
        PluralCategory::Few => "few",
        PluralCategory::Many => "many",
        PluralCategory::Other => "other",
    }
}

fn fallback_plural_categories(nplurals: Option<usize>) -> Vec<String> {
    let categories = match nplurals.unwrap_or(2) {
        0 | 1 => vec!["other"],
        2 => vec!["one", "other"],
        3 => vec!["one", "few", "other"],
        4 => vec!["one", "few", "many", "other"],
        5 => vec!["zero", "one", "few", "many", "other"],
        _ => vec!["zero", "one", "two", "few", "many", "other"],
    };

    categories.into_iter().map(str::to_owned).collect()
}

fn sorted_plural_keys(map: &BTreeMap<String, String>) -> Vec<String> {
    let mut keys: Vec<_> = map.keys().cloned().collect();
    keys.sort_by_key(|key| plural_key_rank(key));
    if !keys.iter().any(|key| key == "other") {
        keys.push("other".to_owned());
    }
    keys
}

fn plural_key_rank(key: &str) -> usize {
    match key {
        "zero" => 0,
        "one" => 1,
        "two" => 2,
        "few" => 3,
        "many" => 4,
        "other" => 5,
        _ => 6,
    }
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        if !push_unique_string(&out, &value) {
            out.push(value);
        }
    }
    out
}

fn merge_unique_strings(target: &mut Vec<String>, incoming: Vec<String>) {
    if target.len() + incoming.len() < 8 {
        for value in incoming {
            if !push_unique_string(target, &value) {
                target.push(value);
            }
        }
        return;
    }

    let mut seen = target.iter().cloned().collect::<BTreeSet<_>>();
    for value in incoming {
        if seen.insert(value.clone()) {
            target.push(value);
        }
    }
}

fn push_unique_string(target: &[String], value: &str) -> bool {
    if target.len() < 8 {
        target.iter().any(|existing| existing == value)
    } else {
        target
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .contains(value)
    }
}

fn dedupe_origins(values: Vec<CatalogOrigin>) -> Vec<CatalogOrigin> {
    let mut out = Vec::new();
    for value in values {
        if !push_unique_origin(&out, &value) {
            out.push(value);
        }
    }
    out
}

fn merge_unique_origins(target: &mut Vec<CatalogOrigin>, incoming: Vec<CatalogOrigin>) {
    if target.len() + incoming.len() < 8 {
        for value in incoming {
            if !push_unique_origin(target, &value) {
                target.push(value);
            }
        }
        return;
    }

    let mut seen = target
        .iter()
        .map(|origin| (origin.file.clone(), origin.line))
        .collect::<BTreeSet<_>>();
    for value in incoming {
        if seen.insert((value.file.clone(), value.line)) {
            target.push(value);
        }
    }
}

fn push_unique_origin(target: &[CatalogOrigin], value: &CatalogOrigin) -> bool {
    if target.len() < 8 {
        target
            .iter()
            .any(|origin| origin.file == value.file && origin.line == value.line)
    } else {
        target
            .iter()
            .any(|origin| origin.file == value.file && origin.line == value.line)
    }
}

fn dedupe_placeholders(
    placeholders: BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, Vec<String>> {
    placeholders
        .into_iter()
        .map(|(key, values)| (key, dedupe_strings(values)))
        .collect()
}

fn merge_placeholders(
    target: &mut BTreeMap<String, Vec<String>>,
    incoming: BTreeMap<String, Vec<String>>,
) {
    for (key, values) in incoming {
        merge_unique_strings(target.entry(key).or_default(), values);
    }
}

fn derive_plural_variable(placeholders: &BTreeMap<String, Vec<String>>) -> Option<String> {
    if placeholders.contains_key("count") {
        return Some("count".to_owned());
    }

    let mut named = placeholders
        .keys()
        .filter(|key| !key.chars().all(|ch| ch.is_ascii_digit()))
        .cloned();
    let first = named.next()?;
    if named.next().is_none() {
        Some(first)
    } else {
        None
    }
}

fn synthesize_icu_plural(variable: &str, branches: &BTreeMap<String, String>) -> String {
    let mut out = String::new();
    out.push('{');
    out.push_str(variable);
    out.push_str(", plural,");
    for (category, value) in branches {
        out.push(' ');
        out.push_str(category);
        out.push_str(" {");
        out.push_str(value);
        out.push('}');
    }
    out.push('}');
    out
}

fn project_icu_plural(input: &str) -> IcuPluralProjection {
    if !looks_like_icu_message(input.as_bytes()) {
        return IcuPluralProjection::NotPlural;
    }

    let Ok(message) = parse_icu(input) else {
        return IcuPluralProjection::Malformed;
    };

    let Some(IcuNode::Plural {
        name,
        kind: IcuPluralKind::Cardinal,
        offset,
        options,
    }) = only_node(&message)
    else {
        return IcuPluralProjection::NotPlural;
    };

    if *offset != 0 {
        return IcuPluralProjection::Unsupported(
            "ICU plural offset syntax is not projected into the current catalog plural model.",
        );
    }

    let mut branches = BTreeMap::new();
    for option in options {
        if option.selector.starts_with('=') {
            return IcuPluralProjection::Unsupported(
                "ICU exact-match plural selectors are not projected into the current catalog plural model.",
            );
        }

        let value = match render_projectable_icu_nodes(&option.value) {
            Ok(value) => value,
            Err(message) => return IcuPluralProjection::Unsupported(message),
        };
        branches.insert(option.selector.clone(), value);
    }

    if !branches.contains_key("other") {
        return IcuPluralProjection::Malformed;
    }

    IcuPluralProjection::Projected(ParsedIcuPlural {
        variable: name.clone(),
        branches,
    })
}

#[inline]
fn looks_like_icu_message(input: &[u8]) -> bool {
    input.iter().any(|byte| matches!(byte, b'{' | b'}' | b'<'))
}

fn only_node(message: &IcuMessage) -> Option<&IcuNode> {
    match message.nodes.as_slice() {
        [node] => Some(node),
        _ => None,
    }
}

fn render_projectable_icu_nodes(nodes: &[IcuNode]) -> Result<String, &'static str> {
    let mut out = String::new();
    for node in nodes {
        render_projectable_icu_node(node, &mut out)?;
    }
    Ok(out)
}

fn render_projectable_icu_node(node: &IcuNode, out: &mut String) -> Result<(), &'static str> {
    match node {
        IcuNode::Literal(value) => append_escaped_icu_literal(out, value),
        IcuNode::Argument { name } => {
            out.push('{');
            out.push_str(name);
            out.push('}');
        }
        IcuNode::Number { name, style } => render_formatter("number", name, style.as_deref(), out),
        IcuNode::Date { name, style } => render_formatter("date", name, style.as_deref(), out),
        IcuNode::Time { name, style } => render_formatter("time", name, style.as_deref(), out),
        IcuNode::List { name, style } => render_formatter("list", name, style.as_deref(), out),
        IcuNode::Duration { name, style } => {
            render_formatter("duration", name, style.as_deref(), out);
        }
        IcuNode::Ago { name, style } => render_formatter("ago", name, style.as_deref(), out),
        IcuNode::Name { name, style } => render_formatter("name", name, style.as_deref(), out),
        IcuNode::Pound => out.push('#'),
        IcuNode::Tag { name, children } => {
            out.push('<');
            out.push_str(name);
            out.push('>');
            for child in children {
                render_projectable_icu_node(child, out)?;
            }
            out.push_str("</");
            out.push_str(name);
            out.push('>');
        }
        IcuNode::Select { .. } | IcuNode::Plural { .. } => {
            return Err(
                "Nested ICU select/plural structures are not projected into the current catalog plural model.",
            );
        }
    }

    Ok(())
}

fn render_formatter(kind: &str, name: &str, style: Option<&str>, out: &mut String) {
    out.push('{');
    out.push_str(name);
    out.push_str(", ");
    out.push_str(kind);
    if let Some(style) = style {
        out.push_str(", ");
        out.push_str(style);
    }
    out.push('}');
}

fn append_escaped_icu_literal(out: &mut String, value: &str) {
    if !value
        .as_bytes()
        .iter()
        .any(|byte| matches!(byte, b'\'' | b'{' | b'}' | b'#' | b'<' | b'>'))
    {
        out.push_str(value);
        return;
    }

    for ch in value.chars() {
        match ch {
            '\'' => out.push_str("''"),
            '{' | '}' | '#' | '<' | '>' => {
                out.push('\'');
                out.push(ch);
                out.push('\'');
            }
            _ => out.push(ch),
        }
    }
}

fn atomic_write(path: &Path, content: &str) -> Result<(), ApiError> {
    let directory = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(directory)?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            ApiError::InvalidArguments("target_path must have a file name".to_owned())
        })?;
    let temp_path = directory.join(format!(".{file_name}.ferrocat.tmp"));
    fs::write(&temp_path, content)?;
    fs::rename(&temp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        ApiError, CatalogMessageKey, CatalogUpdateInput, CompileCatalogOptions,
        CompiledKeyStrategy, CompiledTranslation, DiagnosticSeverity, EffectiveTranslation,
        EffectiveTranslationRef, ExtractedMessage, ExtractedPluralMessage,
        ExtractedSingularMessage, ObsoleteStrategy, ParseCatalogOptions, PluralEncoding,
        PluralSource, SourceExtractedMessage, TranslationShape, UpdateCatalogFileOptions,
        UpdateCatalogOptions, cached_icu_plural_categories_for, compiled_key_for, parse_catalog,
        update_catalog, update_catalog_file,
    };
    use crate::parse_po;
    use std::collections::{BTreeMap, HashMap};
    use std::fs;
    use std::sync::Mutex;

    fn structured_input(messages: Vec<ExtractedMessage>) -> CatalogUpdateInput {
        CatalogUpdateInput::Structured(messages)
    }

    fn source_first_input(messages: Vec<SourceExtractedMessage>) -> CatalogUpdateInput {
        CatalogUpdateInput::SourceFirst(messages)
    }

    fn normalized_catalog(
        content: &str,
        locale: Option<&str>,
        plural_encoding: PluralEncoding,
    ) -> super::NormalizedParsedCatalog {
        parse_catalog(ParseCatalogOptions {
            content: content.to_owned(),
            source_locale: "en".to_owned(),
            locale: locale.map(str::to_owned),
            plural_encoding,
            ..ParseCatalogOptions::default()
        })
        .expect("parse catalog")
        .into_normalized_view()
        .expect("normalized view")
    }

    #[test]
    fn compile_catalog_returns_empty_catalog_for_empty_input() {
        let normalized = normalized_catalog("", Some("de"), PluralEncoding::Icu);
        let compiled = normalized
            .compile(&CompileCatalogOptions::default())
            .expect("compile");

        assert!(compiled.is_empty());
        assert_eq!(compiled.len(), 0);
        assert!(compiled.get("missing").is_none());
    }

    #[test]
    fn compile_catalog_preserves_singular_translation_and_source_key() {
        let normalized = normalized_catalog(
            "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
            Some("de"),
            PluralEncoding::Icu,
        );
        let compiled = normalized
            .compile(&CompileCatalogOptions::default())
            .expect("compile");

        let (_, message) = compiled.iter().next().expect("compiled message");
        assert_eq!(message.source_key, CatalogMessageKey::new("Hello", None));
        assert!(matches!(
            &message.translation,
            CompiledTranslation::Singular(value) if value == "Hallo"
        ));
        assert_eq!(compiled.get(&message.key), Some(message));
    }

    #[test]
    fn compile_catalog_changes_key_when_context_changes() {
        let without_context = compiled_key_for(
            CompiledKeyStrategy::FerrocatV1,
            &CatalogMessageKey::new("Save", None),
        );
        let with_context = compiled_key_for(
            CompiledKeyStrategy::FerrocatV1,
            &CatalogMessageKey::new("Save", Some("menu".to_owned())),
        );
        let repeated = compiled_key_for(
            CompiledKeyStrategy::FerrocatV1,
            &CatalogMessageKey::new("Save", None),
        );

        assert_eq!(without_context, repeated);
        assert_ne!(without_context, with_context);
        assert_eq!(without_context.len(), 11);
        assert!(
            without_context
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        );
    }

    #[test]
    fn compile_catalog_changes_key_when_msgid_changes() {
        let left = compiled_key_for(
            CompiledKeyStrategy::FerrocatV1,
            &CatalogMessageKey::new("Save", None),
        );
        let right = compiled_key_for(
            CompiledKeyStrategy::FerrocatV1,
            &CatalogMessageKey::new("Store", None),
        );

        assert_ne!(left, right);
    }

    #[test]
    fn compile_catalog_preserves_plural_translation_shape() {
        let normalized = normalized_catalog(
            concat!(
                "msgid \"\"\n",
                "msgstr \"\"\n",
                "\"Language: ru\\n\"\n",
                "\"Plural-Forms: nplurals=3; plural=(n%10==1 && n%100!=11 ? 0 : ",
                "n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2);\\n\"\n\n",
                "msgid \"day\"\n",
                "msgid_plural \"days\"\n",
                "msgstr[0] \"den\"\n",
                "msgstr[1] \"dnya\"\n",
                "msgstr[2] \"dney\"\n",
            ),
            Some("ru"),
            PluralEncoding::Gettext,
        );
        let compiled = normalized
            .compile(&CompileCatalogOptions::default())
            .expect("compile");

        let (_, message) = compiled.iter().next().expect("compiled message");
        match &message.translation {
            CompiledTranslation::Plural(values) => {
                assert_eq!(values.get("one").map(String::as_str), Some("den"));
                assert_eq!(values.get("few").map(String::as_str), Some("dnya"));
                assert!(values.values().any(|value| value == "dney"));
            }
            other => panic!("expected plural translation, got {other:?}"),
        }
    }

    #[test]
    fn compile_catalog_keeps_empty_source_values_by_default() {
        let normalized = normalized_catalog(
            "msgid \"Hello\"\nmsgstr \"\"\n",
            Some("en"),
            PluralEncoding::Icu,
        );
        let compiled = normalized
            .compile(&CompileCatalogOptions::default())
            .expect("compile");

        let (_, message) = compiled.iter().next().expect("compiled message");
        assert!(matches!(
            &message.translation,
            CompiledTranslation::Singular(value) if value.is_empty()
        ));
    }

    #[test]
    fn compile_catalog_can_fill_source_values_when_requested() {
        let normalized = normalized_catalog(
            "msgid \"Hello\"\nmsgstr \"\"\n",
            Some("en"),
            PluralEncoding::Icu,
        );
        let compiled = normalized
            .compile(&CompileCatalogOptions {
                source_fallback: true,
                source_locale: Some("en".to_owned()),
                ..CompileCatalogOptions::default()
            })
            .expect("compile");

        let (_, message) = compiled.iter().next().expect("compiled message");
        assert!(matches!(
            &message.translation,
            CompiledTranslation::Singular(value) if value == "Hello"
        ));
    }

    #[test]
    fn compile_catalog_requires_source_locale_when_source_fallback_is_enabled() {
        let normalized = normalized_catalog(
            "msgid \"Hello\"\nmsgstr \"\"\n",
            Some("en"),
            PluralEncoding::Icu,
        );
        let error = normalized
            .compile(&CompileCatalogOptions {
                source_fallback: true,
                source_locale: None,
                ..CompileCatalogOptions::default()
            })
            .expect_err("missing source locale");

        match error {
            ApiError::InvalidArguments(message) => {
                assert!(message.contains("source_locale"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn compile_catalog_reports_key_collisions() {
        let normalized = normalized_catalog(
            concat!(
                "msgid \"Hello\"\n",
                "msgstr \"Hallo\"\n\n",
                "msgctxt \"menu\"\n",
                "msgid \"Save\"\n",
                "msgstr \"Speichern\"\n",
            ),
            Some("de"),
            PluralEncoding::Icu,
        );
        let error = normalized
            .compile_with_key_generator(&CompileCatalogOptions::default(), |_, _| {
                "fc1_collision".to_owned()
            })
            .expect_err("collision");

        match error {
            ApiError::Conflict(message) => {
                assert!(message.contains("Hello"));
                assert!(message.contains("Save"));
                assert!(message.contains("collision"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn update_catalog_creates_new_source_locale_messages() {
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("en".to_owned()),
            input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            })]),
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        let parsed = parse_po(&result.content).expect("parse output");
        assert_eq!(parsed.items.len(), 1);
        assert_eq!(parsed.items[0].msgid, "Hello");
        assert_eq!(parsed.items[0].msgstr[0], "Hello");
        assert!(result.created);
        assert!(result.updated);
        assert_eq!(result.stats.added, 1);
    }

    #[test]
    fn update_catalog_preserves_non_source_translations() {
        let existing = "msgid \"Hello\"\nmsgstr \"Hallo\"\n";
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("de".to_owned()),
            existing: Some(existing.to_owned()),
            input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            })]),
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        let parsed = parse_po(&result.content).expect("parse output");
        assert_eq!(parsed.items[0].msgstr[0], "Hallo");
        assert_eq!(result.stats.unchanged, 1);
    }

    #[test]
    fn overwrite_source_translations_refreshes_source_locale() {
        let existing = "msgid \"Hello\"\nmsgstr \"Old\"\n";
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("en".to_owned()),
            existing: Some(existing.to_owned()),
            overwrite_source_translations: true,
            input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            })]),
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        let parsed = parse_po(&result.content).expect("parse output");
        assert_eq!(parsed.items[0].msgstr[0], "Hello");
        assert_eq!(result.stats.changed, 1);
    }

    #[test]
    fn obsolete_strategy_delete_removes_missing_messages() {
        let existing = "msgid \"keep\"\nmsgstr \"x\"\n\nmsgid \"drop\"\nmsgstr \"y\"\n";
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("de".to_owned()),
            existing: Some(existing.to_owned()),
            obsolete_strategy: ObsoleteStrategy::Delete,
            input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "keep".to_owned(),
                ..ExtractedSingularMessage::default()
            })]),
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        let parsed = parse_po(&result.content).expect("parse output");
        assert_eq!(parsed.items.len(), 1);
        assert_eq!(result.stats.obsolete_removed, 1);
    }

    #[test]
    fn duplicate_conflicts_fail_hard() {
        let error = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("en".to_owned()),
            input: structured_input(vec![
                ExtractedMessage::Singular(ExtractedSingularMessage {
                    msgid: "Hello".to_owned(),
                    ..ExtractedSingularMessage::default()
                }),
                ExtractedMessage::Plural(ExtractedPluralMessage {
                    msgid: "Hello".to_owned(),
                    source: PluralSource {
                        one: Some("One".to_owned()),
                        other: "Many".to_owned(),
                    },
                    ..ExtractedPluralMessage::default()
                }),
            ]),
            ..UpdateCatalogOptions::default()
        })
        .expect_err("conflict");

        assert!(matches!(error, ApiError::Conflict(_)));
    }

    #[test]
    fn plural_icu_export_uses_structural_input() {
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("en".to_owned()),
            input: structured_input(vec![ExtractedMessage::Plural(ExtractedPluralMessage {
                msgid: "{count, plural, one {# item} other {# items}}".to_owned(),
                source: PluralSource {
                    one: Some("# item".to_owned()),
                    other: "# items".to_owned(),
                },
                placeholders: BTreeMap::from([("count".to_owned(), vec!["count".to_owned()])]),
                ..ExtractedPluralMessage::default()
            })]),
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        let parsed = parse_po(&result.content).expect("parse output");
        assert!(parsed.items[0].msgid.contains("{count, plural,"));
        assert!(parsed.items[0].msgid_plural.is_none());
    }

    #[test]
    fn source_first_plain_messages_normalize_as_singular() {
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("en".to_owned()),
            input: source_first_input(vec![SourceExtractedMessage {
                msgid: "Welcome".to_owned(),
                ..SourceExtractedMessage::default()
            }]),
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        let parsed = parse_po(&result.content).expect("parse output");
        assert_eq!(parsed.items[0].msgid, "Welcome");
        assert_eq!(parsed.items[0].msgstr[0], "Welcome");
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn source_first_simple_icu_plural_projects_into_plural_update() {
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("en".to_owned()),
            input: source_first_input(vec![SourceExtractedMessage {
                msgid: "{items, plural, one {# file} other {# files}}".to_owned(),
                placeholders: BTreeMap::from([("items".to_owned(), vec!["items".to_owned()])]),
                ..SourceExtractedMessage::default()
            }]),
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        let parsed = parse_po(&result.content).expect("parse output");
        assert!(parsed.items[0].msgid.contains("{items, plural,"));
        assert_eq!(
            parsed.items[0].msgstr[0],
            "{items, plural, one {# file} other {# files}}"
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn source_first_unsupported_icu_plural_falls_back_to_singular_with_warning() {
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("en".to_owned()),
            input: source_first_input(vec![SourceExtractedMessage {
                msgid: "{count, plural, one {{gender, select, male {He has one file} other {They have one file}}} other {{gender, select, male {He has # files} other {They have # files}}}}".to_owned(),
                ..SourceExtractedMessage::default()
            }]),
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        let parsed = parse_po(&result.content).expect("parse output");
        assert_eq!(
            parsed.items[0].msgid,
            "{count, plural, one {{gender, select, male {He has one file} other {They have one file}}} other {{gender, select, male {He has # files} other {They have # files}}}}"
        );
        assert_eq!(parsed.items[0].msgstr[0], parsed.items[0].msgid);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "plural.source_first_fallback")
        );
    }

    #[test]
    fn parse_catalog_projects_gettext_plural_into_structured_shape() {
        let parsed = parse_catalog(ParseCatalogOptions {
            content: concat!(
                "msgid \"book\"\n",
                "msgid_plural \"books\"\n",
                "msgstr[0] \"Buch\"\n",
                "msgstr[1] \"Buecher\"\n",
            )
            .to_owned(),
            locale: Some("de".to_owned()),
            source_locale: "en".to_owned(),
            plural_encoding: PluralEncoding::Gettext,
            strict: false,
        })
        .expect("parse");

        match &parsed.messages[0].translation {
            TranslationShape::Plural {
                source,
                translation,
            } => {
                assert_eq!(source.one.as_deref(), Some("book"));
                assert_eq!(source.other, "books");
                assert_eq!(translation.get("one").map(String::as_str), Some("Buch"));
                assert_eq!(
                    translation.get("other").map(String::as_str),
                    Some("Buecher")
                );
            }
            other => panic!("expected plural translation, got {other:?}"),
        }
    }

    #[test]
    fn normalized_view_indexes_messages_by_key() {
        let parsed = parse_catalog(ParseCatalogOptions {
            content: concat!(
                "msgctxt \"nav\"\n",
                "msgid \"Home\"\n",
                "msgstr \"Start\"\n",
            )
            .to_owned(),
            locale: Some("de".to_owned()),
            source_locale: "en".to_owned(),
            plural_encoding: PluralEncoding::Gettext,
            strict: false,
        })
        .expect("parse");

        let normalized = parsed.into_normalized_view().expect("normalized view");
        let key = CatalogMessageKey::new("Home", Some("nav".to_owned()));

        assert!(normalized.contains_key(&key));
        assert_eq!(normalized.message_count(), 1);
        assert!(matches!(
            normalized.effective_translation(&key),
            Some(EffectiveTranslationRef::Singular("Start"))
        ));
        assert_eq!(normalized.iter().count(), 1);
    }

    #[test]
    fn normalized_view_rejects_duplicate_keys() {
        let parsed = parse_catalog(ParseCatalogOptions {
            content: concat!(
                "msgid \"Hello\"\n",
                "msgstr \"Hallo\"\n",
                "\n",
                "msgid \"Hello\"\n",
                "msgstr \"Servus\"\n",
            )
            .to_owned(),
            locale: Some("de".to_owned()),
            source_locale: "en".to_owned(),
            plural_encoding: PluralEncoding::Gettext,
            strict: false,
        })
        .expect("parse");

        let error = parsed
            .into_normalized_view()
            .expect_err("duplicate keys should fail");
        assert!(matches!(error, ApiError::Conflict(_)));
    }

    #[test]
    fn normalized_view_can_apply_source_locale_fallbacks() {
        let parsed = parse_catalog(ParseCatalogOptions {
            content: concat!(
                "msgid \"book\"\n",
                "msgid_plural \"books\"\n",
                "msgstr[0] \"\"\n",
                "msgstr[1] \"\"\n",
                "\n",
                "msgid \"Welcome\"\n",
                "msgstr \"\"\n",
            )
            .to_owned(),
            locale: Some("en".to_owned()),
            source_locale: "en".to_owned(),
            plural_encoding: PluralEncoding::Gettext,
            strict: false,
        })
        .expect("parse");

        let normalized = parsed.into_normalized_view().expect("normalized view");
        let plural_key = CatalogMessageKey::new("book", None);
        let singular_key = CatalogMessageKey::new("Welcome", None);

        assert!(matches!(
            normalized.effective_translation(&singular_key),
            Some(EffectiveTranslationRef::Singular(""))
        ));
        assert_eq!(
            normalized.effective_translation_with_source_fallback(&singular_key, "en"),
            Some(EffectiveTranslation::Singular("Welcome".to_owned()))
        );

        assert_eq!(
            normalized.effective_translation_with_source_fallback(&plural_key, "en"),
            Some(EffectiveTranslation::Plural(BTreeMap::from([
                ("one".to_owned(), "book".to_owned()),
                ("other".to_owned(), "books".to_owned()),
            ])))
        );
    }

    #[test]
    fn normalized_view_skips_source_fallback_for_non_source_locale_catalogs() {
        let parsed = parse_catalog(ParseCatalogOptions {
            content: concat!("msgid \"Hello\"\n", "msgstr \"\"\n").to_owned(),
            locale: Some("de".to_owned()),
            source_locale: "en".to_owned(),
            plural_encoding: PluralEncoding::Gettext,
            strict: false,
        })
        .expect("parse");

        let normalized = parsed.into_normalized_view().expect("normalized view");
        let key = CatalogMessageKey::new("Hello", None);

        assert_eq!(
            normalized.effective_translation_with_source_fallback(&key, "en"),
            Some(EffectiveTranslation::Singular(String::new()))
        );
    }

    #[test]
    fn parse_catalog_uses_icu_plural_categories_for_french_gettext() {
        let parsed = parse_catalog(ParseCatalogOptions {
            content: concat!(
                "msgid \"fichier\"\n",
                "msgid_plural \"fichiers\"\n",
                "msgstr[0] \"fichier\"\n",
                "msgstr[1] \"millions de fichiers\"\n",
                "msgstr[2] \"fichiers\"\n",
            )
            .to_owned(),
            locale: Some("fr".to_owned()),
            source_locale: "en".to_owned(),
            plural_encoding: PluralEncoding::Gettext,
            strict: false,
        })
        .expect("parse");

        match &parsed.messages[0].translation {
            TranslationShape::Plural { translation, .. } => {
                assert_eq!(translation.get("one").map(String::as_str), Some("fichier"));
                assert_eq!(
                    translation.get("many").map(String::as_str),
                    Some("millions de fichiers")
                );
                assert_eq!(
                    translation.get("other").map(String::as_str),
                    Some("fichiers")
                );
            }
            other => panic!("expected plural translation, got {other:?}"),
        }
    }

    #[test]
    fn parse_catalog_prefers_gettext_slot_count_when_it_disagrees_with_locale_categories() {
        let parsed = parse_catalog(ParseCatalogOptions {
            content: concat!(
                "msgid \"\"\n",
                "msgstr \"\"\n",
                "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n",
                "\n",
                "msgid \"livre\"\n",
                "msgid_plural \"livres\"\n",
                "msgstr[0] \"livre\"\n",
                "msgstr[1] \"livres\"\n",
            )
            .to_owned(),
            locale: Some("fr".to_owned()),
            source_locale: "en".to_owned(),
            plural_encoding: PluralEncoding::Gettext,
            strict: false,
        })
        .expect("parse");

        match &parsed.messages[0].translation {
            TranslationShape::Plural { translation, .. } => {
                assert_eq!(translation.len(), 2);
                assert_eq!(translation.get("one").map(String::as_str), Some("livre"));
                assert_eq!(translation.get("other").map(String::as_str), Some("livres"));
                assert!(translation.get("many").is_none());
            }
            other => panic!("expected plural translation, got {other:?}"),
        }
    }

    #[test]
    fn parse_catalog_reports_plural_forms_locale_mismatch() {
        let parsed = parse_catalog(ParseCatalogOptions {
            content: concat!(
                "msgid \"\"\n",
                "msgstr \"\"\n",
                "\"Language: fr\\n\"\n",
                "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n",
            )
            .to_owned(),
            locale: Some("fr".to_owned()),
            source_locale: "en".to_owned(),
            plural_encoding: PluralEncoding::Gettext,
            strict: false,
        })
        .expect("parse");

        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "plural.nplurals_locale_mismatch")
        );
    }

    #[test]
    fn parse_catalog_detects_simple_icu_plural_when_requested() {
        let parsed = parse_catalog(ParseCatalogOptions {
            content: concat!(
                "msgid \"{count, plural, one {# item} other {# items}}\"\n",
                "msgstr \"{count, plural, one {# Artikel} other {# Artikel}}\"\n",
            )
            .to_owned(),
            locale: Some("de".to_owned()),
            source_locale: "en".to_owned(),
            plural_encoding: PluralEncoding::Icu,
            strict: false,
        })
        .expect("parse");

        match &parsed.messages[0].translation {
            TranslationShape::Plural { translation, .. } => {
                assert_eq!(
                    translation.get("one").map(String::as_str),
                    Some("# Artikel")
                );
                assert_eq!(
                    translation.get("other").map(String::as_str),
                    Some("# Artikel")
                );
            }
            other => panic!("expected plural translation, got {other:?}"),
        }
    }

    #[test]
    fn parse_catalog_warns_and_falls_back_for_unsupported_nested_icu_plural() {
        let parsed = parse_catalog(ParseCatalogOptions {
            content: concat!(
                "msgid \"{count, plural, one {{gender, select, male {He has one item} other {They have one item}}} other {{gender, select, male {He has # items} other {They have # items}}}}\"\n",
                "msgstr \"{count, plural, one {{gender, select, male {Er hat einen Artikel} other {Sie haben einen Artikel}}} other {{gender, select, male {Er hat # Artikel} other {Sie haben # Artikel}}}}\"\n",
            )
            .to_owned(),
            locale: Some("de".to_owned()),
            source_locale: "en".to_owned(),
            plural_encoding: PluralEncoding::Icu,
            strict: false,
        })
        .expect("parse");

        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "plural.unsupported_icu_projection")
        );
        assert!(matches!(
            parsed.messages[0].translation,
            TranslationShape::Singular { .. }
        ));
    }

    #[test]
    fn parse_catalog_strict_fails_on_malformed_icu_plural() {
        let error = parse_catalog(ParseCatalogOptions {
            content: concat!(
                "msgid \"{count, plural, one {# item} other {# items}\"\n",
                "msgstr \"{count, plural, one {# Artikel} other {# Artikel}}\"\n",
            )
            .to_owned(),
            locale: Some("de".to_owned()),
            source_locale: "en".to_owned(),
            plural_encoding: PluralEncoding::Icu,
            strict: true,
        })
        .expect_err("strict parse should fail");

        match error {
            ApiError::Unsupported(message) => assert!(message.contains("strict mode")),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn update_catalog_file_writes_only_when_changed() {
        let temp_dir = std::env::temp_dir().join("ferrocat-po-update-file-test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("create temp dir");
        let path = temp_dir.join("messages.po");

        let first = update_catalog_file(UpdateCatalogFileOptions {
            target_path: path.clone(),
            source_locale: "en".to_owned(),
            locale: Some("en".to_owned()),
            input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            })]),
            ..UpdateCatalogFileOptions::default()
        })
        .expect("first write");
        assert!(first.created);

        let second = update_catalog_file(UpdateCatalogFileOptions {
            target_path: path.clone(),
            source_locale: "en".to_owned(),
            locale: Some("en".to_owned()),
            input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            })]),
            ..UpdateCatalogFileOptions::default()
        })
        .expect("second write");
        assert!(!second.created);
        assert!(!second.updated);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn update_catalog_gettext_export_emits_plural_slots() {
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("de".to_owned()),
            plural_encoding: PluralEncoding::Gettext,
            input: structured_input(vec![ExtractedMessage::Plural(ExtractedPluralMessage {
                msgid: "books".to_owned(),
                source: PluralSource {
                    one: Some("book".to_owned()),
                    other: "books".to_owned(),
                },
                placeholders: BTreeMap::from([("count".to_owned(), vec!["count".to_owned()])]),
                ..ExtractedPluralMessage::default()
            })]),
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        let parsed = parse_po(&result.content).expect("parse output");
        assert_eq!(parsed.items[0].msgid, "book");
        assert_eq!(parsed.items[0].msgid_plural.as_deref(), Some("books"));
        assert_eq!(parsed.items[0].msgstr.len(), 2);
    }

    #[test]
    fn update_catalog_gettext_export_uses_icu_plural_categories_for_french() {
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("fr".to_owned()),
            plural_encoding: PluralEncoding::Gettext,
            input: structured_input(vec![ExtractedMessage::Plural(ExtractedPluralMessage {
                msgid: "files".to_owned(),
                source: PluralSource {
                    one: Some("file".to_owned()),
                    other: "files".to_owned(),
                },
                placeholders: BTreeMap::from([("count".to_owned(), vec!["count".to_owned()])]),
                ..ExtractedPluralMessage::default()
            })]),
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        let parsed = parse_po(&result.content).expect("parse output");
        assert_eq!(parsed.items[0].msgstr.len(), 3);
    }

    #[test]
    fn update_catalog_gettext_sets_safe_plural_forms_header_for_two_form_locale() {
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("de".to_owned()),
            plural_encoding: PluralEncoding::Gettext,
            input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            })]),
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        let parsed = parse_po(&result.content).expect("parse output");
        let plural_forms = parsed
            .headers
            .iter()
            .find(|header| header.key == "Plural-Forms")
            .map(|header| header.value.as_str());
        assert_eq!(plural_forms, Some("nplurals=2; plural=(n != 1);"));
    }

    #[test]
    fn update_catalog_gettext_reports_when_no_safe_plural_forms_header_is_known() {
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("fr".to_owned()),
            plural_encoding: PluralEncoding::Gettext,
            input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Bonjour".to_owned(),
                ..ExtractedSingularMessage::default()
            })]),
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "plural.missing_plural_forms_header")
        );
    }

    #[test]
    fn update_catalog_gettext_completes_partial_plural_forms_header_when_safe() {
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("de".to_owned()),
            plural_encoding: PluralEncoding::Gettext,
            existing: Some(
                concat!(
                    "msgid \"\"\n",
                    "msgstr \"\"\n",
                    "\"Language: de\\n\"\n",
                    "\"Plural-Forms: nplurals=2;\\n\"\n",
                )
                .to_owned(),
            ),
            input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            })]),
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "plural.completed_plural_forms_header")
        );

        let parsed = parse_po(&result.content).expect("parse output");
        let plural_forms = parsed
            .headers
            .iter()
            .find(|header| header.key == "Plural-Forms")
            .map(|header| header.value.as_str());
        assert_eq!(plural_forms, Some("nplurals=2; plural=(n != 1);"));
    }

    #[test]
    fn update_catalog_gettext_preserves_existing_complete_plural_forms_header() {
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("de".to_owned()),
            plural_encoding: PluralEncoding::Gettext,
            existing: Some(
                concat!(
                    "msgid \"\"\n",
                    "msgstr \"\"\n",
                    "\"Language: de\\n\"\n",
                    "\"Plural-Forms: nplurals=2; plural=(n > 1);\\n\"\n",
                )
                .to_owned(),
            ),
            input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            })]),
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        assert!(
            !result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "plural.completed_plural_forms_header")
        );

        let parsed = parse_po(&result.content).expect("parse output");
        let plural_forms = parsed
            .headers
            .iter()
            .find(|header| header.key == "Plural-Forms")
            .map(|header| header.value.as_str());
        assert_eq!(plural_forms, Some("nplurals=2; plural=(n > 1);"));
    }

    #[test]
    fn parse_catalog_requires_source_locale() {
        let error = parse_catalog(ParseCatalogOptions {
            content: String::new(),
            source_locale: String::new(),
            ..ParseCatalogOptions::default()
        })
        .expect_err("missing source locale");

        match error {
            ApiError::InvalidArguments(message) => {
                assert!(message.contains("source_locale"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn warnings_use_expected_namespace() {
        let mut placeholders = BTreeMap::new();
        placeholders.insert("first".to_owned(), vec!["first".to_owned()]);
        placeholders.insert("second".to_owned(), vec!["second".to_owned()]);

        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("de".to_owned()),
            input: structured_input(vec![ExtractedMessage::Plural(ExtractedPluralMessage {
                msgid: "Developers".to_owned(),
                source: PluralSource {
                    one: Some("Developer".to_owned()),
                    other: "Developers".to_owned(),
                },
                placeholders,
                ..ExtractedPluralMessage::default()
            })]),
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.starts_with("plural."))
        );
        assert!(result.diagnostics.iter().all(|diagnostic| matches!(
            diagnostic.severity,
            DiagnosticSeverity::Warning | DiagnosticSeverity::Error | DiagnosticSeverity::Info
        )));
    }

    #[test]
    fn cached_icu_plural_categories_reads_poisoned_cache_entries() {
        let cache = Mutex::new(HashMap::new());
        let _ = std::panic::catch_unwind(|| {
            let mut guard = cache.lock().expect("lock");
            guard.insert(
                "fr".to_owned(),
                Some(vec![
                    "one".to_owned(),
                    "many".to_owned(),
                    "other".to_owned(),
                ]),
            );
            panic!("poison cache");
        });

        let categories = cached_icu_plural_categories_for("fr", &cache);
        assert_eq!(
            categories,
            Some(vec![
                "one".to_owned(),
                "many".to_owned(),
                "other".to_owned()
            ])
        );
    }

    #[test]
    fn cached_icu_plural_categories_computes_with_poisoned_cache() {
        let cache = Mutex::new(HashMap::new());
        let _ = std::panic::catch_unwind(|| {
            let _guard = cache.lock().expect("lock");
            panic!("poison cache");
        });

        let categories = cached_icu_plural_categories_for("de", &cache);
        assert!(categories.is_some());
        assert!(
            categories
                .expect("categories")
                .iter()
                .any(|category| category == "other")
        );
    }
}
