mod catalog;
mod compile;
mod compile_types;
mod file_io;
mod helpers;
mod ndjson;
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
    ApiError, CatalogMessage, CatalogMessageExtra, CatalogMessageKey, CatalogOrigin,
    CatalogSemantics, CatalogStats, CatalogStorageFormat, CatalogUpdateInput, CatalogUpdateResult,
    Diagnostic, DiagnosticSeverity, EffectiveTranslation, EffectiveTranslationRef,
    ExtractedMessage, ExtractedPluralMessage, ExtractedSingularMessage, NormalizedParsedCatalog,
    ObsoleteStrategy, OrderBy, ParseCatalogOptions, ParsedCatalog, PlaceholderCommentMode,
    PluralEncoding, PluralSource, SourceExtractedMessage, TranslationShape,
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

fn validate_catalog_semantics(
    semantics: CatalogSemantics,
    storage_format: CatalogStorageFormat,
    plural_encoding: PluralEncoding,
) -> Result<(), ApiError> {
    match semantics {
        CatalogSemantics::IcuNative if plural_encoding != PluralEncoding::Icu => {
            Err(ApiError::InvalidArguments(
                "CatalogSemantics::IcuNative requires PluralEncoding::Icu".to_owned(),
            ))
        }
        CatalogSemantics::GettextCompat if plural_encoding != PluralEncoding::Gettext => {
            Err(ApiError::InvalidArguments(
                "CatalogSemantics::GettextCompat requires PluralEncoding::Gettext".to_owned(),
            ))
        }
        CatalogSemantics::GettextCompat if storage_format == CatalogStorageFormat::Ndjson => {
            Err(ApiError::Unsupported(
                "CatalogSemantics::GettextCompat is not supported for NDJSON catalogs".to_owned(),
            ))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod unit_tests {
    use super::{
        ApiError, CatalogSemantics, CatalogStorageFormat, PluralEncoding,
        validate_catalog_semantics, validate_source_locale,
    };

    #[test]
    fn validate_source_locale_rejects_empty_values() {
        assert!(validate_source_locale("en").is_ok());
        assert!(validate_source_locale(" en ").is_ok());
        assert!(matches!(
            validate_source_locale(" \n\t "),
            Err(ApiError::InvalidArguments(message)) if message.contains("must not be empty")
        ));
    }

    #[test]
    fn validate_catalog_semantics_accepts_only_supported_combinations() {
        assert!(
            validate_catalog_semantics(
                CatalogSemantics::IcuNative,
                CatalogStorageFormat::Po,
                PluralEncoding::Icu
            )
            .is_ok()
        );
        assert!(
            validate_catalog_semantics(
                CatalogSemantics::IcuNative,
                CatalogStorageFormat::Ndjson,
                PluralEncoding::Icu
            )
            .is_ok()
        );
        assert!(
            validate_catalog_semantics(
                CatalogSemantics::GettextCompat,
                CatalogStorageFormat::Po,
                PluralEncoding::Gettext
            )
            .is_ok()
        );

        assert!(matches!(
            validate_catalog_semantics(
                CatalogSemantics::IcuNative,
                CatalogStorageFormat::Po,
                PluralEncoding::Gettext
            ),
            Err(ApiError::InvalidArguments(message))
                if message.contains("IcuNative requires PluralEncoding::Icu")
        ));
        assert!(matches!(
            validate_catalog_semantics(
                CatalogSemantics::GettextCompat,
                CatalogStorageFormat::Po,
                PluralEncoding::Icu
            ),
            Err(ApiError::InvalidArguments(message))
                if message.contains("GettextCompat requires PluralEncoding::Gettext")
        ));
        assert!(matches!(
            validate_catalog_semantics(
                CatalogSemantics::GettextCompat,
                CatalogStorageFormat::Ndjson,
                PluralEncoding::Gettext
            ),
            Err(ApiError::Unsupported(message))
                if message.contains("not supported for NDJSON")
        ));
    }
}

#[cfg(test)]
mod tests;
