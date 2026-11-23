use std::collections::BTreeMap;

use ferrocat_po::{
    CatalogMode, CatalogUpdateInput, ExtractedMessage, ExtractedPluralMessage, ParseCatalogOptions,
    PluralSource, TranslationShape, UpdateCatalogOptions, parse_catalog, update_catalog,
};

fn input(id: &str, variable: &str) -> CatalogUpdateInput {
    CatalogUpdateInput::Structured(vec![ExtractedMessage::Plural(ExtractedPluralMessage {
        msgid: id.into(),
        source: PluralSource {
            one: Some("# book".into()),
            other: "# books".into(),
        },
        placeholders: BTreeMap::from([(variable.into(), vec![])]),
        ..Default::default()
    })])
}

#[test]
fn native_structured_plurals_round_trip_and_keep_translations() {
    for mode in [CatalogMode::IcuPo, CatalogMode::IcuFcl] {
        let created = update_catalog(
            UpdateCatalogOptions::new("en", input("legacy-id", "items"))
                .with_mode(mode)
                .with_locale("en"),
        )
        .expect("roundtrip succeeds");
        let source = "{items, plural, one {# book} other {# books}}";
        assert!(created.content.contains(source));
        let translation = "{items, plural, one {# Band} other {# Bände}}";
        let (before_target, after_target) = created
            .content
            .rsplit_once(source)
            .expect("roundtrip succeeds");
        let translated = format!("{before_target}{translation}{after_target}");
        let updated = update_catalog(
            UpdateCatalogOptions::new("en", input("different-id", "items"))
                .with_mode(mode)
                .with_locale("de")
                .with_existing(&translated),
        )
        .expect("roundtrip succeeds");
        let parsed = parse_catalog(
            ParseCatalogOptions::new(&updated.content, "en")
                .with_mode(mode)
                .with_locale("de"),
        )
        .expect("roundtrip succeeds");
        assert_eq!(parsed.messages.len(), 1);
        assert_eq!(parsed.messages[0].msgid, source);
        assert!(
            matches!(&parsed.messages[0].translation, TranslationShape::Singular { value } if value == translation)
        );
        let overwritten = update_catalog(
            UpdateCatalogOptions::new("en", input("legacy-id", "items"))
                .with_mode(mode)
                .with_locale("en")
                .with_existing(&updated.content)
                .with_overwrite_source_translations(true),
        )
        .expect("roundtrip succeeds");
        let parsed_overwrite = parse_catalog(
            ParseCatalogOptions::new(&overwritten.content, "en")
                .with_mode(mode)
                .with_locale("en"),
        )
        .expect("roundtrip succeeds");
        assert!(
            matches!(&parsed_overwrite.messages[0].translation, TranslationShape::Singular { value } if value == source)
        );
    }
}

#[test]
fn native_structured_plurals_reject_empty_ids() {
    for mode in [CatalogMode::IcuPo, CatalogMode::IcuFcl] {
        assert!(
            update_catalog(UpdateCatalogOptions::new("en", input("", "count")).with_mode(mode))
                .is_err()
        );
    }
}

#[test]
fn native_structured_plural_rejects_ambiguous_existing_raw_id() {
    use ferrocat_po::SourceExtractedMessage;
    for mode in [CatalogMode::IcuPo, CatalogMode::IcuFcl] {
        let existing = update_catalog(
            UpdateCatalogOptions::new(
                "en",
                CatalogUpdateInput::SourceFirst(vec![SourceExtractedMessage {
                    msgid: "legacy-id".into(),
                    ..Default::default()
                }]),
            )
            .with_mode(mode)
            .with_locale("en"),
        )
        .expect("create source-first catalog");
        let result = update_catalog(
            UpdateCatalogOptions::new("en", input("legacy-id", "items"))
                .with_mode(mode)
                .with_locale("de")
                .with_existing(&existing.content),
        );
        assert!(matches!(result, Err(ferrocat_po::ApiError::Conflict(_))));
    }
}

#[test]
fn distinct_raw_plural_ids_with_the_same_native_identity_conflict() {
    for mode in [CatalogMode::IcuPo, CatalogMode::IcuFcl] {
        let CatalogUpdateInput::Structured(mut first) = input("first", "items") else {
            unreachable!()
        };
        let CatalogUpdateInput::Structured(second) = input("second", "items") else {
            unreachable!()
        };
        first.extend(second);
        let result = update_catalog(
            UpdateCatalogOptions::new("en", CatalogUpdateInput::Structured(first)).with_mode(mode),
        );
        assert!(matches!(result, Err(ferrocat_po::ApiError::Conflict(_))));
    }
}
