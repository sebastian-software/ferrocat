#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs, rustdoc::broken_intra_doc_links)]
//! Public Rust entry point for the `ferrocat` workspace.
//!
//! This crate re-exports the primary API surface from the lower-level
//! `ferrocat-po` and `ferrocat-icu` crates so application code can depend on a
//! single package.
//!
//! # Feature flags
//!
//! The default feature set is `full`, which enables the complete current API
//! surface through `catalog` and `serde`.
//!
//! - `catalog` exposes high-level catalog parsing, updates, combining, audits,
//!   machine-translation metadata, plural handling, FCL storage, and runtime
//!   artifact compilation.
//! - `serde` enables serde support in the re-exported `ferrocat-po` and
//!   `ferrocat-icu` types.
//! - `compile`, `mt`, and `plurals` are reserved subsystem aliases. Today they
//!   imply `catalog`; they do not reduce or split the catalog API surface.
//!
//! Use `default-features = false` for low-level PO and ICU parsing without the
//! catalog layer. Enabling `compile`, `mt`, or `plurals` currently has the same
//! dependency effect as enabling `catalog`.
//!
//! # Examples
//!
//! ```rust
//! use ferrocat::{icu, po};
//!
//! let po = po::parse_po("msgid \"Hello\"\nmsgstr \"Hallo\"\n")?;
//! let icu = icu::parse_icu("Hello {name}")?;
//!
//! assert_eq!(po.items[0].msgid, "Hello");
//! assert_eq!(icu.nodes.len(), 2);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ```rust
//! use ferrocat::catalog::{
//!     CompileSelectedCatalogArtifactOptions, CompiledCatalogIdIndex, CompiledKeyStrategy,
//!     ParseCatalogOptions, compile_catalog_artifact_selected, parse_catalog,
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
//! let index = CompiledCatalogIdIndex::new(&[&requested, &source], CompiledKeyStrategy::FerrocatV1)?;
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
//! use ferrocat::catalog::{
//!     CatalogAuditOptions, ParseCatalogOptions, audit_catalogs, parse_catalog,
//! };
//!
//! let source = parse_catalog(ParseCatalogOptions {
//!     locale: Some("en"),
//!     ..ParseCatalogOptions::new("msgid \"Checkout\"\nmsgstr \"Checkout\"\n", "en")
//! })?
//! .into_normalized_view()?;
//! let target = parse_catalog(ParseCatalogOptions {
//!     locale: Some("de"),
//!     ..ParseCatalogOptions::new("", "en")
//! })?
//! .into_normalized_view()?;
//! let report = audit_catalogs(&[&source, &target], &CatalogAuditOptions::new("en"))?;
//!
//! assert!(report.has_errors());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

/// High-level catalog maintenance, audit, and runtime artifact APIs.
#[cfg(feature = "catalog")]
#[cfg_attr(docsrs, doc(cfg(feature = "catalog")))]
pub mod catalog {
    pub use ferrocat_po::diagnostic_codes;
    pub use ferrocat_po::{
        AiProvenance, ApiError, COMPILED_CATALOG_ARTIFACT_SCHEMA_VERSION, CatalogAuditChecks,
        CatalogAuditDiagnostic, CatalogAuditIcuOptions, CatalogAuditMessageRef,
        CatalogAuditOptions, CatalogAuditReport, CatalogAuditSummary, CatalogCombineInput,
        CatalogCombineResult, CatalogCombineSelection, CatalogCombineStats,
        CatalogConflictStrategy, CatalogCoverageMessage, CatalogCoverageOptions,
        CatalogCoverageReport, CatalogFileCombineResult, CatalogFileFormat, CatalogLocaleCoverage,
        CatalogLocaleReview, CatalogMachineTranslationMessage, CatalogMachineTranslationReview,
        CatalogMachineTranslationStatus, CatalogMessage, CatalogMessageKey, CatalogMessageStatus,
        CatalogMode, CatalogOrigin, CatalogReviewOptions, CatalogReviewReport,
        CatalogReviewSummary, CatalogReviewTranslation, CatalogSemantics, CatalogSourceChange,
        CatalogSourceChangeKind, CatalogSourceChangeReport, CatalogStats, CatalogStorageFormat,
        CatalogTranslationChange, CatalogTranslationChangeReport, CatalogUpdateInput,
        CatalogUpdateResult, CombineCatalogFilesOptions, CombineCatalogOptions,
        CompileCatalogArtifactIcuOptions, CompileCatalogArtifactOptions,
        CompileCatalogArtifactReportOptions, CompileCatalogArtifactReportSelection,
        CompileCatalogOptions, CompileSelectedCatalogArtifactOptions, CompiledCatalog,
        CompiledCatalogArtifact, CompiledCatalogArtifactReport, CompiledCatalogDiagnostic,
        CompiledCatalogIdDescription, CompiledCatalogIdIndex, CompiledCatalogMissingMessage,
        CompiledCatalogProvenanceReport, CompiledCatalogResolution, CompiledCatalogResolutionKind,
        CompiledCatalogTranslationKind, CompiledCatalogUnavailableId, CompiledKeyStrategy,
        CompiledMessage, CompiledTranslation, DescribeCompiledIdsReport, Diagnostic,
        DiagnosticSeverity, EffectiveTranslation, EffectiveTranslationRef, ExtractedMessage,
        ExtractedPluralMessage, ExtractedSingularMessage, IcuFormatterSupportPolicy,
        IcuPseudolocalizationOptions, IcuSyntaxPolicy, MachineMetadata, NormalizedParsedCatalog,
        ObsoleteInfo, ObsoleteStrategy, OrderBy, ParseCatalogOptions, ParsedCatalog,
        PlaceholderCommentMode, PluralEncoding, PluralSource, PoVec, RenderOptions, SmallVec,
        SourceExtractedMessage, TranslationShape, UpdateCatalogFileOptions, UpdateCatalogOptions,
        audit_catalogs, audit_catalogs_with_icu_options, catalog_coverage, catalog_review,
        combine_catalog_files, combine_catalogs, compile_catalog_artifact,
        compile_catalog_artifact_report, compile_catalog_artifact_selected,
        compile_catalog_artifact_selected_with_icu_options,
        compile_catalog_artifact_with_icu_options, compiled_key, machine_translation_hash,
        parse_catalog, pseudolocalize_compiled_catalog_artifact,
        pseudolocalize_compiled_catalog_artifact_with_syntax_policy, update_catalog,
        update_catalog_file,
    };
}

