#![warn(missing_docs, rustdoc::broken_intra_doc_links)]
//! Performance-first PO parsing and serialization.
//!
//! The crate exposes both owned and borrowed parsers for gettext PO files,
//! plus helpers for serialization and higher-level catalog update workflows.
//!
//! # Examples
//!
//! ```rust
//! use ferrocat_po::{PoFile, SerializeOptions, parse_po, stringify_po};
//!
//! let input = "msgid \"Hello\"\nmsgstr \"Hallo\"\n";
//! let file = parse_po(input)?;
//! assert_eq!(file.items[0].msgid, "Hello");
//!
//! let output = stringify_po(&file, &SerializeOptions::default());
//! assert!(output.contains("msgid \"Hello\""));
//! # Ok::<(), ferrocat_po::ParseError>(())
//! ```
//!
//! ```rust
//! use ferrocat_po::{
//!     CompileCatalogArtifactOptions, CompileSelectedCatalogArtifactOptions,
//!     CompiledCatalogIdIndex, ParseCatalogOptions, compile_catalog_artifact_selected,
//!     parse_catalog,
//! };
//!
//! let source = parse_catalog(ParseCatalogOptions {
//!     locale: Some("en"),
//!     ..ParseCatalogOptions::new("msgid \"Hello\"\nmsgstr \"Hello\"\n", "en")
//! })?
//! .into_normalized_view()?;
//! let requested = parse_catalog(ParseCatalogOptions {
//!     locale: Some("de"),
//!     ..ParseCatalogOptions::new("msgid \"Hello\"\nmsgstr \"Hallo\"\n", "en")
//! })?
//! .into_normalized_view()?;
//! let index = CompiledCatalogIdIndex::new(&[&requested, &source], ferrocat_po::CompiledKeyStrategy::FerrocatV1)?;
//! let compiled_ids = index.iter().map(|(id, _)| id.to_owned()).collect::<Vec<_>>();
//! let compiled = compile_catalog_artifact_selected(
//!     &[&requested, &source],
//!     &index,
//!     &CompileSelectedCatalogArtifactOptions::new("de", "en", &compiled_ids),
//! )?;
//!
//! assert_eq!(compiled.messages.len(), 1);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ```rust
//! use ferrocat_po::{CatalogAuditOptions, ParseCatalogOptions, audit_catalogs, parse_catalog};
//!
//! let source = parse_catalog(ParseCatalogOptions {
//!     locale: Some("en"),
//!     ..ParseCatalogOptions::new("msgid \"Hello {name}\"\nmsgstr \"Hello {name}\"\n", "en")
//! })?
//! .into_normalized_view()?;
//! let target = parse_catalog(ParseCatalogOptions {
//!     locale: Some("de"),
//!     ..ParseCatalogOptions::new("msgid \"Hello {name}\"\nmsgstr \"Hallo\"\n", "en")
//! })?
//! .into_normalized_view()?;
//! let report = audit_catalogs(&[&source, &target], &CatalogAuditOptions::new("en"))?;
//!
//! assert!(report.has_errors());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#[cfg(feature = "catalog")]
mod api;
mod borrowed;
pub mod diagnostic_codes;
mod merge;
mod parse;
mod scan;
mod serialize;
mod text;
mod utf8;

