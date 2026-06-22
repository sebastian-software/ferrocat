use ferrocat_icu::{IcuArgumentKind, IcuDiagnosticSeverity, IcuFormatter, IcuFormatterSupport};

use super::*;

#[test]
fn compile_catalog_returns_empty_catalog_for_empty_input() {
    let normalized = normalized_catalog("", Some("de"), PluralEncoding::Icu);
    let compiled = normalized
        .compile(&CompileCatalogOptions::default())
        .expect("compile");

    assert!(compiled.is_empty());
    assert_eq!(compiled.len(), 0);
    assert!(compiled.get("missing").is_none());
}

#[test]
fn compile_catalog_preserves_singular_translation_and_source_key() {
    let normalized = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );
    let compiled = normalized
        .compile(&CompileCatalogOptions::default())
        .expect("compile");

    let (_, message) = compiled.iter().next().expect("compiled message");
    assert_eq!(message.source_key, CatalogMessageKey::new("Hello", None));
    assert!(matches!(
        &message.translation,
        CompiledTranslation::Singular(value) if value == "Hallo"
    ));
    assert_eq!(compiled.get(&message.key), Some(message));
}

#[test]
fn compile_catalog_artifact_matches_between_po_and_ndjson_storage() {
    let po_requested = normalized_catalog(
        concat!(
            "msgid \"About us\"\n",
            "msgstr \"Ueber uns\"\n\n",
            "msgid \"{count, plural, one {# file} other {# files}}\"\n",
            "msgstr \"{count, plural, one {# Datei} other {# Dateien}}\"\n",
        ),
        Some("de"),
        PluralEncoding::Icu,
    );
    let ndjson_requested = normalized_ndjson_catalog(
        concat!(
            "---\n",
            "format: ferrocat.ndjson.v1\n",
            "locale: de\n",
            "source_locale: en\n",
            "---\n",
            "{\"id\":\"About us\",\"str\":\"Ueber uns\"}\n",
            "{\"id\":\"{count, plural, one {# file} other {# files}}\",\"str\":\"{count, plural, one {# Datei} other {# Dateien}}\"}\n",
        ),
        Some("de"),
    );
    let source = normalized_ndjson_catalog(
        concat!(
            "---\n",
            "format: ferrocat.ndjson.v1\n",
            "locale: en\n",
            "source_locale: en\n",
            "---\n",
            "{\"id\":\"About us\",\"str\":\"About us\"}\n",
            "{\"id\":\"{count, plural, one {# file} other {# files}}\",\"str\":\"{count, plural, one {# file} other {# files}}\"}\n",
        ),
        Some("en"),
    );

    let po_artifact = compile_catalog_artifact(
        &[&po_requested, &source],
        &CompileCatalogArtifactOptions::new("de", "en"),
    )
    .expect("compile po artifact");
    let ndjson_artifact = compile_catalog_artifact(
        &[&ndjson_requested, &source],
        &CompileCatalogArtifactOptions::new("de", "en"),
    )
    .expect("compile ndjson artifact");

    assert_eq!(po_artifact.messages, ndjson_artifact.messages);
    assert_eq!(po_artifact.missing, ndjson_artifact.missing);
    assert_eq!(po_artifact.diagnostics, ndjson_artifact.diagnostics);
}

#[test]
fn compile_catalog_changes_key_when_context_changes() {
    let without_context = compiled_key("Save", None);
    let with_context = compiled_key("Save", Some("menu"));
    let repeated = compiled_key("Save", None);

    assert_eq!(without_context, repeated);
    assert_ne!(without_context, with_context);
    assert_eq!(without_context.len(), 11);
    assert!(
        without_context
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    );
}

#[test]
fn compile_catalog_changes_key_when_msgid_changes() {
    let left = compiled_key("Save", None);
    let right = compiled_key("Store", None);

    assert_ne!(left, right);
}

#[test]
fn compiled_key_matches_internal_ferrocat_v1_contract() {
    let public = compiled_key("Save", Some("menu"));
    let internal = compiled_key_for(
        CompiledKeyStrategy::FerrocatV1,
        &CatalogMessageKey::new("Save", Some("menu".to_owned())),
    );

    assert_eq!(public, internal);
}

#[test]
fn compiled_key_matches_compiled_catalog_entries() {
    let normalized = normalized_catalog(
        "msgctxt \"menu\"\nmsgid \"Save\"\nmsgstr \"Speichern\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );
    let compiled = normalized
        .compile(&CompileCatalogOptions::default())
        .expect("compile");
    let expected = compiled_key("Save", Some("menu"));

    let (actual_key, message) = compiled.iter().next().expect("compiled message");

    assert_eq!(actual_key, expected);
    assert_eq!(message.key, expected);
}

#[test]
fn compile_catalog_preserves_plural_translation_shape() {
    let normalized = normalized_catalog(
        concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Language: ru\\n\"\n",
            "\"Plural-Forms: nplurals=3; plural=(n%10==1 && n%100!=11 ? 0 : ",
            "n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2);\\n\"\n\n",
            "msgid \"day\"\n",
            "msgid_plural \"days\"\n",
            "msgstr[0] \"den\"\n",
            "msgstr[1] \"dnya\"\n",
            "msgstr[2] \"dney\"\n",
        ),
        Some("ru"),
        PluralEncoding::Gettext,
    );
    let compiled = normalized
        .compile(&CompileCatalogOptions {
            semantics: CatalogSemantics::GettextCompat,
            ..CompileCatalogOptions::default()
        })
        .expect("compile");

    let (_, message) = compiled.iter().next().expect("compiled message");
    match &message.translation {
        CompiledTranslation::Plural(values) => {
            assert_eq!(values.get("one").map(String::as_str), Some("den"));
            assert_eq!(values.get("few").map(String::as_str), Some("dnya"));
            assert!(values.values().any(|value| value == "dney"));
        }
        other => panic!("expected plural translation, got {other:?}"),
    }
}

