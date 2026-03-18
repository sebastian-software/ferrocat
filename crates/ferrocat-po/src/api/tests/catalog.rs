use super::*;

#[test]
fn update_catalog_creates_new_source_locale_messages() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("en"),
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "Hello".to_owned(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogOptions::default()
    })
    .expect("update");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items.len(), 1);
    assert_eq!(parsed.items[0].msgid, "Hello");
    assert_eq!(parsed.items[0].msgstr[0], "Hello");
    assert!(result.created);
    assert!(result.updated);
    assert_eq!(result.stats.added, 1);
}

#[test]
fn update_catalog_preserves_non_source_translations() {
    let existing = "msgid \"Hello\"\nmsgstr \"Hallo\"\n";
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        existing: Some(existing),
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "Hello".to_owned(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogOptions::default()
    })
    .expect("update");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items[0].msgstr[0], "Hallo");
    assert_eq!(result.stats.unchanged, 1);
}

#[test]
fn overwrite_source_translations_refreshes_source_locale() {
    let existing = "msgid \"Hello\"\nmsgstr \"Old\"\n";
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("en"),
        existing: Some(existing),
        overwrite_source_translations: true,
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "Hello".to_owned(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogOptions::default()
    })
    .expect("update");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items[0].msgstr[0], "Hello");
    assert_eq!(result.stats.changed, 1);
}

#[test]
fn obsolete_strategy_delete_removes_missing_messages() {
    let existing = "msgid \"keep\"\nmsgstr \"x\"\n\nmsgid \"drop\"\nmsgstr \"y\"\n";
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        existing: Some(existing),
        obsolete_strategy: ObsoleteStrategy::Delete,
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "keep".to_owned(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogOptions::default()
    })
    .expect("update");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items.len(), 1);
    assert_eq!(result.stats.obsolete_removed, 1);
}

#[test]
fn duplicate_conflicts_fail_hard() {
    let error = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("en"),
        input: structured_input(vec![
            ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            }),
            ExtractedMessage::Plural(ExtractedPluralMessage {
                msgid: "Hello".to_owned(),
                source: PluralSource {
                    one: Some("One".to_owned()),
                    other: "Many".to_owned(),
                },
                ..ExtractedPluralMessage::default()
            }),
        ]),
        ..UpdateCatalogOptions::default()
    })
    .expect_err("conflict");

    assert!(matches!(error, ApiError::Conflict(_)));
}

#[test]
fn plural_icu_export_uses_structural_input() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("en"),
        input: structured_input(vec![ExtractedMessage::Plural(ExtractedPluralMessage {
            msgid: "{count, plural, one {# item} other {# items}}".to_owned(),
            source: PluralSource {
                one: Some("# item".to_owned()),
                other: "# items".to_owned(),
            },
            placeholders: BTreeMap::from([("count".to_owned(), vec!["count".to_owned()])]),
            ..ExtractedPluralMessage::default()
        })]),
        ..UpdateCatalogOptions::default()
    })
    .expect("update");

    let parsed = parse_po(&result.content).expect("parse output");
    assert!(parsed.items[0].msgid.contains("{count, plural,"));
    assert!(parsed.items[0].msgid_plural.is_none());
}

