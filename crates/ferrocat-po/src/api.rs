mod catalog;
mod compile;
mod compile_types;
mod file_io;
mod helpers;
mod plural;
mod types;

pub use self::catalog::{parse_catalog, update_catalog, update_catalog_file};
pub use self::compile::{
    compile_catalog_artifact, compile_catalog_artifact_selected, compiled_key,
};
pub use self::compile_types::{
    CompileCatalogArtifactOptions, CompileCatalogOptions, CompileSelectedCatalogArtifactOptions,
    CompiledCatalog, CompiledCatalogArtifact, CompiledCatalogDiagnostic,
    CompiledCatalogIdDescription, CompiledCatalogIdIndex, CompiledCatalogMissingMessage,
    CompiledCatalogTranslationKind, CompiledCatalogUnavailableId, CompiledKeyStrategy,
    CompiledMessage, CompiledTranslation, DescribeCompiledIdsReport,
};
pub use self::types::{
    ApiError, CatalogMessage, CatalogMessageExtra, CatalogMessageKey, CatalogOrigin, CatalogStats,
    CatalogUpdateInput, CatalogUpdateResult, Diagnostic, DiagnosticSeverity, EffectiveTranslation,
    EffectiveTranslationRef, ExtractedMessage, ExtractedPluralMessage, ExtractedSingularMessage,
    NormalizedParsedCatalog, ObsoleteStrategy, OrderBy, ParseCatalogOptions, ParsedCatalog,
    PlaceholderCommentMode, PluralEncoding, PluralSource, SourceExtractedMessage, TranslationShape,
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
mod tests;
