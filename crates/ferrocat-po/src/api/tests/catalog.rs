use super::*;

use crate::SerializeOptions;

const POLISH_PLURAL_FORMS: &str = "nplurals=3; plural=(n == 1 ? 0 : (n % 10 >= 2 && n % 10 <= 4 && (n % 100 < 10 || n % 100 >= 20)) ? 1 : 2);";

#[test]
fn update_catalog_creates_new_source_locale_messages() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("en"),
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "Hello".to_owned(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
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
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items[0].msgstr[0], "Hallo");
    assert_eq!(result.stats.unchanged, 1);
}

#[test]
fn parse_catalog_reads_po_machine_metadata() {
    let hash = machine_translation_hash(EffectiveTranslationRef::Singular("Hallo"));
    let content = format!(
        "#@ lock: {hash}\n#@ ai: openai/gpt-5.5-high:0.95\nmsgid \"Hello\"\nmsgstr \"Hallo\"\n"
    );
    let parsed = parse_catalog(ParseCatalogOptions {
        content: &content,
        source_locale: "en",
        locale: Some("de"),
        ..ParseCatalogOptions::new("", "en")
    })
    .expect("parse");

    let metadata = parsed.messages[0]
        .machine
        .as_ref()
        .expect("machine metadata");
    assert_eq!(metadata.lock, hash);
    let ai = metadata.ai.as_ref().expect("ai provenance");
    assert_eq!(ai.model, "openai/gpt-5.5-high");
    assert_eq!(ai.confidence, Some(0.95));
}

#[test]
fn parse_catalog_reads_origin_scope_and_keeps_translator_comments_out_of_notes() {
    // A `#scope` anchor on the reference becomes `CatalogOrigin::scope`. Only the
    // extractor-owned `#.` comments become public notes; translator-owned `#`
    // comments are preserved internally for round-tripping and are deliberately
    // not projected into `CatalogMessage` (ADR 0027).
    let content = "# translator note\n\
         #. extracted note\n\
         #: src/Button.tsx#Button\n\
         msgid \"Save\"\nmsgstr \"Speichern\"\n";
    let parsed = parse_catalog(ParseCatalogOptions {
        content,
        source_locale: "en",
        locale: Some("de"),
        ..ParseCatalogOptions::new("", "en")
    })
    .expect("parse");

    let message = &parsed.messages[0];
    assert_eq!(message.origin[0].file, "src/Button.tsx");
    assert_eq!(message.origin[0].scope.as_deref(), Some("Button"));
    assert_eq!(message.comments, vec!["extracted note".to_owned()]);
}

#[test]
fn obsolete_stamps_since_from_clock_and_age_cleanup_drops_old_entries() {
    // "Old" disappears from source and is marked obsolete, stamped with the
    // host-provided clock.
    let stamped = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        existing: Some("msgid \"Old\"\nmsgstr \"Alt\"\n"),
        input: structured_input(vec![]),
        now: Some("2026-06-30"),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("mark obsolete");
    assert!(stamped.content.contains("obsolete-since: 2026-06-30"));

    // The since date round-trips through parsing.
    let parsed = parse_catalog(ParseCatalogOptions {
        content: &stamped.content,
        source_locale: "en",
        locale: Some("de"),
        ..ParseCatalogOptions::new("", "en")
    })
    .expect("parse stamped");
    let info = parsed.messages[0].obsolete.as_ref().expect("obsolete");
    assert_eq!(info.since.as_deref(), Some("2026-06-30"));

    // A cutoff after the since date drops the aged entry.
    let dropped = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        existing: Some(&stamped.content),
        input: structured_input(vec![]),
        obsolete_strategy: ObsoleteStrategy::DropObsoleteBefore("2026-09-01".to_owned()),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("drop old");
    assert!(!dropped.content.contains("Old"));

    // A cutoff before the since date keeps it.
    let kept = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        existing: Some(&stamped.content),
        input: structured_input(vec![]),
        obsolete_strategy: ObsoleteStrategy::DropObsoleteBefore("2026-01-01".to_owned()),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("keep recent");
    assert!(kept.content.contains("Old"));
}

#[test]
fn update_catalog_keeps_valid_po_machine_metadata() {
    let hash = machine_translation_hash(EffectiveTranslationRef::Singular("Hallo"));
    let existing = format!(
        "#@ lock: {hash}\n#@ ai: openai/gpt-5.5-high:0.95\nmsgid \"Hello\"\nmsgstr \"Hallo\"\n"
    );

    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        existing: Some(&existing),
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "Hello".to_owned(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update");

    assert!(result.content.contains("#@ lock: "));
    assert!(result.content.contains("#@ ai: openai/gpt-5.5-high:0.95"));
    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items[0].metadata[0].0, "lock");
}

#[test]
fn update_catalog_drops_stale_po_machine_metadata() {
    let existing = concat!(
        "#@ lock: stale\n",
        "#@ ai: openai/gpt-5.5-high:0.95\n",
        "msgid \"Hello\"\n",
        "msgstr \"Hallo\"\n",
    );

    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        existing: Some(existing),
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "Hello".to_owned(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update");

    assert!(!result.content.contains("#@ lock"));
    assert!(!result.content.contains("#@ ai"));
    let parsed = parse_catalog(ParseCatalogOptions {
        content: existing,
        source_locale: "en",
        locale: Some("de"),
        ..ParseCatalogOptions::new("", "en")
    })
    .expect("parse stale metadata");
    assert!(parsed.messages[0].machine.is_some());
}

#[test]
fn parse_catalog_rejects_malformed_po_machine_metadata() {
    for content in [
        // duplicate `lock`
        "#@ lock: a\n#@ lock: b\nmsgid \"H\"\nmsgstr \"He\"\n",
        // duplicate `ai`
        "#@ lock: a\n#@ ai: x\n#@ ai: y\nmsgid \"H\"\nmsgstr \"He\"\n",
        // `ai` without a `lock`
        "#@ ai: openai/gpt:0.5\nmsgid \"H\"\nmsgstr \"He\"\n",
    ] {
        assert!(
            parse_catalog(ParseCatalogOptions {
                content,
                source_locale: "en",
                locale: Some("de"),
                ..ParseCatalogOptions::new("", "en")
            })
            .is_err()
        );
    }
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
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
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
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items.len(), 1);
    assert_eq!(result.stats.obsolete_removed, 1);
}

#[test]
fn obsolete_strategy_mark_marks_missing_active_messages() {
    let existing = "msgid \"keep\"\nmsgstr \"x\"\n\nmsgid \"drop\"\nmsgstr \"y\"\n";
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        existing: Some(existing),
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "keep".to_owned(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update");

    let parsed = parse_po(&result.content).expect("parse output");
    assert!(
        parsed
            .items
            .iter()
            .any(|item| item.msgid == "drop" && item.obsolete)
    );
    assert_eq!(result.stats.obsolete_marked, 1);
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
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
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
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
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
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
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
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
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
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
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
        mode: CatalogMode::GettextPo,
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
        mode: CatalogMode::GettextPo,
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
        mode: CatalogMode::GettextPo,
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
        mode: CatalogMode::GettextPo,
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
        mode: CatalogMode::GettextPo,
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
        mode: CatalogMode::GettextPo,
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
        mode: CatalogMode::GettextPo,
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
            "\"Plural-Forms: nplurals=3; plural=(n != 1);\\n\"\n",
        ),
        locale: Some("fr"),
        source_locale: "en",
        mode: CatalogMode::GettextPo,
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
fn parse_catalog_accepts_safe_gettext_plural_forms_for_french() {
    let parsed = parse_catalog(ParseCatalogOptions {
        content: concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Language: fr\\n\"\n",
            "\"Plural-Forms: nplurals=2; plural=(n > 1);\\n\"\n",
        ),
        locale: Some("fr"),
        mode: CatalogMode::GettextPo,
        ..ParseCatalogOptions::new("", "en")
    })
    .expect("parse");

    assert!(
        !parsed
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
        mode: CatalogMode::IcuPo,
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
        mode: CatalogMode::IcuPo,
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
        mode: CatalogMode::IcuPo,
        strict: true,
    })
    .expect("strict parse");

    assert!(matches!(
        parsed.messages[0].translation,
        TranslationShape::Singular { .. }
    ));
}
#[test]
fn update_catalog_file_writes_only_when_changed() {
    let temp_dir = std::env::temp_dir().join("ferrocat-po-update-file-test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("create temp dir");
    let path = temp_dir.join("messages.po");

    let first = update_catalog_file(UpdateCatalogFileOptions {
        target_path: &path,
        durability: WriteDurability::Full,
        options: UpdateCatalogOptions {
            locale: Some("en"),
            input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            })]),
            ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
        },
    })
    .expect("first write");
    assert!(first.created);

    let second = update_catalog_file(UpdateCatalogFileOptions {
        target_path: &path,
        durability: WriteDurability::Full,
        options: UpdateCatalogOptions {
            locale: Some("en"),
            input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            })]),
            ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
        },
    })
    .expect("second write");
    assert!(!second.created);
    assert!(!second.updated);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn update_catalog_file_read_error_includes_path_context() {
    let temp_dir = std::env::temp_dir().join("ferrocat-po-update-file-read-error-test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("create temp dir");

    let error = update_catalog_file(UpdateCatalogFileOptions {
        target_path: &temp_dir,
        durability: WriteDurability::Full,
        options: UpdateCatalogOptions {
            locale: Some("en"),
            ..UpdateCatalogOptions::new(
                "en",
                structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                    msgid: "Hello".to_owned(),
                    ..ExtractedSingularMessage::default()
                })]),
            )
        },
    })
    .expect_err("directory read should fail");

    assert_eq!(error.path(), Some(temp_dir.as_path()));
    assert!(matches!(error, ApiError::Io(_)));

    let _ = fs::remove_dir_all(&temp_dir);
}
#[test]
fn update_catalog_gettext_export_emits_plural_slots() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        mode: CatalogMode::GettextPo,
        input: structured_input(vec![ExtractedMessage::Plural(ExtractedPluralMessage {
            msgid: "books".to_owned(),
            source: PluralSource {
                one: Some("book".to_owned()),
                other: "books".to_owned(),
            },
            placeholders: BTreeMap::from([("count".to_owned(), vec!["count".to_owned()])]),
            ..ExtractedPluralMessage::default()
        })]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items[0].msgid, "book");
    assert_eq!(parsed.items[0].msgid_plural.as_deref(), Some("books"));
    assert_eq!(parsed.items[0].msgstr.len(), 2);
}