#[test]
fn source_first_plain_messages_normalize_as_singular() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("en"),
        input: source_first_input(vec![SourceExtractedMessage {
            msgid: "Welcome".to_owned(),
            ..SourceExtractedMessage::default()
        }]),
        ..UpdateCatalogOptions::default()
    })
    .expect("update");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items[0].msgid, "Welcome");
    assert_eq!(parsed.items[0].msgstr[0], "Welcome");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn source_first_simple_icu_plural_stays_singular_in_native_mode() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("en"),
        input: source_first_input(vec![SourceExtractedMessage {
            msgid: "{items, plural, one {# file} other {# files}}".to_owned(),
            placeholders: BTreeMap::from([("items".to_owned(), vec!["items".to_owned()])]),
            ..SourceExtractedMessage::default()
        }]),
        ..UpdateCatalogOptions::default()
    })
    .expect("update");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(
        parsed.items[0].msgid,
        "{items, plural, one {# file} other {# files}}"
    );
    assert_eq!(
        parsed.items[0].msgstr[0],
        "{items, plural, one {# file} other {# files}}"
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn source_first_nested_icu_plural_stays_singular_without_projection_warning() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("en"),
        input: source_first_input(vec![SourceExtractedMessage {
            msgid: "{count, plural, one {{gender, select, male {He has one file} other {They have one file}}} other {{gender, select, male {He has # files} other {They have # files}}}}".to_owned(),
            ..SourceExtractedMessage::default()
        }]),
        ..UpdateCatalogOptions::default()
    })
    .expect("update");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(
        parsed.items[0].msgid,
        "{count, plural, one {{gender, select, male {He has one file} other {They have one file}}} other {{gender, select, male {He has # files} other {They have # files}}}}"
    );
    assert_eq!(parsed.items[0].msgstr[0], parsed.items[0].msgid);
    assert!(result.diagnostics.is_empty());
}

#[test]
fn parse_catalog_projects_gettext_plural_into_structured_shape() {
    let parsed = parse_catalog(ParseCatalogOptions {
        content: concat!(
            "msgid \"book\"\n",
            "msgid_plural \"books\"\n",
            "msgstr[0] \"Buch\"\n",
            "msgstr[1] \"Buecher\"\n",
        ),
        locale: Some("de"),
        source_locale: "en",
        storage_format: CatalogStorageFormat::Po,
        semantics: CatalogSemantics::GettextCompat,
        plural_encoding: PluralEncoding::Gettext,
        strict: false,
    })
    .expect("parse");

    match &parsed.messages[0].translation {
        TranslationShape::Plural {
            source,
            translation,
            variable,
        } => {
            assert_eq!(source.one.as_deref(), Some("book"));
            assert_eq!(source.other, "books");
            assert_eq!(variable, "count");
            assert_eq!(translation.get("one").map(String::as_str), Some("Buch"));
            assert_eq!(
                translation.get("other").map(String::as_str),
                Some("Buecher")
            );
        }
        other => panic!("expected plural translation, got {other:?}"),
    }
}

#[test]
fn normalized_view_indexes_messages_by_key() {
    let parsed = parse_catalog(ParseCatalogOptions {
        content: concat!(
            "msgctxt \"nav\"\n",
            "msgid \"Home\"\n",
            "msgstr \"Start\"\n",
        ),
        locale: Some("de"),
        source_locale: "en",
        storage_format: CatalogStorageFormat::Po,
        semantics: CatalogSemantics::GettextCompat,
        plural_encoding: PluralEncoding::Gettext,
        strict: false,
    })
    .expect("parse");

    let normalized = parsed.into_normalized_view().expect("normalized view");
    let key = CatalogMessageKey::new("Home", Some("nav".to_owned()));

    assert!(normalized.contains_key(&key));
    assert_eq!(normalized.message_count(), 1);
    assert!(matches!(
        normalized.effective_translation(&key),
        Some(EffectiveTranslationRef::Singular("Start"))
    ));
    assert_eq!(normalized.iter().count(), 1);
}

#[test]
fn normalized_view_rejects_duplicate_keys() {
    let parsed = parse_catalog(ParseCatalogOptions {
        content: concat!(
            "msgid \"Hello\"\n",
            "msgstr \"Hallo\"\n",
            "\n",
            "msgid \"Hello\"\n",
            "msgstr \"Servus\"\n",
        ),
        locale: Some("de"),
        source_locale: "en",
        storage_format: CatalogStorageFormat::Po,
        semantics: CatalogSemantics::GettextCompat,
        plural_encoding: PluralEncoding::Gettext,
        strict: false,
    })
    .expect("parse");

    let error = parsed
        .into_normalized_view()
        .expect_err("duplicate keys should fail");
    assert!(matches!(error, ApiError::Conflict(_)));
}

