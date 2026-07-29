//! Catalog export helpers for PO and FCL rendering.

use std::collections::{BTreeMap, BTreeSet};

use crate::SerializeOptions;
use crate::diagnostic_codes;
use crate::serialize::{write_keyword, write_prefixed_line};
use crate::text::escape_string_into;

use super::catalog::{
    CanonicalMessage, CanonicalTranslation, Catalog, PO_OBSOLETE_SINCE_KEY, format_origin,
};
use super::mt::{
    MachineMetadata, PO_AI_KEY, PO_LOCK_KEY, format_ai_descriptor, machine_translation_hash,
    validate_machine_metadata,
};
use super::plural::{PluralProfile, synthesize_icu_plural, synthesize_icu_plural_source};
use super::{
    ApiError, CatalogSemantics, Diagnostic, DiagnosticSeverity, EffectiveTranslationRef,
    PlaceholderCommentMode, UpdateCatalogOptions,
};

pub(super) fn export_catalog_content(
    catalog: &Catalog,
    options: &UpdateCatalogOptions<'_>,
    locale: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<String, ApiError> {
    match options.mode.storage_format() {
        super::CatalogStorageFormat::Po => {
            stringify_catalog_po(catalog, options, locale, diagnostics)
        }
        super::CatalogStorageFormat::Fcl => Ok(super::fcl::stringify_catalog_fcl(
            catalog,
            locale,
            options.source_locale,
            &options.render,
        )),
    }
}

fn stringify_catalog_po(
    catalog: &Catalog,
    options: &UpdateCatalogOptions<'_>,
    locale: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<String, ApiError> {
    let serialize_options = SerializeOptions::default();
    let mut out = String::with_capacity(estimate_catalog_po_capacity(catalog));
    let mut scratch = String::new();

    for comment in &catalog.file_comments {
        write_prefixed_line(&mut out, "", "#", comment);
    }
    for comment in &catalog.file_extracted_comments {
        write_prefixed_line(&mut out, "", "#.", comment);
    }

    out.push_str("msgid \"\"\n");
    out.push_str("msgstr \"\"\n");
    for (key, value) in &catalog.headers {
        out.push('"');
        escape_string_into(&mut out, key);
        out.push_str(": ");
        escape_string_into(&mut out, value);
        out.push_str("\\n");
        out.push_str("\"\n");
    }
    out.push('\n');

    let mut plural_profiles = PluralProfileCache::new(locale);
    let mut iter = catalog.messages.iter().peekable();
    while let Some(message) = iter.next() {
        write_catalog_po_message(
            &mut out,
            &mut scratch,
            message,
            options,
            &mut plural_profiles,
            diagnostics,
            &serialize_options,
        )?;
        if iter.peek().is_some() {
            out.push('\n');
        }
    }

    Ok(out)
}

/// Reuses gettext plural profiles per observed slot count.
///
/// The locale is constant for a whole export, so profiles only differ by the
/// number of translated categories; a large catalog would otherwise rebuild the
/// same profile for every plural message.
struct PluralProfileCache<'a> {
    locale: Option<&'a str>,
    entries: Vec<(usize, PluralProfile)>,
}

impl<'a> PluralProfileCache<'a> {
    const fn new(locale: Option<&'a str>) -> Self {
        Self {
            locale,
            entries: Vec::new(),
        }
    }

    fn for_gettext_translation(
        &mut self,
        translation_by_category: &BTreeMap<String, String>,
    ) -> &PluralProfile {
        let nplurals = translation_by_category.len();
        let index = match self
            .entries
            .iter()
            .position(|(slots, _)| *slots == nplurals)
        {
            Some(index) => index,
            None => {
                self.entries.push((
                    nplurals,
                    PluralProfile::for_gettext_translation(self.locale, translation_by_category),
                ));
                self.entries.len() - 1
            }
        };

        &self.entries[index].1
    }
}

fn estimate_catalog_po_capacity(catalog: &Catalog) -> usize {
    let comments_len: usize = catalog
        .file_comments
        .iter()
        .chain(catalog.file_extracted_comments.iter())
        .map(String::len)
        .sum();
    let headers_len: usize = catalog
        .headers
        .iter()
        .map(|(key, value)| key.len() + value.len() + 8)
        .sum();
    let messages_len: usize = catalog
        .messages
        .iter()
        .map(|message| {
            // `msgid ""`, `msgstr ""`, the blank separator line and quoting add a
            // fixed per-message overhead on top of the raw payload bytes; each
            // comment and reference line costs its own `#x ` prefix plus newline.
            MESSAGE_LINE_OVERHEAD
                + message.msgid.len()
                + message
                    .msgctxt
                    .as_ref()
                    .map_or(0, |context| context.len() + 12)
                + message
                    .comments
                    .iter()
                    .map(|comment| comment.len() + COMMENT_LINE_OVERHEAD)
                    .sum::<usize>()
                + message
                    .origins
                    .iter()
                    .map(|origin| {
                        origin.file.len()
                            + origin.scope.as_ref().map_or(0, |scope| scope.len() + 1)
                            + COMMENT_LINE_OVERHEAD
                    })
                    .sum::<usize>()
                + translation_len(&message.translation)
        })
        .sum();

    comments_len + headers_len + messages_len + 256
}

/// `msgid ""\n` plus `msgstr ""\n` plus the blank line between messages.
const MESSAGE_LINE_OVERHEAD: usize = 24;
/// A `#. ` style prefix plus the trailing newline.
const COMMENT_LINE_OVERHEAD: usize = 4;

fn translation_len(translation: &CanonicalTranslation) -> usize {
    match translation {
        CanonicalTranslation::Singular { value } => value.len(),
        CanonicalTranslation::Plural {
            source,
            translation_by_category,
            ..
        } => {
            // Plural messages add a `msgid_plural` line plus one quoted
            // `msgstr[n] ""` line per category.
            source.one.as_ref().map_or(0, String::len)
                + source.other.len()
                + 16
                + translation_by_category
                    .values()
                    .map(|value| value.len() + 14)
                    .sum::<usize>()
        }
    }
}

fn write_catalog_po_message(
    out: &mut String,
    scratch: &mut String,
    message: &CanonicalMessage,
    options: &UpdateCatalogOptions<'_>,
    plural_profiles: &mut PluralProfileCache<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    serialize_options: &SerializeOptions,
) -> Result<(), ApiError> {
    let obsolete_prefix = if message.obsolete.is_some() {
        "#~ "
    } else {
        ""
    };
    write_catalog_po_metadata(out, obsolete_prefix, message, options);
    if let Some(context) = &message.msgctxt {
        write_keyword(
            out,
            scratch,
            obsolete_prefix,
            "msgctxt",
            context,
            None,
            serialize_options,
        );
    }

    match &message.translation {
        CanonicalTranslation::Singular { value } => {
            write_keyword(
                out,
                scratch,
                obsolete_prefix,
                "msgid",
                &message.msgid,
                None,
                serialize_options,
            );
            write_keyword(
                out,
                scratch,
                obsolete_prefix,
                "msgstr",
                value,
                None,
                serialize_options,
            );
            Ok(())
        }
        CanonicalTranslation::Plural {
            source,
            translation_by_category,
            variable,
        } => {
            if options.mode.semantics() == CatalogSemantics::IcuNative {
                let msgid = synthesize_icu_plural_source(variable, source);
                let msgstr = synthesize_icu_plural(variable, translation_by_category);
                write_keyword(
                    out,
                    scratch,
                    obsolete_prefix,
                    "msgid",
                    &msgid,
                    None,
                    serialize_options,
                );
                write_keyword(
                    out,
                    scratch,
                    obsolete_prefix,
                    "msgstr",
                    &msgstr,
                    None,
                    serialize_options,
                );
                return Ok(());
            }

            let plural_profile = plural_profiles.for_gettext_translation(translation_by_category);
            let nplurals = plural_profile
                .nplurals()
                .max(translation_by_category.len().max(1));

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

            let msgid = source.one.as_deref().unwrap_or(&source.other);
            write_keyword(
                out,
                scratch,
                obsolete_prefix,
                "msgid",
                msgid,
                None,
                serialize_options,
            );
            write_keyword(
                out,
                scratch,
                obsolete_prefix,
                "msgid_plural",
                &source.other,
                None,
                serialize_options,
            );
            let values = plural_profile.gettext_values(translation_by_category);
            write_plural_msgstr(
                out,
                scratch,
                obsolete_prefix,
                &values,
                nplurals,
                serialize_options,
            );

            Ok(())
        }
    }
}

fn write_catalog_po_metadata(
    out: &mut String,
    obsolete_prefix: &str,
    message: &CanonicalMessage,
    options: &UpdateCatalogOptions<'_>,
) {
    for comment in &message.comments {
        write_prefixed_line(out, obsolete_prefix, "#.", comment);
    }
    write_placeholder_comments(
        out,
        obsolete_prefix,
        &message.comments,
        &message.placeholders,
        &options.render.print_placeholders_in_comments,
    );

    if let Some(info) = &message.obsolete
        && let Some(since) = &info.since
    {
        write_metadata_line(out, obsolete_prefix, PO_OBSOLETE_SINCE_KEY, since);
    }
    if let Some(metadata) = valid_machine_metadata(message) {
        write_metadata_line(out, obsolete_prefix, PO_LOCK_KEY, &metadata.lock);
        if let Some(ai) = &metadata.ai {
            write_metadata_line(out, obsolete_prefix, PO_AI_KEY, &format_ai_descriptor(ai));
        }
    }

    if options.render.include_origins {
        // Origins serialize as `file` or `file#scope`; keep the first occurrence
        // and drop exact duplicates of the rendered reference.
        let mut references: Vec<String> = Vec::new();
        for origin in &message.origins {
            let reference = format_origin(origin);
            if !references.iter().any(|existing| existing == &reference) {
                out.push_str(obsolete_prefix);
                out.push_str("#: ");
                out.push_str(&reference);
                out.push('\n');
                references.push(reference);
            }
        }
    }
}

fn write_placeholder_comments(
    out: &mut String,
    obsolete_prefix: &str,
    comments: &[String],
    placeholders: &BTreeMap<String, Vec<String>>,
    mode: &PlaceholderCommentMode,
) {
    for_each_placeholder_comment(comments, placeholders, mode, |comment| {
        write_prefixed_line(out, obsolete_prefix, "#.", comment);
    });
}

fn write_plural_msgstr(
    out: &mut String,
    scratch: &mut String,
    obsolete_prefix: &str,
    values: &[&str],
    nplurals: usize,
    options: &SerializeOptions,
) {
    if values.is_empty() {
        for index in 0..nplurals.max(1) {
            write_keyword(
                out,
                scratch,
                obsolete_prefix,
                "msgstr",
                "",
                Some(index),
                options,
            );
        }
        return;
    }

    for (index, value) in values.iter().enumerate() {
        write_keyword(
            out,
            scratch,
            obsolete_prefix,
            "msgstr",
            value,
            Some(index),
            options,
        );
    }
}

fn write_metadata_line(out: &mut String, obsolete_prefix: &str, key: &str, value: &str) {
    out.push_str(obsolete_prefix);
    out.push_str("#@ ");
    out.push_str(key);
    out.push_str(": ");
    out.push_str(value);
    out.push('\n');
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

pub(super) fn for_each_placeholder_comment(
    comments: &[String],
    placeholders: &BTreeMap<String, Vec<String>>,
    mode: &PlaceholderCommentMode,
    mut visit: impl FnMut(&str),
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
                visit(&comment);
            }
        }
    }
}