#[test]
fn update_catalog_preserves_existing_plural_translations() {
    // Reuse path in merge_message: when the existing catalog already has a
    // translated plural and the source plural is unchanged, the merge keeps the
    // existing translation and re-materializes it for the locale's plural
    // categories instead of blanking it.
    let existing = concat!(
        "msgid \"\"\n",
        "msgstr \"\"\n",
        "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n",
        "\n",
        "msgid \"book\"\n",
        "msgid_plural \"books\"\n",
        "msgstr[0] \"Buch\"\n",
        "msgstr[1] \"Bücher\"\n",
    );
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        mode: CatalogMode::GettextPo,
        existing: Some(existing),
        input: structured_input(vec![ExtractedMessage::Plural(ExtractedPluralMessage {
            msgid: "book".to_owned(),
            source: PluralSource {
                one: Some("book".to_owned()),
                other: "books".to_owned(),
            },
            placeholders: BTreeMap::from([("count".to_owned(), vec!["count".to_owned()])]),
            ..ExtractedPluralMessage::default()
        })]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items[0].msgid, "book");
    assert_eq!(parsed.items[0].msgid_plural.as_deref(), Some("books"));
    // The existing translations survive the merge instead of being reset.
    assert_eq!(parsed.items[0].msgstr[0], "Buch");
    assert_eq!(parsed.items[0].msgstr[1], "Bücher");
    assert!(
        !result.content.contains("#~"),
        "entry must not be obsoleted"
    );
    assert_eq!(result.stats.added, 0);
}

#[test]
fn update_catalog_gettext_export_uses_safe_plural_profile_for_french() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("fr"),
        mode: CatalogMode::GettextPo,
        input: structured_input(vec![ExtractedMessage::Plural(ExtractedPluralMessage {
            msgid: "files".to_owned(),
            source: PluralSource {
                one: Some("file".to_owned()),
                other: "files".to_owned(),
            },
            placeholders: BTreeMap::from([("count".to_owned(), vec!["count".to_owned()])]),
            ..ExtractedPluralMessage::default()
        })]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items[0].msgstr.len(), 2);
    let plural_forms = parsed
        .headers
        .iter()
        .find(|header| header.key == "Plural-Forms")
        .map(|header| header.value.as_str());
    assert_eq!(plural_forms, Some("nplurals=2; plural=(n > 1);"));
}

#[test]
fn update_catalog_gettext_sets_safe_plural_forms_header_for_two_form_locale() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        mode: CatalogMode::GettextPo,
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "Hello".to_owned(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
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
fn update_catalog_gettext_sets_safe_plural_forms_header_for_multi_form_locale() {
    let result = update_catalog(UpdateCatalogOptions {
        locale: Some("pl"),
        mode: CatalogMode::GettextPo,
        ..UpdateCatalogOptions::new(
            "en",
            structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            })]),
        )
    })
    .expect("update");

    assert!(
        !result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "plural.missing_plural_forms_header")
    );

    let parsed = parse_po(&result.content).expect("parse output");
    let plural_forms = parsed
        .headers
        .iter()
        .find(|header| header.key == "Plural-Forms")
        .map(|header| header.value.as_str());
    assert_eq!(plural_forms, Some(POLISH_PLURAL_FORMS));
}

