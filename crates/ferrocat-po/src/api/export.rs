//! Catalog export helpers for PO and FCL rendering.

use std::collections::{BTreeMap, BTreeSet};

use crate::diagnostic_codes;
use crate::{Header, MsgStr, PoFile, PoItem, PoVec, SerializeOptions, stringify_po};

use super::catalog::{CanonicalMessage, CanonicalTranslation, Catalog};
use super::mt::{
    MachineMetadata, PO_AI_KEY, PO_LOCK_KEY, format_ai_descriptor, machine_translation_hash,
    validate_machine_metadata,
};
use super::plural::{PluralProfile, synthesize_icu_plural};
use super::{
    ApiError, CatalogSemantics, Diagnostic, DiagnosticSeverity, EffectiveTranslationRef,
    PlaceholderCommentMode, PluralSource, UpdateCatalogOptions,
};

pub(super) fn export_catalog_content(
    catalog: &Catalog,
    options: &UpdateCatalogOptions<'_>,
    locale: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<String, ApiError> {
    match options.mode.storage_format() {
        super::CatalogStorageFormat::Po => {
            let file = export_catalog_to_po(catalog, options, locale, diagnostics)?;
            Ok(stringify_po(&file, &SerializeOptions::default()))
        }
        super::CatalogStorageFormat::Fcl => Ok(super::fcl::stringify_catalog_fcl(
            catalog,
            locale,
            options.source_locale,
            &options.render,
        )),
    }
}

fn export_catalog_to_po(
    catalog: &Catalog,
    options: &UpdateCatalogOptions<'_>,
    locale: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<PoFile, ApiError> {
    let mut file = PoFile {
        comments: catalog.file_comments.clone(),
        extracted_comments: catalog.file_extracted_comments.clone(),
        headers: catalog
            .headers
            .iter()
            .map(|(key, value)| Header {
                key: key.clone(),
                value: value.clone(),
            })
            .collect(),
        items: Vec::with_capacity(catalog.messages.len()),
    };

    for message in &catalog.messages {
        file.items
            .push(export_message_to_po(message, options, locale, diagnostics)?);
    }

    Ok(file)
}

fn export_message_to_po(
    message: &CanonicalMessage,
    options: &UpdateCatalogOptions<'_>,
    locale: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<PoItem, ApiError> {
    match &message.translation {
        CanonicalTranslation::Singular { value } => {
            let mut item = base_po_item(message, options, 1);
            item.msgid.clone_from(&message.msgid);
            item.msgstr = MsgStr::from(value.clone());
            Ok(item)
        }
        CanonicalTranslation::Plural {
            source,
            translation_by_category,
            variable,
        } => {
            if options.mode.semantics() == CatalogSemantics::IcuNative {
                let mut item = base_po_item(message, options, 1);
                item.msgid = synthesize_icu_plural(variable, &plural_source_branches(source));
                item.msgstr =
                    MsgStr::from(synthesize_icu_plural(variable, translation_by_category));
                return Ok(item);
            }

            let plural_profile =
                PluralProfile::for_gettext_translation(locale, translation_by_category);
            let nplurals = plural_profile
                .nplurals()
                .max(translation_by_category.len().max(1));
            let mut item = base_po_item(message, options, nplurals);

            if !translation_by_category.contains_key("other") {
                diagnostics.push(
                    Diagnostic::new(
                        DiagnosticSeverity::Error,
                        diagnostic_codes::plural::UNSUPPORTED_GETTEXT_EXPORT,
                        "Plural translation is missing the required \"other\" category.",
                    )
                    .with_identity(&message.msgid, message.msgctxt.as_deref()),
                );
                return Err(ApiError::Unsupported(
                    "plural translation is missing the required \"other\" category".to_owned(),
                ));
            }
            item.msgid = source.one.clone().unwrap_or_else(|| source.other.clone());
            item.msgid_plural = Some(source.other.clone());
            item.msgstr = MsgStr::from(plural_profile.gettext_values(translation_by_category));
            item.nplurals = plural_profile.nplurals();

            Ok(item)
        }
    }
}

fn base_po_item(
    message: &CanonicalMessage,
    options: &UpdateCatalogOptions<'_>,
    nplurals: usize,
) -> PoItem {
    let mut item = PoItem::new(nplurals);
    item.msgctxt.clone_from(&message.msgctxt);
    item.comments = message.translator_comments.iter().cloned().collect();
    item.flags = message.flags.iter().cloned().collect();
    item.obsolete = message.obsolete;
    let mut extracted_comments = message.comments.clone();
    append_placeholder_comments(
        &mut extracted_comments,
        &message.placeholders,
        &options.render.print_placeholders_in_comments,
    );
    item.extracted_comments = extracted_comments.into();
    if let Some(metadata) = valid_machine_metadata(message) {
        item.metadata
            .push((PO_LOCK_KEY.to_owned(), metadata.lock.clone()));
        if let Some(ai) = &metadata.ai {
            item.metadata
                .push((PO_AI_KEY.to_owned(), format_ai_descriptor(ai)));
        }
    }
    item.references = if options.render.include_origins {
        // References are file-only now, so distinct origins can collapse to the
        // same path; keep the first occurrence and drop exact duplicates.
        let mut references: PoVec<String> = PoVec::new();
        for origin in &message.origins {
            if !references.iter().any(|existing| existing == &origin.file) {
                references.push(origin.file.clone());
            }
        }
        references
    } else {
        PoVec::new()
    };
    item
}

fn valid_machine_metadata(message: &CanonicalMessage) -> Option<&MachineMetadata> {
    let metadata = message.machine.as_ref()?;
    if validate_machine_metadata(metadata).is_err() {
        return None;
    }
    (metadata.lock == machine_translation_hash(canonical_translation_ref(&message.translation)))
        .then_some(metadata)
}

fn canonical_translation_ref(translation: &CanonicalTranslation) -> EffectiveTranslationRef<'_> {
    match translation {
        CanonicalTranslation::Singular { value } => EffectiveTranslationRef::Singular(value),
        CanonicalTranslation::Plural {
            translation_by_category,
            ..
        } => EffectiveTranslationRef::Plural(translation_by_category),
    }
}

pub(super) fn plural_source_branches(source: &PluralSource) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    if let Some(one) = &source.one {
        map.insert("one".to_owned(), one.clone());
    }
    map.insert("other".to_owned(), source.other.clone());
    map
}

pub(super) fn append_placeholder_comments(
    comments: &mut Vec<String>,
    placeholders: &BTreeMap<String, Vec<String>>,
    mode: &PlaceholderCommentMode,
) {
    let limit = match mode {
        PlaceholderCommentMode::Disabled => return,
        PlaceholderCommentMode::Enabled { limit } => *limit,
    };

    let mut seen = comments.iter().cloned().collect::<BTreeSet<String>>();

    for (name, values) in placeholders {
        if !name.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        for value in values.iter().take(limit) {
            let comment = format!(
                "placeholder {{{name}}}: {}",
                normalize_placeholder_value(value)
            );
            if seen.insert(comment.clone()) {
                comments.push(comment);
            }
        }
    }
}

fn normalize_placeholder_value(value: &str) -> String {
    value.replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::super::{AiProvenance, CatalogMode, CatalogOrigin, CatalogUpdateInput};
    use super::*;

    fn message_with_translation(translation: CanonicalTranslation) -> CanonicalMessage {
        CanonicalMessage {
            msgid: "items".to_owned(),
            msgctxt: None,
            translation,
            comments: Vec::new(),
            origins: vec![CatalogOrigin {
                file: "src/lib.rs".to_owned(),
            }]
            .into(),
            placeholders: BTreeMap::new(),
            obsolete: false,
            machine: None,
            translator_comments: Vec::new(),
            flags: Vec::new(),
        }
    }

    fn singular_message(value: &str) -> CanonicalMessage {
        message_with_translation(CanonicalTranslation::Singular {
            value: value.to_owned(),
        })
    }

    fn plural_message(translation_by_category: BTreeMap<String, String>) -> CanonicalMessage {
        message_with_translation(CanonicalTranslation::Plural {
            source: PluralSource {
                one: Some("item".to_owned()),
                other: "items".to_owned(),
            },
            translation_by_category,
            variable: "count".to_owned(),
        })
    }

    #[test]
    fn gettext_export_rejects_plural_translation_without_other_category() {
        let catalog = Catalog {
            messages: vec![plural_message(BTreeMap::from([(
                "one".to_owned(),
                "Artikel".to_owned(),
            )]))],
            ..Catalog::default()
        };
        let options = UpdateCatalogOptions {
            mode: CatalogMode::GettextPo,
            ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
        };
        let mut diagnostics = Vec::new();

        let error = export_catalog_content(&catalog, &options, Some("de"), &mut diagnostics)
            .expect_err("missing other category should be rejected");

        assert!(matches!(error, ApiError::Unsupported(message) if message.contains("other")));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].code.as_str(),
            diagnostic_codes::plural::UNSUPPORTED_GETTEXT_EXPORT
        );
    }

    #[test]
    fn base_po_item_renders_origin_without_line_and_can_omit_origins() {
        let message = singular_message("Hallo");
        let options = UpdateCatalogOptions::new("en", CatalogUpdateInput::default());

        let item = base_po_item(&message, &options, 1);
        assert_eq!(item.references.as_slice(), vec!["src/lib.rs"].as_slice());

        let options = UpdateCatalogOptions {
            render: super::super::RenderOptions {
                include_origins: false,
                ..Default::default()
            },
            ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
        };
        let item = base_po_item(&message, &options, 1);
        assert!(item.references.is_empty());
    }

    #[test]
    fn machine_metadata_validation_handles_invalid_and_plural_cases() {
        let mut invalid = singular_message("Hallo");
        invalid.machine = Some(MachineMetadata {
            lock: machine_translation_hash(EffectiveTranslationRef::Singular("Hallo")),
            ai: Some(AiProvenance {
                model: String::new(),
                confidence: None,
            }),
        });
        assert!(valid_machine_metadata(&invalid).is_none());

        let translations = BTreeMap::from([
            ("one".to_owned(), "Artikel".to_owned()),
            ("other".to_owned(), "Artikel".to_owned()),
        ]);
        let hash = machine_translation_hash(EffectiveTranslationRef::Plural(&translations));
        let mut plural = plural_message(translations);
        plural.machine = Some(MachineMetadata {
            lock: hash.clone(),
            ai: Some(AiProvenance {
                model: "openai/gpt-5.5-high".to_owned(),
                confidence: Some(0.95),
            }),
        });

        assert_eq!(
            valid_machine_metadata(&plural).map(|metadata| metadata.lock.as_str()),
            Some(hash.as_str())
        );
    }
}
