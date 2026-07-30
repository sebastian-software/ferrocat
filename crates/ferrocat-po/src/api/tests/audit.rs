use ferrocat_icu::{
    MessageArgumentKind, MessageArgumentMetadataInput, MessageMetadataInput, MessageSelectorKind,
    MessageSelectorMetadata,
};

use crate::{CatalogAuditChecks, CatalogAuditIcuOptions};

use super::super::icu_syntax::{icu_parse_count, reset_icu_parse_count};
use super::{
    CatalogAuditOptions, CatalogMode, DiagnosticSeverity, IcuSyntaxPolicy, ParseCatalogOptions,
    audit_catalogs, parse_catalog,
};

fn catalog(content: &str, locale: &str) -> super::super::NormalizedParsedCatalog {
    parse_catalog(ParseCatalogOptions {
        locale: Some(locale),
        mode: CatalogMode::IcuPo,
        ..ParseCatalogOptions::new(content, "en")
    })
    .expect("parse catalog")
    .into_normalized_view()
    .expect("normalize catalog")
}

fn diagnostic_codes(report: &super::super::CatalogAuditReport) -> Vec<&str> {
    report
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

#[test]
fn default_audit_parses_each_compatibility_message_once() {
    let source = catalog(
        "msgid \"Hello {name}\"\nmsgstr \"Hello {name}\"\n\nmsgid \"Total {count, number}\"\nmsgstr \"Total {count, number}\"\n",
        "en",
    );
    let german = catalog(
        "msgid \"Hello {name}\"\nmsgstr \"Hallo {name}\"\n\nmsgid \"Total {count, number}\"\nmsgstr \"Summe {count, number}\"\n",
        "de",
    );
    let french = catalog(
        "msgid \"Hello {name}\"\nmsgstr \"Bonjour {name}\"\n\nmsgid \"Total {count, number}\"\nmsgstr \"Total {count, number}\"\n",
        "fr",
    );
    reset_icu_parse_count();

    let report = audit_catalogs(
        &[&source, &german, &french],
        &CatalogAuditOptions::new("en"),
    )
    .expect("audit");

    assert!(report.diagnostics.is_empty());
    assert_eq!(icu_parse_count(), 6);
}

#[test]
fn syntax_only_audit_reports_invalid_messages_without_populating_caches() {
    let source = catalog("msgid \"Broken {\"\nmsgstr \"Broken {\"\n", "en");
    let checks = CatalogAuditChecks::default().with_icu_compatibility(false);

    let report = audit_catalogs(
        &[&source],
        &CatalogAuditOptions::new("en").with_checks(checks),
    )
    .expect("audit");

    assert!(diagnostic_codes(&report).contains(&"icu.invalid_syntax"));
}

#[test]
fn compatibility_only_audit_parses_uncached_target_messages() {
    let source = catalog("msgid \"Hello {name}\"\nmsgstr \"Hello {name}\"\n", "en");
    let target = catalog("msgid \"Hello {name}\"\nmsgstr \"Hallo {other}\"\n", "de");
    let checks = CatalogAuditChecks::default().with_icu_syntax(false);

    let report = audit_catalogs(
        &[&source, &target],
        &CatalogAuditOptions::new("en").with_checks(checks),
    )
    .expect("audit");

    assert!(diagnostic_codes(&report).contains(&"icu.missing_argument"));
}

#[test]
fn compatibility_only_audit_skips_uncached_invalid_target_messages() {
    let source = catalog("msgid \"Hello {name}\"\nmsgstr \"Hello {name}\"\n", "en");
    let target = catalog("msgid \"Hello {name}\"\nmsgstr \"Broken {\"\n", "de");
    let checks = CatalogAuditChecks::default().with_icu_syntax(false);

    let report = audit_catalogs(
        &[&source, &target],
        &CatalogAuditOptions::new("en").with_checks(checks),
    )
    .expect("audit");

    assert!(!diagnostic_codes(&report).contains(&"icu.invalid_syntax"));
}

#[test]
fn audit_reports_missing_translation_for_active_source_key() {
    let source = catalog("msgid \"Hello\"\nmsgstr \"Hello\"\n", "en");
    let target = catalog("", "de");

    let report =
        audit_catalogs(&[&source, &target], &CatalogAuditOptions::new("en")).expect("audit");

    assert!(diagnostic_codes(&report).contains(&"catalog.missing_translation"));
    assert!(report.has_errors());
}

#[test]
fn audit_reports_empty_target_translation() {
    let source = catalog("msgid \"Hello\"\nmsgstr \"Hello\"\n", "en");
    let target = catalog("msgid \"Hello\"\nmsgstr \"\"\n", "de");

    let report =
        audit_catalogs(&[&source, &target], &CatalogAuditOptions::new("en")).expect("audit");

    assert!(diagnostic_codes(&report).contains(&"catalog.empty_translation"));
}

#[test]
fn audit_reports_target_only_active_message_as_extra() {
    let source = catalog("msgid \"Hello\"\nmsgstr \"Hello\"\n", "en");
    let target = catalog(
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n\nmsgid \"Only target\"\nmsgstr \"Nur Ziel\"\n",
        "de",
    );

    let report =
        audit_catalogs(&[&source, &target], &CatalogAuditOptions::new("en")).expect("audit");

    let diagnostic = report
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "catalog.extra_translation")
        .expect("extra diagnostic");
    assert_eq!(diagnostic.severity, DiagnosticSeverity::Warning);
}