#[test]
fn update_catalog_gettext_reports_when_no_safe_plural_forms_header_is_known() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("ga"),
        mode: CatalogMode::GettextPo,
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "Bonjour".to_owned(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
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
        mode: CatalogMode::GettextPo,
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
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
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
        mode: CatalogMode::GettextPo,
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
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
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
fn update_catalog_gettext_preserves_previous_plural_variable_and_translations() {
    let existing = concat!(
        "msgid \"book\"\n",
        "msgid_plural \"books\"\n",
        "msgstr[0] \"Buch\"\n",
        "msgstr[1] \"Buecher\"\n",
    );
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        existing: Some(existing),
        mode: CatalogMode::GettextPo,
        input: structured_input(vec![ExtractedMessage::Plural(ExtractedPluralMessage {
            msgid: "book".to_owned(),
            source: PluralSource {
                one: Some("book".to_owned()),
                other: "books".to_owned(),
            },
            ..ExtractedPluralMessage::default()
        })]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update");

    let parsed = parse_catalog(ParseCatalogOptions {
        content: &result.content,
        locale: Some("de"),
        source_locale: "en",
        mode: CatalogMode::GettextPo,
        strict: false,
    })
    .expect("parse updated catalog");

    match &parsed.messages[0].translation {
        TranslationShape::Plural {
            translation,
            variable,
            ..
        } => {
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
fn overwrite_rebuild_takes_plural_variable_from_previous_translation() {
    // Source-locale update with overwrite forces the rebuild arm; the plural
    // variable must come from the previous translation instead of falling
    // through to the assumed-variable warning.
    let existing = concat!(
        "msgid \"book\"\n",
        "msgid_plural \"books\"\n",
        "msgstr[0] \"book\"\n",
        "msgstr[1] \"books\"\n",
    );
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("en"),
        existing: Some(existing),
        mode: CatalogMode::GettextPo,
        overwrite_source_translations: true,
        input: structured_input(vec![ExtractedMessage::Plural(ExtractedPluralMessage {
            msgid: "book".to_owned(),
            source: PluralSource {
                one: Some("book".to_owned()),
                other: "books".to_owned(),
            },
            ..ExtractedPluralMessage::default()
        })]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update");

    assert!(
        result
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != crate::diagnostic_codes::plural::ASSUMED_VARIABLE),
        "previous plural translation should supply the variable without a warning"
    );
    assert!(result.content.contains("msgid_plural \"books\""));
}

#[test]
fn update_catalog_applies_custom_header_attributes() {
    let headers = BTreeMap::from([
        ("Language-Team".to_owned(), "Core".to_owned()),
        ("X-Generator".to_owned(), "custom-tool".to_owned()),
    ]);

    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        render: RenderOptions {
            custom_header_attributes: Some(&headers),
            ..RenderOptions::default()
        },
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "Hello".to_owned(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(
        parsed
            .headers
            .iter()
            .find(|header| header.key == "Language-Team")
            .map(|header| header.value.as_str()),
        Some("Core")
    );
    assert_eq!(
        parsed
            .headers
            .iter()
            .find(|header| header.key == "X-Generator")
            .map(|header| header.value.as_str()),
        Some("custom-tool")
    );
}

#[test]
fn update_catalog_rejects_empty_extracted_msgid() {
    let error = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("en"),
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: String::new(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect_err("empty msgid should fail");

    assert!(matches!(error, ApiError::InvalidArguments(message) if message.contains("msgid")));
}

#[test]
fn update_catalog_merges_duplicate_source_first_metadata() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("en"),
        input: source_first_input(vec![
            SourceExtractedMessage {
                msgid: "Hello {name}".to_owned(),
                comments: vec!["First comment".to_owned()],
                origin: vec![CatalogOrigin {
                    file: "src/a.rs".to_owned(),
                    scope: None,
                }],
                placeholders: BTreeMap::from([("0".to_owned(), vec!["customer".to_owned()])]),
                ..SourceExtractedMessage::default()
            },
            SourceExtractedMessage {
                msgid: "Hello {name}".to_owned(),
                comments: vec!["Second comment".to_owned()],
                origin: vec![CatalogOrigin {
                    file: "src/b.rs".to_owned(),
                    scope: None,
                }],
                placeholders: BTreeMap::from([(
                    "0".to_owned(),
                    vec!["account".to_owned(), "customer".to_owned()],
                )]),
                ..SourceExtractedMessage::default()
            },
        ]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("merge duplicates");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items.len(), 1);
    assert_eq!(
        parsed.items[0].extracted_comments.as_slice(),
        vec![
            "First comment".to_owned(),
            "Second comment".to_owned(),
            "placeholder {0}: customer".to_owned(),
            "placeholder {0}: account".to_owned(),
        ]
        .as_slice()
    );
    assert_eq!(
        parsed.items[0].references.as_slice(),
        vec!["src/a.rs".to_owned(), "src/b.rs".to_owned()].as_slice()
    );
}

#[test]
fn update_catalog_obsolete_keep_reactivates_missing_messages() {
    let existing = "#~ msgid \"Old\"\n#~ msgstr \"Alt\"\n";

    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        existing: Some(existing),
        obsolete_strategy: ObsoleteStrategy::Keep,
        input: structured_input(Vec::new()),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("keep obsolete");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items.len(), 1);
    assert!(!parsed.items[0].obsolete);
    assert_eq!(parsed.items[0].msgid, "Old");
}

#[test]
fn update_catalog_origin_sort_and_placeholder_options_are_applied() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("en"),
        render: RenderOptions {
            order_by: OrderBy::Origin,
            print_placeholders_in_comments: PlaceholderCommentMode::Enabled { limit: 1 },
            ..RenderOptions::default()
        },
        input: structured_input(vec![
            ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Second".to_owned(),
                origin: vec![CatalogOrigin {
                    file: "src/z.rs".to_owned(),
                    scope: None,
                }],
                placeholders: BTreeMap::from([(
                    "0".to_owned(),
                    vec!["first".to_owned(), "second".to_owned()],
                )]),
                ..ExtractedSingularMessage::default()
            }),
            ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "First".to_owned(),
                origin: vec![CatalogOrigin {
                    file: "src/a.rs".to_owned(),
                    scope: None,
                }],
                ..ExtractedSingularMessage::default()
            }),
        ]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update with origin sort");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items[0].msgid, "First");
    assert_eq!(parsed.items[1].msgid, "Second");
    assert_eq!(
        parsed.items[1].references.as_slice(),
        vec!["src/z.rs"].as_slice()
    );
    assert_eq!(
        parsed.items[1].extracted_comments.as_slice(),
        vec!["placeholder {0}: first".to_owned()].as_slice()
    );

    let without_placeholders = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("en"),
        render: RenderOptions {
            print_placeholders_in_comments: PlaceholderCommentMode::Disabled,
            ..RenderOptions::default()
        },
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "Hello".to_owned(),
            placeholders: BTreeMap::from([("name".to_owned(), vec!["name".to_owned()])]),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update without placeholder comments");
    assert!(!without_placeholders.content.contains("placeholder"));
}

#[test]
fn update_catalog_default_order_matches_lingui_message_and_context_order() {
    // Entry block from a real Lingui 6 `pofile` catalog ordered by
    // `Intl.Collator("en-US")`. Ferrocat-specific headers are excluded from the
    // comparison, but the serialized entries must match byte for byte.
    const LINGUI_GOLDEN_ENTRIES: &str = r#"msgid "!Alert"
msgstr "!Alert"

msgid "\"Quoted\""
msgstr "\"Quoted\""

msgid "(Parens)"
msgstr "(Parens)"

msgid "Álgebra"
msgstr "Álgebra"

msgctxt "a"
msgid "apple"
msgstr "apple"

msgctxt "z"
msgid "apple"
msgstr "apple"

msgid "Apple"
msgstr "Apple"

msgid "co-op"
msgstr "co-op"

msgid "coop"
msgstr "coop"

msgid "eclair"
msgstr "eclair"

msgid "éclair"
msgstr "éclair"

msgid "Item 10"
msgstr "Item 10"

msgid "Item 2"
msgstr "Item 2"

msgid "MiXeD CaSe"
msgstr "MiXeD CaSe"

msgid "naïve"
msgstr "naïve"

msgid "résumé"
msgstr "résumé"

msgid "Uber"
msgstr "Uber"

msgid "über"
msgstr "über"

msgid "Zebra"
msgstr "Zebra"
"#;
    let messages = [
        ("Zebra", None),
        ("über", None),
        ("Uber", None),
        ("éclair", None),
        ("eclair", None),
        ("Apple", None),
        ("apple", Some("z")),
        ("apple", Some("a")),
        ("Álgebra", None),
        ("!Alert", None),
        ("\"Quoted\"", None),
        ("(Parens)", None),
        ("Item 10", None),
        ("Item 2", None),
        ("co-op", None),
        ("coop", None),
        ("naïve", None),
        ("résumé", None),
        ("MiXeD CaSe", None),
    ];
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("en"),
        input: structured_input(
            messages
                .iter()
                .map(|(msgid, msgctxt)| {
                    ExtractedMessage::Singular(ExtractedSingularMessage {
                        msgid: (*msgid).to_owned(),
                        msgctxt: msgctxt.map(str::to_owned),
                        ..ExtractedSingularMessage::default()
                    })
                })
                .collect(),
        ),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update with default sort");

    let entries = result
        .content
        .split_once("\n\n")
        .map(|(_, entries)| entries)
        .expect("PO header separator");
    assert_eq!(entries, LINGUI_GOLDEN_ENTRIES);

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(
        parsed
            .items
            .iter()
            .map(|item| (item.msgid.as_str(), item.msgctxt.as_deref()))
            .collect::<Vec<_>>(),
        vec![
            ("!Alert", None),
            ("\"Quoted\"", None),
            ("(Parens)", None),
            ("Álgebra", None),
            ("apple", Some("a")),
            ("apple", Some("z")),
            ("Apple", None),
            ("co-op", None),
            ("coop", None),
            ("eclair", None),
            ("éclair", None),
            ("Item 10", None),
            ("Item 2", None),
            ("MiXeD CaSe", None),
            ("naïve", None),
            ("résumé", None),
            ("Uber", None),
            ("über", None),
            ("Zebra", None),
        ]
    );
}

#[test]
fn update_catalog_file_rejects_empty_target_path() {
    let error = update_catalog_file(UpdateCatalogFileOptions {
        target_path: std::path::Path::new(""),
        durability: WriteDurability::Full,
        options: UpdateCatalogOptions::new("en", structured_input(Vec::new())),
    })
    .expect_err("empty path should fail");

    assert!(
        matches!(error, ApiError::InvalidArguments(message) if message.contains("target_path"))
    );
}

#[test]
fn parse_catalog_rejects_classic_plural_shape_in_native_mode() {
    let with_plural_source = parse_catalog(ParseCatalogOptions {
        content: concat!(
            "msgid \"item\"\n",
            "msgid_plural \"items\"\n",
            "msgstr[0] \"item\"\n",
            "msgstr[1] \"items\"\n",
        ),
        locale: Some("en"),
        source_locale: "en",
        mode: CatalogMode::IcuPo,
        strict: false,
    })
    .expect_err("classic plural should fail in native mode");
    assert!(matches!(with_plural_source, ApiError::Unsupported(_)));

    let plural_msgstr_only = parse_catalog(ParseCatalogOptions {
        content: concat!(
            "msgid \"item\"\n",
            "msgstr[0] \"item\"\n",
            "msgstr[1] \"items\"\n",
        ),
        locale: Some("en"),
        source_locale: "en",
        mode: CatalogMode::IcuPo,
        strict: false,
    })
    .expect_err("plural msgstr should fail in native mode");
    assert!(matches!(plural_msgstr_only, ApiError::Unsupported(_)));
}

#[test]
fn parse_catalog_reports_plural_expression_without_nplurals() {
    let parsed = parse_catalog(ParseCatalogOptions {
        content: concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Language: de\\n\"\n",
            "\"Plural-Forms: plural=(n != 1);\\n\"\n",
        ),
        locale: Some("de"),
        source_locale: "en",
        mode: CatalogMode::GettextPo,
        strict: false,
    })
    .expect("parse");

    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "parse.invalid_plural_forms_header")
    );
}

#[test]
fn combine_catalogs_rejects_empty_inputs() {
    let error = combine_catalogs(CombineCatalogOptions::new(&[], "en")).expect_err("empty inputs");

    assert!(matches!(
        error,
        ApiError::InvalidArguments(message) if message.contains("inputs")
    ));
}

#[test]
fn combine_catalogs_renders_valid_fcl_through_shared_export() {
    let first = "%FCL1\tsource=en\tlocale=de\torder=collated\nHello\t\tHallo\n";
    let second = "%FCL1\tsource=en\tlocale=de\torder=collated\nNew\t\tNeu\n";
    let inputs = [
        CatalogCombineInput::labeled(first, "first.fcl"),
        CatalogCombineInput::labeled(second, "second.fcl"),
    ];

    let result = combine_catalogs(CombineCatalogOptions {
        inputs: &inputs,
        source_locale: "en",
        locale: Some("de"),
        mode: CatalogMode::IcuFcl,
        ..CombineCatalogOptions::new(&[], "en")
    })
    .expect("combine valid FCL catalogs");

    assert!(
        result
            .content
            .starts_with("%FCL1\tsource=en\tlocale=de\torder=collated\n")
    );
    assert!(result.content.contains("Hello\t\tHallo\n"));
    assert!(result.content.contains("New\t\tNeu\n"));
}

#[test]
fn combine_catalogs_use_first_preserves_existing_translations_and_adds_missing() {
    let existing = concat!("msgid \"Hello\"\n", "msgstr \"Hallo\"\n",);
    let template = concat!(
        "msgid \"Hello\"\n",
        "msgstr \"\"\n\n",
        "#: src/new.rs:7\n",
        "msgid \"New\"\n",
        "msgstr \"\"\n",
    );
    let inputs = [
        CatalogCombineInput::labeled(existing, "existing.po"),
        CatalogCombineInput::labeled(template, "messages.pot"),
    ];

    let result = combine_catalogs(CombineCatalogOptions {
        inputs: &inputs,
        source_locale: "en",
        locale: Some("de"),
        ..CombineCatalogOptions::new(&[], "en")
    })
    .expect("combine");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items.len(), 2);
    assert_eq!(parsed.items[0].msgid, "Hello");
    assert_eq!(parsed.items[0].msgstr[0], "Hallo");
    assert_eq!(parsed.items[1].msgid, "New");
    assert_eq!(parsed.items[1].msgstr[0], "");
    assert_eq!(
        parsed.items[1].references.as_slice(),
        vec!["src/new.rs"].as_slice()
    );
    assert_eq!(result.stats.inputs, 2);
    assert_eq!(result.stats.definitions, 3);
    assert_eq!(result.stats.selected, 2);
    assert_eq!(result.stats.conflicts_resolved, 0);
}

