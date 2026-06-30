//! Runtime-oriented compilation helpers for normalized catalogs.
//!
//! This module sits on the far side of parsing/update work: it turns normalized
//! catalog messages into stable compiled IDs and final runtime message payloads,
//! including fallback resolution and optional ICU validation.

use std::collections::{BTreeMap, BTreeSet};

use ferrocat_icu::{
    IcuCompatibilityOptions, IcuDiagnosticSeverity, compare_icu_messages,
    validate_icu_formatter_support,
};
use sha2::{Digest, Sha256};

use crate::diagnostic_codes;

use super::icu_syntax::parse_icu_with_syntax_policy;
use super::plural::synthesize_icu_plural;
use super::{
    ApiError, CatalogMessage, CatalogMessageKey, CatalogSemantics,
    CompileCatalogArtifactIcuOptions, CompileCatalogArtifactOptions,
    CompileCatalogArtifactReportOptions, CompileCatalogArtifactReportSelection,
    CompileCatalogOptions, CompileSelectedCatalogArtifactOptions, CompiledCatalog,
    CompiledCatalogArtifact, CompiledCatalogArtifactReport, CompiledCatalogDiagnostic,
    CompiledCatalogIdIndex, CompiledCatalogMissingMessage, CompiledCatalogProvenanceReport,
    CompiledCatalogResolution, CompiledCatalogResolutionKind, CompiledCatalogTranslationKind,
    CompiledKeyStrategy, CompiledMessage, CompiledTranslation, DiagnosticSeverity,
    EffectiveTranslation, NormalizedParsedCatalog, TranslationShape,
};