#[test]
fn compile_catalog_keeps_empty_source_values_by_default() {
    let normalized = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let compiled = normalized
        .compile(&CompileCatalogOptions::default())
        .expect("compile");

    let (_, message) = compiled.iter().next().expect("compiled message");
    assert!(matches!(
        &message.translation,
        CompiledTranslation::Singular(value) if value.is_empty()
    ));
}

#[test]
fn compile_catalog_can_fill_source_values_when_requested() {
    let normalized = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let compiled = normalized
        .compile(&CompileCatalogOptions {
            source_fallback: true,
            source_locale: Some("en"),
            ..CompileCatalogOptions::default()
        })
        .expect("compile");

    let (_, message) = compiled.iter().next().expect("compiled message");
    assert!(matches!(
        &message.translation,
        CompiledTranslation::Singular(value) if value == "Hello"
    ));
}

#[test]
fn compile_catalog_requires_source_locale_when_source_fallback_is_enabled() {
    let normalized = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let error = normalized
        .compile(&CompileCatalogOptions {
            source_fallback: true,
            source_locale: None,
            ..CompileCatalogOptions::default()
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
fn compile_catalog_reports_key_collisions() {
    let normalized = normalized_catalog(
        concat!(
            "msgid \"Hello\"\n",
            "msgstr \"Hallo\"\n\n",
            "msgctxt \"menu\"\n",
            "msgid \"Save\"\n",
            "msgstr \"Speichern\"\n",
        ),
        Some("de"),
        PluralEncoding::Icu,
    );
    let error = normalized
        .compile_with_key_generator(&CompileCatalogOptions::default(), |_, _| {
            "fc1_collision".to_owned()
        })
        .expect_err("collision");

    match error {
        ApiError::Conflict(message) => {
            assert!(message.contains("Hello"));
            assert!(message.contains("Save"));
            assert!(message.contains("collision"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn compile_catalog_artifact_returns_requested_locale_message_map() {
    let source = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hello\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );

    let artifact = compile_catalog_artifact(
        &[&requested, &source],
        &CompileCatalogArtifactOptions::new("de", "en"),
    )
    .expect("compile artifact");

    let key = compiled_key_for(
        CompiledKeyStrategy::FerrocatV1,
        &CatalogMessageKey::new("Hello", None),
    );
    assert_eq!(
        artifact.messages.get(&key).map(String::as_str),
        Some("Hallo")
    );
    assert!(artifact.missing.is_empty());
    assert!(artifact.diagnostics.is_empty());
}

#[test]
fn compile_catalog_artifact_synthesizes_plural_icu_strings() {
    let source = normalized_catalog(
        concat!(
            "msgid \"{count, plural, one {# item} other {# items}}\"\n",
            "msgstr \"{count, plural, one {# item} other {# items}}\"\n",
        ),
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        concat!(
            "msgid \"{count, plural, one {# item} other {# items}}\"\n",
            "msgstr \"{count, plural, one {# Artikel} other {# Artikel}}\"\n",
        ),
        Some("de"),
        PluralEncoding::Icu,
    );

    let artifact = compile_catalog_artifact(
        &[&requested, &source],
        &CompileCatalogArtifactOptions::new("de", "en"),
    )
    .expect("compile artifact");

    let key = compiled_key_for(
        CompiledKeyStrategy::FerrocatV1,
        &CatalogMessageKey::new("{count, plural, one {# item} other {# items}}", None),
    );
    assert_eq!(
        artifact.messages.get(&key).map(String::as_str),
        Some("{count, plural, one {# Artikel} other {# Artikel}}")
    );
}

#[test]
fn compile_catalog_artifact_uses_fallback_chain_before_source_locale() {
    let source = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hello\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );
    let first_fallback = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Bonjour\"\n",
        Some("fr"),
        PluralEncoding::Icu,
    );
    let second_fallback = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Ciao\"\n",
        Some("it"),
        PluralEncoding::Icu,
    );

    let artifact = compile_catalog_artifact(
        &[&requested, &first_fallback, &second_fallback, &source],
        &CompileCatalogArtifactOptions {
            fallback_chain: &["fr".to_owned(), "it".to_owned()],
            source_fallback: true,
            ..CompileCatalogArtifactOptions::new("de", "en")
        },
    )
    .expect("compile artifact");

    let key = compiled_key_for(
        CompiledKeyStrategy::FerrocatV1,
        &CatalogMessageKey::new("Hello", None),
    );
    assert_eq!(
        artifact.messages.get(&key).map(String::as_str),
        Some("Bonjour")
    );
    assert_eq!(artifact.missing.len(), 1);
    assert_eq!(artifact.missing[0].resolved_locale.as_deref(), Some("fr"));
}

#[test]
fn compile_catalog_artifact_reports_missing_message_without_source_fallback() {
    let source = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hello\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );

    let artifact = compile_catalog_artifact(
        &[&requested, &source],
        &CompileCatalogArtifactOptions::new("de", "en"),
    )
    .expect("compile artifact");

    let key = compiled_key_for(
        CompiledKeyStrategy::FerrocatV1,
        &CatalogMessageKey::new("Hello", None),
    );
    assert!(!artifact.messages.contains_key(&key));
    assert_eq!(artifact.missing.len(), 1);
    assert_eq!(artifact.missing[0].resolved_locale, None);
}

#[test]
fn compile_catalog_artifact_can_fill_from_source_locale_when_enabled() {
    let source = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );

    let artifact = compile_catalog_artifact(
        &[&requested, &source],
        &CompileCatalogArtifactOptions {
            source_fallback: true,
            ..CompileCatalogArtifactOptions::new("de", "en")
        },
    )
    .expect("compile artifact");

    let key = compiled_key_for(
        CompiledKeyStrategy::FerrocatV1,
        &CatalogMessageKey::new("Hello", None),
    );
    assert_eq!(
        artifact.messages.get(&key).map(String::as_str),
        Some("Hello")
    );
    assert_eq!(artifact.missing.len(), 1);
    assert_eq!(artifact.missing[0].resolved_locale.as_deref(), Some("en"));
}

#[test]
fn compile_catalog_artifact_materializes_empty_source_locale_messages() {
    let source = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );

    let artifact =
        compile_catalog_artifact(&[&source], &CompileCatalogArtifactOptions::new("en", "en"))
            .expect("compile artifact");

    let key = compiled_key_for(
        CompiledKeyStrategy::FerrocatV1,
        &CatalogMessageKey::new("Hello", None),
    );
    assert_eq!(
        artifact.messages.get(&key).map(String::as_str),
        Some("Hello")
    );
    assert!(artifact.missing.is_empty());
}

#[test]
fn compile_catalog_artifact_skips_obsolete_messages() {
    let source = normalized_catalog("", Some("en"), PluralEncoding::Icu);
    let requested = normalized_catalog(
        "#~ msgid \"Hello\"\n#~ msgstr \"Hallo\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );

    let artifact = compile_catalog_artifact(
        &[&requested, &source],
        &CompileCatalogArtifactOptions::new("de", "en"),
    )
    .expect("compile artifact");

    assert!(artifact.messages.is_empty());
    assert!(artifact.missing.is_empty());
}

#[test]
fn compile_catalog_artifact_requires_requested_and_unique_catalog_locales() {
    let source = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hello\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let duplicate = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );

    let missing_requested = compile_catalog_artifact(
        &[&source],
        &CompileCatalogArtifactOptions {
            semantics: CatalogSemantics::GettextCompat,
            ..CompileCatalogArtifactOptions::new("de", "en")
        },
    )
    .expect_err("missing requested locale");
    assert!(matches!(missing_requested, ApiError::InvalidArguments(_)));

    let duplicate_locale = compile_catalog_artifact(
        &[&source, &duplicate],
        &CompileCatalogArtifactOptions::new("en", "en"),
    )
    .expect_err("duplicate locale");
    assert!(matches!(duplicate_locale, ApiError::InvalidArguments(_)));
}

#[test]
fn compile_catalog_artifact_rejects_invalid_locale_sets_and_fallback_chains() {
    let source = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hello\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );
    let fallback = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Servus\"\n",
        Some("de-AT"),
        PluralEncoding::Icu,
    );
    let no_locale = parse_catalog(ParseCatalogOptions {
        content: "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
        source_locale: "en",
        mode: CatalogMode::IcuPo,
        ..ParseCatalogOptions::new("", "en")
    })
    .expect("parse no-locale catalog")
    .into_normalized_view()
    .expect("normalize no-locale catalog");
    let empty_locale = parse_catalog(ParseCatalogOptions {
        content: "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
        locale: Some("  "),
        source_locale: "en",
        mode: CatalogMode::IcuPo,
        ..ParseCatalogOptions::new("", "en")
    })
    .expect("parse empty-locale catalog")
    .into_normalized_view()
    .expect("normalize empty-locale catalog");

    let cases = [
        (Vec::new(), CompileCatalogArtifactOptions::new("de", "en")),
        (
            vec![&requested, &source],
            CompileCatalogArtifactOptions::new("", "en"),
        ),
        (
            vec![&no_locale],
            CompileCatalogArtifactOptions::new("de", "en"),
        ),
        (
            vec![&empty_locale],
            CompileCatalogArtifactOptions::new("de", "en"),
        ),
        (
            vec![&requested],
            CompileCatalogArtifactOptions::new("de", "en"),
        ),
        (
            vec![&requested, &source],
            CompileCatalogArtifactOptions {
                fallback_chain: &["de".to_owned()],
                ..CompileCatalogArtifactOptions::new("de", "en")
            },
        ),
        (
            vec![&requested, &source],
            CompileCatalogArtifactOptions {
                fallback_chain: &["en".to_owned()],
                ..CompileCatalogArtifactOptions::new("de", "en")
            },
        ),
        (
            vec![&requested, &source, &fallback],
            CompileCatalogArtifactOptions {
                fallback_chain: &["de-AT".to_owned(), "de-AT".to_owned()],
                ..CompileCatalogArtifactOptions::new("de", "en")
            },
        ),
        (
            vec![&requested, &source],
            CompileCatalogArtifactOptions {
                fallback_chain: &["fr".to_owned()],
                ..CompileCatalogArtifactOptions::new("de", "en")
            },
        ),
    ];

    for (catalogs, options) in cases {
        let error = compile_catalog_artifact(&catalogs, &options).expect_err("invalid locale set");
        assert!(matches!(error, ApiError::InvalidArguments(_)));
    }
}