#[test]
fn audit_does_not_let_obsolete_target_satisfy_completeness() {
    let source = catalog("msgid \"Hello\"\nmsgstr \"Hello\"\n", "en");
    let target = catalog("#~ msgid \"Hello\"\n#~ msgstr \"Hallo\"\n", "de");

    let report =
        audit_catalogs(&[&source, &target], &CatalogAuditOptions::new("en")).expect("audit");
    let codes = diagnostic_codes(&report);

    assert!(codes.contains(&"catalog.missing_translation"));
    assert!(codes.contains(&"catalog.obsolete_entry"));
}

#[test]
fn audit_reports_missing_source_locale() {
    let target = catalog("msgid \"Hello\"\nmsgstr \"Hallo\"\n", "de");

    let report = audit_catalogs(&[&target], &CatalogAuditOptions::new("en")).expect("audit");

    assert!(diagnostic_codes(&report).contains(&"catalog.missing_source_locale"));
    assert_eq!(report.summary.errors, 1);
}

#[test]
fn audit_locale_filter_limits_target_checks_and_reports_missing_locale() {
    let source = catalog("msgid \"Hello\"\nmsgstr \"Hello\"\n", "en");
    let de = catalog("msgid \"Hello\"\nmsgstr \"Hallo\"\n", "de");
    let fr = catalog("msgid \"Hello\"\nmsgstr \"\"\n", "fr");
    let requested = ["de", "it"];

    let report = audit_catalogs(
        &[&source, &de, &fr],
        &CatalogAuditOptions {
            locales: &requested,
            ..CatalogAuditOptions::new("en")
        },
    )
    .expect("audit");
    let codes = diagnostic_codes(&report);

    assert!(codes.contains(&"catalog.missing_locale"));
    assert!(!codes.contains(&"catalog.empty_translation"));
    assert_eq!(report.summary.target_locales, 1);
}

#[test]
fn audit_surfaces_icu_syntax_errors() {
    let source = catalog("msgid \"Hello {name\"\nmsgstr \"Hello {name\"\n", "en");
    let target = catalog("msgid \"Hello {name\"\nmsgstr \"Hallo {name\"\n", "de");

    let report =
        audit_catalogs(&[&source, &target], &CatalogAuditOptions::new("en")).expect("audit");

    assert!(diagnostic_codes(&report).contains(&"icu.invalid_syntax"));
}

#[test]
fn audit_strict_policy_reports_literal_apostrophes() {
    let source = catalog(
        "msgid \"Set your hours when you're available.\"\nmsgstr \"Set your hours when you're available.\"\n",
        "en",
    );

    let report = audit_catalogs(&[&source], &CatalogAuditOptions::new("en")).expect("audit");

    assert!(diagnostic_codes(&report).contains(&"icu.invalid_syntax"));
}

#[test]
fn audit_runtime_literal_apostrophes_policy_accepts_runtime_valid_messages() {
    let source = catalog(
        concat!(
            "msgid \"Set your hours when you're available.\"\n",
            "msgstr \"Set your hours when you're available.\"\n\n",
            "msgid \"We've got {count, plural, one {one opening} other {# openings}}.\"\n",
            "msgstr \"We've got {count, plural, one {one opening} other {# openings}}.\"\n",
        ),
        "en",
    );

    let report = audit_catalogs(
        &[&source],
        &CatalogAuditOptions::new("en").with_icu_options(
            CatalogAuditIcuOptions::new()
                .with_syntax_policy(IcuSyntaxPolicy::RuntimeLiteralApostrophes),
        ),
    )
    .expect("audit");

    assert!(!diagnostic_codes(&report).contains(&"icu.invalid_syntax"));
}