impl NormalizedParsedCatalog {
    /// Compiles the normalized catalog into a runtime-oriented lookup map.
    ///
    /// Compiled keys are derived from the canonical gettext identity
    /// (`msgctxt` + `msgid`) using the selected built-in key strategy.
    /// The default configuration keeps translations as-is without filling
    /// missing values from the source text.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::InvalidArguments`] when source fallback is enabled
    /// without a `source_locale`, or [`ApiError::Conflict`] when two source
    /// messages compile to the same derived key.
    ///
    /// ```rust
    /// use ferrocat_po::{CompileCatalogOptions, ParseCatalogOptions, parse_catalog};
    ///
    /// let parsed = parse_catalog(ParseCatalogOptions {
    ///     locale: Some("de"),
    ///     ..ParseCatalogOptions::new("msgid \"Hello\"\nmsgstr \"Hallo\"\n", "en")
    /// })?;
    /// let normalized = parsed.into_normalized_view()?;
    /// let compiled = normalized.compile(&CompileCatalogOptions::default())?;
    ///
    /// assert_eq!(compiled.len(), 1);
    /// let (_, message) = compiled.iter().next().expect("compiled message");
    /// assert_eq!(message.source_key.msgid, "Hello");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn compile(
        &self,
        options: &CompileCatalogOptions<'_>,
    ) -> Result<CompiledCatalog, ApiError> {
        self.compile_with_key_generator(options, compiled_key_for)
    }

    /// Shared compile core used by the public API and collision-focused tests.
    pub(super) fn compile_with_key_generator<F>(
        &self,
        options: &CompileCatalogOptions<'_>,
        mut key_generator: F,
    ) -> Result<CompiledCatalog, ApiError>
    where
        F: FnMut(CompiledKeyStrategy, &CatalogMessageKey) -> String,
    {
        validate_compiled_catalog_semantics(self, options.semantics)?;
        let source_locale = if options.source_fallback {
            Some(options.source_locale.ok_or_else(|| {
                ApiError::InvalidArguments(
                    "compile_catalog source_fallback requires source_locale".to_owned(),
                )
            })?)
        } else {
            None
        };
        let mut entries = BTreeMap::new();

        for (source_key, message) in self.iter() {
            let effective = source_locale.map_or_else(
                || message.effective_translation_owned(),
                |source_locale| {
                    self.effective_translation_with_source_fallback(source_key, source_locale)
                        .expect("normalized catalog lookup")
                },
            );
            let translation = compiled_translation_for_message(
                message,
                effective,
                self.parsed_catalog().semantics,
            )
            .ok_or_else(|| {
                ApiError::InvalidArguments(format!(
                    "catalog semantics {:?} were inconsistent with message {:?} / {:?}",
                    self.parsed_catalog().semantics,
                    source_key.msgctxt,
                    source_key.msgid
                ))
            })?;
            let compiled_key = key_generator(options.key_strategy, source_key);
            let compiled_message = CompiledMessage {
                key: compiled_key.clone(),
                source_key: source_key.clone(),
                translation,
            };

            if let Some(existing) = entries.insert(compiled_key.clone(), compiled_message) {
                return Err(ApiError::Conflict(format!(
                    "compiled catalog key collision for {:?} / {:?} and {:?} / {:?} using key {}",
                    existing.source_key.msgctxt,
                    existing.source_key.msgid,
                    source_key.msgctxt,
                    source_key.msgid,
                    compiled_key
                )));
            }
        }

        Ok(CompiledCatalog { entries })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedArtifactMessage {
    locale: String,
    message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompiledCatalogArtifactOutput {
    artifact: CompiledCatalogArtifact,
    provenance: CompiledCatalogProvenanceReport,
}

#[derive(Debug, Clone, Copy)]
struct CompiledIcuDiagnosticTarget<'a> {
    source_key: &'a CatalogMessageKey,
    compiled_key: &'a str,
    locale: &'a str,
}

/// Compiles one requested-locale runtime artifact from one or more normalized catalogs.
///
/// The artifact is host-neutral: it produces the final runtime message strings keyed by
/// Ferrocat's derived lookup key, plus missing-message records and compile diagnostics.
///
/// # Errors
///
/// Returns [`ApiError::InvalidArguments`] when required locales are missing, duplicated,
/// or inconsistent with the provided catalog set; [`ApiError::Conflict`] when two source
/// identities compile to the same derived key; or [`ApiError::Unsupported`] when
/// `strict_icu` is enabled and a final runtime message fails ICU validation.
pub fn compile_catalog_artifact(
    catalogs: &[&NormalizedParsedCatalog],
    options: &CompileCatalogArtifactOptions<'_>,
) -> Result<CompiledCatalogArtifact, ApiError> {
    compile_catalog_artifact_with_icu_options(
        catalogs,
        options,
        &CompileCatalogArtifactIcuOptions::default(),
    )
}

/// Compiles one requested-locale runtime artifact with explicit ICU syntax options.
///
/// # Errors
///
/// Returns [`ApiError::InvalidArguments`] when required locales are missing, duplicated,
/// or inconsistent with the provided catalog set; [`ApiError::Conflict`] when two source
/// identities compile to the same derived key; or [`ApiError::Unsupported`] when
/// `strict_icu` is enabled and a final runtime message fails ICU validation.
pub fn compile_catalog_artifact_with_icu_options(
    catalogs: &[&NormalizedParsedCatalog],
    options: &CompileCatalogArtifactOptions<'_>,
    icu_options: &CompileCatalogArtifactIcuOptions,
) -> Result<CompiledCatalogArtifact, ApiError> {
    let locales = prepare_compiled_catalog_artifact_catalogs(
        catalogs,
        options.requested_locale,
        options.source_locale,
        options.fallback_chain,
        options.semantics,
    )?;
    compile_catalog_artifact_from_source_keys(
        &locales,
        collect_compiled_catalog_artifact_source_keys(&locales),
        options,
        icu_options,
    )
}

/// Compiles one requested-locale runtime artifact for a selected subset of compiled IDs.
///
/// # Errors
///
/// Returns [`ApiError::InvalidArguments`] when the selected IDs are unknown or the
/// catalog inputs are inconsistent, [`ApiError::Conflict`] on compiled-key collisions,
/// or [`ApiError::Unsupported`] when `strict_icu` is enabled and a final runtime
/// message fails ICU validation.
pub fn compile_catalog_artifact_selected(
    catalogs: &[&NormalizedParsedCatalog],
    index: &CompiledCatalogIdIndex,
    options: &CompileSelectedCatalogArtifactOptions<'_>,
) -> Result<CompiledCatalogArtifact, ApiError> {
    compile_catalog_artifact_selected_with_icu_options(
        catalogs,
        index,
        options,
        &CompileCatalogArtifactIcuOptions::default(),
    )
}

/// Compiles a selected subset of compiled IDs with explicit ICU syntax options.
///
/// # Errors
///
/// Returns [`ApiError::InvalidArguments`] when the selected IDs are unknown or the
/// catalog inputs are inconsistent, [`ApiError::Conflict`] on compiled-key collisions,
/// or [`ApiError::Unsupported`] when `strict_icu` is enabled and a final runtime
/// message fails ICU validation.
pub fn compile_catalog_artifact_selected_with_icu_options(
    catalogs: &[&NormalizedParsedCatalog],
    index: &CompiledCatalogIdIndex,
    options: &CompileSelectedCatalogArtifactOptions<'_>,
    icu_options: &CompileCatalogArtifactIcuOptions,
) -> Result<CompiledCatalogArtifact, ApiError> {
    let artifact_options = &options.options;
    let locales = prepare_compiled_catalog_artifact_catalogs(
        catalogs,
        artifact_options.requested_locale,
        artifact_options.source_locale,
        artifact_options.fallback_chain,
        artifact_options.semantics,
    )?;

    let source_keys = selected_compiled_catalog_artifact_source_keys(
        &locales,
        index,
        options.compiled_ids,
        "compile_catalog_artifact_selected",
    )?;

    compile_catalog_artifact_from_source_keys(&locales, source_keys, artifact_options, icu_options)
}

/// Compiles one requested-locale runtime artifact with a sibling provenance report.
///
/// This is the report-oriented entry point for both full and selected artifact
/// compilation. The returned artifact uses the same schema and semantics as
/// [`compile_catalog_artifact`], while the sibling provenance report records how
/// each compiled source identity resolved.
///
/// # Errors
///
/// Returns [`ApiError::InvalidArguments`] when required locales are missing,
/// duplicated, or inconsistent with the provided catalog set; when selected IDs
/// are unknown or absent from the catalog set; [`ApiError::Conflict`] when two
/// source identities compile to the same derived key; or [`ApiError::Unsupported`]
/// when `strict_icu` is enabled and a final runtime message fails ICU validation.
pub fn compile_catalog_artifact_report(
    catalogs: &[&NormalizedParsedCatalog],
    options: &CompileCatalogArtifactReportOptions<'_>,
) -> Result<CompiledCatalogArtifactReport, ApiError> {
    let artifact_options = &options.options;
    let locales = prepare_compiled_catalog_artifact_catalogs(
        catalogs,
        artifact_options.requested_locale,
        artifact_options.source_locale,
        artifact_options.fallback_chain,
        artifact_options.semantics,
    )?;
    let source_keys = match options.selection {
        CompileCatalogArtifactReportSelection::All => {
            collect_compiled_catalog_artifact_source_keys(&locales)
        }
        CompileCatalogArtifactReportSelection::Selected {
            index,
            compiled_ids,
        } => selected_compiled_catalog_artifact_source_keys(
            &locales,
            index,
            compiled_ids,
            "compile_catalog_artifact_report",
        )?,
    };

    compile_catalog_artifact_report_from_source_keys(
        &locales,
        source_keys,
        artifact_options,
        &options.icu_options,
    )
}

fn compiled_translation_for_message(
    message: &CatalogMessage,
    value: EffectiveTranslation,
    semantics: CatalogSemantics,
) -> Option<CompiledTranslation> {
    match (&message.translation, value) {
        (TranslationShape::Singular { .. }, EffectiveTranslation::Singular(value)) => {
            Some(CompiledTranslation::Singular(value))
        }
        (TranslationShape::Plural { variable, .. }, EffectiveTranslation::Plural(values)) => {
            match semantics {
                CatalogSemantics::IcuNative => Some(CompiledTranslation::Singular(
                    synthesize_icu_plural(variable, &values),
                )),
                CatalogSemantics::GettextCompat => Some(CompiledTranslation::Plural(values)),
            }
        }
        _ => None,
    }
}

/// Derives the default stable runtime lookup key for `msgid` and `msgctxt`.
///
/// This public helper uses the same `FerrocatV1` key contract as
/// [`NormalizedParsedCatalog::compile`] and [`compile_catalog_artifact`].
///
/// ```rust
/// use ferrocat_po::compiled_key;
///
/// let without_context = compiled_key("Save", None);
/// let with_context = compiled_key("Save", Some("menu"));
///
/// assert_eq!(without_context.len(), 11);
/// assert_ne!(without_context, with_context);
/// ```
#[must_use]
pub fn compiled_key(msgid: &str, msgctxt: Option<&str>) -> String {
    compiled_key_for(
        CompiledKeyStrategy::FerrocatV1,
        &CatalogMessageKey::new(msgid, msgctxt.map(str::to_owned)),
    )
}

pub(super) fn compiled_key_for(strategy: CompiledKeyStrategy, key: &CatalogMessageKey) -> String {
    match strategy {
        CompiledKeyStrategy::FerrocatV1 => ferrocat_v1_compiled_key(key),
    }
}

fn ferrocat_v1_compiled_key(key: &CatalogMessageKey) -> String {
    let mut payload = Vec::with_capacity(
        16 + 1 + 4 + key.msgctxt.as_ref().map_or(0, String::len) + 1 + 4 + key.msgid.len(),
    );
    payload.extend_from_slice(b"ferrocat:compile:v1");
    push_compiled_key_component(&mut payload, key.msgctxt.as_deref());
    push_compiled_key_component(&mut payload, Some(key.msgid.as_str()));
    let digest = Sha256::digest(&payload);
    base64_url_no_pad(&digest[..8])
}

fn push_compiled_key_component(out: &mut Vec<u8>, value: Option<&str>) {
    if let Some(value) = value {
        out.push(1);
        let value_len = u32::try_from(value.len()).expect("compiled key component exceeds u32");
        out.extend_from_slice(&value_len.to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    } else {
        out.push(0);
        out.extend_from_slice(&0u32.to_be_bytes());
    }
}

fn base64_url_no_pad(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity((bytes.len() * 4).div_ceil(3));
    let mut index = 0;

    while index + 3 <= bytes.len() {
        let chunk = (u32::from(bytes[index]) << 16)
            | (u32::from(bytes[index + 1]) << 8)
            | u32::from(bytes[index + 2]);
        out.push(ALPHABET[((chunk >> 18) & 0x3f) as usize] as char);
        out.push(ALPHABET[((chunk >> 12) & 0x3f) as usize] as char);
        out.push(ALPHABET[((chunk >> 6) & 0x3f) as usize] as char);
        out.push(ALPHABET[(chunk & 0x3f) as usize] as char);
        index += 3;
    }

    match bytes.len() - index {
        1 => {
            let chunk = u32::from(bytes[index]) << 16;
            out.push(ALPHABET[((chunk >> 18) & 0x3f) as usize] as char);
            out.push(ALPHABET[((chunk >> 12) & 0x3f) as usize] as char);
        }
        2 => {
            let chunk = (u32::from(bytes[index]) << 16) | (u32::from(bytes[index + 1]) << 8);
            out.push(ALPHABET[((chunk >> 18) & 0x3f) as usize] as char);
            out.push(ALPHABET[((chunk >> 12) & 0x3f) as usize] as char);
            out.push(ALPHABET[((chunk >> 6) & 0x3f) as usize] as char);
        }
        _ => {}
    }

    out
}

pub(super) fn describe_compiled_id_catalogs<'a>(
    catalogs: &[&'a NormalizedParsedCatalog],
) -> Result<BTreeMap<String, &'a NormalizedParsedCatalog>, ApiError> {
    let mut locales = BTreeMap::<String, &NormalizedParsedCatalog>::new();

    for catalog in catalogs {
        let locale = catalog
            .parsed_catalog()
            .locale
            .as_deref()
            .ok_or_else(|| {
                ApiError::InvalidArguments(
                    "describe_compiled_ids requires every catalog to declare a locale".to_owned(),
                )
            })?
            .trim()
            .to_owned();
        if locale.is_empty() {
            return Err(ApiError::InvalidArguments(
                "describe_compiled_ids does not accept empty catalog locales".to_owned(),
            ));
        }
        if locales.insert(locale.clone(), *catalog).is_some() {
            return Err(ApiError::InvalidArguments(format!(
                "describe_compiled_ids received duplicate catalog locale {locale:?}"
            )));
        }
    }

    Ok(locales)
}

pub(super) fn compiled_catalog_translation_kind_for_message(
    semantics: CatalogSemantics,
    message: &CatalogMessage,
) -> CompiledCatalogTranslationKind {
    match (semantics, &message.translation) {
        (_, TranslationShape::Singular { .. }) => CompiledCatalogTranslationKind::Singular,
        (CatalogSemantics::IcuNative, TranslationShape::Plural { .. }) => {
            CompiledCatalogTranslationKind::Singular
        }
        (CatalogSemantics::GettextCompat, TranslationShape::Plural { .. }) => {
            CompiledCatalogTranslationKind::Plural
        }
    }
}

/// Validates and indexes the locale set used by artifact compilation.
///
/// This up-front normalization keeps the later artifact loop allocation-light
/// and lets it assume that requested/source/fallback locales are all present and unique.
fn prepare_compiled_catalog_artifact_catalogs<'a>(
    catalogs: &[&'a NormalizedParsedCatalog],
    requested_locale: &str,
    source_locale: &str,
    fallback_chain: &[String],
    semantics: CatalogSemantics,
) -> Result<BTreeMap<String, &'a NormalizedParsedCatalog>, ApiError> {
    super::validate_source_locale(source_locale)?;
    if requested_locale.trim().is_empty() {
        return Err(ApiError::InvalidArguments(
            "requested_locale must not be empty".to_owned(),
        ));
    }
    if catalogs.is_empty() {
        return Err(ApiError::InvalidArguments(
            "compile_catalog_artifact requires at least one catalog".to_owned(),
        ));
    }

    let mut locales = BTreeMap::<String, &NormalizedParsedCatalog>::new();
    for catalog in catalogs {
        validate_compiled_catalog_semantics(catalog, semantics)?;
        let locale = catalog
            .parsed_catalog()
            .locale
            .as_deref()
            .ok_or_else(|| {
                ApiError::InvalidArguments(
                    "compile_catalog_artifact requires every catalog to declare a locale"
                        .to_owned(),
                )
            })?
            .trim()
            .to_owned();
        if locale.is_empty() {
            return Err(ApiError::InvalidArguments(
                "compile_catalog_artifact does not accept empty catalog locales".to_owned(),
            ));
        }
        if locales.insert(locale.clone(), *catalog).is_some() {
            return Err(ApiError::InvalidArguments(format!(
                "compile_catalog_artifact received duplicate catalog locale {locale:?}"
            )));
        }
    }

    if !locales.contains_key(requested_locale) {
        return Err(ApiError::InvalidArguments(format!(
            "compile_catalog_artifact is missing the requested locale catalog {:?}",
            requested_locale
        )));
    }
    if !locales.contains_key(source_locale) {
        return Err(ApiError::InvalidArguments(format!(
            "compile_catalog_artifact is missing the source locale catalog {:?}",
            source_locale
        )));
    }

    let mut seen_fallbacks = BTreeSet::new();
    for locale in fallback_chain {
        if locale == requested_locale || locale == source_locale {
            return Err(ApiError::InvalidArguments(format!(
                "compile_catalog_artifact fallback_chain must not repeat requested or source locale {:?}",
                locale
            )));
        }
        if !seen_fallbacks.insert(locale.clone()) {
            return Err(ApiError::InvalidArguments(format!(
                "compile_catalog_artifact fallback_chain contains duplicate locale {:?}",
                locale
            )));
        }
        if !locales.contains_key(locale) {
            return Err(ApiError::InvalidArguments(format!(
                "compile_catalog_artifact fallback locale {:?} was not provided",
                locale
            )));
        }
    }

    Ok(locales)
}