#[test]
fn combine_catalogs_use_last_overlays_conflicting_translation() {
    let first = "msgid \"Hello\"\nmsgstr \"Hallo\"\n";
    let second = "msgid \"Hello\"\nmsgstr \"Servus\"\n";
    let inputs = [
        CatalogCombineInput::labeled(first, "first.po"),
        CatalogCombineInput::labeled(second, "second.po"),
    ];

    let result = combine_catalogs(CombineCatalogOptions {
        inputs: &inputs,
        source_locale: "en",
        locale: Some("de"),
        conflict_strategy: CatalogConflictStrategy::UseLast,
        ..CombineCatalogOptions::new(&[], "en")
    })
    .expect("combine");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items[0].msgstr[0], "Servus");
    assert_eq!(result.stats.conflicts_resolved, 1);
    assert!(result.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "combine.conflict_resolved"
            && diagnostic.msgid.as_deref() == Some("Hello")
    }));
}

#[test]
fn combine_catalogs_use_last_keeps_existing_translation_when_later_template_is_empty() {
    let first = "msgid \"Hello\"\nmsgstr \"Hallo\"\n";
    let second = "msgid \"Hello\"\nmsgstr \"\"\n";
    let inputs = [
        CatalogCombineInput::labeled(first, "first.po"),
        CatalogCombineInput::labeled(second, "template.pot"),
    ];

    let result = combine_catalogs(CombineCatalogOptions {
        inputs: &inputs,
        source_locale: "en",
        locale: Some("de"),
        conflict_strategy: CatalogConflictStrategy::UseLast,
        ..CombineCatalogOptions::new(&[], "en")
    })
    .expect("combine");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items[0].msgstr[0], "Hallo");
    assert_eq!(result.stats.conflicts_resolved, 0);
    assert!(result.diagnostics.is_empty());
}

#[test]
fn combine_catalogs_use_first_fills_empty_translation_from_later_definition() {
    let first = "msgid \"Hello\"\nmsgstr \"\"\n";
    let second = "msgid \"Hello\"\nmsgstr \"Hallo\"\n";
    let inputs = [
        CatalogCombineInput::labeled(first, "template.pot"),
        CatalogCombineInput::labeled(second, "de.po"),
    ];

    let result = combine_catalogs(CombineCatalogOptions {
        inputs: &inputs,
        source_locale: "en",
        locale: Some("de"),
        ..CombineCatalogOptions::new(&[], "en")
    })
    .expect("combine");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items[0].msgstr[0], "Hallo");
    assert_eq!(result.stats.conflicts_resolved, 0);
    assert!(result.diagnostics.is_empty());
}

#[test]
fn combine_catalogs_error_rejects_conflicting_translation() {
    let inputs = [
        CatalogCombineInput::new("msgid \"Hello\"\nmsgstr \"Hallo\"\n"),
        CatalogCombineInput::new("msgid \"Hello\"\nmsgstr \"Servus\"\n"),
    ];

    let error = combine_catalogs(CombineCatalogOptions {
        inputs: &inputs,
        source_locale: "en",
        locale: Some("de"),
        conflict_strategy: CatalogConflictStrategy::Error,
        ..CombineCatalogOptions::new(&[], "en")
    })
    .expect_err("conflict");

    assert!(matches!(error, ApiError::Conflict(message) if message.contains("Hello")));
}

#[test]
fn combine_catalogs_selection_rules_filter_by_definition_count() {
    let first = concat!(
        "msgid \"shared\"\n",
        "msgstr \"one\"\n\n",
        "msgid \"only-first\"\n",
        "msgstr \"one\"\n",
    );
    let second = concat!(
        "msgid \"shared\"\n",
        "msgstr \"one\"\n\n",
        "msgid \"only-second\"\n",
        "msgstr \"two\"\n",
    );
    let inputs = [
        CatalogCombineInput::new(first),
        CatalogCombineInput::new(second),
    ];

    let unique = combine_catalogs(CombineCatalogOptions {
        inputs: &inputs,
        source_locale: "en",
        locale: Some("de"),
        selection: CatalogCombineSelection::Unique,
        ..CombineCatalogOptions::new(&[], "en")
    })
    .expect("unique combine");
    let unique = parse_po(&unique.content).expect("parse unique");
    assert_eq!(unique.items.len(), 2);
    assert!(unique.items.iter().all(|item| item.msgid != "shared"));

    let common = combine_catalogs(CombineCatalogOptions {
        inputs: &inputs,
        source_locale: "en",
        locale: Some("de"),
        selection: CatalogCombineSelection::MoreThan(1),
        ..CombineCatalogOptions::new(&[], "en")
    })
    .expect("common combine");
    let common = parse_po(&common.content).expect("parse common");
    assert_eq!(common.items.len(), 1);
    assert_eq!(common.items[0].msgid, "shared");

    let less_than_two = combine_catalogs(CombineCatalogOptions {
        inputs: &inputs,
        source_locale: "en",
        locale: Some("de"),
        selection: CatalogCombineSelection::LessThan(2),
        ..CombineCatalogOptions::new(&[], "en")
    })
    .expect("less-than combine");
    let less_than_two = parse_po(&less_than_two.content).expect("parse less-than");
    assert_eq!(less_than_two.items.len(), 2);
    assert!(
        less_than_two
            .items
            .iter()
            .all(|item| item.msgid != "shared")
    );
}

#[test]
fn combine_catalogs_treats_contexts_as_distinct_identities() {
    let first = concat!(
        "msgctxt \"button\"\n",
        "msgid \"Open\"\n",
        "msgstr \"Oeffnen\"\n",
    );
    let second = concat!(
        "msgctxt \"menu\"\n",
        "msgid \"Open\"\n",
        "msgstr \"Aufmachen\"\n",
    );
    let inputs = [
        CatalogCombineInput::new(first),
        CatalogCombineInput::new(second),
    ];

    let result = combine_catalogs(CombineCatalogOptions {
        inputs: &inputs,
        source_locale: "en",
        locale: Some("de"),
        ..CombineCatalogOptions::new(&[], "en")
    })
    .expect("combine");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items.len(), 2);
    assert_eq!(parsed.items[0].msgctxt.as_deref(), Some("button"));
    assert_eq!(parsed.items[1].msgctxt.as_deref(), Some("menu"));
}

#[test]
fn combine_catalogs_rejects_singular_plural_shape_conflicts() {
    let singular = "msgid \"item\"\nmsgstr \"Ding\"\n";
    let plural = concat!(
        "msgid \"item\"\n",
        "msgid_plural \"items\"\n",
        "msgstr[0] \"Ding\"\n",
        "msgstr[1] \"Dinge\"\n",
    );
    let inputs = [
        CatalogCombineInput::new(singular),
        CatalogCombineInput::new(plural),
    ];

    let error = combine_catalogs(CombineCatalogOptions {
        inputs: &inputs,
        source_locale: "en",
        locale: Some("de"),
        mode: CatalogMode::GettextPo,
        ..CombineCatalogOptions::new(&[], "en")
    })
    .expect_err("shape conflict");

    assert!(matches!(error, ApiError::Conflict(message) if message.contains("shape")));
}

#[test]
fn combine_catalogs_skips_obsolete_by_default_and_can_include_them() {
    let active = "msgid \"active\"\nmsgstr \"aktiv\"\n";
    let obsolete = "#~ msgid \"old\"\n#~ msgstr \"alt\"\n";
    let inputs = [
        CatalogCombineInput::new(active),
        CatalogCombineInput::new(obsolete),
    ];

    let skipped = combine_catalogs(CombineCatalogOptions {
        inputs: &inputs,
        source_locale: "en",
        locale: Some("de"),
        ..CombineCatalogOptions::new(&[], "en")
    })
    .expect("combine without obsolete");
    let skipped = parse_po(&skipped.content).expect("parse skipped");
    assert_eq!(skipped.items.len(), 1);
    assert_eq!(skipped.items[0].msgid, "active");

    let included = combine_catalogs(CombineCatalogOptions {
        inputs: &inputs,
        source_locale: "en",
        locale: Some("de"),
        include_obsolete: true,
        ..CombineCatalogOptions::new(&[], "en")
    })
    .expect("combine with obsolete");
    let included = parse_po(&included.content).expect("parse included");
    assert_eq!(included.items.len(), 2);
    assert!(included.items.iter().any(|item| item.obsolete));
}

#[test]
fn combine_catalogs_merges_obsolete_keeping_earliest_since() {
    // The same obsolete entry in two inputs merges into one, keeping the earliest
    // `since` date.
    let a = "#~ #@ obsolete-since: 2026-06-01\n#~ msgid \"old\"\n#~ msgstr \"alt\"\n";
    let b = "#~ #@ obsolete-since: 2026-03-01\n#~ msgid \"old\"\n#~ msgstr \"alt\"\n";
    let inputs = [CatalogCombineInput::new(a), CatalogCombineInput::new(b)];

    let result = combine_catalogs(CombineCatalogOptions {
        inputs: &inputs,
        source_locale: "en",
        locale: Some("de"),
        include_obsolete: true,
        ..CombineCatalogOptions::new(&[], "en")
    })
    .expect("combine obsolete");
    let parsed = parse_po(&result.content).expect("parse");
    assert_eq!(parsed.items.len(), 1);
    assert!(parsed.items[0].obsolete);
    assert!(result.content.contains("obsolete-since: 2026-03-01"));
    assert!(!result.content.contains("2026-06-01"));
}

