mod audit;
mod catalog;
mod catalog_index;
mod combine;
mod compile;
mod compile_types;
mod coverage;
mod export;
mod fcl;
mod file_io;
mod helpers;
mod icu_syntax;
mod message_status;
mod mt;
mod plural;
mod pseudolocalization;
mod review;
mod types;

pub use ferrocat_icu::IcuPseudolocalizationOptions;

pub use self::audit::{
    CatalogAuditChecks, CatalogAuditDiagnostic, CatalogAuditIcuOptions, CatalogAuditMessageRef,
    CatalogAuditOptions, CatalogAuditReport, CatalogAuditSummary, audit_catalogs,
    audit_catalogs_with_icu_options,
};
pub use self::catalog::{parse_catalog, update_catalog, update_catalog_file};
pub use self::combine::{combine_catalog_files, combine_catalogs};
pub use self::compile::{
    compile_catalog_artifact, compile_catalog_artifact_report, compile_catalog_artifact_selected,
    compile_catalog_artifact_selected_with_icu_options, compile_catalog_artifact_with_icu_options,
    compiled_key,
};
pub use self::compile_types::{
    COMPILED_CATALOG_ARTIFACT_SCHEMA_VERSION, CompileCatalogArtifactIcuOptions,
    CompileCatalogArtifactOptions, CompileCatalogArtifactReportOptions,
    CompileCatalogArtifactReportSelection, CompileCatalogOptions,
    CompileSelectedCatalogArtifactOptions, CompiledCatalog, CompiledCatalogArtifact,
    CompiledCatalogArtifactReport, CompiledCatalogDiagnostic, CompiledCatalogIdDescription,
    CompiledCatalogIdIndex, CompiledCatalogMissingMessage, CompiledCatalogProvenanceReport,
    CompiledCatalogResolution, CompiledCatalogResolutionKind, CompiledCatalogTranslationKind,
    CompiledCatalogUnavailableId, CompiledKeyStrategy, CompiledMessage, CompiledTranslation,
    DescribeCompiledIdsReport, IcuFormatterSupportPolicy,
};
pub use self::coverage::{
    CatalogCoverageMessage, CatalogCoverageOptions, CatalogCoverageReport, CatalogLocaleCoverage,
    catalog_coverage,
};
pub use self::message_status::CatalogMessageStatus;
pub use self::mt::{MachineTranslationMetadata, machine_translation_hash};
pub use self::pseudolocalization::{
    pseudolocalize_compiled_catalog_artifact,
    pseudolocalize_compiled_catalog_artifact_with_syntax_policy,
};
pub use self::review::{
    CatalogLocaleReview, CatalogMachineTranslationMessage, CatalogMachineTranslationReview,
    CatalogMachineTranslationStatus, CatalogReviewOptions, CatalogReviewReport,
    CatalogReviewSummary, CatalogReviewTranslation, CatalogSourceChange, CatalogSourceChangeKind,
    CatalogSourceChangeReport, CatalogTranslationChange, CatalogTranslationChangeReport,
    catalog_review,
};
pub use self::types::{
    ApiError, CatalogCombineInput, CatalogCombineResult, CatalogCombineSelection,
    CatalogCombineStats, CatalogConflictStrategy, CatalogFileCombineResult, CatalogFileFormat,
    CatalogMessage, CatalogMessageExtra, CatalogMessageKey, CatalogMode, CatalogOrigin,
    CatalogSemantics, CatalogStats, CatalogStorageFormat, CatalogUpdateInput, CatalogUpdateResult,
    CombineCatalogFilesOptions, CombineCatalogOptions, Diagnostic, DiagnosticSeverity,
    EffectiveTranslation, EffectiveTranslationRef, ExtractedMessage, ExtractedPluralMessage,
    ExtractedSingularMessage, IcuSyntaxPolicy, NormalizedParsedCatalog, ObsoleteStrategy, OrderBy,
    ParseCatalogOptions, ParsedCatalog, PlaceholderCommentMode, PluralEncoding, PluralSource,
    RenderOptions, SourceExtractedMessage, TranslationShape, UpdateCatalogFileOptions,
    UpdateCatalogOptions,
};
fn validate_source_locale(source_locale: &str) -> Result<(), ApiError> {
    if source_locale.trim().is_empty() {
        return Err(ApiError::InvalidArguments(
            "source_locale must not be empty".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod unit_tests {
    use super::{ApiError, validate_source_locale};

    #[test]
    fn validate_source_locale_rejects_empty_values() {
        assert!(validate_source_locale("en").is_ok());
        assert!(validate_source_locale(" en ").is_ok());
        assert!(matches!(
            validate_source_locale(" \n\t "),
            Err(ApiError::InvalidArguments(message)) if message.contains("must not be empty")
        ));
    }
}

#[cfg(test)]
mod tests;
