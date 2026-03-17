#![cfg_attr(docsrs, warn(missing_docs, rustdoc::broken_intra_doc_links))]
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
//! use ferrocat_po::{CompileCatalogOptions, ParseCatalogOptions, parse_catalog};
//!
//! let parsed = parse_catalog(ParseCatalogOptions {
//!     content: "msgid \"Hello\"\nmsgstr \"Hallo\"\n".to_owned(),
//!     source_locale: "en".to_owned(),
//!     locale: Some("de".to_owned()),
//!     ..ParseCatalogOptions::default()
//! })?;
//! let normalized = parsed.into_normalized_view()?;
//! let compiled = normalized.compile(&CompileCatalogOptions::default())?;
//!
//! assert_eq!(compiled.len(), 1);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod api;
mod borrowed;
mod merge;
mod parse;
mod scan;
mod serialize;
mod text;

pub use api::{
    ApiError, CatalogMessage, CatalogMessageExtra, CatalogMessageKey, CatalogOrigin, CatalogStats,
    CatalogUpdateInput, CatalogUpdateResult, CompileCatalogOptions, CompiledCatalog,
    CompiledKeyStrategy, CompiledMessage, CompiledTranslation, Diagnostic, DiagnosticSeverity,
    EffectiveTranslation, EffectiveTranslationRef, ExtractedMessage, ExtractedPluralMessage,
    ExtractedSingularMessage, NormalizedParsedCatalog, ObsoleteStrategy, OrderBy,
    ParseCatalogOptions, ParsedCatalog, PlaceholderCommentMode, PluralEncoding, PluralSource,
    SourceExtractedMessage, TranslationShape, UpdateCatalogFileOptions, UpdateCatalogOptions,
    parse_catalog, update_catalog, update_catalog_file,
};
pub use borrowed::{
    BorrowedHeader, BorrowedMsgStr, BorrowedPoFile, BorrowedPoItem, parse_po_borrowed,
};
pub use merge::{ExtractedMessage as MergeExtractedMessage, merge_catalog};
pub use parse::parse_po;
pub use serialize::stringify_po;
pub use text::{escape_string, extract_quoted, extract_quoted_cow, unescape_string};

use core::{fmt, ops::Index};

/// An owned PO document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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
pub struct Header {
    /// Header name such as `Language` or `Plural-Forms`.
    pub key: String,
    /// Header value without the trailing newline.
    pub value: String,
}

/// A single gettext message entry.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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

/// Error returned when parsing or unescaping PO content fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    message: String,
}

impl ParseError {
    /// Creates a new parse error with the provided message.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
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
    use super::MsgStr;

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
}
