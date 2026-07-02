use std::collections::BTreeMap;

use ferrocat_icu::{IcuFormatter, IcuFormatterSupport};

use super::{
    ApiError, CatalogMessageKey, CatalogSemantics, IcuSyntaxPolicy, NormalizedParsedCatalog,
    compile::{
        compiled_catalog_translation_kind_for_message, compiled_key_for,
        describe_compiled_id_catalogs,
    },
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// JSON schema version emitted by [`CompiledCatalogArtifact`] serialization.
pub const COMPILED_CATALOG_ARTIFACT_SCHEMA_VERSION: u16 = 1;

/// Translation value stored in a compiled runtime catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "kind", content = "value", rename_all = "snake_case")
)]
pub enum CompiledTranslation {
    /// Singular runtime value.
    Singular(String),
    /// Structured plural runtime value.
    Plural(BTreeMap<String, String>),
}

/// Built-in key strategy used when compiling runtime catalogs.
///
/// This enum is non-exhaustive so additional stable key strategies can be added
/// without breaking downstream matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum CompiledKeyStrategy {
    /// `ferrocat` v1 key format: SHA-256 over a versioned, length-delimited
    /// `msgctxt`/`msgid` payload, truncated to 64 bits and encoded as unpadded
    /// `Base64URL`.
    #[default]
    FerrocatV1,
}

/// Callback used to validate runtime support for ICU formatters.
///
/// The callback receives each formatter discovered in a final runtime ICU
/// message and returns whether that runtime supports the formatter kind and
/// style.
///
/// This is intentionally a non-capturing function pointer so ICU options stay
/// cheap to copy.
pub type IcuFormatterSupportPolicy = fn(&IcuFormatter) -> IcuFormatterSupport;

/// Options controlling runtime catalog compilation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CompileCatalogOptions<'a> {
    /// Built-in strategy used to derive stable runtime keys.
    pub key_strategy: CompiledKeyStrategy,
    /// Whether empty source-locale values should be filled from the source text.
    pub source_fallback: bool,
    /// Source locale used when `source_fallback` is enabled.
    pub source_locale: Option<&'a str>,
    /// High-level semantics used by the input catalog set.
    pub semantics: CatalogSemantics,
}

impl Default for CompileCatalogOptions<'_> {
    fn default() -> Self {
        Self {
            key_strategy: CompiledKeyStrategy::FerrocatV1,
            source_fallback: false,
            source_locale: None,
            semantics: CatalogSemantics::IcuNative,
        }
    }
}

impl<'a> CompileCatalogOptions<'a> {
    /// Creates runtime catalog compile options with default behavior.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns options that derive runtime keys with the given strategy.
    #[must_use]
    pub fn with_key_strategy(mut self, key_strategy: CompiledKeyStrategy) -> Self {
        self.key_strategy = key_strategy;
        self
    }

    /// Returns options that enable or disable source-locale fallback.
    #[must_use]
    pub fn with_source_fallback(mut self, source_fallback: bool) -> Self {
        self.source_fallback = source_fallback;
        self
    }

    /// Returns options that use the given source locale for source fallback.
    #[must_use]
    pub fn with_source_locale(mut self, source_locale: &'a str) -> Self {
        self.source_locale = Some(source_locale);
        self
    }

    /// Returns options that interpret input catalogs with the given semantics.
    #[must_use]
    pub fn with_semantics(mut self, semantics: CatalogSemantics) -> Self {
        self.semantics = semantics;
        self
    }
}

