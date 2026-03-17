//! Public Rust entry point for the `ferrocat` workspace.
//!
//! This crate re-exports the primary API surface from the lower-level
//! `ferrocat-po` and `ferrocat-icu` crates so application code can depend on a
//! single package.
//!
//! # Examples
//!
//! ```rust
//! use ferrocat::{parse_icu, parse_po};
//!
//! let po = parse_po("msgid \"Hello\"\nmsgstr \"Hallo\"\n")?;
//! let icu = parse_icu("Hello {name}")?;
//!
//! assert_eq!(po.items[0].msgid, "Hello");
//! assert_eq!(icu.nodes.len(), 2);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub use ferrocat_icu::has_selectordinal as has_select_ordinal;
pub use ferrocat_icu::{
    IcuErrorKind, IcuMessage, IcuNode, IcuOption, IcuParseError, IcuParserOptions, IcuPluralKind,
    IcuPosition, extract_variables, has_plural, has_select, has_selectordinal, has_tag, parse_icu,
    parse_icu_with_options, validate_icu,
};
pub use ferrocat_po::{
    ApiError, BorrowedHeader, BorrowedMsgStr, BorrowedPoFile, BorrowedPoItem, CatalogMessage,
    CatalogMessageExtra, CatalogMessageKey, CatalogOrigin, CatalogStats, CatalogUpdateInput,
    CatalogUpdateResult, Diagnostic, DiagnosticSeverity, EffectiveTranslation,
    EffectiveTranslationRef, ExtractedMessage, ExtractedPluralMessage, ExtractedSingularMessage,
    Header, MergeExtractedMessage, MsgStr, MsgStrIter, NormalizedParsedCatalog, ObsoleteStrategy,
    OrderBy, ParseCatalogOptions, ParseError, ParsedCatalog, PlaceholderCommentMode,
    PluralEncoding, PluralSource, PoFile, PoItem, SerializeOptions, SourceExtractedMessage,
    TranslationShape, UpdateCatalogFileOptions, UpdateCatalogOptions, escape_string,
    extract_quoted, extract_quoted_cow, merge_catalog, parse_catalog, parse_po, parse_po_borrowed,
    stringify_po, unescape_string, update_catalog, update_catalog_file,
};