#[cfg(feature = "catalog")]
pub use api::{
    ApiError, COMPILED_CATALOG_ARTIFACT_SCHEMA_VERSION, CatalogAuditChecks, CatalogAuditDiagnostic,
    CatalogAuditMessageRef, CatalogAuditOptions, CatalogAuditReport, CatalogAuditSummary,
    CatalogCombineInput, CatalogCombineResult, CatalogCombineSelection, CatalogCombineStats,
    CatalogConflictStrategy, CatalogMessage, CatalogMessageExtra, CatalogMessageKey, CatalogMode,
    CatalogOrigin, CatalogSemantics, CatalogStats, CatalogStorageFormat, CatalogUpdateInput,
    CatalogUpdateResult, CombineCatalogOptions, CompileCatalogArtifactOptions,
    CompileCatalogOptions, CompileSelectedCatalogArtifactOptions, CompiledCatalog,
    CompiledCatalogArtifact, CompiledCatalogDiagnostic, CompiledCatalogIdDescription,
    CompiledCatalogIdIndex, CompiledCatalogMissingMessage, CompiledCatalogTranslationKind,
    CompiledCatalogUnavailableId, CompiledKeyStrategy, CompiledMessage, CompiledTranslation,
    DescribeCompiledIdsReport, Diagnostic, DiagnosticSeverity, EffectiveTranslation,
    EffectiveTranslationRef, ExtractedMessage, ExtractedPluralMessage, ExtractedSingularMessage,
    MachineTranslationMetadata, NdjsonCatalogReader, NdjsonCatalogReaderOptions,
    NdjsonCatalogWriter, NdjsonCatalogWriterOptions, NormalizedParsedCatalog, ObsoleteStrategy,
    OrderBy, ParseCatalogOptions, ParsedCatalog, PlaceholderCommentMode, PluralEncoding,
    PluralSource, RenderOptions, SourceExtractedMessage, TranslationShape,
    UpdateCatalogFileOptions, UpdateCatalogOptions, audit_catalogs, combine_catalogs,
    compile_catalog_artifact, compile_catalog_artifact_selected, compiled_key,
    machine_translation_hash, parse_catalog, update_catalog, update_catalog_file,
};
pub use borrowed::{
    BorrowedHeader, BorrowedMsgStr, BorrowedPoFile, BorrowedPoItem, parse_po_borrowed,
};
pub use merge::{ExtractedMessage as MergeExtractedMessage, merge_catalog};
pub use parse::parse_po;
pub use serialize::stringify_po;
pub use text::{escape_string, extract_quoted, extract_quoted_cow, unescape_string};

use core::{fmt, ops::Index};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// An owned PO document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct PoFile {
    /// File-level translator comments that appear before the header block.
    pub comments: Vec<String>,
    /// File-level extracted comments that appear before the header block.
    pub extracted_comments: Vec<String>,
    /// Parsed header entries from the leading empty `msgid` block.
    pub headers: Vec<Header>,
    /// Regular catalog items in source order.
    pub items: Vec<PoItem>,
}

/// A single header entry from the PO header block.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Header {
    /// Header name such as `Language` or `Plural-Forms`.
    pub key: String,
    /// Header value without the trailing newline.
    pub value: String,
}

/// A single gettext message entry.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct PoItem {
    /// Source message identifier.
    pub msgid: String,
    /// Optional gettext message context.
    pub msgctxt: Option<String>,
    /// Source references such as `src/app.rs:10`.
    pub references: Vec<String>,
    /// Optional plural source identifier.
    pub msgid_plural: Option<String>,
    /// Translation payload for the message.
    pub msgstr: MsgStr,
    /// Translator comments attached to the item.
    pub comments: Vec<String>,
    /// Extracted comments attached to the item.
    pub extracted_comments: Vec<String>,
    /// Flags such as `fuzzy`.
    pub flags: Vec<String>,
    /// Raw metadata lines that do not fit the dedicated fields.
    pub metadata: Vec<(String, String)>,
    /// Whether the item is marked obsolete.
    pub obsolete: bool,
    /// Number of plural slots expected when the item is serialized.
    pub nplurals: usize,
}

impl PoItem {
    /// Creates an empty message entry with space for `nplurals` plural slots.
    #[must_use]
    pub fn new(nplurals: usize) -> Self {
        Self {
            nplurals,
            ..Self::default()
        }
    }

    pub(crate) fn clear_for_reuse(&mut self, nplurals: usize) {
        self.msgid.clear();
        self.msgctxt = None;
        self.references.clear();
        self.msgid_plural = None;
        self.msgstr = MsgStr::None;
        self.comments.clear();
        self.extracted_comments.clear();
        self.flags.clear();
        self.metadata.clear();
        self.obsolete = false;
        self.nplurals = nplurals;
    }
}

/// Message translation payload for a PO item.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "kind", content = "value", rename_all = "snake_case")
)]
pub enum MsgStr {
    /// No translation values are present.
    #[default]
    None,
    /// Single translation string.
    Singular(String),
    /// Plural translation strings indexed by plural slot.
    Plural(Vec<String>),
}