/// Options controlling high-level compiled catalog artifact generation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CompileCatalogArtifactOptions<'a> {
    /// Locale for which the runtime artifact should be produced.
    pub requested_locale: &'a str,
    /// Source locale used for explicit source fallback behavior.
    pub source_locale: &'a str,
    /// Ordered fallback locales consulted after the requested locale.
    pub fallback_chain: &'a [&'a str],
    /// Built-in strategy used to derive stable runtime keys.
    pub key_strategy: CompiledKeyStrategy,
    /// Whether source text should be used when no non-source translation exists.
    pub source_fallback: bool,
    /// Whether invalid final ICU messages should fail compilation instead of producing diagnostics.
    pub strict_icu: bool,
    /// Whether final ICU messages should be checked against source ICU structure.
    pub icu_compatibility: bool,
    /// High-level semantics used by the input catalog set.
    pub semantics: CatalogSemantics,
}

impl<'a> CompileCatalogArtifactOptions<'a> {
    /// Creates artifact compile options with required locales set.
    ///
    /// Optional fields default to an empty fallback chain, `FerrocatV1` keys,
    /// no source fallback, non-strict ICU diagnostics, no ICU compatibility
    /// check, and ICU-native semantics.
    #[must_use]
    pub fn new(requested_locale: &'a str, source_locale: &'a str) -> Self {
        Self {
            requested_locale,
            source_locale,
            fallback_chain: &[],
            key_strategy: CompiledKeyStrategy::FerrocatV1,
            source_fallback: false,
            strict_icu: false,
            icu_compatibility: false,
            semantics: CatalogSemantics::IcuNative,
        }
    }

    /// Returns options that compile the given requested locale.
    #[must_use]
    pub fn with_requested_locale(mut self, requested_locale: &'a str) -> Self {
        self.requested_locale = requested_locale;
        self
    }

    /// Returns options that use the given source locale.
    #[must_use]
    pub fn with_source_locale(mut self, source_locale: &'a str) -> Self {
        self.source_locale = source_locale;
        self
    }

    /// Returns options that consult the given ordered fallback locales.
    #[must_use]
    pub fn with_fallback_chain(mut self, fallback_chain: &'a [&'a str]) -> Self {
        self.fallback_chain = fallback_chain;
        self
    }

    /// Returns options that derive runtime keys with the given strategy.
    #[must_use]
    pub fn with_key_strategy(mut self, key_strategy: CompiledKeyStrategy) -> Self {
        self.key_strategy = key_strategy;
        self
    }

    /// Returns options that enable or disable source-text fallback.
    #[must_use]
    pub fn with_source_fallback(mut self, source_fallback: bool) -> Self {
        self.source_fallback = source_fallback;
        self
    }

    /// Returns options that enable or disable hard errors for invalid final ICU messages.
    #[must_use]
    pub fn with_strict_icu(mut self, strict_icu: bool) -> Self {
        self.strict_icu = strict_icu;
        self
    }

    /// Returns options that enable or disable final ICU compatibility checks.
    #[must_use]
    pub fn with_icu_compatibility(mut self, icu_compatibility: bool) -> Self {
        self.icu_compatibility = icu_compatibility;
        self
    }

    /// Returns options that interpret input catalogs with the given semantics.
    #[must_use]
    pub fn with_semantics(mut self, semantics: CatalogSemantics) -> Self {
        self.semantics = semantics;
        self
    }
}

/// ICU-specific options used while compiling catalog artifacts.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct CompileCatalogArtifactIcuOptions {
    /// ICU parser behavior used for final runtime message validation.
    pub syntax_policy: IcuSyntaxPolicy,
    /// Optional runtime support policy for ICU formatter kinds and styles.
    pub formatter_support: Option<IcuFormatterSupportPolicy>,
}

impl CompileCatalogArtifactIcuOptions {
    /// Creates artifact ICU options with default strict parser behavior.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns options that parse messages with the given ICU syntax policy.
    #[must_use]
    pub fn with_syntax_policy(mut self, syntax_policy: IcuSyntaxPolicy) -> Self {
        self.syntax_policy = syntax_policy;
        self
    }

    /// Returns options that validate formatter support with the given callback.
    #[must_use]
    pub fn with_formatter_support(mut self, formatter_support: IcuFormatterSupportPolicy) -> Self {
        self.formatter_support = Some(formatter_support);
        self
    }
}