/// ICU MessageFormat parsing, analysis, compatibility, and metadata APIs.
pub mod icu {
    pub use ferrocat_icu::diagnostic_codes;
    pub use ferrocat_icu::has_selectordinal as has_select_ordinal;
    pub use ferrocat_icu::{
        IcuAnalysis, IcuArgument, IcuArgumentKind, IcuCompatibilityOptions, IcuCompatibilityReport,
        IcuDiagnostic, IcuDiagnosticSeverity, IcuErrorKind, IcuFormatter, IcuFormatterSupport,
        IcuMessage, IcuNode, IcuOption, IcuParseError, IcuParserOptions, IcuPluralKind,
        IcuPluralSummary, IcuPosition, IcuPseudolocalizationOptions, IcuSelectSummary,
        IcuStyleKind, IcuTagSummary, MessageArgumentFormatMetadata, MessageArgumentKind,
        MessageArgumentMetadata, MessageArgumentMetadataInput, MessageFormatStyleKind,
        MessageMetadata, MessageMetadataDiagnostic, MessageMetadataInput,
        MessageMetadataValidationReport, MessageOriginMetadata, MessageSelectorKind,
        MessageSelectorMetadata, analyze_icu, compare_icu_messages,
        derive_message_metadata_from_icu, extract_argument_names, extract_tag_names,
        extract_variables, has_plural, has_select, has_tag, normalize_message_metadata, parse_icu,
        parse_icu_with_options, pseudolocalize_icu, pseudolocalize_icu_message, stringify_icu,
        validate_icu, validate_icu_formatter_support, validate_icu_formatter_support_from_analysis,
        validate_message_metadata,
    };
}

/// Low-level PO parsing, serialization, and text merge APIs.
pub mod po {
    pub use ferrocat_po::MergeExtractedMessage as MergeMessageInput;
    pub use ferrocat_po::diagnostic_codes;
    pub use ferrocat_po::{
        BorrowedHeader, BorrowedMsgStr, BorrowedPoFile, BorrowedPoItem, Header, MsgStr, MsgStrIter,
        ParseError, ParsePosition, PoFile, PoItem, PoVec, SerializeOptions, SmallVec,
        escape_string, extract_quoted, extract_quoted_cow, merge_catalog, parse_po,
        parse_po_borrowed, parse_po_bytes, stringify_po, unescape_string,
    };
}

