mod audit;
mod catalog;
mod compile;
mod compile_types;
mod file_io;
mod helpers;
mod mt;
mod ndjson;
mod plural;
mod types;

pub use self::audit::{
    CatalogAuditChecks, CatalogAuditDiagnostic, CatalogAuditMessageRef, CatalogAuditOptions,
    CatalogAuditReport, CatalogAuditSummary, audit_catalogs,
};
pub use self::catalog::{combine_catalogs, parse_catalog, update_catalog, update_catalog_file};
pub use self::compile::{
    compile_catalog_artifact, compile_catalog_artifact_selected, compiled_key,
};
pub use self::compile_types::{
    COMPILED_CATALOG_ARTIFACT_SCHEMA_VERSION, CompileCatalogArtifactOptions, CompileCatalogOptions,
    CompileSelectedCatalogArtifactOptions, CompiledCatalog, CompiledCatalogArtifact,
    CompiledCatalogDiagnostic, CompiledCatalogIdDescription, CompiledCatalogIdIndex,
    CompiledCatalogMissingMessage, CompiledCatalogTranslationKind, CompiledCatalogUnavailableId,
    CompiledKeyStrategy, CompiledMessage, CompiledTranslation, DescribeCompiledIdsReport,
};
pub use self::mt::{MachineTranslationMetadata, machine_translation_hash};
pub use self::ndjson::{
    NdjsonCatalogReader, NdjsonCatalogReaderOptions, NdjsonCatalogWriter,
    NdjsonCatalogWriterOptions,
};
pub use self::types::{
    ApiError, CatalogCombineInput, CatalogCombineResult, CatalogCombineSelection,
    CatalogCombineStats, CatalogConflictStrategy, CatalogMessage, CatalogMessageExtra,
    CatalogMessageKey, CatalogMode, CatalogOrigin, CatalogSemantics, CatalogStats,
    CatalogStorageFormat, CatalogUpdateInput, CatalogUpdateResult, CombineCatalogOptions,
    Diagnostic, DiagnosticSeverity, EffectiveTranslation, EffectiveTranslationRef,
    ExtractedMessage, ExtractedPluralMessage, ExtractedSingularMessage, NormalizedParsedCatalog,
    ObsoleteStrategy, OrderBy, ParseCatalogOptions, ParsedCatalog, PlaceholderCommentMode,
    PluralEncoding, PluralSource, RenderOptions, SourceExtractedMessage, TranslationShape,
    UpdateCatalogFileOptions, UpdateCatalogOptions,
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
