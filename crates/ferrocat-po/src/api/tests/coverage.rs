use super::{
    ApiError, CatalogCoverageOptions, CatalogMessageKey, CatalogMessageStatus, PluralEncoding,
    catalog_coverage, normalized_catalog,
};

fn invalid_arguments_message(error: ApiError) -> String {
    match error {
        ApiError::InvalidArguments(message) => message,
        other => panic!("expected invalid arguments error, got {other:?}"),
    }
}

#[test]
fn coverage_report_counts_expected_message_statuses() {
    let source = normalized_catalog(
        concat!(
            "msgid \"Hello\"\nmsgstr \"Hello\"\n\n",
            "msgid \"Empty\"\nmsgstr \"Empty\"\n\n",
            "msgid \"Fuzzy\"\nmsgstr \"Fuzzy\"\n\n",
            "msgid \"Gone\"\nmsgstr \"Gone\"\n\n",
            "msgid \"Missing\"\nmsgstr \"Missing\"\n",
        ),
        Some("en"),
        PluralEncoding::Icu,
    );
    let target = normalized_catalog(
        concat!(
            "msgid \"Hello\"\nmsgstr \"Hallo\"\n\n",
            "msgid \"Empty\"\nmsgstr \"\"\n\n",
            "#, fuzzy\nmsgid \"Fuzzy\"\nmsgstr \"Unscharf\"\n\n",
            "#~ msgid \"Gone\"\n#~ msgstr \"Weg\"\n\n",
            "msgid \"Extra\"\nmsgstr \"Extra\"\n",
        ),
        Some("de"),
        PluralEncoding::Icu,
    );

    let report = catalog_coverage(
        &[&source, &target],
        &CatalogCoverageOptions::new("en").with_details(true),
    )
    .expect("coverage");
    let locale = &report.locales[0];

    assert_eq!(report.source_messages, 5);
    assert_eq!(report.target_locales, 1);
    assert_eq!(locale.total, 5);
    assert_eq!(locale.translated, 1);
    assert_eq!(locale.empty, 1);
    assert_eq!(locale.fuzzy, 1);
    assert_eq!(locale.obsolete, 1);
    assert_eq!(locale.missing, 1);
    assert_eq!(locale.extra, 1);
    assert_eq!(locale.incomplete(), 4);
    assert_eq!(locale.completion_percent(), 20.0);
}

#[test]
fn coverage_report_includes_optional_details() {
    let source = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hello\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let target = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n\nmsgid \"Extra\"\nmsgstr \"Extra\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );

    let report = catalog_coverage(
        &[&source, &target],
        &CatalogCoverageOptions::new("en").with_details(true),
    )
    .expect("coverage");
    let details = &report.locales[0].details;

    assert!(details.iter().any(|detail| {
        detail.source_key == CatalogMessageKey::new("Hello", None)
            && detail.status == CatalogMessageStatus::Translated
    }));
    assert!(details.iter().any(|detail| {
        detail.source_key == CatalogMessageKey::new("Extra", None)
            && detail.status == CatalogMessageStatus::Extra
    }));
}

#[test]
fn coverage_report_filters_requested_locales() {
    let source = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hello\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let de = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );
    let fr = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Bonjour\"\n",
        Some("fr"),
        PluralEncoding::Icu,
    );
    let requested = ["fr"];

    let report = catalog_coverage(
        &[&source, &de, &fr],
        &CatalogCoverageOptions {
            locales: &requested,
            ..CatalogCoverageOptions::new("en")
        },
    )
    .expect("coverage");

    assert_eq!(report.target_locales, 1);
    assert_eq!(report.locales[0].locale, "fr");
}

#[test]
fn coverage_report_skips_source_and_duplicate_requested_locales() {
    let source = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hello\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let de = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );
    let fr = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Bonjour\"\n",
        Some("fr"),
        PluralEncoding::Icu,
    );
    let requested = ["en", "de", "de", "fr"];

    let report = catalog_coverage(
        &[&source, &de, &fr],
        &CatalogCoverageOptions {
            locales: &requested,
            ..CatalogCoverageOptions::new("en")
        },
    )
    .expect("coverage");

    assert_eq!(report.target_locales, 2);
    assert_eq!(report.locales[0].locale, "de");
    assert_eq!(report.locales[1].locale, "fr");
}

#[test]
fn coverage_report_treats_empty_source_set_as_complete() {
    let source = normalized_catalog("", Some("en"), PluralEncoding::Icu);
    let target = normalized_catalog("", Some("de"), PluralEncoding::Icu);

    let report = catalog_coverage(&[&source, &target], &CatalogCoverageOptions::new("en"))
        .expect("coverage");
    let locale = &report.locales[0];

    assert_eq!(report.source_messages, 0);
    assert_eq!(locale.total, 0);
    assert_eq!(locale.incomplete(), 0);
    assert_eq!(locale.completion_ratio(), 1.0);
    assert_eq!(locale.completion_percent(), 100.0);
    assert!(locale.details.is_empty());
}

#[test]
fn coverage_report_classifies_plural_messages_with_empty_slots() {
    let source = normalized_catalog(
        concat!(
            "msgid \"book\"\n",
            "msgid_plural \"books\"\n",
            "msgstr[0] \"book\"\n",
            "msgstr[1] \"books\"\n",
        ),
        Some("en"),
        PluralEncoding::Gettext,
    );
    let target = normalized_catalog(
        concat!(
            "msgid \"book\"\n",
            "msgid_plural \"books\"\n",
            "msgstr[0] \"Buch\"\n",
            "msgstr[1] \"\"\n",
        ),
        Some("de"),
        PluralEncoding::Gettext,
    );

    let report = catalog_coverage(
        &[&source, &target],
        &CatalogCoverageOptions::new("en").with_details(true),
    )
    .expect("coverage");
    let locale = &report.locales[0];

    assert_eq!(locale.empty, 1);
    assert!(locale.details.iter().any(|detail| {
        detail.source_key == CatalogMessageKey::new("book", None)
            && detail.status == CatalogMessageStatus::Empty
    }));
}

#[test]
fn coverage_report_rejects_invalid_catalog_locale_inputs() {
    let source = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hello\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let duplicate_source = normalized_catalog(
        "msgid \"Bye\"\nmsgstr \"Bye\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let missing_locale = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
        None,
        PluralEncoding::Icu,
    );
    let de = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );
    let requested = ["fr"];

    let missing_source = catalog_coverage(&[&de], &CatalogCoverageOptions::new("en"))
        .expect_err("missing source locale should fail");
    assert!(invalid_arguments_message(missing_source).contains("source locale"));

    let duplicate_locale = catalog_coverage(
        &[&source, &duplicate_source],
        &CatalogCoverageOptions::new("en"),
    )
    .expect_err("duplicate locale should fail");
    assert!(invalid_arguments_message(duplicate_locale).contains("duplicate catalog locale"));

    let undeclared_locale =
        catalog_coverage(&[&missing_locale], &CatalogCoverageOptions::new("en"))
            .expect_err("missing declared locale should fail");
    assert!(invalid_arguments_message(undeclared_locale).contains("declare a locale"));

    let missing_requested = catalog_coverage(
        &[&source, &de],
        &CatalogCoverageOptions {
            locales: &requested,
            ..CatalogCoverageOptions::new("en")
        },
    )
    .expect_err("missing requested locale should fail");
    assert!(invalid_arguments_message(missing_requested).contains("requested locale"));
}