#[test]
fn compile_catalog_artifact_collects_or_raises_invalid_icu_messages() {
    let source = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hello\"\n",
        Some("en"),
        PluralEncoding::Gettext,
    );
    let requested = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"{unclosed\"\n",
        Some("de"),
        PluralEncoding::Gettext,
    );

    let artifact = compile_catalog_artifact(
        &[&requested, &source],
        &CompileCatalogArtifactOptions {
            semantics: CatalogSemantics::GettextCompat,
            ..CompileCatalogArtifactOptions::new("de", "en")
        },
    )
    .expect("compile artifact");
    assert_eq!(artifact.diagnostics.len(), 1);
    assert_eq!(artifact.diagnostics[0].code, "compile.invalid_icu_message");

    let error = compile_catalog_artifact(
        &[&requested, &source],
        &CompileCatalogArtifactOptions {
            strict_icu: true,
            semantics: CatalogSemantics::GettextCompat,
            ..CompileCatalogArtifactOptions::new("de", "en")
        },
    )
    .expect_err("strict invalid icu should fail");
    assert!(matches!(error, ApiError::Unsupported(_)));
}

#[test]
fn compile_catalog_artifact_strict_policy_reports_literal_apostrophes() {
    let source = normalized_catalog(
        "msgid \"Hours\"\nmsgstr \"Hours\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        "msgid \"Hours\"\nmsgstr \"You're available.\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );

    let artifact = compile_catalog_artifact(
        &[&requested, &source],
        &CompileCatalogArtifactOptions::new("de", "en"),
    )
    .expect("compile artifact");

    assert!(
        artifact
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "compile.invalid_icu_message")
    );
}