impl MsgStr {
    /// Returns `true` when no translation values are present.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns the number of translation values present.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::None => 0,
            Self::Singular(_) => 1,
            Self::Plural(values) => values.len(),
        }
    }

    /// Returns the first translation value, if present.
    #[must_use]
    pub fn first(&self) -> Option<&String> {
        match self {
            Self::None => None,
            Self::Singular(value) => Some(value),
            Self::Plural(values) => values.first(),
        }
    }

    /// Returns the first translation value as `&str`, if present.
    #[must_use]
    pub fn first_str(&self) -> Option<&str> {
        self.first().map(String::as_str)
    }

    /// Returns the translation at `index` without panicking.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&str> {
        match self {
            Self::Singular(value) if index == 0 => Some(value.as_str()),
            Self::None | Self::Singular(_) => None,
            Self::Plural(values) => values.get(index).map(String::as_str),
        }
    }

    /// Iterates over all translation values in order.
    #[must_use]
    pub fn iter(&self) -> MsgStrIter<'_> {
        match self {
            Self::None => MsgStrIter::empty(),
            Self::Singular(value) => MsgStrIter::single(value),
            Self::Plural(values) => MsgStrIter::many(values.iter()),
        }
    }

    /// Converts the translation payload into an owned vector.
    #[must_use]
    pub fn into_vec(self) -> Vec<String> {
        match self {
            Self::None => Vec::new(),
            Self::Singular(value) => vec![value],
            Self::Plural(values) => values,
        }
    }
}

impl From<String> for MsgStr {
    fn from(value: String) -> Self {
        Self::Singular(value)
    }
}

impl From<Vec<String>> for MsgStr {
    fn from(values: Vec<String>) -> Self {
        match values.len() {
            0 => Self::None,
            1 => Self::Singular(values.into_iter().next().expect("single msgstr value")),
            _ => Self::Plural(values),
        }
    }
}

impl<'a> IntoIterator for &'a MsgStr {
    type Item = &'a String;
    type IntoIter = MsgStrIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Index<usize> for MsgStr {
    type Output = String;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            Self::None => panic!("msgstr index out of bounds: no translations present"),
            Self::Singular(value) if index == 0 => value,
            Self::Singular(_) => panic!("msgstr index out of bounds: singular translation"),
            Self::Plural(values) => &values[index],
        }
    }
}

/// Iterator over [`MsgStr`] values.
pub struct MsgStrIter<'a> {
    inner: MsgStrIterInner<'a>,
}

enum MsgStrIterInner<'a> {
    Empty,
    Single(Option<&'a String>),
    Many(std::slice::Iter<'a, String>),
}

impl<'a> MsgStrIter<'a> {
    const fn empty() -> Self {
        Self {
            inner: MsgStrIterInner::Empty,
        }
    }

    const fn single(value: &'a String) -> Self {
        Self {
            inner: MsgStrIterInner::Single(Some(value)),
        }
    }

    const fn many(iter: std::slice::Iter<'a, String>) -> Self {
        Self {
            inner: MsgStrIterInner::Many(iter),
        }
    }
}

impl<'a> Iterator for MsgStrIter<'a> {
    type Item = &'a String;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            MsgStrIterInner::Empty => None,
            MsgStrIterInner::Single(value) => value.take(),
            MsgStrIterInner::Many(iter) => iter.next(),
        }
    }
}

/// Options controlling PO serialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializeOptions {
    /// Preferred soft line-wrap limit for long string literals.
    pub fold_length: usize,
    /// When `true`, one-line values stay compact instead of always expanding.
    pub compact_multiline: bool,
}

impl Default for SerializeOptions {
    fn default() -> Self {
        Self {
            fold_length: 80,
            compact_multiline: true,
        }
    }
}

/// One-based line/column context plus the byte offset for a parse error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsePosition {
    offset: usize,
    line: usize,
    column: usize,
}

impl ParsePosition {
    /// Creates a new parse position.
    ///
    /// `offset` is zero-based and counts bytes from the parsed input after any
    /// parser-specific pre-processing, while `line` and `column` are one-based.
    #[must_use]
    pub const fn new(offset: usize, line: usize, column: usize) -> Self {
        Self {
            offset,
            line,
            column,
        }
    }

    /// Returns the zero-based byte offset in the parsed input.
    #[must_use]
    pub const fn offset(self) -> usize {
        self.offset
    }

    /// Returns the one-based line number.
    #[must_use]
    pub const fn line(self) -> usize {
        self.line
    }

    /// Returns the one-based column number.
    #[must_use]
    pub const fn column(self) -> usize {
        self.column
    }
}

/// Error returned when parsing or unescaping PO content fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    message: String,
    position: Option<ParsePosition>,
}

