#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs, rustdoc::broken_intra_doc_links)]
//! Performance-first PO parsing and serialization.
//!
//! The crate exposes both owned and borrowed parsers for gettext PO files,
//! a byte-oriented UTF-8 parser entry point, plus helpers for serialization
//! and higher-level catalog update workflows.
//!
//! # Feature flags
//!
//! The default feature set is `full`, which currently enables the `catalog`
//! workflow layer.
//!
//! - `catalog` exposes high-level catalog parsing, updates, combining,
//!   conversion, audits, machine-translation metadata, plural handling, FCL
//!   storage, and runtime artifact compilation. It also enables the
//!   catalog-layer dependencies used for hashing, atomic file updates, serde
//!   JSON output, ICU diagnostics, and CLDR plural data.
//! - `serde` enables serde implementations for low-level PO document types and
//!   is also enabled by `catalog` for catalog-layer JSON/report shapes.
//! - `compile`, `mt`, and `plurals` are reserved subsystem aliases. Today they
//!   imply `catalog`; they do not reduce or split the catalog API surface.
//!
//! Use `default-features = false` for the low-level PO parser, borrowed parser,
//! serializer, string helpers, and lightweight `merge_catalog` helper without
//! catalog-layer dependencies. Enabling `compile`, `mt`, or `plurals` currently
//! has the same dependency effect as enabling `catalog`.
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
//! use ferrocat_po::parse_po_bytes;
//!
//! let input = b"msgid \"Hello\"\nmsgstr \"Hallo\"\n";
//! let file = parse_po_bytes(input)?;
//! assert_eq!(file.items[0].msgstr[0], "Hallo");
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
//! let source = parse_catalog(
//!     ParseCatalogOptions::new("msgid \"Hello\"\nmsgstr \"Hello\"\n", "en").with_locale("en"),
//! )?
//! .into_normalized_view()?;
//! let requested = parse_catalog(
//!     ParseCatalogOptions::new("msgid \"Hello\"\nmsgstr \"Hallo\"\n", "en").with_locale("de"),
//! )?
//! .into_normalized_view()?;
//! let index = CompiledCatalogIdIndex::new(&[&requested, &source], ferrocat_po::CompiledKeyStrategy::FerrocatV1)?;
//! let compiled_ids = index.iter().map(|(id, _)| id).collect::<Vec<_>>();
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
//! use ferrocat_po::{
//!     CatalogAuditOptions, ParseCatalogOptions, audit_catalogs, parse_catalog_for_review,
//! };
//!
//! let source = parse_catalog_for_review(
//!     ParseCatalogOptions::new("msgid \"Hello {name}\"\nmsgstr \"Hello {name}\"\n", "en")
//!         .with_locale("en"),
//! )?;
//! let target = parse_catalog_for_review(
//!     ParseCatalogOptions::new("msgid \"Hello {name}\"\nmsgstr \"Hallo\"\n", "en")
//!         .with_locale("de"),
//! )?;
//! let report = audit_catalogs(&[&source, &target], &CatalogAuditOptions::new("en"))?;
//!
//! assert!(report.has_errors());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#[cfg(feature = "catalog")]
#[cfg_attr(docsrs, doc(cfg(feature = "catalog")))]
mod api;
mod borrowed;
pub mod diagnostic_codes;
mod line_state;
mod merge;
mod parse;
mod scan;
mod serialize;
mod text;
mod utf8;