#[test]
fn compile_catalog_artifact_runtime_literal_apostrophes_policy_accepts_runtime_valid_messages() {
    let source = normalized_catalog(
        "msgid \"Openings\"\nmsgstr \"Openings\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        "msgid \"Openings\"\nmsgstr \"We've got {count, plural, one {one opening} other {# openings}}.\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );

    let artifact = compile_catalog_artifact_with_icu_options(
        &[&requested, &source],
        &CompileCatalogArtifactOptions::new("de", "en"),
        &CompileCatalogArtifactIcuOptions::new()
            .with_syntax_policy(IcuSyntaxPolicy::RuntimeLiteralApostrophes),
    )
    .expect("compile artifact");

    assert!(artifact.diagnostics.is_empty());
}

#[test]
fn compile_catalog_artifact_selected_uses_runtime_literal_apostrophes_policy() {
    let source = normalized_catalog(
        "msgid \"Hours\"\nmsgstr \"Hours\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        "msgid \"Hours\"\nmsgstr \"You're available.\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );
    let index =
        CompiledCatalogIdIndex::new(&[&requested, &source], CompiledKeyStrategy::FerrocatV1)
            .expect("index");
    let compiled_ids = index
        .iter()
        .map(|(id, _)| id.to_owned())
        .collect::<Vec<_>>();

    let artifact = compile_catalog_artifact_selected_with_icu_options(
        &[&requested, &source],
        &index,
        &CompileSelectedCatalogArtifactOptions {
            compiled_ids: &compiled_ids,
            options: CompileCatalogArtifactOptions::new("de", "en"),
        },
        &CompileCatalogArtifactIcuOptions::new()
            .with_syntax_policy(IcuSyntaxPolicy::RuntimeLiteralApostrophes),
    )
    .expect("compile selected artifact");

    assert!(artifact.diagnostics.is_empty());
}

#[test]
fn strict_icu_respects_runtime_literal_apostrophes_policy() {
    let source = normalized_catalog(
        "msgid \"Hours\"\nmsgstr \"Hours\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        "msgid \"Hours\"\nmsgstr \"You're available.\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );

    let artifact = compile_catalog_artifact_with_icu_options(
        &[&requested, &source],
        &CompileCatalogArtifactOptions {
            strict_icu: true,
            ..CompileCatalogArtifactOptions::new("de", "en")
        },
        &CompileCatalogArtifactIcuOptions::new()
            .with_syntax_policy(IcuSyntaxPolicy::RuntimeLiteralApostrophes),
    )
    .expect("strict runtime-valid artifact");

    assert!(artifact.diagnostics.is_empty());
}

#[test]
fn strict_icu_keeps_real_syntax_errors_with_runtime_literal_apostrophes_policy() {
    let source = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hello\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hello {{name}}\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );

    let error = compile_catalog_artifact_with_icu_options(
        &[&requested, &source],
        &CompileCatalogArtifactOptions {
            strict_icu: true,
            ..CompileCatalogArtifactOptions::new("de", "en")
        },
        &CompileCatalogArtifactIcuOptions::new()
            .with_syntax_policy(IcuSyntaxPolicy::RuntimeLiteralApostrophes),
    )
    .expect_err("real invalid icu should fail");

    assert!(matches!(error, ApiError::Unsupported(_)));
}

#[test]
fn compile_catalog_artifact_formatter_support_accepts_supported_runtime_styles() {
    let source = normalized_catalog(
        concat!(
            "msgid \"Number default\"\nmsgstr \"Number default\"\n\n",
            "msgid \"Number percent\"\nmsgstr \"Number percent\"\n\n",
            "msgid \"Number integer\"\nmsgstr \"Number integer\"\n\n",
            "msgid \"Number skeleton percent\"\nmsgstr \"Number skeleton percent\"\n\n",
            "msgid \"Number skeleton integer\"\nmsgstr \"Number skeleton integer\"\n\n",
            "msgid \"Number skeleton currency\"\nmsgstr \"Number skeleton currency\"\n\n",
            "msgid \"Date short\"\nmsgstr \"Date short\"\n\n",
            "msgid \"Time full\"\nmsgstr \"Time full\"\n",
        ),
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        concat!(
            "msgid \"Number default\"\nmsgstr \"{count, number}\"\n\n",
            "msgid \"Number percent\"\nmsgstr \"{ratio, number, percent}\"\n\n",
            "msgid \"Number integer\"\nmsgstr \"{count, number, integer}\"\n\n",
            "msgid \"Number skeleton percent\"\nmsgstr \"{ratio, number, ::percent}\"\n\n",
            "msgid \"Number skeleton integer\"\nmsgstr \"{count, number, ::integer}\"\n\n",
            "msgid \"Number skeleton currency\"\nmsgstr \"{price, number, ::currency/USD}\"\n\n",
            "msgid \"Date short\"\nmsgstr \"{created, date, short}\"\n\n",
            "msgid \"Time full\"\nmsgstr \"{created, time, full}\"\n",
        ),
        Some("de"),
        PluralEncoding::Icu,
    );

    let artifact = compile_catalog_artifact_with_icu_options(
        &[&requested, &source],
        &CompileCatalogArtifactOptions::new("de", "en"),
        &CompileCatalogArtifactIcuOptions::new().with_formatter_support(runtime_formatter_support),
    )
    .expect("compile artifact");

    assert!(artifact.diagnostics.is_empty());
}

#[test]
fn compile_catalog_artifact_formatter_support_reports_unsupported_styles_and_kinds() {
    let source = normalized_catalog(
        concat!(
            "msgid \"Currency predefined\"\nmsgstr \"Currency predefined\"\n\n",
            "msgid \"List formatter\"\nmsgstr \"List formatter\"\n",
        ),
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        concat!(
            "msgid \"Currency predefined\"\nmsgstr \"{price, number, currency}\"\n\n",
            "msgid \"List formatter\"\nmsgstr \"{items, list}\"\n",
        ),
        Some("de"),
        PluralEncoding::Icu,
    );

    let artifact = compile_catalog_artifact_with_icu_options(
        &[&requested, &source],
        &CompileCatalogArtifactOptions::new("de", "en"),
        &CompileCatalogArtifactIcuOptions::new().with_formatter_support(runtime_formatter_support),
    )
    .expect("compile artifact");

    assert!(artifact.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "icu.unsupported_formatter_style"
            && diagnostic.severity == DiagnosticSeverity::Warning
            && diagnostic.msgid == "Currency predefined"
            && diagnostic.locale == "de"
    }));
    assert!(artifact.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "icu.unsupported_formatter_kind"
            && diagnostic.severity == DiagnosticSeverity::Error
            && diagnostic.msgid == "List formatter"
            && diagnostic.locale == "de"
    }));
}