/// Options controlling selected-subset compiled catalog artifact generation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CompileSelectedCatalogArtifactOptions<'a> {
    /// Requested compiled runtime IDs to include in the artifact.
    pub compiled_ids: &'a [&'a str],
    /// Shared artifact compile options applied to the selected IDs.
    pub options: CompileCatalogArtifactOptions<'a>,
}

impl<'a> CompileSelectedCatalogArtifactOptions<'a> {
    /// Creates selected artifact compile options with required locales and IDs set.
    ///
    /// Optional fields on the nested artifact options use
    /// [`CompileCatalogArtifactOptions::new`] defaults.
    #[must_use]
    pub fn new(
        requested_locale: &'a str,
        source_locale: &'a str,
        compiled_ids: &'a [&'a str],
    ) -> Self {
        Self {
            compiled_ids,
            options: CompileCatalogArtifactOptions::new(requested_locale, source_locale),
        }
    }

    /// Returns options that include the given compiled runtime IDs.
    #[must_use]
    pub fn with_compiled_ids(mut self, compiled_ids: &'a [&'a str]) -> Self {
        self.compiled_ids = compiled_ids;
        self
    }

    /// Returns options that use the given shared artifact compile options.
    #[must_use]
    pub fn with_options(mut self, options: CompileCatalogArtifactOptions<'a>) -> Self {
        self.options = options;
        self
    }
}

/// Message selection for [`super::compile_catalog_artifact_report`].
#[derive(Debug, Clone, Copy)]
pub enum CompileCatalogArtifactReportSelection<'a> {
    /// Compile and report every non-obsolete source identity available in the catalog set.
    All,
    /// Compile and report only the requested compiled runtime IDs.
    Selected {
        /// Stable ID index used to map compiled IDs back to source identities.
        index: &'a CompiledCatalogIdIndex,
        /// Requested compiled runtime IDs to include in the artifact and provenance report.
        compiled_ids: &'a [&'a str],
    },
}

/// Options controlling compiled artifact generation with a sibling provenance report.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CompileCatalogArtifactReportOptions<'a> {
    /// Shared artifact compile options applied to the generated artifact.
    pub options: CompileCatalogArtifactOptions<'a>,
    /// ICU-specific options applied while validating final runtime messages.
    pub icu_options: CompileCatalogArtifactIcuOptions,
    /// Source identity selection for the generated artifact and provenance report.
    pub selection: CompileCatalogArtifactReportSelection<'a>,
}

impl<'a> CompileCatalogArtifactReportOptions<'a> {
    /// Creates report compile options for every non-obsolete source identity.
    #[must_use]
    pub fn new(requested_locale: &'a str, source_locale: &'a str) -> Self {
        Self {
            options: CompileCatalogArtifactOptions::new(requested_locale, source_locale),
            icu_options: CompileCatalogArtifactIcuOptions::new(),
            selection: CompileCatalogArtifactReportSelection::All,
        }
    }

    /// Creates report compile options for a selected subset of compiled runtime IDs.
    #[must_use]
    pub fn selected(
        requested_locale: &'a str,
        source_locale: &'a str,
        index: &'a CompiledCatalogIdIndex,
        compiled_ids: &'a [&'a str],
    ) -> Self {
        Self {
            options: CompileCatalogArtifactOptions::new(requested_locale, source_locale),
            icu_options: CompileCatalogArtifactIcuOptions::new(),
            selection: CompileCatalogArtifactReportSelection::Selected {
                index,
                compiled_ids,
            },
        }
    }

    /// Returns report options that use the given shared artifact compile options.
    #[must_use]
    pub fn with_options(mut self, options: CompileCatalogArtifactOptions<'a>) -> Self {
        self.options = options;
        self
    }

    /// Returns report options that use the given ICU validation options.
    #[must_use]
    pub fn with_icu_options(mut self, icu_options: CompileCatalogArtifactIcuOptions) -> Self {
        self.icu_options = icu_options;
        self
    }

    /// Returns report options that use the given source identity selection.
    #[must_use]
    pub fn with_selection(mut self, selection: CompileCatalogArtifactReportSelection<'a>) -> Self {
        self.selection = selection;
        self
    }
}