#[test]
fn combine_catalogs_gettext_compat_preserves_plural_slots() {
    let existing = concat!(
        "msgid \"\"\n",
        "msgstr \"\"\n",
        "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\n",
        "msgid \"item\"\n",
        "msgid_plural \"items\"\n",
        "msgstr[0] \"Ding\"\n",
        "msgstr[1] \"Dinge\"\n",
    );
    let template = concat!(
        "msgid \"item\"\n",
        "msgid_plural \"items\"\n",
        "msgstr[0] \"\"\n",
        "msgstr[1] \"\"\n",
    );
    let inputs = [
        CatalogCombineInput::new(existing),
        CatalogCombineInput::new(template),
    ];

    let result = combine_catalogs(CombineCatalogOptions {
        inputs: &inputs,
        source_locale: "en",
        locale: Some("de"),
        mode: CatalogMode::GettextPo,
        ..CombineCatalogOptions::new(&[], "en")
    })
    .expect("combine");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items[0].msgid, "item");
    assert_eq!(parsed.items[0].msgid_plural.as_deref(), Some("items"));
    assert_eq!(parsed.items[0].msgstr[0], "Ding");
    assert_eq!(parsed.items[0].msgstr[1], "Dinge");
}

#[test]
fn combine_catalogs_use_last_preserves_non_empty_plural_slots_when_later_slot_is_empty() {
    let existing = concat!(
        "msgid \"\"\n",
        "msgstr \"\"\n",
        "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\n",
        "msgid \"item\"\n",
        "msgid_plural \"items\"\n",
        "msgstr[0] \"Ding\"\n",
        "msgstr[1] \"Dinge\"\n",
    );
    let overlay = concat!(
        "msgid \"item\"\n",
        "msgid_plural \"items\"\n",
        "msgstr[0] \"Teil\"\n",
        "msgstr[1] \"\"\n",
    );
    let inputs = [
        CatalogCombineInput::labeled(existing, "existing.po"),
        CatalogCombineInput::labeled(overlay, "overlay.po"),
    ];

    let result = combine_catalogs(CombineCatalogOptions {
        inputs: &inputs,
        source_locale: "en",
        locale: Some("de"),
        mode: CatalogMode::GettextPo,
        conflict_strategy: CatalogConflictStrategy::UseLast,
        ..CombineCatalogOptions::new(&[], "en")
    })
    .expect("combine");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items[0].msgstr[0], "Teil");
    assert_eq!(parsed.items[0].msgstr[1], "Dinge");
    assert_eq!(result.stats.conflicts_resolved, 1);
}

const LONG_TRANSLATION: &str = "This translation is deliberately much longer than eighty characters so the default serializer folds it.";

#[test]
fn update_catalog_can_disable_po_width_folding() {
    let existing = format!(
        "msgid \"Hello\"\nmsgstr \"{LONG_TRANSLATION}\"\n\nmsgid \"Multi\"\nmsgstr \"line1\\nline2\"\n"
    );
    let build_options = |render: RenderOptions<'static>| UpdateCatalogOptions {
        locale: Some("de"),
        render,
        input: structured_input(vec![
            ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            }),
            ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Multi".to_owned(),
                ..ExtractedSingularMessage::default()
            }),
        ]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    };

    let folded = update_catalog(UpdateCatalogOptions {
        existing: Some(&existing),
        ..build_options(RenderOptions::default())
    })
    .expect("update with default folding");
    assert!(
        !folded
            .content
            .contains(&format!("msgstr \"{LONG_TRANSLATION}\"")),
        "default output should fold the long translation across lines"
    );

    let unfolded = update_catalog(UpdateCatalogOptions {
        existing: Some(&existing),
        ..build_options(
            RenderOptions::default()
                .with_po_serialize_options(SerializeOptions::default().with_fold_length(0)),
        )
    })
    .expect("update without width folding");
    assert!(
        unfolded
            .content
            .contains(&format!("msgstr \"{LONG_TRANSLATION}\"\n")),
        "disabling width folding must keep the long translation on one line"
    );
    assert!(
        unfolded.content.contains("msgstr \"line1\\n\"\n\"line2\""),
        "embedded newlines must keep valid multiline PO syntax without width folding"
    );

    let reparsed = parse_po(&unfolded.content).expect("reparse unfolded output");
    let multi = reparsed
        .items
        .iter()
        .find(|item| item.msgid == "Multi")
        .expect("multi entry");
    assert_eq!(multi.msgstr[0], "line1\nline2");
}

#[test]
fn combine_catalogs_can_disable_po_width_folding() {
    let existing = format!("msgid \"Hello\"\nmsgstr \"{LONG_TRANSLATION}\"\n");
    let inputs = [CatalogCombineInput::labeled(&existing, "existing.po")];

    let result = combine_catalogs(
        CombineCatalogOptions {
            inputs: &inputs,
            source_locale: "en",
            locale: Some("de"),
            ..CombineCatalogOptions::new(&[], "en")
        }
        .with_po_serialize_options(SerializeOptions::default().with_fold_length(0)),
    )
    .expect("combine without width folding");

    assert!(
        result
            .content
            .contains(&format!("msgstr \"{LONG_TRANSLATION}\"\n")),
        "combine output must respect the disabled width folding"
    );
}