#[test]
fn normalized_view_can_apply_source_locale_fallbacks() {
    let parsed = parse_catalog(ParseCatalogOptions {
        content: concat!(
            "msgid \"book\"\n",
            "msgid_plural \"books\"\n",
            "msgstr[0] \"\"\n",
            "msgstr[1] \"\"\n",
            "\n",
            "msgid \"Welcome\"\n",
            "msgstr \"\"\n",
        ),
        locale: Some("en"),
        source_locale: "en",
        storage_format: CatalogStorageFormat::Po,
        semantics: CatalogSemantics::GettextCompat,
        plural_encoding: PluralEncoding::Gettext,
        strict: false,
    })
    .expect("parse");

    let normalized = parsed.into_normalized_view().expect("normalized view");
    let plural_key = CatalogMessageKey::new("book", None);
    let singular_key = CatalogMessageKey::new("Welcome", None);

    assert!(matches!(
        normalized.effective_translation(&singular_key),
        Some(EffectiveTranslationRef::Singular(""))
    ));
    assert_eq!(
        normalized.effective_translation_with_source_fallback(&singular_key, "en"),
        Some(EffectiveTranslation::Singular("Welcome".to_owned()))
    );

    assert_eq!(
        normalized.effective_translation_with_source_fallback(&plural_key, "en"),
        Some(EffectiveTranslation::Plural(BTreeMap::from([
            ("one".to_owned(), "book".to_owned()),
            ("other".to_owned(), "books".to_owned()),
        ])))
    );
}

#[test]
fn normalized_view_skips_source_fallback_for_non_source_locale_catalogs() {
    let parsed = parse_catalog(ParseCatalogOptions {
        content: concat!("msgid \"Hello\"\n", "msgstr \"\"\n"),
        locale: Some("de"),
        source_locale: "en",
        storage_format: CatalogStorageFormat::Po,
        semantics: CatalogSemantics::GettextCompat,
        plural_encoding: PluralEncoding::Gettext,
        strict: false,
    })
    .expect("parse");

    let normalized = parsed.into_normalized_view().expect("normalized view");
    let key = CatalogMessageKey::new("Hello", None);

    assert_eq!(
        normalized.effective_translation_with_source_fallback(&key, "en"),
        Some(EffectiveTranslation::Singular(String::new()))
    );
}

#[test]
fn parse_catalog_uses_icu_plural_categories_for_french_gettext() {
    let parsed = parse_catalog(ParseCatalogOptions {
        content: concat!(
            "msgid \"fichier\"\n",
            "msgid_plural \"fichiers\"\n",
            "msgstr[0] \"fichier\"\n",
            "msgstr[1] \"millions de fichiers\"\n",
            "msgstr[2] \"fichiers\"\n",
        ),
        locale: Some("fr"),
        source_locale: "en",
        storage_format: CatalogStorageFormat::Po,
        semantics: CatalogSemantics::GettextCompat,
        plural_encoding: PluralEncoding::Gettext,
        strict: false,
    })
    .expect("parse");

    match &parsed.messages[0].translation {
        TranslationShape::Plural { translation, .. } => {
            assert_eq!(translation.get("one").map(String::as_str), Some("fichier"));
            assert_eq!(
                translation.get("many").map(String::as_str),
                Some("millions de fichiers")
            );
            assert_eq!(
                translation.get("other").map(String::as_str),
                Some("fichiers")
            );
        }
        other => panic!("expected plural translation, got {other:?}"),
    }
}