/// High-level translation kind associated with a compiled runtime ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum CompiledCatalogTranslationKind {
    /// Translation is a single string value.
    Singular,
    /// Translation is a plural/category map.
    Plural,
}

/// A compiled runtime message keyed by a derived lookup key.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CompiledMessage {
    /// Stable runtime key derived from the source identity.
    pub key: String,
    /// Original gettext identity preserved for diagnostics and tooling.
    pub source_key: CatalogMessageKey,
    /// Materialized translation payload for runtime lookup.
    pub translation: CompiledTranslation,
}

/// Runtime-oriented lookup structure compiled from a normalized catalog.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CompiledCatalog {
    pub(super) entries: BTreeMap<String, CompiledMessage>,
}

impl CompiledCatalog {
    /// Returns the compiled message for `key`, if present.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&CompiledMessage> {
        self.entries.get(key)
    }

    /// Returns the number of compiled entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` when the compiled catalog has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterates over compiled entries in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &CompiledMessage)> + '_ {
        self.entries
            .iter()
            .map(|(key, message)| (key.as_str(), message))
    }
}

/// Stable compiled runtime ID index built from one or more normalized catalogs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CompiledCatalogIdIndex {
    pub(super) ids: BTreeMap<String, CatalogMessageKey>,
}

/// Metadata describing one compiled runtime ID for a specific catalog set.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CompiledCatalogIdDescription {
    /// Stable runtime ID derived from the source identity.
    pub compiled_id: String,
    /// Original gettext identity preserved for diagnostics and tooling.
    pub source_key: CatalogMessageKey,
    /// Locales from the provided catalog set that contain this non-obsolete message.
    pub available_locales: Vec<String>,
    /// Whether the message is singular or plural in the provided catalog set.
    pub translation_kind: CompiledCatalogTranslationKind,
}

/// Report returned by [`CompiledCatalogIdIndex::describe_compiled_ids`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DescribeCompiledIdsReport {
    /// Metadata for requested IDs that were known to the index and present in the provided catalogs.
    pub described: Vec<CompiledCatalogIdDescription>,
    /// Requested compiled IDs that were not known to the index at all.
    pub unknown_compiled_ids: Vec<String>,
    /// Requested compiled IDs that were known to the index but not present in the provided catalogs.
    pub unavailable_compiled_ids: Vec<CompiledCatalogUnavailableId>,
}

/// Known compiled runtime ID that was not present in the provided catalog set.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CompiledCatalogUnavailableId {
    /// Stable runtime ID derived from the source identity.
    pub compiled_id: String,
    /// Original gettext identity preserved for diagnostics and tooling.
    pub source_key: CatalogMessageKey,
}

impl CompiledCatalogIdIndex {
    /// Builds a deterministic compiled-ID index for the union of non-obsolete messages.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Conflict`] when two different source identities compile to the same ID.
    pub fn new(
        catalogs: &[&NormalizedParsedCatalog],
        key_strategy: CompiledKeyStrategy,
    ) -> Result<Self, ApiError> {
        Self::new_with_key_generator(catalogs, key_strategy, compiled_key_for)
    }