#[test]
fn combine_catalog_files_can_disable_po_width_folding() {
    let temp_dir = unique_catalog_temp_dir("combine-files-fold");
    let ours = temp_dir.join("ours.po");
    fs::write(
        &ours,
        format!("msgid \"Hello\"\nmsgstr \"{LONG_TRANSLATION}\"\n"),
    )
    .expect("write ours");
    let input_paths = vec![ours.clone()];

    let result = combine_catalog_files(
        CombineCatalogFilesOptions {
            input_paths: &input_paths,
            output_path: &ours,
            locale: Some("de"),
            ..CombineCatalogFilesOptions::new(&[], &ours, "en")
        }
        .with_po_serialize_options(SerializeOptions::default().with_fold_length(0)),
    )
    .expect("combine files without width folding");

    let output = fs::read_to_string(&result.output_path).expect("read output");
    assert!(
        output.contains(&format!("msgstr \"{LONG_TRANSLATION}\"\n")),
        "file combine output must respect the disabled width folding"
    );

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn combine_catalog_files_merges_po_inputs_with_use_first_and_preserves_first_header() {
    let temp_dir = unique_catalog_temp_dir("combine-files-po");
    let ours = temp_dir.join("ours.po");
    let theirs = temp_dir.join("theirs.po");
    fs::write(
        &ours,
        concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Language: de\\n\"\n\n",
            "msgid \"Hello\"\n",
            "msgstr \"Hallo\"\n",
        ),
    )
    .expect("write ours");
    fs::write(
        &theirs,
        concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Language: fr\\n\"\n\n",
            "msgid \"Hello\"\n",
            "msgstr \"Bonjour\"\n\n",
            "msgid \"New\"\n",
            "msgstr \"Neu\"\n",
        ),
    )
    .expect("write theirs");
    let input_paths = vec![ours.clone(), theirs];

    let result = combine_catalog_files(CombineCatalogFilesOptions {
        input_paths: &input_paths,
        output_path: &ours,
        locale: Some("de"),
        ..CombineCatalogFilesOptions::new(&[], &ours, "en")
    })
    .expect("combine files");

    assert_eq!(result.output_path, ours);
    assert_eq!(result.format, CatalogFileFormat::Po);
    let parsed = parse_po(&fs::read_to_string(&result.output_path).expect("read output"))
        .expect("parse output");
    assert_eq!(
        parsed
            .headers
            .iter()
            .find(|header| header.key == "Language")
            .map(|header| header.value.as_str()),
        Some("de")
    );
    let hello = parsed
        .items
        .iter()
        .find(|item| item.msgid == "Hello")
        .expect("hello");
    assert_eq!(hello.msgstr[0], "Hallo");
    assert!(parsed.items.iter().any(|item| item.msgid == "New"));

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn combine_catalog_files_applies_default_collated_order_to_rendered_output() {
    let temp_dir = unique_catalog_temp_dir("combine-files-collated");
    let markup = temp_dir.join("markup.po");
    let placeholder = temp_dir.join("placeholder.po");
    let output = temp_dir.join("merged.po");
    fs::write(
        &markup,
        "msgid \"<0>Continue</0>\"\nmsgstr \"<0>Continue</0>\"\n",
    )
    .expect("write markup catalog");
    fs::write(
        &placeholder,
        "msgid \"{count, plural, one {#} other {#}}\"\n\
         msgstr \"{count, plural, one {#} other {#}}\"\n",
    )
    .expect("write placeholder catalog");
    let input_paths = vec![markup, placeholder];

    combine_catalog_files(
        CombineCatalogFilesOptions::new(&input_paths, &output, "en").with_locale("en"),
    )
    .expect("combine with default order");

    let parsed =
        parse_po(&fs::read_to_string(&output).expect("read output")).expect("parse output");
    assert_eq!(
        parsed
            .items
            .iter()
            .map(|item| item.msgid.as_str())
            .collect::<Vec<_>>(),
        vec!["{count, plural, one {#} other {#}}", "<0>Continue</0>"]
    );

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn combine_catalog_files_can_use_gettext_po_mode_for_classic_plural_slots() {
    let temp_dir = unique_catalog_temp_dir("combine-files-gettext-po");
    let existing = temp_dir.join("existing.po");
    let template = temp_dir.join("template.pot");
    let output = temp_dir.join("merged.po");
    fs::write(
        &existing,
        concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\n",
            "msgid \"item\"\n",
            "msgid_plural \"items\"\n",
            "msgstr[0] \"Ding\"\n",
            "msgstr[1] \"Dinge\"\n",
        ),
    )
    .expect("write existing");
    fs::write(
        &template,
        concat!(
            "msgid \"item\"\n",
            "msgid_plural \"items\"\n",
            "msgstr[0] \"\"\n",
            "msgstr[1] \"\"\n",
        ),
    )
    .expect("write template");
    let input_paths = vec![existing, template];

    combine_catalog_files(CombineCatalogFilesOptions {
        input_paths: &input_paths,
        output_path: &output,
        mode: Some(CatalogMode::GettextPo),
        locale: Some("de"),
        ..CombineCatalogFilesOptions::new(&[], &output, "en")
    })
    .expect("combine gettext po files");

    let parsed =
        parse_po(&fs::read_to_string(&output).expect("read output")).expect("parse output");
    assert_eq!(parsed.items[0].msgid, "item");
    assert_eq!(parsed.items[0].msgid_plural.as_deref(), Some("items"));
    assert_eq!(parsed.items[0].msgstr[0], "Ding");
    assert_eq!(parsed.items[0].msgstr[1], "Dinge");

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn combine_catalog_files_treats_contexts_as_distinct_and_skips_obsolete_entries() {
    let temp_dir = unique_catalog_temp_dir("combine-files-contexts");
    let ours = temp_dir.join("ours.po");
    let theirs = temp_dir.join("theirs.po");
    let output = temp_dir.join("merged.po");
    fs::write(
        &ours,
        concat!(
            "msgid \"Open\"\n",
            "msgstr \"Oeffnen\"\n\n",
            "msgctxt \"\"\n",
            "msgid \"Open\"\n",
            "msgstr \"Leer\"\n",
        ),
    )
    .expect("write ours");
    fs::write(
        &theirs,
        concat!(
            "msgctxt \"menu\"\n",
            "msgid \"Open\"\n",
            "msgstr \"Menue\"\n\n",
            "#~ msgid \"Old\"\n",
            "#~ msgstr \"Alt\"\n",
        ),
    )
    .expect("write theirs");
    let input_paths = vec![ours, theirs];

    combine_catalog_files(CombineCatalogFilesOptions {
        input_paths: &input_paths,
        output_path: &output,
        locale: Some("de"),
        ..CombineCatalogFilesOptions::new(&[], &output, "en")
    })
    .expect("combine files");

    let parsed =
        parse_po(&fs::read_to_string(&output).expect("read output")).expect("parse output");
    assert!(
        parsed
            .items
            .iter()
            .any(|item| item.msgid == "Open" && item.msgctxt.is_none())
    );
    assert!(
        parsed
            .items
            .iter()
            .any(|item| item.msgid == "Open" && item.msgctxt.as_deref() == Some(""))
    );
    assert!(
        parsed
            .items
            .iter()
            .any(|item| item.msgid == "Open" && item.msgctxt.as_deref() == Some("menu"))
    );
    assert!(!parsed.items.iter().any(|item| item.msgid == "Old"));

    let _ = fs::remove_dir_all(temp_dir);
}
#[test]
fn combine_catalog_files_leaves_output_unchanged_when_format_is_unsupported() {
    let temp_dir = unique_catalog_temp_dir("combine-files-unsupported");
    let ours = temp_dir.join("ours.txt");
    let theirs = temp_dir.join("theirs.txt");
    let output = temp_dir.join("merged.txt");
    fs::write(&ours, "msgid \"Hello\"\nmsgstr \"Hallo\"\n").expect("write ours");
    fs::write(&theirs, "msgid \"New\"\nmsgstr \"Neu\"\n").expect("write theirs");
    fs::write(&output, "unchanged").expect("write output");
    let input_paths = vec![ours, theirs];

    let error = combine_catalog_files(CombineCatalogFilesOptions {
        input_paths: &input_paths,
        output_path: &output,
        locale: Some("de"),
        ..CombineCatalogFilesOptions::new(&[], &output, "en")
    })
    .expect_err("unsupported format");

    assert!(matches!(error, ApiError::Unsupported(message) if message.contains("could not infer")));
    assert_eq!(
        fs::read_to_string(&output).expect("read output"),
        "unchanged"
    );

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn combine_catalog_files_accepts_explicit_format_for_extensionless_paths() {
    let temp_dir = unique_catalog_temp_dir("combine-files-explicit-format");
    let ours = temp_dir.join("ours");
    let theirs = temp_dir.join("theirs");
    let output = temp_dir.join("merged");
    fs::write(&ours, "msgid \"Hello\"\nmsgstr \"Hallo\"\n").expect("write ours");
    fs::write(&theirs, "msgid \"New\"\nmsgstr \"Neu\"\n").expect("write theirs");
    let input_paths = vec![ours, theirs];

    let result = combine_catalog_files(CombineCatalogFilesOptions {
        input_paths: &input_paths,
        output_path: &output,
        format: Some(CatalogFileFormat::Po),
        locale: Some("de"),
        ..CombineCatalogFilesOptions::new(&[], &output, "en")
    })
    .expect("combine files");

    assert_eq!(result.format, CatalogFileFormat::Po);
    let parsed =
        parse_po(&fs::read_to_string(&output).expect("read output")).expect("parse output");
    assert!(parsed.items.iter().any(|item| item.msgid == "Hello"));
    assert!(parsed.items.iter().any(|item| item.msgid == "New"));

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn combine_catalog_files_rejects_empty_input_paths() {
    let temp_dir = unique_catalog_temp_dir("combine-files-empty-inputs");
    let output = temp_dir.join("merged.po");

    let error = combine_catalog_files(CombineCatalogFilesOptions {
        input_paths: &[],
        output_path: &output,
        locale: Some("de"),
        ..CombineCatalogFilesOptions::new(&[], &output, "en")
    })
    .expect_err("empty inputs");

    assert!(matches!(
        error,
        ApiError::InvalidArguments(message) if message.contains("input_paths")
    ));
    assert!(!output.exists());

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn combine_catalog_files_rejects_mode_that_does_not_match_file_format() {
    let temp_dir = unique_catalog_temp_dir("combine-files-mode-mismatch");
    let input = temp_dir.join("ours.fcl");
    let output = temp_dir.join("merged.fcl");
    fs::write(&input, "%FCL1\tsource=en\nHello\t\tHallo\n").expect("write input");
    fs::write(&output, "unchanged").expect("write output");
    let input_paths = vec![input];

    let error = combine_catalog_files(CombineCatalogFilesOptions {
        input_paths: &input_paths,
        output_path: &output,
        mode: Some(CatalogMode::GettextPo),
        locale: Some("de"),
        ..CombineCatalogFilesOptions::new(&[], &output, "en")
    })
    .expect_err("mode mismatch");

    assert!(matches!(
        error,
        ApiError::InvalidArguments(message) if message.contains("does not match")
    ));
    assert_eq!(
        fs::read_to_string(&output).expect("read output"),
        "unchanged"
    );

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn combine_catalog_files_leaves_output_unchanged_when_input_read_fails() {
    let temp_dir = unique_catalog_temp_dir("combine-files-read-error");
    let missing = temp_dir.join("missing.po");
    let output = temp_dir.join("merged.po");
    fs::write(&output, "unchanged").expect("write output");
    let input_paths = vec![missing.clone()];

    let error = combine_catalog_files(CombineCatalogFilesOptions {
        input_paths: &input_paths,
        output_path: &output,
        locale: Some("de"),
        ..CombineCatalogFilesOptions::new(&[], &output, "en")
    })
    .expect_err("read error");

    assert_eq!(error.path(), Some(missing.as_path()));
    assert_eq!(
        fs::read_to_string(&output).expect("read output"),
        "unchanged"
    );

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn combine_catalog_files_leaves_output_unchanged_when_formats_are_mixed() {
    let temp_dir = unique_catalog_temp_dir("combine-files-mixed");
    let ours = temp_dir.join("ours.po");
    let theirs = temp_dir.join("theirs.fcl");
    let output = temp_dir.join("merged.po");
    fs::write(&ours, "msgid \"Hello\"\nmsgstr \"Hallo\"\n").expect("write ours");
    fs::write(&theirs, "%FCL1\tsource=en\nNew\t\tNeu\n").expect("write theirs");
    fs::write(&output, "unchanged").expect("write output");
    let input_paths = vec![ours, theirs];

    let error = combine_catalog_files(CombineCatalogFilesOptions {
        input_paths: &input_paths,
        output_path: &output,
        locale: Some("de"),
        ..CombineCatalogFilesOptions::new(&[], &output, "en")
    })
    .expect_err("mixed formats");

    assert!(matches!(error, ApiError::InvalidArguments(message) if message.contains("uses")));
    assert_eq!(
        fs::read_to_string(&output).expect("read output"),
        "unchanged"
    );

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn parse_catalog_requires_source_locale() {
    let error = parse_catalog(ParseCatalogOptions {
        content: "",
        source_locale: "",
        ..ParseCatalogOptions::new("", "en")
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
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
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

fn unique_catalog_temp_dir(name: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("ferrocat-po-{name}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn update_catalog_roundtrips_fcl_via_public_api() {
    // Exercises the FCL parse + stringify dispatch arms and the IcuFcl mode
    // mappings through the public catalog API.
    let existing = "%FCL1\tsource=en\tlocale=de\nHello\t\tHallo\tr=src/a.tsx:3\n";
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        existing: Some(existing),
        mode: CatalogMode::IcuFcl,
        input: structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
            msgid: "Hello".to_owned(),
            ..ExtractedSingularMessage::default()
        })]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update");

    assert!(result.content.starts_with("%FCL1\tsource=en"));
    assert!(result.content.contains("Hello\t\tHallo"));

    let parsed = parse_catalog(ParseCatalogOptions {
        content: &result.content,
        locale: Some("de"),
        source_locale: "en",
        mode: CatalogMode::IcuFcl,
        strict: false,
    })
    .expect("parse");
    assert_eq!(parsed.messages.len(), 1);
    assert_eq!(parsed.messages[0].msgid, "Hello");
}

#[test]
fn update_catalog_rejects_duplicate_serialized_fcl_identities() {
    let plural_id = "{count, plural, one {item} other {items}}";
    let error = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        mode: CatalogMode::IcuFcl,
        input: structured_input(vec![
            ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: plural_id.to_owned(),
                ..ExtractedSingularMessage::default()
            }),
            ExtractedMessage::Plural(ExtractedPluralMessage {
                msgid: "cart.items".to_owned(),
                source: PluralSource {
                    one: Some("item".to_owned()),
                    other: "items".to_owned(),
                },
                ..ExtractedPluralMessage::default()
            }),
        ]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect_err("duplicate serialized FCL identity");

    assert!(matches!(
        error,
        ApiError::Conflict(message)
            if message.contains("duplicate FCL identity")
                && message.contains(plural_id)
    ));
}

#[test]
fn update_catalog_file_keeps_existing_fcl_on_identity_conflict() {
    let directory = tempfile::tempdir().expect("tempdir");
    let target = directory.path().join("de.fcl");
    let original = "%FCL1\tsource=en\tlocale=de\torder=collated\n";
    fs::write(&target, original).expect("write existing FCL");
    let plural_id = "{count, plural, one {item} other {items}}";
    let options = UpdateCatalogOptions::new(
        "en",
        structured_input(vec![
            ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: plural_id.to_owned(),
                ..ExtractedSingularMessage::default()
            }),
            ExtractedMessage::Plural(ExtractedPluralMessage {
                msgid: "cart.items".to_owned(),
                source: PluralSource {
                    one: Some("item".to_owned()),
                    other: "items".to_owned(),
                },
                ..ExtractedPluralMessage::default()
            }),
        ]),
    )
    .with_locale("de")
    .with_mode(CatalogMode::IcuFcl);

    let error = update_catalog_file(
        UpdateCatalogFileOptions::new(&target, "en", CatalogUpdateInput::default())
            .with_options(options),
    )
    .expect_err("duplicate serialized FCL identity");

    assert!(matches!(
        error,
        ApiError::Conflict(message) if message.contains("duplicate FCL identity")
    ));
    assert_eq!(fs::read_to_string(&target).expect("read target"), original);
}

#[test]
fn update_catalog_applies_default_collated_order_to_fcl() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("en"),
        mode: CatalogMode::IcuFcl,
        input: structured_input(
            ["Zebra", "über", "Uber", "Apple", "apple", "Álgebra"]
                .into_iter()
                .map(|msgid| {
                    ExtractedMessage::Singular(ExtractedSingularMessage {
                        msgid: msgid.to_owned(),
                        ..ExtractedSingularMessage::default()
                    })
                })
                .collect(),
        ),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update FCL with collated order");

    assert!(
        result
            .content
            .starts_with("%FCL1\tsource=en\tlocale=en\torder=collated\n")
    );
    let parsed = parse_catalog(ParseCatalogOptions {
        content: &result.content,
        source_locale: "en",
        locale: Some("en"),
        mode: CatalogMode::IcuFcl,
        strict: false,
    })
    .expect("parse FCL output");
    assert_eq!(
        parsed
            .messages
            .iter()
            .map(|message| message.msgid.as_str())
            .collect::<Vec<_>>(),
        vec!["Álgebra", "apple", "Apple", "Uber", "über", "Zebra"]
    );
}

#[test]
fn update_catalog_fcl_respects_origin_render_options() {
    let input = structured_input(vec![ExtractedMessage::Singular(ExtractedSingularMessage {
        msgid: "Hello".to_owned(),
        origin: vec![CatalogOrigin {
            file: "src/app.rs".to_owned(),
            scope: None,
        }],
        ..ExtractedSingularMessage::default()
    })]);

    let without_origins = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        mode: CatalogMode::IcuFcl,
        render: RenderOptions {
            include_origins: false,
            ..RenderOptions::default()
        },
        input: input.clone(),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update without origins");
    assert!(!without_origins.content.contains("\tr="));

    // References render the file only; line numbers are never emitted.
    let with_origins = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        mode: CatalogMode::IcuFcl,
        input,
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update with origins");
    assert!(with_origins.content.contains("\tr=src/app.rs"));
    assert!(!with_origins.content.contains("\tr=src/app.rs:"));
}

#[test]
fn update_catalog_fcl_keeps_collated_storage_order_for_origin_strategy() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("en"),
        mode: CatalogMode::IcuFcl,
        render: RenderOptions::default().with_order_by(OrderBy::Origin),
        input: structured_input(vec![
            ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "<0>Continue</0>".to_owned(),
                origin: vec![CatalogOrigin {
                    file: "src/a.rs".to_owned(),
                    scope: None,
                }],
                ..ExtractedSingularMessage::default()
            }),
            ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "{count, plural, one {#} other {#}}".to_owned(),
                origin: vec![CatalogOrigin {
                    file: "src/z.rs".to_owned(),
                    scope: None,
                }],
                ..ExtractedSingularMessage::default()
            }),
        ]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update FCL with origin strategy");

    assert!(
        result
            .content
            .lines()
            .next()
            .unwrap_or_default()
            .contains("order=collated")
    );
    let ids = result
        .content
        .lines()
        .skip(1)
        .map(|line| line.split('\t').next().unwrap_or_default())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec!["{count, plural, one {#} other {#}}", "<0>Continue</0>"]
    );
}