/// Collects every non-obsolete source key that might need to appear in an
/// artifact compiled from the provided locale set.
fn collect_compiled_catalog_artifact_source_keys(
    locales: &BTreeMap<String, &NormalizedParsedCatalog>,
) -> BTreeSet<CatalogMessageKey> {
    let mut source_keys = BTreeSet::new();
    for catalog in locales.values() {
        for (source_key, message) in catalog.iter() {
            if !message.obsolete {
                source_keys.insert(source_key.clone());
            }
        }
    }
    source_keys
}

fn compiled_catalog_artifact_catalogs_contain_key(
    locales: &BTreeMap<String, &NormalizedParsedCatalog>,
    source_key: &CatalogMessageKey,
) -> bool {
    locales.values().any(|catalog| {
        catalog
            .get(source_key)
            .is_some_and(|message| !message.obsolete)
    })
}

fn selected_compiled_catalog_artifact_source_keys(
    locales: &BTreeMap<String, &NormalizedParsedCatalog>,
    index: &CompiledCatalogIdIndex,
    compiled_ids: &[String],
    function_name: &str,
) -> Result<BTreeSet<CatalogMessageKey>, ApiError> {
    let mut source_keys = BTreeSet::new();
    for compiled_id in compiled_ids {
        let source_key = index.get(compiled_id).ok_or_else(|| {
            ApiError::InvalidArguments(format!(
                "{function_name} received unknown compiled ID {compiled_id:?}"
            ))
        })?;
        if !compiled_catalog_artifact_catalogs_contain_key(locales, source_key) {
            return Err(ApiError::InvalidArguments(format!(
                "{function_name} compiled ID {compiled_id:?} was not present in the provided catalog set"
            )));
        }
        source_keys.insert(source_key.clone());
    }
    Ok(source_keys)
}