    pub(super) fn new_with_key_generator<F>(
        catalogs: &[&NormalizedParsedCatalog],
        key_strategy: CompiledKeyStrategy,
        mut key_generator: F,
    ) -> Result<Self, ApiError>
    where
        F: FnMut(CompiledKeyStrategy, &CatalogMessageKey) -> String,
    {
        let mut ids = BTreeMap::<String, CatalogMessageKey>::new();

        for catalog in catalogs {
            for (source_key, message) in catalog.iter() {
                if message.obsolete.is_some() {
                    continue;
                }
                let compiled_id = key_generator(key_strategy, source_key);
                if let Some(existing) = ids.get(&compiled_id) {
                    if existing != source_key {
                        return Err(ApiError::Conflict(format!(
                            "compiled catalog key collision for {:?} / {:?} and {:?} / {:?} using key {}",
                            existing.msgctxt,
                            existing.msgid,
                            source_key.msgctxt,
                            source_key.msgid,
                            compiled_id
                        )));
                    }
                    continue;
                }
                ids.insert(compiled_id, source_key.clone());
            }
        }

        Ok(Self { ids })
    }

    /// Returns the source key for `compiled_id`, if present.
    #[must_use]
    pub fn get(&self, compiled_id: &str) -> Option<&CatalogMessageKey> {
        self.ids.get(compiled_id)
    }

    /// Returns `true` when the index contains `compiled_id`.
    #[must_use]
    pub fn contains_id(&self, compiled_id: &str) -> bool {
        self.ids.contains_key(compiled_id)
    }

    /// Returns the number of indexed compiled IDs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Returns `true` when the index contains no compiled IDs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Iterates over compiled IDs in sorted order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &CatalogMessageKey)> + '_ {
        self.ids
            .iter()
            .map(|(compiled_id, source_key)| (compiled_id.as_str(), source_key))
    }

    /// Returns the underlying ordered compiled-ID map by reference.
    #[must_use]
    pub fn as_btreemap(&self) -> &BTreeMap<String, CatalogMessageKey> {
        &self.ids
    }

    /// Consumes the index and returns the underlying ordered compiled-ID map.
    #[must_use]
    pub fn into_btreemap(self) -> BTreeMap<String, CatalogMessageKey> {
        self.ids
    }

    /// Describes selected compiled IDs against a provided catalog set.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::InvalidArguments`] when a provided catalog does not declare
    /// a locale, or [`ApiError::Conflict`] when the same compiled ID maps to different
    /// translation kinds across the provided catalogs.
    pub fn describe_compiled_ids(
        &self,
        catalogs: &[&NormalizedParsedCatalog],
        compiled_ids: &[&str],
    ) -> Result<DescribeCompiledIdsReport, ApiError> {
        let locales = describe_compiled_id_catalogs(catalogs)?;
        let mut report = DescribeCompiledIdsReport::default();

        for compiled_id in std::collections::BTreeSet::from_iter(compiled_ids.iter().copied()) {
            let Some(source_key) = self.get(compiled_id).cloned() else {
                report.unknown_compiled_ids.push(compiled_id.to_owned());
                continue;
            };

            let mut available_locales = Vec::new();
            let mut translation_kind = None;

            for (locale, catalog) in &locales {
                let Some(message) = catalog.get(&source_key) else {
                    continue;
                };
                if message.obsolete.is_some() {
                    continue;
                }
                let next_kind = compiled_catalog_translation_kind_for_message(
                    catalog.parsed_catalog().semantics,
                    message,
                );
                if let Some(existing_kind) = translation_kind {
                    if existing_kind != next_kind {
                        return Err(ApiError::Conflict(format!(
                            "compiled ID {:?} resolves to inconsistent translation shapes across the provided catalogs",
                            compiled_id
                        )));
                    }
                } else {
                    translation_kind = Some(next_kind);
                }
                available_locales.push(locale.clone());
            }

            if let Some(translation_kind) = translation_kind {
                report.described.push(CompiledCatalogIdDescription {
                    compiled_id: compiled_id.to_owned(),
                    source_key,
                    available_locales,
                    translation_kind,
                });
            } else {
                report
                    .unavailable_compiled_ids
                    .push(CompiledCatalogUnavailableId {
                        compiled_id: compiled_id.to_owned(),
                        source_key,
                    });
            }
        }

        Ok(report)
    }
}