#[test]
fn infers_fcl_file_format_from_extension() {
    let format = crate::CatalogFileFormat::infer_from_path(std::path::Path::new("messages.de.fcl"))
        .expect("infer .fcl");
    assert!(matches!(format, crate::CatalogFileFormat::Fcl));
    assert_eq!(format.default_mode(), CatalogMode::IcuFcl);
}

// --- opaque translator metadata round-trip (ADR 0027) ----------------------

/// PO fixture carrying every kind of per-entry metadata the update path has to
/// keep apart: translator-owned `#`, extractor-owned `#.`/`#:`, and flags.
const TRANSLATOR_METADATA_PO: &str = "# translator note\n\
     # second translator note\n\
     #. stale extractor note\n\
     #: src/Old.tsx\n\
     #, fuzzy, x-custom\n\
     msgctxt \"button\"\n\
     msgid \"Hello\"\n\
     msgstr \"Hallo\"\n";

fn extracted(msgid: &str, msgctxt: Option<&str>, comments: Vec<String>) -> ExtractedMessage {
    ExtractedMessage::Singular(ExtractedSingularMessage {
        msgid: msgid.to_owned(),
        msgctxt: msgctxt.map(str::to_owned),
        comments,
        ..ExtractedSingularMessage::default()
    })
}

#[test]
fn update_catalog_keeps_translator_comments_and_refreshes_extracted_comments() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        existing: Some(TRANSLATOR_METADATA_PO),
        input: structured_input(vec![extracted(
            "Hello",
            Some("button"),
            vec!["fresh extractor note".to_owned()],
        )]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update");

    // Translator comments stay translator comments; the extractor-owned note is
    // replaced by the freshly extracted one instead of being converted in kind.
    assert!(result.content.contains("# translator note\n"));
    assert!(result.content.contains("# second translator note\n"));
    assert!(result.content.contains("#. fresh extractor note\n"));
    assert!(!result.content.contains("stale extractor note"));
    assert!(!result.content.contains("#. translator note"));

    // The low-level parser sees them under the right comment kinds again.
    let parsed = parse_po(&result.content).expect("parse output");
    let item = &parsed.items[0];
    assert_eq!(
        item.comments.as_slice(),
        ["translator note", "second translator note"]
    );
    assert_eq!(item.extracted_comments.as_slice(), ["fresh extractor note"]);
}

#[test]
fn update_catalog_preserves_unknown_flags_verbatim() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        existing: Some(TRANSLATOR_METADATA_PO),
        input: structured_input(vec![extracted("Hello", Some("button"), Vec::new())]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update");

    // Update treats flags as opaque round-trip state: `fuzzy` and `x-custom`
    // both survive unchanged. Report classification is tested separately.
    assert!(result.content.contains("#, fuzzy, x-custom\n"));
    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items[0].flags.as_slice(), ["fuzzy", "x-custom"]);
    assert_eq!(parsed.items[0].msgstr[0], "Hallo");
}