#[test]
fn compile_catalog_artifact_selected_uses_formatter_support_diagnostics() {
    let source = normalized_catalog(
        "msgid \"List formatter\"\nmsgstr \"List formatter\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        "msgid \"List formatter\"\nmsgstr \"{items, list}\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );
    let index =
        CompiledCatalogIdIndex::new(&[&requested, &source], CompiledKeyStrategy::FerrocatV1)
            .expect("index");
    let compiled_ids = index
        .iter()
        .map(|(id, _)| id.to_owned())
        .collect::<Vec<_>>();

    let artifact = compile_catalog_artifact_selected_with_icu_options(
        &[&requested, &source],
        &index,
        &CompileSelectedCatalogArtifactOptions {
            compiled_ids: &compiled_ids,
            options: CompileCatalogArtifactOptions::new("de", "en"),
        },
        &CompileCatalogArtifactIcuOptions::new().with_formatter_support(runtime_formatter_support),
    )
    .expect("compile selected artifact");

    let diagnostic = artifact
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "icu.unsupported_formatter_kind")
        .expect("formatter diagnostic");
    assert_eq!(diagnostic.msgid, "List formatter");
    assert_eq!(diagnostic.locale, "de");
    assert!(artifact.messages.contains_key(&diagnostic.key));
}

