use ferrocat_icu::{IcuPseudolocalizationOptions, pseudolocalize_icu};

use super::{ApiError, CompiledCatalogArtifact};

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
    options: &IcuPseudolocalizationOptions<'_>,
) -> Result<CompiledCatalogArtifact, ApiError> {
    let mut pseudolocalized = artifact.clone();
    for message in pseudolocalized.messages.values_mut() {
        *message = pseudolocalize_icu(message, options).map_err(|error| {
            ApiError::Unsupported(format!(
                "compiled catalog artifact message cannot be pseudolocalized: {error}"
            ))
        })?;
    }
    Ok(pseudolocalized)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ferrocat_icu::IcuPseudolocalizationOptions;

    use super::pseudolocalize_compiled_catalog_artifact;
    use crate::api::{CatalogMessageKey, CompiledCatalogArtifact, CompiledCatalogMissingMessage};

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
            &IcuPseudolocalizationOptions::new().with_expansion_percent(0),
        )
        .expect("pseudolocalize artifact");

        assert_eq!(
            pseudolocalized.messages["runtime-key"],
            "[!! Ĥéļļö {name} !!]"
        );
        assert_eq!(pseudolocalized.missing, artifact.missing);
        assert_eq!(pseudolocalized.diagnostics, artifact.diagnostics);
    }
}