#[test]
fn parse_catalog_prefers_gettext_slot_count_when_it_disagrees_with_locale_categories() {
    let parsed = parse_catalog(ParseCatalogOptions {
        content: concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n",
            "\n",
            "msgid \"livre\"\n",
            "msgid_plural \"livres\"\n",
            "msgstr[0] \"livre\"\n",
            "msgstr[1] \"livres\"\n",
        ),
        locale: Some("fr"),
        source_locale: "en",
        storage_format: CatalogStorageFormat::Po,
        semantics: CatalogSemantics::GettextCompat,
        plural_encoding: PluralEncoding::Gettext,
        strict: false,
    })
    .expect("parse");

    match &parsed.messages[0].translation {
        TranslationShape::Plural { translation, .. } => {
            assert_eq!(translation.len(), 2);
            assert_eq!(translation.get("one").map(String::as_str), Some("livre"));
            assert_eq!(translation.get("other").map(String::as_str), Some("livres"));
            assert!(translation.get("many").is_none());
        }
        other => panic!("expected plural translation, got {other:?}"),
    }
}

#[test]
fn parse_catalog_reports_plural_forms_locale_mismatch() {
    let parsed = parse_catalog(ParseCatalogOptions {
        content: concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Language: fr\\n\"\n",
            "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n",
        ),
        locale: Some("fr"),
        source_locale: "en",
        storage_format: CatalogStorageFormat::Po,
        semantics: CatalogSemantics::GettextCompat,
        plural_encoding: PluralEncoding::Gettext,
        strict: false,
    })
    .expect("parse");

    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "plural.nplurals_locale_mismatch")
    );
}