/// Compiles the final runtime artifact for a known set of source identities.
///
/// This is where derived key collision checks, fallback bookkeeping, and final
/// ICU validation come together before the artifact is returned.
fn compile_catalog_artifact_from_source_keys<I>(
    locales: &BTreeMap<String, &NormalizedParsedCatalog>,
    source_keys: I,
    options: &CompileCatalogArtifactOptions<'_>,
    icu_options: &CompileCatalogArtifactIcuOptions,
) -> Result<CompiledCatalogArtifact, ApiError>
where
    I: IntoIterator<Item = CatalogMessageKey>,
{
    let output = compile_catalog_artifact_from_source_keys_inner(
        locales,
        source_keys,
        options,
        icu_options,
    )?;
    Ok(output.artifact)
}

fn compile_catalog_artifact_report_from_source_keys<I>(
    locales: &BTreeMap<String, &NormalizedParsedCatalog>,
    source_keys: I,
    options: &CompileCatalogArtifactOptions<'_>,
    icu_options: &CompileCatalogArtifactIcuOptions,
) -> Result<CompiledCatalogArtifactReport, ApiError>
where
    I: IntoIterator<Item = CatalogMessageKey>,
{
    let output = compile_catalog_artifact_from_source_keys_inner(
        locales,
        source_keys,
        options,
        icu_options,
    )?;
    Ok(CompiledCatalogArtifactReport {
        artifact: output.artifact,
        provenance: output.provenance,
    })
}