impl ParseError {
    /// Creates a new parse error with the provided message.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            position: None,
        }
    }

    /// Creates a new parse error with source position metadata.
    #[must_use]
    pub fn with_position(message: impl Into<String>, position: ParsePosition) -> Self {
        Self {
            message: message.into(),
            position: Some(position),
        }
    }

    /// Returns the human-readable error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns source position metadata when the parser could attach it.
    #[must_use]
    pub const fn position(&self) -> Option<ParsePosition> {
        self.position
    }

    pub(crate) fn with_position_if_missing(mut self, position: ParsePosition) -> Self {
        self.position.get_or_insert(position);
        self
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::{Header, MsgStr, ParseError, ParsePosition, PoFile, PoItem};

    #[test]
    fn parse_error_accessors_preserve_message_and_optional_position() {
        let error = ParseError::new("invalid PO string");
        assert_eq!(error.message(), "invalid PO string");
        assert_eq!(error.position(), None);
        assert_eq!(error.to_string(), "invalid PO string");

        let position = ParsePosition::new(12, 2, 3);
        let positioned = ParseError::with_position("invalid PO string", position);
        assert_eq!(positioned.message(), "invalid PO string");
        assert_eq!(positioned.position(), Some(position));
        assert_eq!(positioned.position().map(ParsePosition::offset), Some(12));
        assert_eq!(positioned.position().map(ParsePosition::line), Some(2));
        assert_eq!(positioned.position().map(ParsePosition::column), Some(3));
        assert_eq!(positioned.to_string(), "invalid PO string");
    }

    #[test]
    fn msgstr_get_returns_none_for_empty_values() {
        let msgstr = MsgStr::None;

        assert_eq!(msgstr.get(0), None);
    }

    #[test]
    fn msgstr_get_returns_singular_value_at_zero() {
        let msgstr = MsgStr::from("Hallo".to_owned());

        assert_eq!(msgstr.get(0), Some("Hallo"));
        assert_eq!(msgstr.get(1), None);
    }

    #[test]
    fn msgstr_get_returns_plural_values_by_index() {
        let msgstr = MsgStr::from(vec!["eins".to_owned(), "viele".to_owned()]);

        assert_eq!(msgstr.get(0), Some("eins"));
        assert_eq!(msgstr.get(1), Some("viele"));
        assert_eq!(msgstr.get(2), None);
    }

    #[test]
    fn msgstr_helpers_cover_empty_singular_and_plural_shapes() {
        let empty = MsgStr::from(Vec::<String>::new());
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert_eq!(empty.first(), None);
        assert_eq!(empty.first_str(), None);
        assert_eq!(empty.iter().count(), 0);
        assert_eq!(empty.into_vec(), Vec::<String>::new());

        let singular = MsgStr::from(vec!["Hallo".to_owned()]);
        assert!(!singular.is_empty());
        assert_eq!(singular.len(), 1);
        assert_eq!(singular.first().map(String::as_str), Some("Hallo"));
        assert_eq!(singular.first_str(), Some("Hallo"));
        assert_eq!((&singular).into_iter().collect::<Vec<_>>(), vec!["Hallo"]);
        assert_eq!(singular[0], "Hallo");
        assert_eq!(singular.into_vec(), vec!["Hallo"]);

        let plural = MsgStr::from(vec!["eins".to_owned(), "viele".to_owned()]);
        assert_eq!(plural.len(), 2);
        assert_eq!(plural.first_str(), Some("eins"));
        assert_eq!(plural.iter().collect::<Vec<_>>(), vec!["eins", "viele"]);
        assert_eq!(plural[1], "viele");
        assert_eq!(plural.into_vec(), vec!["eins", "viele"]);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn po_file_serde_round_trips_owned_document_shape() {
        let file = PoFile {
            comments: vec!["translator note".to_owned()],
            headers: vec![Header {
                key: "Language".to_owned(),
                value: "de".to_owned(),
            }],
            items: vec![PoItem {
                msgid: "Hello".to_owned(),
                msgstr: MsgStr::from("Hallo".to_owned()),
                references: vec!["src/app.rs:10".to_owned()],
                nplurals: 1,
                ..PoItem::default()
            }],
            ..PoFile::default()
        };

        let json = serde_json::to_value(&file).expect("PO file serialization must succeed");
        assert_eq!(json["items"][0]["msgstr"]["kind"], "singular");
        assert_eq!(json["items"][0]["msgstr"]["value"], "Hallo");

        let roundtrip: PoFile =
            serde_json::from_value(json).expect("PO file deserialization must succeed");
        assert_eq!(roundtrip, file);
    }
}