fn normalize_placeholder_value(value: &str) -> String {
    value.replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::super::{
        AiProvenance, CatalogMode, CatalogOrigin, CatalogUpdateInput, ObsoleteInfo, PluralSource,
    };
    use super::*;

    fn message_with_translation(translation: CanonicalTranslation) -> CanonicalMessage {
        CanonicalMessage {
            msgid: "items".to_owned(),
            msgctxt: None,
            translation,
            comments: Vec::new(),
            origins: vec![CatalogOrigin {
                file: "src/lib.rs".to_owned(),
                scope: None,
            }]
            .into(),
            placeholders: BTreeMap::new(),
            obsolete: None,
            machine: None,
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
    fn po_export_renders_origin_without_line_and_can_omit_origins() {
        let catalog = Catalog {
            messages: vec![singular_message("Hallo")],
            ..Catalog::default()
        };
        let options = UpdateCatalogOptions::new("en", CatalogUpdateInput::default());
        let mut diagnostics = Vec::new();

        let output = export_catalog_content(&catalog, &options, Some("de"), &mut diagnostics)
            .expect("export succeeds");
        assert!(output.contains("#: src/lib.rs\n"));

        let options = UpdateCatalogOptions {
            render: super::super::RenderOptions {
                include_origins: false,
                ..Default::default()
            },
            ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
        };
        let output = export_catalog_content(&catalog, &options, Some("de"), &mut diagnostics)
            .expect("export succeeds");
        assert!(!output.contains("#: src/lib.rs\n"));
    }

    #[test]
    fn po_export_writes_catalog_and_message_metadata_directly() {
        let mut message = singular_message("Hallo");
        message.msgctxt = Some("CheckoutButton".to_owned());
        message.comments = vec!["Translator note".to_owned()];
        message.origins = vec![
            CatalogOrigin {
                file: "src/lib.rs".to_owned(),
                scope: Some("CheckoutButton".to_owned()),
            },
            CatalogOrigin {
                file: "src/lib.rs".to_owned(),
                scope: Some("CheckoutButton".to_owned()),
            },
        ]
        .into();
        message.placeholders = BTreeMap::from([
            (
                "0".to_owned(),
                vec!["user\nname".to_owned(), "ignored".to_owned()],
            ),
            ("name".to_owned(), vec!["not generated".to_owned()]),
        ]);
        message.obsolete = Some(ObsoleteInfo {
            since: Some("2026-07-01".to_owned()),
        });
        let lock = machine_translation_hash(EffectiveTranslationRef::Singular("Hallo"));
        message.machine = Some(MachineMetadata {
            lock: lock.clone(),
            ai: Some(AiProvenance {
                model: "openai/gpt-5.5-high".to_owned(),
                confidence: Some(0.92),
            }),
        });

        let catalog = Catalog {
            headers: BTreeMap::from([("Language".to_owned(), "de".to_owned())]),
            file_comments: vec!["Project note".to_owned()],
            file_extracted_comments: vec!["Header hint".to_owned()],
            messages: vec![message],
            ..Catalog::default()
        };
        let options = UpdateCatalogOptions::new("en", CatalogUpdateInput::default());
        let mut diagnostics = Vec::new();

        let output = export_catalog_content(&catalog, &options, Some("de"), &mut diagnostics)
            .expect("export succeeds");

        assert!(diagnostics.is_empty());
        assert!(output.contains("# Project note\n"));
        assert!(output.contains("#. Header hint\n"));
        assert!(output.contains("\"Language: de\\n\"\n"));
        assert!(output.contains("#~ #. Translator note\n"));
        assert!(output.contains("#~ #. placeholder {0}: user name\n"));
        assert!(output.contains("#~ #@ obsolete-since: 2026-07-01\n"));
        assert!(output.contains(&format!("#~ #@ lock: {lock}\n")));
        assert!(output.contains("#~ #@ ai: openai/gpt-5.5-high:0.92\n"));
        assert_eq!(
            output.matches("#~ #: src/lib.rs#CheckoutButton\n").count(),
            1
        );
        assert!(output.contains("#~ msgctxt \"CheckoutButton\"\n"));
        assert!(output.contains("#~ msgid \"items\"\n"));
        assert!(output.contains("#~ msgstr \"Hallo\"\n"));
        assert!(!output.contains("not generated"));
    }

    #[test]
    fn po_export_skips_duplicate_generated_placeholder_comments() {
        let mut message = singular_message("Hallo");
        message.comments = vec!["placeholder {0}: user name".to_owned()];
        message.placeholders = BTreeMap::from([(
            "0".to_owned(),
            vec!["user\nname".to_owned(), "account".to_owned()],
        )]);

        let catalog = Catalog {
            messages: vec![message],
            ..Catalog::default()
        };
        let options = UpdateCatalogOptions::new("en", CatalogUpdateInput::default());
        let mut diagnostics = Vec::new();

        let output = export_catalog_content(&catalog, &options, Some("de"), &mut diagnostics)
            .expect("export succeeds");

        assert!(diagnostics.is_empty());
        assert_eq!(output.matches("#. placeholder {0}: user name\n").count(), 1);
        assert!(output.contains("#. placeholder {0}: account\n"));
    }

    #[test]
    fn icu_po_export_writes_plural_messages_directly() {
        let catalog = Catalog {
            messages: vec![plural_message(BTreeMap::from([
                ("one".to_owned(), "Ein Artikel".to_owned()),
                ("other".to_owned(), "{count} Artikel".to_owned()),
            ]))],
            ..Catalog::default()
        };
        let options = UpdateCatalogOptions::new("en", CatalogUpdateInput::default());
        let mut diagnostics = Vec::new();

        let output = export_catalog_content(&catalog, &options, Some("de"), &mut diagnostics)
            .expect("export succeeds");

        assert!(diagnostics.is_empty());
        assert!(output.contains("msgid \"{count, plural, one {item} other {items}}\"\n"));
        assert!(
            output.contains(
                "msgstr \"{count, plural, one {Ein Artikel} other {{count} Artikel}}\"\n"
            )
        );
    }

    #[test]
    fn plural_msgstr_writer_preserves_declared_empty_slots() {
        let mut output = String::new();
        let mut scratch = String::new();

        write_plural_msgstr(
            &mut output,
            &mut scratch,
            "",
            &[],
            2,
            &SerializeOptions::default(),
        );

        assert_eq!(output, "msgstr[0] \"\"\nmsgstr[1] \"\"\n");
    }

    #[test]
    fn plural_msgstr_writer_renders_declared_values() {
        let mut output = String::new();
        let mut scratch = String::new();

        write_plural_msgstr(
            &mut output,
            &mut scratch,
            "#~ ",
            &["eins", "viele"],
            2,
            &SerializeOptions::default(),
        );

        assert_eq!(output, "#~ msgstr[0] \"eins\"\n#~ msgstr[1] \"viele\"\n");
    }

    #[test]
    fn placeholder_comment_generator_respects_mode_limit_and_duplicates() {
        let mut comments = vec!["placeholder {0}: known value".to_owned()];
        let placeholders = BTreeMap::from([
            (
                "0".to_owned(),
                vec!["known value".to_owned(), "next\nvalue".to_owned()],
            ),
            ("name".to_owned(), vec!["ignored".to_owned()]),
        ]);

        let mut generated = Vec::new();
        {
            let mut collect = |comment: &str| generated.push(comment.to_owned());
            for_each_placeholder_comment(
                &comments,
                &placeholders,
                &PlaceholderCommentMode::Enabled { limit: 2 },
                &mut collect,
            );

            for_each_placeholder_comment(
                &comments,
                &placeholders,
                &PlaceholderCommentMode::Disabled,
                &mut collect,
            );
        }
        comments.extend(generated);

        assert_eq!(
            comments,
            vec![
                "placeholder {0}: known value".to_owned(),
                "placeholder {0}: next value".to_owned(),
            ]
        );

        assert_eq!(
            comments,
            vec![
                "placeholder {0}: known value".to_owned(),
                "placeholder {0}: next value".to_owned(),
            ]
        );
    }

    #[test]
    fn machine_metadata_validation_handles_invalid_and_plural_cases() {
        assert!(valid_machine_metadata(&singular_message("Hallo")).is_none());

        let mut stale = singular_message("Hallo");
        stale.machine = Some(MachineMetadata {
            lock: machine_translation_hash(EffectiveTranslationRef::Singular("Tschuess")),
            ai: None,
        });
        assert!(valid_machine_metadata(&stale).is_none());

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