fn runtime_formatter_support(formatter: &IcuFormatter) -> IcuFormatterSupport {
    match formatter.kind {
        IcuArgumentKind::Number => runtime_formatter_style_support(
            is_supported_runtime_number_style(formatter.style.as_deref()),
        ),
        IcuArgumentKind::Date | IcuArgumentKind::Time => runtime_formatter_style_support(
            is_supported_runtime_date_time_style(formatter.style.as_deref()),
        ),
        _ => IcuFormatterSupport::UnsupportedKind {
            severity: IcuDiagnosticSeverity::Error,
        },
    }
}

const fn runtime_formatter_style_support(supported: bool) -> IcuFormatterSupport {
    if supported {
        IcuFormatterSupport::Supported
    } else {
        IcuFormatterSupport::UnsupportedStyle {
            severity: IcuDiagnosticSeverity::Warning,
        }
    }
}

fn is_supported_runtime_number_style(style: Option<&str>) -> bool {
    let Some(style) = style.map(str::trim).filter(|style| !style.is_empty()) else {
        return true;
    };

    if let Some(skeleton) = style.strip_prefix("::") {
        return matches!(skeleton, "percent" | "integer") || supported_currency_skeleton(skeleton);
    }

    matches!(style, "percent" | "integer")
}

fn supported_currency_skeleton(style: &str) -> bool {
    let Some(currency) = style.strip_prefix("currency/") else {
        return false;
    };

    currency.len() == 3 && currency.bytes().all(|byte| byte.is_ascii_alphabetic())
}

fn is_supported_runtime_date_time_style(style: Option<&str>) -> bool {
    let Some(style) = style.map(str::trim).filter(|style| !style.is_empty()) else {
        return true;
    };

    matches!(style, "short" | "medium" | "long" | "full")
}

#[test]
fn compile_catalog_artifact_icu_compatibility_is_optional() {
    let source = normalized_catalog(
        "msgid \"{count, number, integer} for {name}\"\nmsgstr \"{count, number, integer} for {name}\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        "msgid \"{count, number, integer} for {name}\"\nmsgstr \"{count, number, integer} Dateien\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );

    let default_artifact = compile_catalog_artifact(
        &[&requested, &source],
        &CompileCatalogArtifactOptions::new("de", "en"),
    )
    .expect("compile default artifact");
    assert!(default_artifact.diagnostics.is_empty());

    let checked_artifact = compile_catalog_artifact(
        &[&requested, &source],
        &CompileCatalogArtifactOptions {
            icu_compatibility: true,
            ..CompileCatalogArtifactOptions::new("de", "en")
        },
    )
    .expect("compile checked artifact");

    assert!(checked_artifact.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "icu.missing_argument"
            && diagnostic.severity == DiagnosticSeverity::Error
    }));
}

#[test]
fn compile_catalog_artifact_selected_uses_icu_compatibility_diagnostics() {
    let source = normalized_catalog(
        "msgid \"<link>{name}</link>\"\nmsgstr \"<link>{name}</link>\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        "msgid \"<link>{name}</link>\"\nmsgstr \"<b>{name}</b>\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );
    let index =
        CompiledCatalogIdIndex::new(&[&requested, &source], CompiledKeyStrategy::FerrocatV1)
            .expect("index");
    let compiled_ids = index
        .iter()
        .map(|(id, _)| id.to_owned())
        .collect::<Vec<_>>();

    let artifact = compile_catalog_artifact_selected(
        &[&requested, &source],
        &index,
        &CompileSelectedCatalogArtifactOptions {
            compiled_ids: &compiled_ids,
            options: CompileCatalogArtifactOptions {
                icu_compatibility: true,
                ..CompileCatalogArtifactOptions::new("de", "en")
            },
        },
    )
    .expect("compile selected artifact");

    assert!(
        artifact
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "icu.missing_tag")
    );
    assert!(
        artifact
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "icu.extra_tag")
    );
}

