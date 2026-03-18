pub(super) use super::{
    ApiError, CatalogMessageKey, CatalogUpdateInput, CompileCatalogArtifactOptions,
    CompileCatalogOptions, CompileSelectedCatalogArtifactOptions, CompiledCatalogIdIndex,
    CompiledCatalogTranslationKind, CompiledKeyStrategy, CompiledTranslation, DiagnosticSeverity,
    EffectiveTranslation, EffectiveTranslationRef, ExtractedMessage, ExtractedPluralMessage,
    ExtractedSingularMessage, ObsoleteStrategy, ParseCatalogOptions, PluralEncoding, PluralSource,
    SourceExtractedMessage, TranslationShape, UpdateCatalogFileOptions, UpdateCatalogOptions,
    compile::compiled_key_for, compile_catalog_artifact, compile_catalog_artifact_selected,
    compiled_key, parse_catalog, plural::cached_icu_plural_categories_for, update_catalog,
    update_catalog_file,
};
pub(super) use crate::parse_po;
pub(super) use std::collections::{BTreeMap, HashMap};
pub(super) use std::fs;
pub(super) use std::sync::Mutex;

mod catalog;
mod compile;
mod plural;

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
    parse_catalog(ParseCatalogOptions {
        content,
        source_locale: "en",
        locale,
        plural_encoding,
        ..ParseCatalogOptions::default()
    })
    .expect("parse catalog")
    .into_normalized_view()
    .expect("normalized view")
}
