#![warn(missing_docs, rustdoc::broken_intra_doc_links)]
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
//!
//! ```rust
//! use ferrocat::{
//!     CompileSelectedCatalogArtifactOptions, CompiledCatalogIdIndex, CompiledKeyStrategy,
//!     ParseCatalogOptions, compile_catalog_artifact_selected, parse_catalog,
//! };
//!
//! let source = parse_catalog(ParseCatalogOptions {
//!     content: "msgid \"Hello\"\nmsgstr \"Hello\"\n",
//!     source_locale: "en",
//!     locale: Some("en"),
//!     ..ParseCatalogOptions::default()
//! })?
//! .into_normalized_view()?;
//! let requested = parse_catalog(ParseCatalogOptions {
//!     content: "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
//!     source_locale: "en",
//!     locale: Some("de"),
//!     ..ParseCatalogOptions::default()
//! })?
//! .into_normalized_view()?;
//! let index = CompiledCatalogIdIndex::new(&[&requested, &source], CompiledKeyStrategy::FerrocatV1)?;
//! let compiled_ids = index.iter().map(|(id, _)| id.to_owned()).collect::<Vec<_>>();
//! let compiled = compile_catalog_artifact_selected(
//!     &[&requested, &source],
//!     &index,
//!     &CompileSelectedCatalogArtifactOptions {
//!         requested_locale: "de",
//!         source_locale: "en",
//!         compiled_ids: &compiled_ids,
//!         ..CompileSelectedCatalogArtifactOptions::default()
//!     },
//! )?;
//!
//! assert_eq!(compiled.messages.len(), 1);
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
    CatalogMessageExtra, CatalogMessageKey, CatalogOrigin, CatalogStats, CatalogStorageFormat,
    CatalogUpdateInput, CatalogUpdateResult, CompileCatalogArtifactOptions, CompileCatalogOptions,
    CompileSelectedCatalogArtifactOptions, CompiledCatalog, CompiledCatalogArtifact,
    CompiledCatalogDiagnostic, CompiledCatalogIdDescription, CompiledCatalogIdIndex,
    CompiledCatalogMissingMessage, CompiledCatalogTranslationKind, CompiledCatalogUnavailableId,
    CompiledKeyStrategy, CompiledMessage, CompiledTranslation, DescribeCompiledIdsReport,
    Diagnostic, DiagnosticSeverity, EffectiveTranslation, EffectiveTranslationRef,
    ExtractedMessage, ExtractedPluralMessage, ExtractedSingularMessage, Header,
    MergeExtractedMessage, MsgStr, MsgStrIter, NormalizedParsedCatalog, ObsoleteStrategy, OrderBy,
    ParseCatalogOptions, ParseError, ParsedCatalog, PlaceholderCommentMode, PluralEncoding,
    PluralSource, PoFile, PoItem, SerializeOptions, SourceExtractedMessage, TranslationShape,
    UpdateCatalogFileOptions, UpdateCatalogOptions, compile_catalog_artifact,
    compile_catalog_artifact_selected, compiled_key, escape_string, extract_quoted,
    extract_quoted_cow, merge_catalog, parse_catalog, parse_po, parse_po_borrowed, stringify_po,
    unescape_string, update_catalog, update_catalog_file,
};