#[test]
fn update_catalog_keys_opaque_metadata_by_msgctxt_and_msgid() {
    let existing = "# button note\n\
         #, x-button\n\
         msgctxt \"button\"\n\
         msgid \"Hello\"\n\
         msgstr \"Hallo (Knopf)\"\n\
         \n\
         # title note\n\
         #, x-title\n\
         msgctxt \"title\"\n\
         msgid \"Hello\"\n\
         msgstr \"Hallo (Titel)\"\n";

    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        existing: Some(existing),
        input: structured_input(vec![
            extracted("Hello", Some("button"), Vec::new()),
            extracted("Hello", Some("title"), Vec::new()),
        ]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update");

    let parsed = parse_po(&result.content).expect("parse output");
    let by_context = parsed
        .items
        .iter()
        .map(|item| {
            (
                item.msgctxt.clone().unwrap_or_default(),
                (item.comments.to_vec(), item.flags.to_vec()),
            )
        })
        .collect::<BTreeMap<_, _>>();

    assert_eq!(
        by_context["button"],
        (vec!["button note".to_owned()], vec!["x-button".to_owned()])
    );
    assert_eq!(
        by_context["title"],
        (vec!["title note".to_owned()], vec!["x-title".to_owned()])
    );
}

#[test]
fn update_catalog_carries_opaque_metadata_through_obsolete_and_revival() {
    // The entry disappears from the extractor input and becomes obsolete; the
    // opaque metadata rides along and renders under the `#~` prefix.
    let marked = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        existing: Some(TRANSLATOR_METADATA_PO),
        input: structured_input(vec![]),
        now: Some("2026-07-30"),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("mark obsolete");

    assert!(marked.content.contains("#~ # translator note\n"));
    assert!(marked.content.contains("#~ #, fuzzy, x-custom\n"));

    // Reviving the identity keeps the same metadata and drops the obsolete state.
    let revived = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        existing: Some(&marked.content),
        input: structured_input(vec![extracted("Hello", Some("button"), Vec::new())]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("revive");

    assert!(!revived.content.contains("#~"));
    assert!(revived.content.contains("# translator note\n"));
    assert!(revived.content.contains("#, fuzzy, x-custom\n"));
    let parsed = parse_po(&revived.content).expect("parse revived");
    assert_eq!(parsed.items[0].flags.as_slice(), ["fuzzy", "x-custom"]);
    assert!(!parsed.items[0].obsolete);
}

#[test]
fn update_catalog_is_byte_stable_when_repeated_with_opaque_metadata() {
    let input = || {
        structured_input(vec![extracted(
            "Hello",
            Some("button"),
            vec!["fresh extractor note".to_owned()],
        )])
    };
    let first = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        existing: Some(TRANSLATOR_METADATA_PO),
        input: input(),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("first update");

    let second = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        existing: Some(&first.content),
        input: input(),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("second update");

    assert_eq!(first.content, second.content);
    assert!(!second.updated);
}

#[test]
fn update_catalog_does_not_inherit_opaque_metadata_for_new_entries() {
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        existing: Some(TRANSLATOR_METADATA_PO),
        input: structured_input(vec![
            extracted("Hello", Some("button"), Vec::new()),
            // Same msgid with a different context, plus an unrelated identity.
            extracted("Hello", Some("title"), Vec::new()),
            extracted("Goodbye", None, Vec::new()),
        ]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update");

    let parsed = parse_po(&result.content).expect("parse output");
    assert_eq!(parsed.items.len(), 3);
    for item in &parsed.items {
        let carries_metadata = item.msgctxt.as_deref() == Some("button");
        assert_eq!(!item.comments.is_empty(), carries_metadata);
        assert_eq!(!item.flags.is_empty(), carries_metadata);
    }
}

#[test]
fn combine_catalogs_keeps_the_winning_definition_opaque_metadata() {
    let template = "#. extracted note\nmsgid \"Hello\"\nmsgstr \"\"\n";
    let translated = "# translator note\n#, x-custom\nmsgid \"Hello\"\nmsgstr \"Hallo\"\n";
    let inputs = [
        CatalogCombineInput::labeled(template, "messages.pot"),
        CatalogCombineInput::labeled(translated, "de.po"),
    ];

    let combined = combine_catalogs(CombineCatalogOptions {
        locale: Some("de"),
        ..CombineCatalogOptions::new(&inputs, "en")
    })
    .expect("combine");

    // `de.po` supplies the winning translation, so its opaque metadata comes
    // with it, while extractor-owned notes still cumulate across inputs.
    let parsed = parse_po(&combined.content).expect("parse combined");
    assert_eq!(parsed.items[0].msgstr[0], "Hallo");
    assert_eq!(parsed.items[0].comments.as_slice(), ["translator note"]);
    assert_eq!(parsed.items[0].flags.as_slice(), ["x-custom"]);
    assert_eq!(
        parsed.items[0].extracted_comments.as_slice(),
        ["extracted note"]
    );
}

#[test]
fn combine_catalogs_does_not_union_opaque_metadata_across_inputs() {
    let ours = "# ours note\n#, x-ours\nmsgid \"Hello\"\nmsgstr \"Hallo\"\n";
    let theirs = "# theirs note\n#, x-theirs\nmsgid \"Hello\"\nmsgstr \"Hallo\"\n";
    let inputs = [
        CatalogCombineInput::labeled(ours, "ours.po"),
        CatalogCombineInput::labeled(theirs, "theirs.po"),
    ];

    let combined = combine_catalogs(CombineCatalogOptions {
        locale: Some("de"),
        ..CombineCatalogOptions::new(&inputs, "en")
    })
    .expect("combine");

    let parsed = parse_po(&combined.content).expect("parse combined");
    assert_eq!(parsed.items[0].comments.as_slice(), ["ours note"]);
    assert_eq!(parsed.items[0].flags.as_slice(), ["x-ours"]);
}

#[test]
fn combine_catalogs_does_not_adopt_metadata_from_a_losing_definition() {
    // `UseFirst` keeps the first non-empty translation. The later definition
    // loses the conflict, so its opaque metadata must not attach to a value it
    // does not own -- even though the winner carries no metadata of its own.
    let ours = "msgid \"Hello\"\nmsgstr \"Hallo\"\n";
    let theirs = "# theirs note\n#, x-theirs\nmsgid \"Hello\"\nmsgstr \"Servus\"\n";
    let inputs = [
        CatalogCombineInput::labeled(ours, "ours.po"),
        CatalogCombineInput::labeled(theirs, "theirs.po"),
    ];

    let combined = combine_catalogs(CombineCatalogOptions {
        locale: Some("de"),
        ..CombineCatalogOptions::new(&inputs, "en")
    })
    .expect("combine");

    let parsed = parse_po(&combined.content).expect("parse combined");
    assert_eq!(parsed.items[0].msgstr[0], "Hallo");
    assert!(parsed.items[0].comments.is_empty());
    assert!(parsed.items[0].flags.is_empty());
}

#[test]
fn combine_catalogs_clears_metadata_for_composite_plural_values() {
    // Each input fills a different plural slot, so the final value is a
    // composite no single definition owns. Like the machine block, the opaque
    // metadata clears instead of crediting either input.
    let ours = concat!(
        "# ours note\n",
        "#, x-ours\n",
        "msgid \"book\"\n",
        "msgid_plural \"books\"\n",
        "msgstr[0] \"Buch\"\n",
        "msgstr[1] \"\"\n",
    );
    let theirs = concat!(
        "# theirs note\n",
        "#, x-theirs\n",
        "msgid \"book\"\n",
        "msgid_plural \"books\"\n",
        "msgstr[0] \"\"\n",
        "msgstr[1] \"Buecher\"\n",
    );
    let inputs = [
        CatalogCombineInput::labeled(ours, "ours.po"),
        CatalogCombineInput::labeled(theirs, "theirs.po"),
    ];

    let combined = combine_catalogs(CombineCatalogOptions {
        locale: Some("de"),
        mode: CatalogMode::GettextPo,
        ..CombineCatalogOptions::new(&inputs, "en")
    })
    .expect("combine");

    let parsed = parse_po(&combined.content).expect("parse combined");
    assert_eq!(parsed.items[0].msgstr[0], "Buch");
    assert_eq!(parsed.items[0].msgstr[1], "Buecher");
    assert!(parsed.items[0].comments.is_empty());
    assert!(parsed.items[0].flags.is_empty());
}

#[test]
fn combine_catalog_files_keeps_the_winning_definition_opaque_metadata() {
    let temp_dir = unique_catalog_temp_dir("combine-files-opaque-metadata");
    let template = temp_dir.join("messages.pot");
    let translated = temp_dir.join("de.po");
    let output = temp_dir.join("merged.po");
    fs::write(
        &template,
        "#. extracted note\nmsgid \"Hello\"\nmsgstr \"\"\n",
    )
    .expect("write pot");
    fs::write(
        &translated,
        "# translator note\n#, x-custom\nmsgid \"Hello\"\nmsgstr \"Hallo\"\n",
    )
    .expect("write po");
    let input_paths = vec![template, translated];

    combine_catalog_files(CombineCatalogFilesOptions {
        input_paths: &input_paths,
        output_path: &output,
        locale: Some("de"),
        ..CombineCatalogFilesOptions::new(&[], &output, "en")
    })
    .expect("combine files");

    let parsed =
        parse_po(&fs::read_to_string(&output).expect("read output")).expect("parse output");
    assert_eq!(parsed.items[0].comments.as_slice(), ["translator note"]);
    assert_eq!(parsed.items[0].flags.as_slice(), ["x-custom"]);

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn update_catalog_roundtrips_opaque_metadata_through_fcl_tags() {
    let existing = "%FCL1\tsource=en\tlocale=de\torder=collated\n\
         Hello\tbutton\tHallo\tc=extracted note\ttc=translator note\tf=fuzzy\tf=x-custom\n";
    let result = update_catalog(UpdateCatalogOptions {
        source_locale: "en",
        locale: Some("de"),
        existing: Some(existing),
        mode: CatalogMode::IcuFcl,
        input: structured_input(vec![extracted(
            "Hello",
            Some("button"),
            vec!["extracted note".to_owned()],
        )]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })
    .expect("update");

    assert_eq!(result.content, existing);
    assert!(!result.updated);
}