#[test]
fn strict_icu_remains_a_hard_syntax_error_with_compatibility_enabled() {
    let source = normalized_catalog(
        "msgid \"Hello {name}\"\nmsgstr \"Hello {name}\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        "msgid \"Hello {name}\"\nmsgstr \"{unclosed\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );

    let error = compile_catalog_artifact(
        &[&requested, &source],
        &CompileCatalogArtifactOptions {
            strict_icu: true,
            icu_compatibility: true,
            ..CompileCatalogArtifactOptions::new("de", "en")
        },
    )
    .expect_err("strict syntax failure");

    assert!(matches!(error, ApiError::Unsupported(_)));
}

#[test]
fn compiled_catalog_id_index_indexes_non_obsolete_compiled_ids() {
    let requested = normalized_catalog(
        concat!(
            "msgid \"Hello\"\n",
            "msgstr \"Hallo\"\n\n",
            "#~ msgid \"Obsolete\"\n",
            "#~ msgstr \"Alt\"\n",
        ),
        Some("de"),
        PluralEncoding::Icu,
    );
    let source = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hello\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );

    let index =
        CompiledCatalogIdIndex::new(&[&requested, &source], CompiledKeyStrategy::FerrocatV1)
            .expect("compiled id index");

    let key = compiled_key_for(
        CompiledKeyStrategy::FerrocatV1,
        &CatalogMessageKey::new("Hello", None),
    );
    let obsolete_key = compiled_key_for(
        CompiledKeyStrategy::FerrocatV1,
        &CatalogMessageKey::new("Obsolete", None),
    );

    assert_eq!(index.len(), 1);
    assert!(index.contains_id(&key));
    assert_eq!(
        index.get(&key),
        Some(&CatalogMessageKey::new("Hello", None))
    );
    assert!(!index.contains_id(&obsolete_key));
}