#[test]
fn audit_runtime_literal_apostrophes_policy_keeps_real_invalid_icu() {
    let source = catalog("msgid \"Hello\"\nmsgstr \"Hello {{name}}\"\n", "en");

    let report = audit_catalogs(
        &[&source],
        &CatalogAuditOptions::new("en").with_icu_options(
            CatalogAuditIcuOptions::new()
                .with_syntax_policy(IcuSyntaxPolicy::RuntimeLiteralApostrophes),
        ),
    )
    .expect("audit");

    assert!(diagnostic_codes(&report).contains(&"icu.invalid_syntax"));
}

#[test]
fn audit_runtime_literal_apostrophes_policy_still_reports_compatibility_changes() {
    let source = catalog(
        "msgid \"We've got {count, plural, one {one opening} other {# openings}}.\"\nmsgstr \"We've got {count, plural, one {one opening} other {# openings}}.\"\n",
        "en",
    );
    let target = catalog(
        "msgid \"We've got {count, plural, one {one opening} other {# openings}}.\"\nmsgstr \"Wir haben freie Termine.\"\n",
        "de",
    );

    let report = audit_catalogs(
        &[&source, &target],
        &CatalogAuditOptions::new("en").with_icu_options(
            CatalogAuditIcuOptions::new()
                .with_syntax_policy(IcuSyntaxPolicy::RuntimeLiteralApostrophes),
        ),
    )
    .expect("audit");

    assert!(diagnostic_codes(&report).contains(&"icu.missing_argument"));
}

#[test]
fn audit_runtime_literal_apostrophes_policy_keeps_quoted_braces_literal() {
    let source = catalog("msgid \"L'{title}\"\nmsgstr \"L'{title}\"\n", "en");
    let target = catalog("msgid \"L'{title}\"\nmsgstr \"L'{name}\"\n", "de");

    let report = audit_catalogs(
        &[&source, &target],
        &CatalogAuditOptions::new("en").with_icu_options(
            CatalogAuditIcuOptions::new()
                .with_syntax_policy(IcuSyntaxPolicy::RuntimeLiteralApostrophes),
        ),
    )
    .expect("audit");

    assert!(!diagnostic_codes(&report).contains(&"icu.missing_argument"));
    assert!(!diagnostic_codes(&report).contains(&"icu.unexpected_argument"));
}

#[test]
fn audit_reuses_icu_compatibility_codes() {
    let source = catalog("msgid \"Hello {name}\"\nmsgstr \"Hello {name}\"\n", "en");
    let target = catalog("msgid \"Hello {name}\"\nmsgstr \"Hallo\"\n", "de");

    let report =
        audit_catalogs(&[&source, &target], &CatalogAuditOptions::new("en")).expect("audit");

    assert!(diagnostic_codes(&report).contains(&"icu.missing_argument"));
}

#[test]
fn audit_reports_duplicate_unknown_and_conflicting_metadata() {
    let source = catalog(
        "msgid \"{count, plural, one {One item} other {# items}}\"\nmsgstr \"{count, plural, one {One item} other {# items}}\"\n",
        "en",
    );
    let mut args = std::collections::BTreeMap::new();
    args.insert(
        "count".to_owned(),
        MessageArgumentMetadataInput::Kind(MessageArgumentKind::String),
    );
    let mut selectors = std::collections::BTreeMap::new();
    selectors.insert(
        "count".to_owned(),
        MessageSelectorMetadata {
            kind: MessageSelectorKind::Plural,
            cases: vec!["one".to_owned()],
            offset: None,
        },
    );
    let mut first = MessageMetadataInput::new("{count, plural, one {One item} other {# items}}");
    first.args = Some(args);
    first.selectors = Some(selectors);
    let duplicate = MessageMetadataInput::new("{count, plural, one {One item} other {# items}}");
    let unknown = MessageMetadataInput::new("Unknown message");
    let metadata = [first, duplicate, unknown];

    let report = audit_catalogs(
        &[&source],
        &CatalogAuditOptions {
            metadata: &metadata,
            ..CatalogAuditOptions::new("en")
        },
    )
    .expect("audit");
    let codes = diagnostic_codes(&report);

    assert!(codes.contains(&"catalog.duplicate_metadata"));
    assert!(codes.contains(&"catalog.metadata_unknown_message"));
    assert!(codes.contains(&"metadata.argument_kind_mismatch"));
    assert!(codes.contains(&"metadata.missing_selector_case"));
}
