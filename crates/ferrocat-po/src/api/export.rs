//! Catalog export helpers for PO and NDJSON rendering.

use std::collections::{BTreeMap, BTreeSet};

use crate::diagnostic_codes;
use crate::{Header, MsgStr, PoFile, PoItem, SerializeOptions, stringify_po};

use super::catalog::{CanonicalMessage, CanonicalTranslation, Catalog};
use super::mt::{
    MachineTranslationMetadata, PO_MACHINE_TRANSLATION_KEY, format_po_machine_translation_metadata,
    machine_translation_hash, validate_machine_translation_metadata,
};
use super::ndjson::stringify_catalog_ndjson;
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
        super::CatalogStorageFormat::Ndjson => Ok(stringify_catalog_ndjson(
            catalog,
            locale,
            options.source_locale,
            &options.render.print_placeholders_in_comments,
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
    item.comments.clone_from(&message.translator_comments);
    item.flags.clone_from(&message.flags);
    item.obsolete = message.obsolete;
    item.extracted_comments.clone_from(&message.comments);
    append_placeholder_comments(
        &mut item.extracted_comments,
        &message.placeholders,
        &options.render.print_placeholders_in_comments,
    );
    if let Some(metadata) = valid_machine_translation_metadata(message) {
        item.metadata.push((
            PO_MACHINE_TRANSLATION_KEY.to_owned(),
            format_po_machine_translation_metadata(metadata),
        ));
    }
    item.references = if options.render.include_origins {
        message
            .origins
            .iter()
            .map(|origin| {
                if options.render.include_line_numbers {
                    origin.line.map_or_else(
                        || origin.file.clone(),
                        |line| format!("{}:{line}", origin.file),
                    )
                } else {
                    origin.file.clone()
                }
            })
            .collect()
    } else {
        Vec::new()
    };
    item
}

fn valid_machine_translation_metadata(
    message: &CanonicalMessage,
) -> Option<&MachineTranslationMetadata> {
    let metadata = message.machine_translation.as_ref()?;
    if validate_machine_translation_metadata(metadata).is_err() {
        return None;
    }
    (metadata.hash == machine_translation_hash(canonical_translation_ref(&message.translation)))
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
