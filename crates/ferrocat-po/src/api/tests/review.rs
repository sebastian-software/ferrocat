use super::{
    CatalogReviewOptions, EffectiveTranslationRef, PluralEncoding, machine_translation_hash,
    normalized_catalog, review_catalogs,
};

#[test]
fn review_report_exposes_public_summary_and_detail_api() {
    let previous_source = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hello\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let current_source = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hello\"\n\nmsgid \"Added\"\nmsgstr \"Added\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let previous_target = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );
    let hash = machine_translation_hash(EffectiveTranslationRef::Singular("Hallo neu"));
    let current_target = normalized_catalog(
        &format!(
            "#@ lock: {hash}\n#@ ai: openai/gpt-5.5-high\nmsgid \"Hello\"\nmsgstr \"Hallo neu\"\n",
        ),
        Some("de"),
        PluralEncoding::Icu,
    );

    let report = review_catalogs(
        &[&previous_source, &previous_target],
        &[&current_source, &current_target],
        &CatalogReviewOptions::new("en").with_details(true),
    )
    .expect("review");

    assert_eq!(report.summary.source_added, 1);
    assert_eq!(report.summary.translation_changed, 1);
    assert_eq!(report.summary.machine_translation_current, 1);
    assert_eq!(report.locales[0].coverage.missing, 1);
    assert_eq!(report.locales[0].translations.details.len(), 1);
}

#[test]
fn review_report_uses_fuzzy_coverage_and_skips_fuzzy_translation_changes() {
    let source = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hello\"\n",
        Some("en"),
        PluralEncoding::Icu,
    );
    let previous_target = normalized_catalog(
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );
    let current_target = normalized_catalog(
        "#, fuzzy\nmsgid \"Hello\"\nmsgstr \"Guten Tag\"\n",
        Some("de"),
        PluralEncoding::Icu,
    );

    let report = review_catalogs(
        &[&source, &previous_target],
        &[&source, &current_target],
        &CatalogReviewOptions::new("en").with_details(true),
    )
    .expect("review");

    assert_eq!(report.locales[0].coverage.translated, 0);
    assert_eq!(report.locales[0].coverage.fuzzy(), 1);
    assert_eq!(report.locales[0].translations.changed, 0);
}