#[cfg(feature = "catalog")]
#[cfg_attr(docsrs, doc(cfg(feature = "catalog")))]
pub use api::{
    AiProvenance, ApiError, COMPILED_CATALOG_ARTIFACT_SCHEMA_VERSION, CatalogAuditChecks,
    CatalogAuditDiagnostic, CatalogAuditIcuOptions, CatalogAuditMessageRef, CatalogAuditOptions,
    CatalogAuditReport, CatalogAuditSummary, CatalogCombineInput, CatalogCombineResult,
    CatalogCombineSelection, CatalogCombineStats, CatalogConflictStrategy, CatalogConvertResult,
    CatalogCoverageMessage, CatalogCoverageOptions, CatalogCoverageReport,
    CatalogFileCombineResult, CatalogFileConvertResult, CatalogFileFormat, CatalogLocaleCoverage,
    CatalogLocaleReview, CatalogMachineTranslationMessage, CatalogMachineTranslationReview,
    CatalogMachineTranslationStatus, CatalogMessage, CatalogMessageKey, CatalogMessageStatus,
    CatalogMode, CatalogOrigin, CatalogReviewOptions, CatalogReviewReport, CatalogReviewSummary,
    CatalogReviewTranslation, CatalogSemantics, CatalogSourceChange, CatalogSourceChangeKind,
    CatalogSourceChangeReport, CatalogStats, CatalogStorageFormat, CatalogTranslationChange,
    CatalogTranslationChangeReport, CatalogUpdateInput, CatalogUpdateResult,
    CombineCatalogFilesOptions, CombineCatalogOptions, CompileCatalogArtifactIcuOptions,
    CompileCatalogArtifactOptions, CompileCatalogArtifactReportOptions,
    CompileCatalogArtifactReportSelection, CompileCatalogOptions,
    CompileSelectedCatalogArtifactOptions, CompiledCatalog, CompiledCatalogArtifact,
    CompiledCatalogArtifactReport, CompiledCatalogDiagnostic, CompiledCatalogIdDescription,
    CompiledCatalogIdIndex, CompiledCatalogMissingMessage, CompiledCatalogProvenanceReport,
    CompiledCatalogPseudolocalizationOptions, CompiledCatalogResolution,
    CompiledCatalogResolutionKind, CompiledCatalogTranslationKind, CompiledCatalogUnavailableId,
    CompiledKeyStrategy, CompiledMessage, CompiledTranslation, ConvertCatalogFileOptions,
    ConvertCatalogOptions, DescribeCompiledIdsReport, Diagnostic, DiagnosticSeverity,
    EffectiveTranslation, EffectiveTranslationRef, ExtractedMessage, ExtractedPluralMessage,
    ExtractedSingularMessage, IcuFormatterSupportPolicy, IcuPseudolocalizationOptions,
    IcuSyntaxPolicy, MachineMetadata, NormalizedParsedCatalog, ObsoleteInfo, ObsoleteStrategy,
    OrderBy, ParseCatalogOptions, ParsedCatalog, PlaceholderCommentMode, PluralEncoding,
    PluralSource, RenderOptions, SourceExtractedMessage, TranslationShape,
    UpdateCatalogFileOptions, UpdateCatalogOptions, WriteDurability, audit_catalogs,
    canonicalize_icu_with_policy, combine_catalog_files, combine_catalogs,
    compile_catalog_artifact, compile_catalog_artifact_report, compile_catalog_artifact_selected,
    compiled_key, compiled_key_with_policy, convert_catalog, convert_catalog_file,
    machine_translation_hash, measure_catalog_coverage, parse_catalog, parse_catalog_for_review,
    pseudolocalize_compiled_catalog_artifact, review_catalogs, update_catalog, update_catalog_file,
};
pub use borrowed::{
    BorrowedHeader, BorrowedMsgStr, BorrowedPoFile, BorrowedPoItem, parse_po_borrowed,
};
pub use diagnostic_codes::DiagnosticCode;
pub use merge::{MergeMessageInput, merge_catalog};
pub use parse::{parse_po, parse_po_bytes};
pub use serialize::stringify_po;
pub use text::{escape_string, extract_quoted, extract_quoted_cow, unescape_string};

use core::{
    fmt,
    iter::FusedIterator,
    ops::{Deref, DerefMut, Index},
    slice,
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use smallvec::{IntoIter as SmallVecIntoIter, SmallVec};

/// Inline-capable vector for the small per-item collections (references, flags,
/// comments, metadata) that hold a single element in the overwhelmingly common
/// case, avoiding a heap allocation for the backing buffer.
///
/// The inline capacity and backing collection are private implementation
/// details. Use this type by value in PO/catalog structures and read it through
/// its slice view.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct PoVec<T>(SmallVec<[T; 1]>);

impl<T> PoVec<T> {
    /// Creates an empty vector.
    #[must_use]
    pub fn new() -> Self {
        Self(SmallVec::new())
    }

    /// Creates an empty vector with room for at least `capacity` elements.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(SmallVec::with_capacity(capacity))
    }

    /// Returns the number of stored elements.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` when the vector contains no elements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the values as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        self.0.as_slice()
    }

    /// Returns the values as a mutable slice.
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.0.as_mut_slice()
    }

    /// Returns an iterator over the values.
    pub fn iter(&self) -> slice::Iter<'_, T> {
        self.as_slice().iter()
    }

    /// Returns a mutable iterator over the values.
    pub fn iter_mut(&mut self) -> slice::IterMut<'_, T> {
        self.as_mut_slice().iter_mut()
    }

    /// Appends `value` to the end of the vector.
    pub fn push(&mut self, value: T) {
        self.0.push(value);
    }

    /// Removes all values.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Converts the collection into a standard [`Vec`].
    #[must_use]
    pub fn into_vec(self) -> Vec<T> {
        self.0.into_vec()
    }
}