/// Host-neutral compiled runtime artifact for one requested locale.
///
/// When the `serde` feature is enabled, this type serializes with
/// [`COMPILED_CATALOG_ARTIFACT_SCHEMA_VERSION`] as a required
/// `schema_version` field. Deserialization rejects unknown artifact schema
/// versions.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompiledCatalogArtifact {
    /// Final runtime message map keyed by the derived lookup key.
    pub messages: BTreeMap<String, String>,
    /// Messages that were missing from the requested locale and had to fall back.
    pub missing: Vec<CompiledCatalogMissingMessage>,
    /// Diagnostics collected while validating final runtime messages.
    pub diagnostics: Vec<CompiledCatalogDiagnostic>,
}

/// Result returned by [`super::compile_catalog_artifact_report`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CompiledCatalogArtifactReport {
    /// Host-neutral runtime artifact produced by the same compile path as
    /// [`super::compile_catalog_artifact`].
    pub artifact: CompiledCatalogArtifact,
    /// Sibling report describing how each compiled message resolved.
    pub provenance: CompiledCatalogProvenanceReport,
}

/// Provenance metadata for one compiled requested-locale artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CompiledCatalogProvenanceReport {
    /// Requested locale used for artifact compilation.
    pub requested_locale: String,
    /// Source locale used for explicit source fallback behavior.
    pub source_locale: String,
    /// Ordered fallback locales configured for this compile request.
    pub fallback_chain: Vec<String>,
    /// Per-message resolution rows in the same deterministic source-key order as compilation.
    pub messages: Vec<CompiledCatalogResolution>,
}

/// Provenance row for one compiled runtime message identity.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CompiledCatalogResolution {
    /// Stable runtime key derived from the source identity.
    pub key: String,
    /// Original gettext identity preserved for diagnostics and tooling.
    pub source_key: CatalogMessageKey,
    /// Locale that ultimately provided the runtime value, if any.
    pub resolved_locale: Option<String>,
    /// Resolution category for this message.
    pub kind: CompiledCatalogResolutionKind,
}

/// How one compiled runtime message resolved for a requested-locale artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum CompiledCatalogResolutionKind {
    /// The requested locale provided the final runtime message.
    Requested,
    /// A configured non-source fallback locale provided the final runtime message.
    Fallback,
    /// The source locale provided the final runtime message through source fallback.
    SourceFallback,
    /// No locale provided a final runtime message.
    Unresolved,
}

#[cfg(feature = "serde")]
#[derive(Serialize)]
struct CompiledCatalogArtifactWireRef<'a> {
    schema_version: u16,
    messages: &'a BTreeMap<String, String>,
    missing: &'a [CompiledCatalogMissingMessage],
    diagnostics: &'a [CompiledCatalogDiagnostic],
}

#[cfg(feature = "serde")]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CompiledCatalogArtifactWire {
    schema_version: u16,
    #[serde(default)]
    messages: BTreeMap<String, String>,
    #[serde(default)]
    missing: Vec<CompiledCatalogMissingMessage>,
    #[serde(default)]
    diagnostics: Vec<CompiledCatalogDiagnostic>,
}

#[cfg(feature = "serde")]
impl Serialize for CompiledCatalogArtifact {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        CompiledCatalogArtifactWireRef {
            schema_version: COMPILED_CATALOG_ARTIFACT_SCHEMA_VERSION,
            messages: &self.messages,
            missing: &self.missing,
            diagnostics: &self.diagnostics,
        }
        .serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for CompiledCatalogArtifact {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = CompiledCatalogArtifactWire::deserialize(deserializer)?;
        if wire.schema_version != COMPILED_CATALOG_ARTIFACT_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(format!(
                "unsupported compiled catalog artifact schema_version {}; expected {}",
                wire.schema_version, COMPILED_CATALOG_ARTIFACT_SCHEMA_VERSION
            )));
        }

        Ok(Self {
            messages: wire.messages,
            missing: wire.missing,
            diagnostics: wire.diagnostics,
        })
    }
}