fn compile_catalog_artifact_from_source_keys_inner<I>(
    locales: &BTreeMap<String, &NormalizedParsedCatalog>,
    source_keys: I,
    options: &CompileCatalogArtifactOptions<'_>,
    icu_options: &CompileCatalogArtifactIcuOptions,
) -> Result<CompiledCatalogArtifactOutput, ApiError>
where
    I: IntoIterator<Item = CatalogMessageKey>,
{
    let mut compiled_keys = BTreeMap::<String, CatalogMessageKey>::new();
    let mut artifact = CompiledCatalogArtifact::default();
    let mut provenance = CompiledCatalogProvenanceReport {
        requested_locale: options.requested_locale.to_owned(),
        source_locale: options.source_locale.to_owned(),
        fallback_chain: options.fallback_chain.to_vec(),
        messages: Vec::new(),
    };

    for source_key in source_keys {
        let compiled_key = compiled_key_for(options.key_strategy, &source_key);
        if let Some(existing) = compiled_keys.insert(compiled_key.clone(), source_key.clone()) {
            return Err(ApiError::Conflict(format!(
                "compiled catalog key collision for {:?} / {:?} and {:?} / {:?} using key {}",
                existing.msgctxt,
                existing.msgid,
                source_key.msgctxt,
                source_key.msgid,
                compiled_key
            )));
        }

        let resolved = resolve_compiled_catalog_artifact_message(locales, &source_key, options);
        provenance.messages.push(CompiledCatalogResolution {
            key: compiled_key.clone(),
            source_key: source_key.clone(),
            resolved_locale: resolved.as_ref().map(|value| value.locale.clone()),
            kind: compiled_catalog_resolution_kind(options, resolved.as_ref()),
        });

        if options.requested_locale != options.source_locale {
            let resolved_locale = resolved.as_ref().map(|value| value.locale.clone());
            if resolved_locale.as_deref() != Some(options.requested_locale) {
                artifact.missing.push(CompiledCatalogMissingMessage {
                    key: compiled_key.clone(),
                    source_key: source_key.clone(),
                    requested_locale: options.requested_locale.to_owned(),
                    resolved_locale: resolved_locale.clone(),
                });
            }
        }

        let Some(resolved) = resolved else {
            continue;
        };

        let resolved_icu = match parse_icu_with_syntax_policy(
            &resolved.message,
            icu_options.syntax_policy,
        ) {
            Ok(message) => Some(message),
            Err(error) => {
                if options.strict_icu {
                    return Err(ApiError::Unsupported(format!(
                        "compiled catalog artifact produced invalid ICU for locale {:?}, msgid {:?}, context {:?}: {}",
                        resolved.locale, source_key.msgid, source_key.msgctxt, error
                    )));
                }
                artifact.diagnostics.push(CompiledCatalogDiagnostic {
                    severity: DiagnosticSeverity::Error,
                    code: diagnostic_codes::compile::INVALID_ICU_MESSAGE.to_owned(),
                    message: format!("Final runtime message failed ICU validation: {error}"),
                    key: compiled_key.clone(),
                    msgid: source_key.msgid.clone(),
                    msgctxt: source_key.msgctxt.clone(),
                    locale: resolved.locale.clone(),
                });
                None
            }
        };

        if let Some(resolved_icu) = resolved_icu.as_ref() {
            let diagnostic_target = CompiledIcuDiagnosticTarget {
                source_key: &source_key,
                compiled_key: &compiled_key,
                locale: resolved.locale.as_str(),
            };
            push_icu_compatibility_diagnostics(
                locales,
                diagnostic_target,
                resolved_icu,
                options,
                icu_options,
                &mut artifact,
            );
            push_formatter_support_diagnostics(
                diagnostic_target,
                resolved_icu,
                icu_options,
                &mut artifact,
            );
        }

        artifact.messages.insert(compiled_key, resolved.message);
    }

    Ok(CompiledCatalogArtifactOutput {
        artifact,
        provenance,
    })
}