#[test]
fn parse_catalog_keeps_simple_icu_plural_as_singular_in_native_mode() {
    let parsed = parse_catalog(ParseCatalogOptions {
        content: concat!(
            "msgid \"{count, plural, one {# item} other {# items}}\"\n",
            "msgstr \"{count, plural, one {# Artikel} other {# Artikel}}\"\n",
        ),
        locale: Some("de"),
        source_locale: "en",
        storage_format: CatalogStorageFormat::Po,
        semantics: CatalogSemantics::IcuNative,
        plural_encoding: PluralEncoding::Icu,
        strict: false,
    })
    .expect("parse");

    assert!(matches!(
        parsed.messages[0].translation,
        TranslationShape::Singular { .. }
    ));
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn parse_catalog_keeps_nested_icu_plural_as_singular_without_projection_warning() {
    let parsed = parse_catalog(ParseCatalogOptions {
        content: concat!(
            "msgid \"{count, plural, one {{gender, select, male {He has one item} other {They have one item}}} other {{gender, select, male {He has # items} other {They have # items}}}}\"\n",
            "msgstr \"{count, plural, one {{gender, select, male {Er hat einen Artikel} other {Sie haben einen Artikel}}} other {{gender, select, male {Er hat # Artikel} other {Sie haben # Artikel}}}}\"\n",
        ),
        locale: Some("de"),
        source_locale: "en",
        storage_format: CatalogStorageFormat::Po,
        semantics: CatalogSemantics::IcuNative,
        plural_encoding: PluralEncoding::Icu,
        strict: false,
    })
    .expect("parse");

    assert!(matches!(
        parsed.messages[0].translation,
        TranslationShape::Singular { .. }
    ));
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn parse_catalog_strict_keeps_malformed_icu_plural_as_singular_in_native_mode() {
    let parsed = parse_catalog(ParseCatalogOptions {
        content: concat!(
            "msgid \"{count, plural, one {# item} other {# items}\"\n",
            "msgstr \"{count, plural, one {# Artikel} other {# Artikel}}\"\n",
        ),
        locale: Some("de"),
        source_locale: "en",
        storage_format: CatalogStorageFormat::Po,
        semantics: CatalogSemantics::IcuNative,
        plural_encoding: PluralEncoding::Icu,
        strict: true,
    })
    .expect("strict parse");

    assert!(matches!(
        parsed.messages[0].translation,
        TranslationShape::Singular { .. }
    ));
}

#[test]
fn parse_catalog_ndjson_matches_po_semantics() {
    let po = parse_catalog(ParseCatalogOptions {
        content: concat!(
            "msgctxt \"button\"\n",
            "msgid \"Save\"\n",
            "msgstr \"Speichern\"\n\n",
            "#. placeholder {0}: count\n",
            "msgid \"{count, plural, one {# file} other {# files}}\"\n",
            "msgstr \"{count, plural, one {# Datei} other {# Dateien}}\"\n",
        ),
        locale: Some("de"),
        source_locale: "en",
        storage_format: CatalogStorageFormat::Po,
        semantics: CatalogSemantics::IcuNative,
        plural_encoding: PluralEncoding::Icu,
        strict: false,
    })
    .expect("parse po");
    let ndjson = parse_catalog(ParseCatalogOptions {
        content: concat!(
            "---\n",
            "format: ferrocat.ndjson.v1\n",
            "locale: de\n",
            "source_locale: en\n",
            "---\n",
            "{\"id\":\"Save\",\"ctx\":\"button\",\"str\":\"Speichern\"}\n",
            "{\"id\":\"{count, plural, one {# file} other {# files}}\",\"str\":\"{count, plural, one {# Datei} other {# Dateien}}\",\"comments\":[\"placeholder {0}: count\"]}\n",
        ),
        locale: Some("de"),
        source_locale: "en",
        storage_format: CatalogStorageFormat::Ndjson,
        semantics: CatalogSemantics::IcuNative,
        plural_encoding: PluralEncoding::Icu,
        strict: false,
    })
    .expect("parse ndjson");

    assert_eq!(po, ndjson);
}

#[test]
fn parse_catalog_ndjson_rejects_unknown_record_fields() {
    let error = parse_catalog(ParseCatalogOptions {
        content: concat!(
            "---\n",
            "format: ferrocat.ndjson.v1\n",
            "locale: de\n",
            "source_locale: en\n",
            "---\n",
            "{\"id\":\"About us\",\"str\":\"Ueber uns\",\"oops\":true}\n",
        ),
        source_locale: "en",
        storage_format: CatalogStorageFormat::Ndjson,
        ..ParseCatalogOptions::default()
    })
    .expect_err("unknown ndjson fields should fail");

    assert!(
        matches!(error, ApiError::InvalidArguments(message) if message.contains("invalid NDJSON record"))
    );
}

#[test]
fn parse_catalog_ndjson_rejects_source_locale_mismatch() {
    let error = parse_catalog(ParseCatalogOptions {
        content: concat!(
            "---\n",
            "format: ferrocat.ndjson.v1\n",
            "locale: de\n",
            "source_locale: fr\n",
            "---\n",
            "{\"id\":\"About us\",\"str\":\"Ueber uns\"}\n",
        ),
        source_locale: "en",
        storage_format: CatalogStorageFormat::Ndjson,
        ..ParseCatalogOptions::default()
    })
    .expect_err("source locale mismatch should fail");

    assert!(
        matches!(error, ApiError::InvalidArguments(message) if message.contains("source_locale"))
    );
}

#[test]
fn update_catalog_file_writes_only_when_changed() {
    let temp_dir = std::env::temp_dir().join("ferrocat-po-update-file-test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("create temp dir");
    let path = temp_dir.join("messages.po");

    let first = update_catalog_file(UpdateCatalogFileOptions {
        target_path: &path,
        source_locale: "en",
        locale: Some("en"),
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "Hello".to_owned(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogFileOptions::default()
    })
    .expect("first write");
    assert!(first.created);

    let second = update_catalog_file(UpdateCatalogFileOptions {
        target_path: &path,
        source_locale: "en",
        locale: Some("en"),
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "Hello".to_owned(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogFileOptions::default()
    })
    .expect("second write");
    assert!(!second.created);
    assert!(!second.updated);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn update_catalog_ndjson_renders_frontmatter_and_roundtrips() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        storage_format: CatalogStorageFormat::Ndjson,
        input: structured_input(vec![
            ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "About us".to_owned(),
                msgctxt: Some("nav".to_owned()),
                comments: vec!["Main navigation".to_owned()],
                origin: vec![CatalogOrigin {
                    file: "src/nav.rs".to_owned(),
                    line: Some(4),
                }],
                ..ExtractedSingularMessage::default()
            }),
            ExtractedMessage::Plural(ExtractedPluralMessage {
                msgid: "{count, plural, one {# file} other {# files}}".to_owned(),
                source: PluralSource {
                    one: Some("# file".to_owned()),
                    other: "# files".to_owned(),
                },
                placeholders: BTreeMap::from([("0".to_owned(), vec!["count".to_owned()])]),
                ..ExtractedPluralMessage::default()
            }),
        ]),
        ..UpdateCatalogOptions::default()
    })
    .expect("update ndjson");

    assert!(
        result
            .content
            .starts_with("---\nformat: ferrocat.ndjson.v1\n")
    );
    assert!(result.content.contains("\"ctx\":\"nav\""));

    let reparsed = parse_catalog(ParseCatalogOptions {
        content: &result.content,
        locale: Some("de"),
        source_locale: "en",
        storage_format: CatalogStorageFormat::Ndjson,
        semantics: CatalogSemantics::IcuNative,
        plural_encoding: PluralEncoding::Icu,
        strict: false,
    })
    .expect("reparse ndjson");

    assert_eq!(reparsed.locale.as_deref(), Some("de"));
    assert_eq!(reparsed.messages.len(), 2);
    assert!(matches!(
        reparsed.messages[1].translation,
        TranslationShape::Singular { .. }
    ));
}

#[test]
fn update_catalog_file_ndjson_writes_only_when_changed() {
    let temp_dir = std::env::temp_dir().join("ferrocat-po-update-ndjson-file-test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("create temp dir");
    let path = temp_dir.join("messages.fcat.ndjson");

    let first = update_catalog_file(UpdateCatalogFileOptions {
        target_path: &path,
        source_locale: "en",
        locale: Some("en"),
        storage_format: CatalogStorageFormat::Ndjson,
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "Hello".to_owned(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogFileOptions::default()
    })
    .expect("first ndjson write");
    assert!(first.created);

    let second = update_catalog_file(UpdateCatalogFileOptions {
        target_path: &path,
        source_locale: "en",
        locale: Some("en"),
        storage_format: CatalogStorageFormat::Ndjson,
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "Hello".to_owned(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogFileOptions::default()
    })
    .expect("second ndjson write");
    assert!(!second.created);
    assert!(!second.updated);

    let written = fs::read_to_string(&path).expect("read ndjson output");
    assert!(written.contains("\"id\":\"Hello\""));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn update_catalog_gettext_export_emits_plural_slots() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        semantics: CatalogSemantics::GettextCompat,
        plural_encoding: PluralEncoding::Gettext,
        input: structured_input(vec![ExtractedMessage::Plural(ExtractedPluralMessage {
            msgid: "books".to_owned(),
            source: PluralSource {
                one: Some("book".to_owned()),
                other: "books".to_owned(),
            },
            placeholders: BTreeMap::from([("count".to_owned(), vec!["count".to_owned()])]),
            ..ExtractedPluralMessage::default()
        })]),
        ..UpdateCatalogOptions::default()
    })
    .expect("update");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items[0].msgid, "book");
    assert_eq!(parsed.items[0].msgid_plural.as_deref(), Some("books"));
    assert_eq!(parsed.items[0].msgstr.len(), 2);
}

#[test]
fn update_catalog_gettext_export_uses_icu_plural_categories_for_french() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("fr"),
        semantics: CatalogSemantics::GettextCompat,
        plural_encoding: PluralEncoding::Gettext,
        input: structured_input(vec![ExtractedMessage::Plural(ExtractedPluralMessage {
            msgid: "files".to_owned(),
            source: PluralSource {
                one: Some("file".to_owned()),
                other: "files".to_owned(),
            },
            placeholders: BTreeMap::from([("count".to_owned(), vec!["count".to_owned()])]),
            ..ExtractedPluralMessage::default()
        })]),
        ..UpdateCatalogOptions::default()
    })
    .expect("update");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items[0].msgstr.len(), 3);
}