impl<T> AsRef<[T]> for PoVec<T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> AsMut<[T]> for PoVec<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T> Deref for PoVec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T> DerefMut for PoVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T> Extend<T> for PoVec<T> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.0.extend(iter);
    }
}

impl<T> From<Vec<T>> for PoVec<T> {
    fn from(value: Vec<T>) -> Self {
        Self(SmallVec::from_vec(value))
    }
}

impl<T, const N: usize> From<[T; N]> for PoVec<T> {
    fn from(value: [T; N]) -> Self {
        Self(value.into_iter().collect())
    }
}

impl<T> FromIterator<T> for PoVec<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self(iter.into_iter().collect())
    }
}

impl<T> From<PoVec<T>> for Vec<T> {
    fn from(value: PoVec<T>) -> Self {
        value.into_vec()
    }
}

impl<T> IntoIterator for PoVec<T> {
    type Item = T;
    type IntoIter = PoVecIntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        PoVecIntoIter {
            inner: self.0.into_iter(),
        }
    }
}

impl<'a, T> IntoIterator for &'a PoVec<T> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut PoVec<T> {
    type Item = &'a mut T;
    type IntoIter = slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T, U> PartialEq<[U]> for PoVec<T>
where
    T: PartialEq<U>,
{
    fn eq(&self, other: &[U]) -> bool {
        self.as_slice() == other
    }
}

impl<T, U> PartialEq<&[U]> for PoVec<T>
where
    T: PartialEq<U>,
{
    fn eq(&self, other: &&[U]) -> bool {
        self.as_slice() == *other
    }
}

impl<T, U> PartialEq<Vec<U>> for PoVec<T>
where
    T: PartialEq<U>,
{
    fn eq(&self, other: &Vec<U>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

/// Owning iterator returned by [`PoVec::into_iter`].
pub struct PoVecIntoIter<T> {
    inner: SmallVecIntoIter<[T; 1]>,
}

impl<T> Iterator for PoVecIntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> DoubleEndedIterator for PoVecIntoIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

impl<T> ExactSizeIterator for PoVecIntoIter<T> {}

impl<T> FusedIterator for PoVecIntoIter<T> {}

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
    pub references: PoVec<String>,
    /// Optional plural source identifier.
    pub msgid_plural: Option<String>,
    /// Translation payload for the message.
    pub msgstr: MsgStr,
    /// Translator comments attached to the item.
    pub comments: PoVec<String>,
    /// Extracted comments attached to the item.
    pub extracted_comments: PoVec<String>,
    /// Raw gettext flags such as `fuzzy`.
    ///
    /// The low-level PO parser and serializer preserve this field for faithful
    /// PO round trips. Since Ferrocat 2.0, the high-level catalog layer drops
    /// gettext flags, including `fuzzy`, when parsing or writing catalog data;
    /// fuzzy/discard decisions are modeled by catalog-layer behavior instead
    /// of being carried through this raw PO field.
    pub flags: PoVec<String>,
    /// Raw metadata lines that do not fit the dedicated fields.
    pub metadata: PoVec<(String, String)>,
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
    /// Creates a plural translation payload without normalizing by slot count.
    ///
    /// Use this when the plural shape matters even if the value vector has one
    /// slot, such as gettext catalogs for one-form locales. `From<Vec<String>>`
    /// normalizes empty vectors to [`MsgStr::None`] and single-value vectors to
    /// [`MsgStr::Singular`].
    #[must_use]
    pub fn plural(values: Vec<String>) -> Self {
        Self::Plural(values)
    }

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
    pub fn first(&self) -> Option<&str> {
        match self {
            Self::None => None,
            Self::Singular(value) => Some(value.as_str()),
            Self::Plural(values) => values.first().map(String::as_str),
        }
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
            Self::Singular(value) => MsgStrIter::single(value.as_str()),
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
    type Item = &'a str;
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
    Single(Option<&'a str>),
    Many(std::slice::Iter<'a, String>),
}

impl<'a> MsgStrIter<'a> {
    const fn empty() -> Self {
        Self {
            inner: MsgStrIterInner::Empty,
        }
    }

    const fn single(value: &'a str) -> Self {
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
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            MsgStrIterInner::Empty => None,
            MsgStrIterInner::Single(value) => value.take(),
            MsgStrIterInner::Many(iter) => iter.next().map(String::as_str),
        }
    }
}

/// Options controlling PO serialization.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
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

impl SerializeOptions {
    /// Returns options that wrap string literals at the given soft limit.
    #[must_use]
    pub fn with_fold_length(mut self, fold_length: usize) -> Self {
        self.fold_length = fold_length;
        self
    }

    /// Returns options that keep one-line values compact when possible.
    #[must_use]
    pub fn with_compact_multiline(mut self, compact_multiline: bool) -> Self {
        self.compact_multiline = compact_multiline;
        self
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
    use super::{MsgStr, ParseError, ParsePosition, PoVec, SerializeOptions};

    #[cfg(feature = "serde")]
    use super::{Header, PoFile, PoItem};

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
    fn msgstr_plural_constructor_preserves_single_slot_shape() {
        let values = vec!["translation".to_owned()];
        let plural = MsgStr::plural(values.clone());

        assert_eq!(plural, MsgStr::Plural(values.clone()));
        assert_ne!(plural, MsgStr::from(values.clone()));
        assert_eq!(plural.len(), 1);
        assert_eq!(plural.first(), Some("translation"));
        assert_eq!(plural.into_vec(), values);
    }

    #[test]
    fn msgstr_helpers_cover_empty_singular_and_plural_shapes() {
        let empty = MsgStr::from(Vec::<String>::new());
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert_eq!(empty.first(), None);
        assert_eq!(empty.iter().count(), 0);
        assert_eq!(empty.into_vec(), Vec::<String>::new());

        let singular = MsgStr::from(vec!["Hallo".to_owned()]);
        assert!(!singular.is_empty());
        assert_eq!(singular.len(), 1);
        assert_eq!(singular.first(), Some("Hallo"));
        assert_eq!((&singular).into_iter().collect::<Vec<_>>(), vec!["Hallo"]);
        assert_eq!(singular[0], "Hallo");
        assert_eq!(singular.into_vec(), vec!["Hallo"]);

        let plural = MsgStr::from(vec!["eins".to_owned(), "viele".to_owned()]);
        assert_eq!(plural.len(), 2);
        assert_eq!(plural.first(), Some("eins"));
        assert_eq!(plural.iter().collect::<Vec<_>>(), vec!["eins", "viele"]);
        assert_eq!(plural[1], "viele");
        assert_eq!(plural.into_vec(), vec!["eins", "viele"]);
    }

    #[test]
    fn povec_keeps_slice_iteration_and_vec_conversion_ergonomics() {
        let mut values = PoVec::new();
        assert!(values.is_empty());

        values.push("src/app.rs:10".to_owned());
        values.extend(["src/app.rs:20".to_owned()]);

        assert_eq!(values.len(), 2);
        assert_eq!(
            values.as_slice(),
            ["src/app.rs:10".to_owned(), "src/app.rs:20".to_owned()]
        );
        assert_eq!(
            values.iter().map(String::as_str).collect::<Vec<_>>(),
            ["src/app.rs:10", "src/app.rs:20"]
        );

        let from_vec = PoVec::from(vec!["fuzzy".to_owned()]);
        assert_eq!(from_vec, vec!["fuzzy".to_owned()]);
        assert_eq!(Vec::<String>::from(from_vec), vec!["fuzzy".to_owned()]);
    }

    #[test]
    fn povec_trait_views_cover_mutable_slice_and_reference_iteration() {
        let mut values = PoVec::with_capacity(2);
        values.extend([1, 2]);

        assert_eq!(AsRef::<[i32]>::as_ref(&values), [1, 2]);

        AsMut::<[i32]>::as_mut(&mut values)[0] = 3;
        values.as_mut_slice()[1] = 4;
        for value in &mut values {
            *value += 1;
        }
        values.iter_mut().for_each(|value| *value *= 2);

        let as_slice: &[i32] = &values;
        assert_eq!(as_slice, [8, 10]);

        let as_mut_slice: &mut [i32] = &mut values;
        as_mut_slice[0] += 1;

        let expected = [9, 10];
        assert!(values == expected[..]);
        assert!(values == expected.as_slice());
    }

    #[test]
    fn povec_owned_iterator_preserves_values_without_exposing_backing_type() {
        let values = PoVec::from(["one".to_owned(), "other".to_owned()]);

        assert_eq!(
            values.into_iter().collect::<Vec<_>>(),
            vec!["one".to_owned(), "other".to_owned()]
        );
    }

    #[test]
    fn povec_owned_iterator_supports_double_ended_size_hints() {
        let mut iter = PoVec::from([1, 2, 3]).into_iter();

        assert_eq!(iter.size_hint(), (3, Some(3)));
        assert_eq!(iter.next_back(), Some(3));
        assert_eq!(iter.size_hint(), (2, Some(2)));
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next_back(), Some(2));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
    }

    #[test]
    fn serialize_option_builders_set_fields() {
        let options = SerializeOptions::default()
            .with_fold_length(120)
            .with_compact_multiline(false);

        assert_eq!(options.fold_length, 120);
        assert!(!options.compact_multiline);
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
                references: vec!["src/app.rs:10".to_owned()].into(),
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
