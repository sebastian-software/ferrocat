pub(super) use super::{
    ApiError, CatalogAuditOptions, CatalogCombineInput, CatalogCombineSelection,
    CatalogConflictStrategy, CatalogCoverageOptions, CatalogFileFormat, CatalogMessageKey,
    CatalogMessageStatus, CatalogMode, CatalogOrigin, CatalogReviewOptions, CatalogSemantics,
    CatalogUpdateInput, CombineCatalogFilesOptions, CombineCatalogOptions,
    CompileCatalogArtifactIcuOptions, CompileCatalogArtifactOptions,
    CompileCatalogArtifactReportOptions, CompileCatalogOptions,
    CompileSelectedCatalogArtifactOptions, CompiledCatalogIdIndex, CompiledCatalogResolutionKind,
    CompiledCatalogTranslationKind, CompiledKeyStrategy, CompiledTranslation, DiagnosticSeverity,
    EffectiveTranslation, EffectiveTranslationRef, ExtractedMessage, ExtractedPluralMessage,
    ExtractedSingularMessage, IcuSyntaxPolicy, ObsoleteStrategy, OrderBy, ParseCatalogOptions,
    PlaceholderCommentMode, PluralEncoding, PluralSource, RenderOptions, SourceExtractedMessage,
    TranslationShape, UpdateCatalogFileOptions, UpdateCatalogOptions, audit_catalogs,
    combine_catalog_files, combine_catalogs, compile::compiled_key_for, compile_catalog_artifact,
    compile_catalog_artifact_report, compile_catalog_artifact_selected,
    compile_catalog_artifact_selected_with_icu_options, compile_catalog_artifact_with_icu_options,
    compiled_key, machine_translation_hash, measure_catalog_coverage, parse_catalog,
    plural::cached_icu_plural_categories_for, review_catalogs, update_catalog, update_catalog_file,
};
pub(super) use crate::parse_po;
pub(super) use std::collections::{BTreeMap, HashMap};
pub(super) use std::fs;
pub(super) use std::sync::Mutex;

mod audit;
mod catalog;
mod compile;
mod coverage;
mod plural;
mod review;

pub(super) fn structured_input(messages: Vec<ExtractedMessage>) -> CatalogUpdateInput {
    CatalogUpdateInput::Structured(messages)
}

pub(super) fn source_first_input(messages: Vec<SourceExtractedMessage>) -> CatalogUpdateInput {
    CatalogUpdateInput::SourceFirst(messages)
}

pub(super) fn normalized_catalog(
    content: &str,
    locale: Option<&str>,
    plural_encoding: PluralEncoding,
) -> super::NormalizedParsedCatalog {
    let mode = match plural_encoding {
        PluralEncoding::Icu => CatalogMode::IcuPo,
        PluralEncoding::Gettext => CatalogMode::GettextPo,
    };
    parse_catalog(ParseCatalogOptions {
        locale,
        mode,
        ..ParseCatalogOptions::new(content, "en")
    })
    .expect("parse catalog")
    .into_normalized_view()
    .expect("normalized view")
}

pub(super) fn normalized_fcl_catalog(
    content: &str,
    locale: Option<&str>,
) -> super::NormalizedParsedCatalog {
    parse_catalog(ParseCatalogOptions {
        locale,
        mode: CatalogMode::IcuFcl,
        ..ParseCatalogOptions::new(content, "en")
    })
    .expect("parse fcl catalog")
    .into_normalized_view()
    .expect("normalized fcl view")
}