fn compiled_catalog_resolution_kind(
    options: &CompileCatalogArtifactOptions<'_>,
    resolved: Option<&ResolvedArtifactMessage>,
) -> CompiledCatalogResolutionKind {
    let Some(resolved) = resolved else {
        return CompiledCatalogResolutionKind::Unresolved;
    };
    if resolved.locale == options.requested_locale {
        return CompiledCatalogResolutionKind::Requested;
    }
    if resolved.locale == options.source_locale {
        return CompiledCatalogResolutionKind::SourceFallback;
    }
    CompiledCatalogResolutionKind::Fallback
}

fn push_formatter_support_diagnostics(
    target: CompiledIcuDiagnosticTarget<'_>,
    resolved_icu: &ferrocat_icu::IcuMessage,
    icu_options: &CompileCatalogArtifactIcuOptions,
    artifact: &mut CompiledCatalogArtifact,
) {
    let Some(formatter_support) = icu_options.formatter_support else {
        return;
    };
    let report = validate_icu_formatter_support(resolved_icu, formatter_support);
    push_icu_diagnostics_for_compiled_message(report.diagnostics, target, artifact);
}

fn push_icu_compatibility_diagnostics(
    locales: &BTreeMap<String, &NormalizedParsedCatalog>,
    target: CompiledIcuDiagnosticTarget<'_>,
    resolved_icu: &ferrocat_icu::IcuMessage,
    options: &CompileCatalogArtifactOptions<'_>,
    icu_options: &CompileCatalogArtifactIcuOptions,
    artifact: &mut CompiledCatalogArtifact,
) {
    if !options.icu_compatibility {
        return;
    }
    let Some(source_catalog) = locales.get(options.source_locale) else {
        return;
    };
    let Some(source_message) = rendered_compiled_catalog_artifact_message(
        source_catalog,
        target.source_key,
        options.source_locale,
        true,
    ) else {
        return;
    };
    let Ok(source_icu) = parse_icu_with_syntax_policy(&source_message, icu_options.syntax_policy)
    else {
        return;
    };

    let report = compare_icu_messages(
        &source_icu,
        resolved_icu,
        &IcuCompatibilityOptions::default(),
    );
    push_icu_diagnostics_for_compiled_message(report.diagnostics, target, artifact);
}

fn push_icu_diagnostics_for_compiled_message(
    diagnostics: Vec<ferrocat_icu::IcuDiagnostic>,
    target: CompiledIcuDiagnosticTarget<'_>,
    artifact: &mut CompiledCatalogArtifact,
) {
    for diagnostic in diagnostics {
        artifact.diagnostics.push(CompiledCatalogDiagnostic {
            severity: icu_diagnostic_severity(diagnostic.severity),
            code: diagnostic.code,
            message: diagnostic.message,
            key: target.compiled_key.to_owned(),
            msgid: target.source_key.msgid.clone(),
            msgctxt: target.source_key.msgctxt.clone(),
            locale: target.locale.to_owned(),
        });
    }
}

const fn icu_diagnostic_severity(severity: IcuDiagnosticSeverity) -> DiagnosticSeverity {
    match severity {
        IcuDiagnosticSeverity::Info => DiagnosticSeverity::Info,
        IcuDiagnosticSeverity::Warning => DiagnosticSeverity::Warning,
        IcuDiagnosticSeverity::Error => DiagnosticSeverity::Error,
    }
}

/// Resolves one runtime message by trying requested locale, configured
/// fallbacks, and finally the source locale when allowed.
fn resolve_compiled_catalog_artifact_message(
    catalogs: &BTreeMap<String, &NormalizedParsedCatalog>,
    source_key: &CatalogMessageKey,
    options: &CompileCatalogArtifactOptions<'_>,
) -> Option<ResolvedArtifactMessage> {
    for locale in std::iter::once(options.requested_locale)
        .chain(options.fallback_chain.iter().map(String::as_str))
    {
        let Some(catalog) = catalogs.get(locale) else {
            continue;
        };
        let Some(message) = catalog.get(source_key) else {
            continue;
        };
        if message.obsolete || !message_has_runtime_translation(message) {
            continue;
        }
        return rendered_compiled_catalog_artifact_message(
            catalog,
            source_key,
            options.source_locale,
            false,
        )
        .map(|message| ResolvedArtifactMessage {
            locale: locale.to_owned(),
            message,
        });
    }

    let should_consult_source =
        options.requested_locale == options.source_locale || options.source_fallback;
    if !should_consult_source {
        return None;
    }

    let catalog = catalogs.get(options.source_locale)?;
    let message = catalog.get(source_key)?;
    if message.obsolete {
        return None;
    }

    rendered_compiled_catalog_artifact_message(catalog, source_key, options.source_locale, true)
        .map(|message| ResolvedArtifactMessage {
            locale: options.source_locale.to_owned(),
            message,
        })
}

