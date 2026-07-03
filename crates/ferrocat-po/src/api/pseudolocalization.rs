use ferrocat_icu::{IcuPseudolocalizationOptions, pseudolocalize_icu_message, stringify_icu};

use super::icu_syntax::parse_icu_with_syntax_policy;
use super::{ApiError, CompiledCatalogArtifact, IcuSyntaxPolicy};

/// Options controlling compiled catalog artifact pseudolocalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CompiledCatalogPseudolocalizationOptions<'a> {
    /// ICU-aware text transformation options.
    pub icu_options: IcuPseudolocalizationOptions<'a>,
    /// ICU parser behavior used when reading final runtime messages.
    pub syntax_policy: IcuSyntaxPolicy,
}

impl Default for CompiledCatalogPseudolocalizationOptions<'_> {
    fn default() -> Self {
        Self {
            icu_options: IcuPseudolocalizationOptions::new(),
            syntax_policy: IcuSyntaxPolicy::Strict,
        }
    }
}

impl<'a> CompiledCatalogPseudolocalizationOptions<'a> {
    /// Creates artifact pseudolocalization options with the default visible wrapper.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns options that use the given ICU-aware text transformation settings.
    #[must_use]
    pub fn with_icu_options(mut self, icu_options: IcuPseudolocalizationOptions<'a>) -> Self {
        self.icu_options = icu_options;
        self
    }

    /// Returns options that parse runtime messages with the given ICU syntax policy.
    #[must_use]
    pub fn with_syntax_policy(mut self, syntax_policy: IcuSyntaxPolicy) -> Self {
        self.syntax_policy = syntax_policy;
        self
    }
}

/// Pseudolocalizes the final runtime messages in a compiled catalog artifact.
///
/// The helper preserves artifact metadata, missing-message records, and
/// diagnostics. Only values in the artifact `messages` map are transformed, and
/// the ICU-aware transform preserves placeholders, selectors, formatter
/// metadata, plural `#` placeholders, and rich-text tag names.
///
/// # Errors
///
/// Returns [`ApiError::Unsupported`] if any runtime message in the artifact
/// cannot be parsed as ICU MessageFormat by Ferrocat's parser.
pub fn pseudolocalize_compiled_catalog_artifact(
    artifact: &CompiledCatalogArtifact,
    options: &CompiledCatalogPseudolocalizationOptions<'_>,
) -> Result<CompiledCatalogArtifact, ApiError> {
    let mut pseudolocalized = artifact.clone();
    for (key, message) in &mut pseudolocalized.messages {
        let parsed =
            parse_icu_with_syntax_policy(message, options.syntax_policy).map_err(|error| {
                ApiError::Unsupported(format!(
                    "compiled catalog artifact message {key:?} cannot be pseudolocalized: {error}"
                ))
            })?;
        *message = stringify_icu(&pseudolocalize_icu_message(&parsed, &options.icu_options));
    }
    Ok(pseudolocalized)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ferrocat_icu::IcuPseudolocalizationOptions;

    use super::{
        CompiledCatalogPseudolocalizationOptions, pseudolocalize_compiled_catalog_artifact,
    };
    use crate::api::{
        ApiError, CatalogMessageKey, CompiledCatalogArtifact, CompiledCatalogMissingMessage,
        IcuSyntaxPolicy,
    };

    #[test]
    fn pseudolocalize_compiled_artifact_transforms_messages_only() {
        let artifact = CompiledCatalogArtifact {
            messages: BTreeMap::from([("runtime-key".to_owned(), "Hello {name}".to_owned())]),
            missing: vec![CompiledCatalogMissingMessage {
                key: "runtime-key".to_owned(),
                source_key: CatalogMessageKey::new("Hello {name}", None),
                requested_locale: "qps".to_owned(),
                resolved_locale: Some("en".to_owned()),
            }],
            diagnostics: Vec::new(),
        };

        let pseudolocalized = pseudolocalize_compiled_catalog_artifact(
            &artifact,
            &CompiledCatalogPseudolocalizationOptions::new()
                .with_icu_options(IcuPseudolocalizationOptions::new().with_expansion_percent(0)),
        )
        .expect("pseudolocalize artifact");

        assert_eq!(
            pseudolocalized.messages["runtime-key"],
            "[!! Ĥéļļö {name} !!]"
        );
        assert_eq!(pseudolocalized.missing, artifact.missing);
        assert_eq!(pseudolocalized.diagnostics, artifact.diagnostics);
    }

    #[test]
    fn pseudolocalize_compiled_artifact_reports_invalid_runtime_messages() {
        let artifact = CompiledCatalogArtifact {
            messages: BTreeMap::from([("runtime-key".to_owned(), "{broken".to_owned())]),
            missing: Vec::new(),
            diagnostics: Vec::new(),
        };

        let error = pseudolocalize_compiled_catalog_artifact(
            &artifact,
            &CompiledCatalogPseudolocalizationOptions::new(),
        )
        .expect_err("invalid runtime message");

        assert!(
            matches!(error, ApiError::Unsupported(message) if message.contains("message \"runtime-key\" cannot be pseudolocalized"))
        );
    }

    #[test]
    fn pseudolocalize_compiled_artifact_uses_runtime_apostrophe_policy() {
        let artifact = CompiledCatalogArtifact {
            messages: BTreeMap::from([("runtime-key".to_owned(), "You're {name}".to_owned())]),
            missing: Vec::new(),
            diagnostics: Vec::new(),
        };

        let pseudolocalized = pseudolocalize_compiled_catalog_artifact(
            &artifact,
            &CompiledCatalogPseudolocalizationOptions::new()
                .with_icu_options(IcuPseudolocalizationOptions::new().with_expansion_percent(0))
                .with_syntax_policy(IcuSyntaxPolicy::RuntimeLiteralApostrophes),
        )
        .expect("pseudolocalize artifact");

        assert!(pseudolocalized.messages["runtime-key"].contains("{name}"));
        assert_ne!(
            pseudolocalized.messages["runtime-key"],
            artifact.messages["runtime-key"]
        );
    }
}