#[test]
fn compiled_catalog_id_index_reports_compiled_key_collisions() {
    let requested = normalized_catalog(
        concat!(
            "msgid \"Hello\"\n",
            "msgstr \"Hallo\"\n\n",
            "msgctxt \"menu\"\n",
            "msgid \"Save\"\n",
            "msgstr \"Speichern\"\n",
        ),
        Some("de"),
        PluralEncoding::Icu,
    );

    let error = CompiledCatalogIdIndex::new_with_key_generator(
        &[&requested],
        CompiledKeyStrategy::FerrocatV1,
        |_, _| "fc1_collision".to_owned(),
    )
    .expect_err("collision");

    match error {
        ApiError::Conflict(message) => {
            assert!(message.contains("collision"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn compiled_catalog_id_index_exports_btreemap_views() {
    let requested = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );

    let index = CompiledCatalogIdIndex::new(&[&requested], CompiledKeyStrategy::FerrocatV1)
        .expect("compiled id index");
    let key = compiled_key_for(
        CompiledKeyStrategy::FerrocatV1,
        &CatalogMessageKey::new("Hello", None),
    );

    assert_eq!(
        index.as_btreemap().get(&key),
        Some(&CatalogMessageKey::new("Hello", None))
    );

    let owned = index.into_btreemap();
    assert_eq!(
        owned.get(&key),
        Some(&CatalogMessageKey::new("Hello", None))
    );
}

#[test]
fn compiled_catalog_id_index_describes_known_ids() {
    let source = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hello\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );

    let index =
        CompiledCatalogIdIndex::new(&[&requested, &source], CompiledKeyStrategy::FerrocatV1)
            .expect("compiled id index");
    let hello_id = compiled_key_for(
        CompiledKeyStrategy::FerrocatV1,
        &CatalogMessageKey::new("Hello", None),
    );

    let report = index
        .describe_compiled_ids(&[&requested, &source], std::slice::from_ref(&hello_id))
        .expect("describe compiled ids");

    assert!(report.unknown_compiled_ids.is_empty());
    assert!(report.unavailable_compiled_ids.is_empty());
    assert_eq!(report.described.len(), 1);
    assert_eq!(report.described[0].compiled_id, hello_id);
    assert_eq!(
        report.described[0].source_key,
        CatalogMessageKey::new("Hello", None)
    );
    assert_eq!(
        report.described[0].available_locales,
        vec!["de".to_owned(), "en".to_owned()]
    );
    assert_eq!(
        report.described[0].translation_kind,
        CompiledCatalogTranslationKind::Singular
    );
}

#[test]
fn compiled_catalog_id_index_describes_unknown_and_unavailable_ids() {
    let source = normalized_catalog(
        concat!(
            "msgid \"Hello\"\n",
            "msgstr \"Hello\"\n\n",
            "msgid \"SourceOnly\"\n",
            "msgstr \"SourceOnly\"\n",
        ),
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );

    let index =
        CompiledCatalogIdIndex::new(&[&requested, &source], CompiledKeyStrategy::FerrocatV1)
            .expect("compiled id index");
    let hello_id = compiled_key_for(
        CompiledKeyStrategy::FerrocatV1,
        &CatalogMessageKey::new("Hello", None),
    );
    let source_only_id = compiled_key_for(
        CompiledKeyStrategy::FerrocatV1,
        &CatalogMessageKey::new("SourceOnly", None),
    );

    let report = index
        .describe_compiled_ids(
            &[&requested],
            &[
                hello_id.clone(),
                source_only_id.clone(),
                "missing-id".to_owned(),
            ],
        )
        .expect("describe compiled ids");

    assert_eq!(report.described.len(), 1);
    assert_eq!(report.described[0].compiled_id, hello_id);
    assert_eq!(report.unknown_compiled_ids, vec!["missing-id".to_owned()]);
    assert_eq!(report.unavailable_compiled_ids.len(), 1);
    assert_eq!(
        report.unavailable_compiled_ids[0].compiled_id,
        source_only_id
    );
    assert_eq!(
        report.unavailable_compiled_ids[0].source_key,
        CatalogMessageKey::new("SourceOnly", None)
    );
}

#[test]
fn compile_catalog_artifact_selected_returns_only_requested_ids() {
    let source = normalized_catalog(
        concat!(
            "msgid \"Hello\"\n",
            "msgstr \"Hello\"\n\n",
            "msgid \"Bye\"\n",
            "msgstr \"Bye\"\n",
        ),
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        concat!(
            "msgid \"Hello\"\n",
            "msgstr \"Hallo\"\n\n",
            "msgid \"Bye\"\n",
            "msgstr \"Tschuess\"\n",
        ),
        Some("de"),
        PluralEncoding::Icu,
    );

    let index =
        CompiledCatalogIdIndex::new(&[&requested, &source], CompiledKeyStrategy::FerrocatV1)
            .expect("compiled id index");
    let hello_id = compiled_key_for(
        CompiledKeyStrategy::FerrocatV1,
        &CatalogMessageKey::new("Hello", None),
    );
    let bye_id = compiled_key_for(
        CompiledKeyStrategy::FerrocatV1,
        &CatalogMessageKey::new("Bye", None),
    );

    let artifact = compile_catalog_artifact_selected(
        &[&requested, &source],
        &index,
        &CompileSelectedCatalogArtifactOptions::new(
            "de",
            "en",
            &[hello_id.clone(), hello_id.clone()],
        ),
    )
    .expect("compile selected artifact");

    assert_eq!(artifact.messages.len(), 1);
    assert_eq!(
        artifact.messages.get(&hello_id).map(String::as_str),
        Some("Hallo")
    );
    assert!(!artifact.messages.contains_key(&bye_id));
    assert!(artifact.missing.is_empty());
}

#[test]
fn compile_catalog_artifact_selected_reports_unknown_compiled_ids() {
    let source = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hello\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let requested = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );

    let index =
        CompiledCatalogIdIndex::new(&[&requested, &source], CompiledKeyStrategy::FerrocatV1)
            .expect("compiled id index");
    let error = compile_catalog_artifact_selected(
        &[&requested, &source],
        &index,
        &CompileSelectedCatalogArtifactOptions::new("de", "en", &["missing-id".to_owned()]),
    )
    .expect_err("unknown compiled id");

    assert!(matches!(error, ApiError::InvalidArguments(_)));
}

#[test]
fn compile_catalog_artifact_selected_rejects_ids_absent_from_catalog_set() {
    let indexed_source = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hello\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let indexed_requested = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );
    let source = normalized_catalog("", Some("en"), PluralEncoding::Icu);
    let requested = normalized_catalog("", Some("de"), PluralEncoding::Icu);
    let index = CompiledCatalogIdIndex::new(
        &[&indexed_requested, &indexed_source],
        CompiledKeyStrategy::FerrocatV1,
    )
    .expect("compiled id index");
    let hello_id = compiled_key("Hello", None);

    let error = compile_catalog_artifact_selected(
        &[&requested, &source],
        &index,
        &CompileSelectedCatalogArtifactOptions::new("de", "en", &[hello_id]),
    )
    .expect_err("compiled id absent from provided catalogs");

    assert!(
        matches!(error, ApiError::InvalidArguments(message) if message.contains("not present"))
    );
}

#[test]
fn compile_catalog_artifact_selected_preserves_fallback_and_validation_semantics() {
    let source = normalized_catalog(
        concat!(
            "msgid \"Hello\"\n",
            "msgstr \"\"\n\n",
            "msgid \"Broken\"\n",
            "msgstr \"Broken\"\n",
        ),
        Some("en"),
        PluralEncoding::Gettext,
    );
    let requested = normalized_catalog(
        concat!(
            "msgid \"Hello\"\n",
            "msgstr \"\"\n\n",
            "msgid \"Broken\"\n",
            "msgstr \"{unclosed\"\n",
        ),
        Some("de"),
        PluralEncoding::Gettext,
    );

    let index =
        CompiledCatalogIdIndex::new(&[&requested, &source], CompiledKeyStrategy::FerrocatV1)
            .expect("compiled id index");
    let hello_id = compiled_key_for(
        CompiledKeyStrategy::FerrocatV1,
        &CatalogMessageKey::new("Hello", None),
    );
    let broken_id = compiled_key_for(
        CompiledKeyStrategy::FerrocatV1,
        &CatalogMessageKey::new("Broken", None),
    );

    let artifact = compile_catalog_artifact_selected(
        &[&requested, &source],
        &index,
        &CompileSelectedCatalogArtifactOptions {
            compiled_ids: &[hello_id.clone(), broken_id.clone()],
            options: CompileCatalogArtifactOptions {
                source_fallback: true,
                semantics: CatalogSemantics::GettextCompat,
                ..CompileCatalogArtifactOptions::new("de", "en")
            },
        },
    )
    .expect("compile selected artifact");

    assert_eq!(
        artifact.messages.get(&hello_id).map(String::as_str),
        Some("Hello")
    );
    assert_eq!(artifact.missing.len(), 1);
    assert_eq!(artifact.missing[0].key, hello_id);
    assert_eq!(artifact.diagnostics.len(), 1);
    assert_eq!(artifact.diagnostics[0].key, broken_id);
}