/// Renders the final runtime string for one message after translation fallback
/// decisions have already been made.
///
/// Plural messages are re-synthesized into ICU strings so runtime consumers see
/// one uniform message format regardless of the catalog's stored plural encoding.
fn rendered_compiled_catalog_artifact_message(
    catalog: &NormalizedParsedCatalog,
    source_key: &CatalogMessageKey,
    source_locale: &str,
    use_source_fallback: bool,
) -> Option<String> {
    let message = catalog.get(source_key)?;
    let effective = if use_source_fallback {
        catalog.effective_translation_with_source_fallback(source_key, source_locale)?
    } else {
        message.effective_translation_owned()
    };

    match (&message.translation, effective) {
        (TranslationShape::Singular { .. }, EffectiveTranslation::Singular(value)) => Some(value),
        (TranslationShape::Plural { variable, .. }, EffectiveTranslation::Plural(translation)) => {
            Some(synthesize_icu_plural(variable, &translation))
        }
        (TranslationShape::Singular { .. }, EffectiveTranslation::Plural(_))
        | (TranslationShape::Plural { .. }, EffectiveTranslation::Singular(_)) => None,
    }
}

/// Treats an empty singular string or an all-empty plural map as "missing" for
/// runtime artifact purposes.
fn message_has_runtime_translation(message: &CatalogMessage) -> bool {
    match &message.translation {
        TranslationShape::Singular { value } => !value.is_empty(),
        TranslationShape::Plural { translation, .. } => {
            translation.values().any(|value| !value.is_empty())
        }
    }
}

fn validate_compiled_catalog_semantics(
    catalog: &NormalizedParsedCatalog,
    expected: CatalogSemantics,
) -> Result<(), ApiError> {
    let actual = catalog.parsed_catalog().semantics;
    if actual != expected {
        return Err(ApiError::InvalidArguments(format!(
            "compile options requested {:?} semantics, but catalog locale {:?} uses {:?}",
            expected,
            catalog.parsed_catalog().locale,
            actual
        )));
    }
    Ok(())
}

#[cfg(test)]
mod unit_tests {
    use std::collections::BTreeMap;

    use super::{
        ApiError, CatalogMessage, CatalogMessageKey, CatalogSemantics,
        CompiledCatalogTranslationKind, EffectiveTranslation, NormalizedParsedCatalog,
        TranslationShape, collect_compiled_catalog_artifact_source_keys,
        compiled_catalog_artifact_catalogs_contain_key,
        compiled_catalog_translation_kind_for_message, compiled_translation_for_message,
        describe_compiled_id_catalogs, message_has_runtime_translation,
        prepare_compiled_catalog_artifact_catalogs, rendered_compiled_catalog_artifact_message,
        validate_compiled_catalog_semantics,
    };
    use crate::ParsedCatalog;
    use crate::api::PluralSource;

    fn normalized_catalog(
        locale: Option<&str>,
        semantics: CatalogSemantics,
        messages: Vec<CatalogMessage>,
    ) -> NormalizedParsedCatalog {
        NormalizedParsedCatalog::new(ParsedCatalog {
            locale: locale.map(str::to_owned),
            semantics,
            headers: BTreeMap::new(),
            messages,
            diagnostics: Vec::new(),
        })
        .expect("normalized catalog")
    }

    fn singular_message(msgid: &str, value: &str) -> CatalogMessage {
        CatalogMessage {
            msgid: msgid.to_owned(),
            msgctxt: None,
            translation: TranslationShape::Singular {
                value: value.to_owned(),
            },
            comments: Vec::new(),
            origin: crate::PoVec::new(),
            obsolete: false,
            machine: None,
        }
    }

    fn plural_message(msgid: &str) -> CatalogMessage {
        CatalogMessage {
            msgid: msgid.to_owned(),
            msgctxt: None,
            translation: TranslationShape::Plural {
                source: PluralSource {
                    one: Some("# file".to_owned()),
                    other: "# files".to_owned(),
                },
                translation: BTreeMap::from([
                    ("one".to_owned(), "# Datei".to_owned()),
                    ("other".to_owned(), "# Dateien".to_owned()),
                ]),
                variable: "count".to_owned(),
            },
            comments: Vec::new(),
            origin: crate::PoVec::new(),
            obsolete: false,
            machine: None,
        }
    }

    #[test]
    fn compile_translation_helpers_cover_native_compat_and_mismatch_paths() {
        let plural_message = plural_message("files");
        assert_eq!(
            compiled_catalog_translation_kind_for_message(
                CatalogSemantics::IcuNative,
                &plural_message
            ),
            CompiledCatalogTranslationKind::Singular
        );
        assert_eq!(
            compiled_catalog_translation_kind_for_message(
                CatalogSemantics::GettextCompat,
                &plural_message
            ),
            CompiledCatalogTranslationKind::Plural
        );

        assert!(matches!(
            compiled_translation_for_message(
                &plural_message,
                EffectiveTranslation::Plural(BTreeMap::from([
                    ("one".to_owned(), "# Datei".to_owned()),
                    ("other".to_owned(), "# Dateien".to_owned()),
                ])),
                CatalogSemantics::IcuNative,
            ),
            Some(super::CompiledTranslation::Singular(value))
                if value == "{count, plural, one {# Datei} other {# Dateien}}"
        ));
        assert!(
            compiled_translation_for_message(
                &plural_message,
                EffectiveTranslation::Singular("wrong".to_owned()),
                CatalogSemantics::IcuNative,
            )
            .is_none()
        );
    }