#[test]
fn update_catalog_gettext_sets_safe_plural_forms_header_for_two_form_locale() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        semantics: CatalogSemantics::GettextCompat,
        plural_encoding: PluralEncoding::Gettext,
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "Hello".to_owned(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogOptions::default()
    })
    .expect("update");

    let parsed = parse_po(&result.content).expect("parse output");
    let plural_forms = parsed
        .headers
        .iter()
        .find(|header| header.key == "Plural-Forms")
        .map(|header| header.value.as_str());
    assert_eq!(plural_forms, Some("nplurals=2; plural=(n != 1);"));
}

#[test]
fn update_catalog_gettext_reports_when_no_safe_plural_forms_header_is_known() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("fr"),
        semantics: CatalogSemantics::GettextCompat,
        plural_encoding: PluralEncoding::Gettext,
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "Bonjour".to_owned(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogOptions::default()
    })
    .expect("update");

    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "plural.missing_plural_forms_header")
    );
}

#[test]
fn update_catalog_gettext_completes_partial_plural_forms_header_when_safe() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        semantics: CatalogSemantics::GettextCompat,
        plural_encoding: PluralEncoding::Gettext,
        existing: Some(concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Language: de\\n\"\n",
            "\"Plural-Forms: nplurals=2;\\n\"\n",
        )),
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "Hello".to_owned(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogOptions::default()
    })
    .expect("update");

    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "plural.completed_plural_forms_header")
    );

    let parsed = parse_po(&result.content).expect("parse output");
    let plural_forms = parsed
        .headers
        .iter()
        .find(|header| header.key == "Plural-Forms")
        .map(|header| header.value.as_str());
    assert_eq!(plural_forms, Some("nplurals=2; plural=(n != 1);"));
}