/// Missing-message record emitted by [`super::compile_catalog_artifact`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CompiledCatalogMissingMessage {
    /// Stable runtime key derived from the source identity.
    pub key: String,
    /// Original gettext identity preserved for diagnostics and tooling.
    pub source_key: CatalogMessageKey,
    /// Requested locale for this artifact compilation.
    pub requested_locale: String,
    /// Locale that ultimately provided the runtime value, if any.
    pub resolved_locale: Option<String>,
}

/// Diagnostic emitted by [`super::compile_catalog_artifact`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CompiledCatalogDiagnostic {
    /// Severity for the collected diagnostic.
    pub severity: super::DiagnosticSeverity,
    /// Stable machine-readable diagnostic code.
    pub code: String,
    /// Human-readable explanation of the problem.
    pub message: String,
    /// Stable runtime key derived from the source identity.
    pub key: String,
    /// Source `msgid` associated with the diagnostic.
    pub msgid: String,
    /// Source `msgctxt` associated with the diagnostic.
    pub msgctxt: Option<String>,
    /// Locale whose final runtime message produced the diagnostic.
    pub locale: String,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        COMPILED_CATALOG_ARTIFACT_SCHEMA_VERSION, CompileCatalogArtifactIcuOptions,
        CompileCatalogArtifactOptions, CompileCatalogArtifactReportOptions,
        CompileCatalogArtifactReportSelection, CompileCatalogOptions,
        CompileSelectedCatalogArtifactOptions, CompiledCatalogArtifact, CompiledCatalogDiagnostic,
        CompiledCatalogIdIndex, CompiledCatalogMissingMessage, CompiledKeyStrategy,
    };
    use crate::api::{CatalogMessageKey, CatalogSemantics, DiagnosticSeverity, IcuSyntaxPolicy};

    #[test]
    fn compile_option_constructors_set_required_fields_and_keep_defaults() {
        let compile = CompileCatalogOptions::new();
        assert_eq!(compile.key_strategy, CompiledKeyStrategy::FerrocatV1);
        assert!(!compile.source_fallback);

        let artifact = CompileCatalogArtifactOptions::new("de", "en");
        assert_eq!(artifact.requested_locale, "de");
        assert_eq!(artifact.source_locale, "en");
        assert_eq!(artifact.key_strategy, CompiledKeyStrategy::FerrocatV1);
        assert_eq!(artifact.semantics, CatalogSemantics::IcuNative);

        let selected_ids = ["abc123"];
        let selected = CompileSelectedCatalogArtifactOptions::new("de", "en", &selected_ids);
        assert_eq!(selected.options.requested_locale, "de");
        assert_eq!(selected.options.source_locale, "en");
        assert_eq!(selected.compiled_ids, selected_ids.as_slice());
        assert_eq!(
            selected.options.key_strategy,
            CompiledKeyStrategy::FerrocatV1
        );

        let report = CompileCatalogArtifactReportOptions::new("de", "en");
        assert_eq!(report.options.requested_locale, "de");
        assert_eq!(report.options.source_locale, "en");
        assert!(matches!(
            report.selection,
            CompileCatalogArtifactReportSelection::All
        ));
    }

    #[test]
    fn compile_option_builders_set_fields() {
        let compile = CompileCatalogOptions::new()
            .with_key_strategy(CompiledKeyStrategy::FerrocatV1)
            .with_source_fallback(true)
            .with_source_locale("en")
            .with_semantics(CatalogSemantics::GettextCompat);

        assert_eq!(compile.key_strategy, CompiledKeyStrategy::FerrocatV1);
        assert!(compile.source_fallback);
        assert_eq!(compile.source_locale, Some("en"));
        assert_eq!(compile.semantics, CatalogSemantics::GettextCompat);

        let fallback_chain = ["fr", "en"];
        let artifact = CompileCatalogArtifactOptions::new("de", "en")
            .with_requested_locale("fr")
            .with_source_locale("en-US")
            .with_fallback_chain(&fallback_chain)
            .with_key_strategy(CompiledKeyStrategy::FerrocatV1)
            .with_source_fallback(true)
            .with_strict_icu(true)
            .with_icu_compatibility(true)
            .with_semantics(CatalogSemantics::GettextCompat);

        assert_eq!(artifact.requested_locale, "fr");
        assert_eq!(artifact.source_locale, "en-US");
        assert_eq!(artifact.fallback_chain, fallback_chain.as_slice());
        assert!(artifact.source_fallback);
        assert!(artifact.strict_icu);
        assert!(artifact.icu_compatibility);
        assert_eq!(artifact.semantics, CatalogSemantics::GettextCompat);

        let selected_ids = ["id-1"];
        let other_ids = ["id-2"];
        let selected = CompileSelectedCatalogArtifactOptions::new("de", "en", &selected_ids)
            .with_compiled_ids(&other_ids)
            .with_options(artifact.clone());

        assert_eq!(selected.compiled_ids, other_ids.as_slice());
        assert_eq!(selected.options, artifact);

        let index = CompiledCatalogIdIndex::default();
        let selection = CompileCatalogArtifactReportSelection::Selected {
            index: &index,
            compiled_ids: &other_ids,
        };
        let report = CompileCatalogArtifactReportOptions::new("de", "en")
            .with_options(artifact.clone())
            .with_icu_options(
                CompileCatalogArtifactIcuOptions::new()
                    .with_syntax_policy(IcuSyntaxPolicy::RuntimeLiteralApostrophes),
            )
            .with_selection(selection);

        assert_eq!(report.options, artifact);
        assert_eq!(
            report.icu_options.syntax_policy,
            IcuSyntaxPolicy::RuntimeLiteralApostrophes
        );
        assert!(matches!(
            report.selection,
            CompileCatalogArtifactReportSelection::Selected { compiled_ids, .. }
                if compiled_ids == other_ids.as_slice()
        ));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn compiled_catalog_artifact_serde_uses_versioned_wire_contract() {
        let artifact = CompiledCatalogArtifact {
            messages: BTreeMap::from([("runtime-key".to_owned(), "Hallo".to_owned())]),
            missing: vec![CompiledCatalogMissingMessage {
                key: "runtime-key".to_owned(),
                source_key: CatalogMessageKey::new("Hello", None),
                requested_locale: "de".to_owned(),
                resolved_locale: Some("en".to_owned()),
            }],
            diagnostics: vec![CompiledCatalogDiagnostic {
                severity: DiagnosticSeverity::Warning,
                code: "icu.syntax".to_owned(),
                message: "invalid ICU message".to_owned(),
                key: "runtime-key".to_owned(),
                msgid: "Hello".to_owned(),
                msgctxt: None,
                locale: "de".to_owned(),
            }],
        };

        let json = serde_json::to_value(&artifact).expect("artifact serialization must succeed");
        assert_eq!(
            json["schema_version"],
            COMPILED_CATALOG_ARTIFACT_SCHEMA_VERSION
        );
        assert_eq!(json["diagnostics"][0]["severity"], "warning");

        let roundtrip: CompiledCatalogArtifact =
            serde_json::from_value(json).expect("artifact deserialization must succeed");
        assert_eq!(roundtrip, artifact);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn compiled_catalog_artifact_serde_rejects_unknown_schema_version() {
        let json = serde_json::json!({
            "schema_version": COMPILED_CATALOG_ARTIFACT_SCHEMA_VERSION + 1,
            "messages": {},
            "missing": [],
            "diagnostics": [],
        });

        let error = serde_json::from_value::<CompiledCatalogArtifact>(json)
            .expect_err("unknown artifact schema versions must be rejected");
        assert!(
            error
                .to_string()
                .contains("unsupported compiled catalog artifact schema_version")
        );
    }
}
