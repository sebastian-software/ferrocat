use std::collections::BTreeMap;

use super::{
    ApiError, CatalogMessageKey, CatalogSemantics, NormalizedParsedCatalog,
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

/// Options controlling runtime catalog compilation.
#[derive(Debug, Clone, PartialEq, Eq)]
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

impl CompileCatalogOptions<'_> {
    /// Creates runtime catalog compile options with default behavior.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// Options controlling high-level compiled catalog artifact generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileCatalogArtifactOptions<'a> {
    /// Locale for which the runtime artifact should be produced.
    pub requested_locale: &'a str,
    /// Source locale used for explicit source fallback behavior.
    pub source_locale: &'a str,
    /// Ordered fallback locales consulted after the requested locale.
    pub fallback_chain: &'a [String],
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

impl Default for CompileCatalogArtifactOptions<'_> {
    fn default() -> Self {
        Self {
            requested_locale: "",
            source_locale: "",
            fallback_chain: &[],
            key_strategy: CompiledKeyStrategy::FerrocatV1,
            source_fallback: false,
            strict_icu: false,
            icu_compatibility: false,
            semantics: CatalogSemantics::IcuNative,
        }
    }
}

impl<'a> CompileCatalogArtifactOptions<'a> {
    /// Creates artifact compile options with required locales set.
    ///
    /// Optional fields use the same defaults as [`CompileCatalogArtifactOptions::default`].
    #[must_use]
    pub fn new(requested_locale: &'a str, source_locale: &'a str) -> Self {
        Self {
            requested_locale,
            source_locale,
            ..Self::default()
        }
    }
}

/// Options controlling selected-subset compiled catalog artifact generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileSelectedCatalogArtifactOptions<'a> {
    /// Locale for which the runtime artifact should be produced.
    pub requested_locale: &'a str,
    /// Source locale used for explicit source fallback behavior.
    pub source_locale: &'a str,
    /// Ordered fallback locales consulted after the requested locale.
    pub fallback_chain: &'a [String],
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
    /// Requested compiled runtime IDs to include in the artifact.
    pub compiled_ids: &'a [String],
}

impl Default for CompileSelectedCatalogArtifactOptions<'_> {
    fn default() -> Self {
        Self {
            requested_locale: "",
            source_locale: "",
            fallback_chain: &[],
            key_strategy: CompiledKeyStrategy::FerrocatV1,
            source_fallback: false,
            strict_icu: false,
            icu_compatibility: false,
            semantics: CatalogSemantics::IcuNative,
            compiled_ids: &[],
        }
    }
}

impl<'a> CompileSelectedCatalogArtifactOptions<'a> {
    /// Creates selected artifact compile options with required locales and IDs set.
    ///
    /// Optional fields use the same defaults as [`CompileSelectedCatalogArtifactOptions::default`].
    #[must_use]
    pub fn new(
        requested_locale: &'a str,
        source_locale: &'a str,
        compiled_ids: &'a [String],
    ) -> Self {
        Self {
            requested_locale,
            source_locale,
            compiled_ids,
            ..Self::default()
        }
    }

    pub(super) fn artifact_options(&self) -> CompileCatalogArtifactOptions<'_> {
        CompileCatalogArtifactOptions {
            requested_locale: self.requested_locale,
            source_locale: self.source_locale,
            fallback_chain: self.fallback_chain,
            key_strategy: self.key_strategy,
            source_fallback: self.source_fallback,
            strict_icu: self.strict_icu,
            icu_compatibility: self.icu_compatibility,
            semantics: self.semantics,
        }
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
                if message.obsolete {
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
        compiled_ids: &[String],
    ) -> Result<DescribeCompiledIdsReport, ApiError> {
        let locales = describe_compiled_id_catalogs(catalogs)?;
        let mut report = DescribeCompiledIdsReport::default();

        for compiled_id in std::collections::BTreeSet::from_iter(compiled_ids.iter().cloned()) {
            let Some(source_key) = self.get(&compiled_id).cloned() else {
                report.unknown_compiled_ids.push(compiled_id);
                continue;
            };

            let mut available_locales = Vec::new();
            let mut translation_kind = None;

            for (locale, catalog) in &locales {
                let Some(message) = catalog.get(&source_key) else {
                    continue;
                };
                if message.obsolete {
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
                    compiled_id,
                    source_key,
                    available_locales,
                    translation_kind,
                });
            } else {
                report
                    .unavailable_compiled_ids
                    .push(CompiledCatalogUnavailableId {
                        compiled_id,
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
        COMPILED_CATALOG_ARTIFACT_SCHEMA_VERSION, CompileCatalogArtifactOptions,
        CompileCatalogOptions, CompileSelectedCatalogArtifactOptions, CompiledCatalogArtifact,
        CompiledCatalogDiagnostic, CompiledCatalogMissingMessage, CompiledKeyStrategy,
    };
    use crate::api::{CatalogMessageKey, CatalogSemantics, DiagnosticSeverity};

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

        let selected_ids = vec!["abc123".to_owned()];
        let selected = CompileSelectedCatalogArtifactOptions::new("de", "en", &selected_ids);
        assert_eq!(selected.requested_locale, "de");
        assert_eq!(selected.source_locale, "en");
        assert_eq!(selected.compiled_ids, selected_ids.as_slice());
        assert_eq!(selected.key_strategy, CompiledKeyStrategy::FerrocatV1);
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