#[test]
fn update_catalog_gettext_preserves_existing_complete_plural_forms_header() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        semantics: CatalogSemantics::GettextCompat,
        plural_encoding: PluralEncoding::Gettext,
        existing: Some(concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Language: de\\n\"\n",
            "\"Plural-Forms: nplurals=2; plural=(n > 1);\\n\"\n",
        )),
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "Hello".to_owned(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogOptions::default()
    })
    .expect("update");

    assert!(
        !result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "plural.completed_plural_forms_header")
    );

    let parsed = parse_po(&result.content).expect("parse output");
    let plural_forms = parsed
        .headers
        .iter()
        .find(|header| header.key == "Plural-Forms")
        .map(|header| header.value.as_str());
    assert_eq!(plural_forms, Some("nplurals=2; plural=(n > 1);"));
}

#[test]
fn parse_catalog_requires_source_locale() {
    let error = parse_catalog(ParseCatalogOptions {
        content: "",
        source_locale: "",
        ..ParseCatalogOptions::default()
    })
    .expect_err("missing source locale");

    match error {
        ApiError::InvalidArguments(message) => {
            assert!(message.contains("source_locale"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn warnings_use_expected_namespace() {
    let mut placeholders = BTreeMap::new();
    placeholders.insert("first".to_owned(), vec!["first".to_owned()]);
    placeholders.insert("second".to_owned(), vec!["second".to_owned()]);

    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        input: structured_input(vec![ExtractedMessage::Plural(ExtractedPluralMessage {
            msgid: "Developers".to_owned(),
            source: PluralSource {
                one: Some("Developer".to_owned()),
                other: "Developers".to_owned(),
            },
            placeholders,
            ..ExtractedPluralMessage::default()
        })]),
        ..UpdateCatalogOptions::default()
    })
    .expect("update");

    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.starts_with("plural."))
    );
    assert!(result.diagnostics.iter().all(|diagnostic| matches!(
        diagnostic.severity,
        DiagnosticSeverity::Warning | DiagnosticSeverity::Error | DiagnosticSeverity::Info
    )));
}
