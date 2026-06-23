use super::{
    CatalogCoverageOptions, CatalogMessageKey, CatalogMessageStatus, PluralEncoding,
    catalog_coverage, normalized_catalog,
};

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