    #[test]
    fn compile_artifact_preparation_rejects_invalid_locale_sets() {
        let de = normalized_catalog(
            Some("de"),
            CatalogSemantics::IcuNative,
            vec![singular_message("Hello", "Hallo")],
        );
        let en = normalized_catalog(
            Some("en"),
            CatalogSemantics::IcuNative,
            vec![singular_message("Hello", "Hello")],
        );
        let compat = normalized_catalog(
            Some("fr"),
            CatalogSemantics::GettextCompat,
            vec![singular_message("Hello", "Bonjour")],
        );

        assert!(matches!(
            prepare_compiled_catalog_artifact_catalogs(
                &[],
                "de",
                "en",
                &[],
                CatalogSemantics::IcuNative,
            ),
            Err(ApiError::InvalidArguments(message))
                if message.contains("at least one catalog")
        ));
        assert!(matches!(
            prepare_compiled_catalog_artifact_catalogs(
                &[&de],
                " ",
                "en",
                &[],
                CatalogSemantics::IcuNative,
            ),
            Err(ApiError::InvalidArguments(message))
                if message.contains("requested_locale")
        ));
        assert!(matches!(
            prepare_compiled_catalog_artifact_catalogs(
                &[&de, &compat],
                "de",
                "en",
                &[],
                CatalogSemantics::IcuNative,
            ),
            Err(ApiError::InvalidArguments(message))
                if message.contains("uses")
        ));
        assert!(matches!(
            prepare_compiled_catalog_artifact_catalogs(
                &[&de, &en],
                "de",
                "en",
                &[String::from("de")],
                CatalogSemantics::IcuNative,
            ),
            Err(ApiError::InvalidArguments(message))
                if message.contains("must not repeat")
        ));
        assert!(matches!(
            prepare_compiled_catalog_artifact_catalogs(
                &[&de, &en],
                "de",
                "en",
                &[String::from("fr")],
                CatalogSemantics::IcuNative,
            ),
            Err(ApiError::InvalidArguments(message))
                if message.contains("was not provided")
        ));
    }

    #[test]
    fn compile_artifact_helper_views_cover_lookup_and_runtime_rendering() {
        let mut obsolete = singular_message("Old", "Alt");
        obsolete.obsolete = true;
        let de = normalized_catalog(
            Some("de"),
            CatalogSemantics::IcuNative,
            vec![
                singular_message("Hello", "Hallo"),
                plural_message("files"),
                obsolete,
            ],
        );
        let en = normalized_catalog(
            Some("en"),
            CatalogSemantics::IcuNative,
            vec![singular_message("Hello", "Hello"), plural_message("files")],
        );
        let locales = BTreeMap::from([("de".to_owned(), &de), ("en".to_owned(), &en)]);

        let source_keys = collect_compiled_catalog_artifact_source_keys(&locales);
        assert!(source_keys.contains(&CatalogMessageKey::new("Hello", None)));
        assert!(!source_keys.contains(&CatalogMessageKey::new("Old", None)));
        assert!(compiled_catalog_artifact_catalogs_contain_key(
            &locales,
            &CatalogMessageKey::new("files", None)
        ));
        assert!(!compiled_catalog_artifact_catalogs_contain_key(
            &locales,
            &CatalogMessageKey::new("missing", None)
        ));

        assert_eq!(
            rendered_compiled_catalog_artifact_message(
                &de,
                &CatalogMessageKey::new("Hello", None),
                "en",
                false,
            ),
            Some("Hallo".to_owned())
        );
        assert_eq!(
            rendered_compiled_catalog_artifact_message(
                &de,
                &CatalogMessageKey::new("files", None),
                "en",
                false,
            ),
            Some("{count, plural, one {# Datei} other {# Dateien}}".to_owned())
        );
        assert!(message_has_runtime_translation(&singular_message(
            "Hello", "Hallo"
        )));
        assert!(!message_has_runtime_translation(&singular_message(
            "Hello", ""
        )));
        assert!(message_has_runtime_translation(&plural_message("files")));
        assert!(validate_compiled_catalog_semantics(&de, CatalogSemantics::IcuNative).is_ok());
    }

    #[test]
    fn describe_compiled_id_catalogs_rejects_missing_empty_and_duplicate_locales() {
        let missing_locale = normalized_catalog(
            None,
            CatalogSemantics::IcuNative,
            vec![singular_message("Hello", "Hallo")],
        );
        let blank_locale = normalized_catalog(
            Some(" "),
            CatalogSemantics::IcuNative,
            vec![singular_message("Hello", "Hallo")],
        );
        let de_one = normalized_catalog(
            Some("de"),
            CatalogSemantics::IcuNative,
            vec![singular_message("Hello", "Hallo")],
        );
        let de_two = normalized_catalog(
            Some("de"),
            CatalogSemantics::IcuNative,
            vec![singular_message("Bye", "Tschuess")],
        );

        assert!(describe_compiled_id_catalogs(&[&missing_locale]).is_err());
        assert!(describe_compiled_id_catalogs(&[&blank_locale]).is_err());
        assert!(describe_compiled_id_catalogs(&[&de_one, &de_two]).is_err());
    }
}