#[deprecated(
    since = "2.0.0",
    note = "use ferrocat::has_select_ordinal or ferrocat::icu::has_select_ordinal"
)]
pub use ferrocat_icu::has_selectordinal;
pub use ferrocat_icu::{
    IcuAnalysis, IcuArgument, IcuArgumentKind, IcuCompatibilityOptions, IcuCompatibilityReport,
    IcuDiagnostic, IcuDiagnosticSeverity, IcuErrorKind, IcuFormatter, IcuFormatterSupport,
    IcuMessage, IcuNode, IcuOption, IcuParseError, IcuParserOptions, IcuPluralKind,
    IcuPluralSummary, IcuPosition, IcuPseudolocalizationOptions, IcuSelectSummary, IcuStyleKind,
    IcuTagSummary, MessageArgumentFormatMetadata, MessageArgumentKind, MessageArgumentMetadata,
    MessageArgumentMetadataInput, MessageFormatStyleKind, MessageMetadata,
    MessageMetadataDiagnostic, MessageMetadataInput, MessageMetadataValidationReport,
    MessageOriginMetadata, MessageSelectorKind, MessageSelectorMetadata, analyze_icu,
    compare_icu_messages, derive_message_metadata_from_icu, extract_argument_names,
    extract_tag_names, extract_variables, has_plural, has_select, has_tag,
    normalize_message_metadata, parse_icu, parse_icu_with_options, pseudolocalize_icu,
    pseudolocalize_icu_message, stringify_icu, validate_icu, validate_icu_formatter_support,
    validate_icu_formatter_support_from_analysis, validate_message_metadata,
};

#[cfg(feature = "catalog")]
#[cfg_attr(docsrs, doc(cfg(feature = "catalog")))]
/// JSON schema version emitted by [`CompiledCatalogArtifact`] serialization.
pub const COMPILED_CATALOG_ARTIFACT_SCHEMA_VERSION: u16 =
    ferrocat_po::COMPILED_CATALOG_ARTIFACT_SCHEMA_VERSION;

#[deprecated(
    since = "2.0.0",
    note = "use ferrocat::po::MergeMessageInput for the PO merge helper input"
)]
pub use ferrocat_po::MergeExtractedMessage;
#[cfg(feature = "catalog")]
#[cfg_attr(docsrs, doc(cfg(feature = "catalog")))]
pub use ferrocat_po::{
    AiProvenance, ApiError, CatalogAuditChecks, CatalogAuditDiagnostic, CatalogAuditIcuOptions,
    CatalogAuditMessageRef, CatalogAuditOptions, CatalogAuditReport, CatalogAuditSummary,
    CatalogCombineInput, CatalogCombineResult, CatalogCombineSelection, CatalogCombineStats,
    CatalogConflictStrategy, CatalogCoverageMessage, CatalogCoverageOptions, CatalogCoverageReport,
    CatalogFileCombineResult, CatalogFileFormat, CatalogLocaleCoverage, CatalogLocaleReview,
    CatalogMachineTranslationMessage, CatalogMachineTranslationReview,
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
    CompiledCatalogResolution, CompiledCatalogResolutionKind, CompiledCatalogTranslationKind,
    CompiledCatalogUnavailableId, CompiledKeyStrategy, CompiledMessage, CompiledTranslation,
    DescribeCompiledIdsReport, Diagnostic, DiagnosticSeverity, EffectiveTranslation,
    EffectiveTranslationRef, ExtractedMessage, ExtractedPluralMessage, ExtractedSingularMessage,
    IcuFormatterSupportPolicy, IcuSyntaxPolicy, MachineMetadata, NormalizedParsedCatalog,
    ObsoleteInfo, ObsoleteStrategy, OrderBy, ParseCatalogOptions, ParsedCatalog,
    PlaceholderCommentMode, PluralEncoding, PluralSource, RenderOptions, SourceExtractedMessage,
    TranslationShape, UpdateCatalogFileOptions, UpdateCatalogOptions, audit_catalogs,
    audit_catalogs_with_icu_options, catalog_coverage, catalog_review, combine_catalog_files,
    combine_catalogs, compile_catalog_artifact, compile_catalog_artifact_report,
    compile_catalog_artifact_selected, compile_catalog_artifact_selected_with_icu_options,
    compile_catalog_artifact_with_icu_options, compiled_key, machine_translation_hash,
    parse_catalog, pseudolocalize_compiled_catalog_artifact,
    pseudolocalize_compiled_catalog_artifact_with_syntax_policy, update_catalog,
    update_catalog_file,
};
pub use ferrocat_po::{
    BorrowedHeader, BorrowedMsgStr, BorrowedPoFile, BorrowedPoItem, Header, MsgStr, MsgStrIter,
    ParseError, ParsePosition, PoFile, PoItem, PoVec, SerializeOptions, SmallVec, escape_string,
    extract_quoted, extract_quoted_cow, merge_catalog, parse_po, parse_po_borrowed, parse_po_bytes,
    stringify_po, unescape_string,
};
pub use icu::has_select_ordinal;
