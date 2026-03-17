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

mod api;
mod borrowed;
mod merge;
mod parse;
mod scan;
mod serialize;
mod text;

pub use api::{
    ApiError, CatalogMessage, CatalogMessageExtra, CatalogMessageKey, CatalogOrigin, CatalogStats,
    CatalogUpdateInput, CatalogUpdateResult, Diagnostic, DiagnosticSeverity, EffectiveTranslation,
    EffectiveTranslationRef, ExtractedMessage, ExtractedPluralMessage, ExtractedSingularMessage,
    NormalizedParsedCatalog, ObsoleteStrategy, OrderBy, ParseCatalogOptions, ParsedCatalog,
    PlaceholderCommentMode, PluralEncoding, PluralSource, SourceExtractedMessage, TranslationShape,
    UpdateCatalogFileOptions, UpdateCatalogOptions, parse_catalog, update_catalog,
    update_catalog_file,
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
    pub comments: Vec<String>,
    pub extracted_comments: Vec<String>,
    pub headers: Vec<Header>,
    pub items: Vec<PoItem>,
}

/// A single header entry from the PO header block.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Header {
    pub key: String,
    pub value: String,
}

/// A single gettext message entry.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PoItem {
    pub msgid: String,
    pub msgctxt: Option<String>,
    pub references: Vec<String>,
    pub msgid_plural: Option<String>,
    pub msgstr: MsgStr,
    pub comments: Vec<String>,
    pub extracted_comments: Vec<String>,
    pub flags: Vec<String>,
    pub metadata: Vec<(String, String)>,
    pub obsolete: bool,
    pub nplurals: usize,
}

impl PoItem {
    /// Creates an empty message entry with space for `nplurals` plural slots.
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
    #[default]
    None,
    Singular(String),
    Plural(Vec<String>),
}

impl MsgStr {
    /// Returns `true` when no translation values are present.
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns the number of translation values present.
    pub fn len(&self) -> usize {
        match self {
            Self::None => 0,
            Self::Singular(_) => 1,
            Self::Plural(values) => values.len(),
        }
    }

    /// Returns the first translation value, if present.
    pub fn first(&self) -> Option<&String> {
        match self {
            Self::None => None,
            Self::Singular(value) => Some(value),
            Self::Plural(values) => values.first(),
        }
    }

    /// Returns the first translation value as `&str`, if present.
    pub fn first_str(&self) -> Option<&str> {
        self.first().map(String::as_str)
    }

    /// Returns the translation at `index` without panicking.
    pub fn get(&self, index: usize) -> Option<&str> {
        match self {
            Self::None => None,
            Self::Singular(value) if index == 0 => Some(value.as_str()),
            Self::Singular(_) => None,
            Self::Plural(values) => values.get(index).map(String::as_str),
        }
    }

    /// Iterates over all translation values in order.
    pub fn iter(&self) -> MsgStrIter<'_> {
        match self {
            Self::None => MsgStrIter::empty(),
            Self::Singular(value) => MsgStrIter::single(value),
            Self::Plural(values) => MsgStrIter::many(values.iter()),
        }
    }

    /// Converts the translation payload into an owned vector.
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
    fn empty() -> Self {
        Self {
            inner: MsgStrIterInner::Empty,
        }
    }

    fn single(value: &'a String) -> Self {
        Self {
            inner: MsgStrIterInner::Single(Some(value)),
        }
    }

    fn many(iter: std::slice::Iter<'a, String>) -> Self {
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
    pub fold_length: usize,
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
