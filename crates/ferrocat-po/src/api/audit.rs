use std::collections::{BTreeMap, BTreeSet};

use ferrocat_icu::{
    IcuCompatibilityOptions, IcuDiagnosticSeverity, MessageMetadataInput, compare_icu_messages,
    normalize_message_metadata, validate_message_metadata,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::diagnostic_codes::{self, DiagnosticCode};

use super::icu_syntax::parse_icu_with_syntax_policy;
use super::message_status::{
    CatalogMessageStatus, active_message_keys, classify_expected_message, is_extra_target_message,
};
use super::{
    ApiError, CatalogMessage, CatalogMessageKey, DiagnosticSeverity, EffectiveTranslationRef,
    IcuSyntaxPolicy, NormalizedParsedCatalog, validate_source_locale,
};

/// Options controlling catalog audit checks.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct CatalogAuditOptions<'a> {
    /// Source locale used as the expected message set.
    pub source_locale: &'a str,
    /// Optional target locale filter. Empty means all non-source locales present in `catalogs`.
    pub locales: &'a [&'a str],
    /// Optional source-side semantic metadata records.
    pub metadata: &'a [MessageMetadataInput],
    /// Individual audit checks to run.
    pub checks: CatalogAuditChecks,
    /// ICU-specific options used by syntax and compatibility checks.
    pub icu_options: CatalogAuditIcuOptions,
}

impl<'a> CatalogAuditOptions<'a> {
    /// Creates audit options with the required source locale set.
    #[must_use]
    pub fn new(source_locale: &'a str) -> Self {
        Self {
            source_locale,
            ..Self::default()
        }
    }

    /// Returns options that audit only the given target locales.
    #[must_use]
    pub fn with_locales(mut self, locales: &'a [&'a str]) -> Self {
        self.locales = locales;
        self
    }

    /// Returns options that validate the given source-side metadata records.
    #[must_use]
    pub fn with_metadata(mut self, metadata: &'a [MessageMetadataInput]) -> Self {
        self.metadata = metadata;
        self
    }

    /// Returns options that run the given audit check set.
    #[must_use]
    pub fn with_checks(mut self, checks: CatalogAuditChecks) -> Self {
        self.checks = checks;
        self
    }

    /// Returns options that use the given ICU parser and compatibility settings.
    #[must_use]
    pub fn with_icu_options(mut self, icu_options: CatalogAuditIcuOptions) -> Self {
        self.icu_options = icu_options;
        self
    }
}

/// ICU-specific options used by catalog audit checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct CatalogAuditIcuOptions {
    /// ICU parser behavior used by syntax and compatibility checks.
    pub syntax_policy: IcuSyntaxPolicy,
}

impl CatalogAuditIcuOptions {
    /// Creates audit ICU options with default strict parser behavior.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns options that parse messages with the given ICU syntax policy.
    #[must_use]
    pub fn with_syntax_policy(mut self, syntax_policy: IcuSyntaxPolicy) -> Self {
        self.syntax_policy = syntax_policy;
        self
    }
}

/// Enables or disables individual catalog audit checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CatalogAuditChecks {
    /// Check that target locales cover active source messages.
    pub completeness: bool,
    /// Check for active target messages that are not active in the source catalog.
    pub extra_messages: bool,
    /// Validate active source and target message strings as ICU MessageFormat v1.
    pub icu_syntax: bool,
    /// Compare target ICU structure against source ICU structure.
    pub icu_compatibility: bool,
    /// Validate source-side semantic message metadata.
    pub semantic_metadata: bool,
    /// Report obsolete entries.
    pub obsolete_entries: bool,
}

impl Default for CatalogAuditChecks {
    fn default() -> Self {
        Self {
            completeness: true,
            extra_messages: true,
            icu_syntax: true,
            icu_compatibility: true,
            semantic_metadata: true,
            obsolete_entries: true,
        }
    }
}

impl CatalogAuditChecks {
    /// Returns checks with completeness validation enabled or disabled.
    #[must_use]
    pub fn with_completeness(mut self, completeness: bool) -> Self {
        self.completeness = completeness;
        self
    }

    /// Returns checks with extra-message validation enabled or disabled.
    #[must_use]
    pub fn with_extra_messages(mut self, extra_messages: bool) -> Self {
        self.extra_messages = extra_messages;
        self
    }

    /// Returns checks with ICU syntax validation enabled or disabled.
    #[must_use]
    pub fn with_icu_syntax(mut self, icu_syntax: bool) -> Self {
        self.icu_syntax = icu_syntax;
        self
    }

    /// Returns checks with ICU compatibility validation enabled or disabled.
    #[must_use]
    pub fn with_icu_compatibility(mut self, icu_compatibility: bool) -> Self {
        self.icu_compatibility = icu_compatibility;
        self
    }

    /// Returns checks with semantic metadata validation enabled or disabled.
    #[must_use]
    pub fn with_semantic_metadata(mut self, semantic_metadata: bool) -> Self {
        self.semantic_metadata = semantic_metadata;
        self
    }

    /// Returns checks with obsolete-entry reporting enabled or disabled.
    #[must_use]
    pub fn with_obsolete_entries(mut self, obsolete_entries: bool) -> Self {
        self.obsolete_entries = obsolete_entries;
        self
    }
}

/// Summary counters for a catalog audit report.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CatalogAuditSummary {
    /// Active source messages considered expected by the audit.
    pub source_messages: usize,
    /// Target locales audited.
    pub target_locales: usize,
    /// Total diagnostics emitted.
    pub diagnostics: usize,
    /// Error diagnostics emitted.
    pub errors: usize,
    /// Warning diagnostics emitted.
    pub warnings: usize,
    /// Informational diagnostics emitted.
    pub infos: usize,
}

/// Catalog message reference attached to audit diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CatalogAuditMessageRef {
    /// Locale associated with the diagnostic, when known.
    pub locale: Option<String>,
    /// Source message identifier.
    pub msgid: String,
    /// Optional gettext context.
    pub msgctxt: Option<String>,
}

impl CatalogAuditMessageRef {
    fn new(locale: Option<&str>, key: &CatalogMessageKey) -> Self {
        Self {
            locale: locale.map(str::to_owned),
            msgid: key.msgid.clone(),
            msgctxt: key.msgctxt.clone(),
        }
    }
}

/// One machine-readable catalog audit diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CatalogAuditDiagnostic {
    /// Severity for the diagnostic.
    pub severity: DiagnosticSeverity,
    /// Stable machine-readable diagnostic code.
    pub code: DiagnosticCode,
    /// Human-readable explanation of the condition.
    pub message: String,
    /// Message identity associated with the diagnostic, when applicable.
    pub source_key: Option<CatalogAuditMessageRef>,
    /// Argument, selector, tag, locale, or field name associated with the diagnostic.
    pub name: Option<String>,
}

impl CatalogAuditDiagnostic {
    fn new(
        severity: DiagnosticSeverity,
        code: impl Into<DiagnosticCode>,
        message: impl Into<String>,
        source_key: Option<CatalogAuditMessageRef>,
        name: Option<String>,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
            source_key,
            name,
        }
    }
}

/// Report returned by [`audit_catalogs`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CatalogAuditReport {
    /// Aggregate audit counters.
    pub summary: CatalogAuditSummary,
    /// Diagnostics found by the audit.
    pub diagnostics: Vec<CatalogAuditDiagnostic>,
}

impl CatalogAuditReport {
    /// Returns `true` when the report contains at least one error diagnostic.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    }
}

/// Audits a normalized catalog set for catalog QA and authoring issues.
///
/// The audit is read-only: it does not rewrite catalogs, generate fuzzy
/// matches, or apply source fallback to hide missing target translations.
///
/// # Errors
///
/// Returns [`ApiError::InvalidArguments`] when `source_locale` is empty or when
/// catalogs cannot be inspected because their declared locales are missing or
/// duplicated.
///
/// # Examples
///
/// ```rust
/// use ferrocat_po::{CatalogAuditOptions, ParseCatalogOptions, audit_catalogs, parse_catalog};
///
/// let source = parse_catalog(
///     ParseCatalogOptions::new("msgid \"Checkout\"\nmsgstr \"Checkout\"\n", "en").with_locale("en"),
/// )?
/// .into_normalized_view()?;
/// let target = parse_catalog(
///     ParseCatalogOptions::new("msgid \"Checkout\"\nmsgstr \"\"\n", "en").with_locale("de"),
/// )?
/// .into_normalized_view()?;
///
/// let report = audit_catalogs(&[&source, &target], &CatalogAuditOptions::new("en"))?;
/// assert!(report.has_errors());
/// # Ok::<(), ferrocat_po::ApiError>(())
/// ```
pub fn audit_catalogs(
    catalogs: &[&NormalizedParsedCatalog],
    options: &CatalogAuditOptions<'_>,
) -> Result<CatalogAuditReport, ApiError> {
    validate_source_locale(options.source_locale)?;
    let catalog_index = index_catalogs(catalogs)?;
    let mut report = CatalogAuditReport::default();
    let icu_options = &options.icu_options;

    let Some(source_catalog) = catalog_index.get(options.source_locale).copied() else {
        report.diagnostics.push(CatalogAuditDiagnostic::new(
            DiagnosticSeverity::Error,
            diagnostic_codes::catalog::MISSING_SOURCE_LOCALE,
            format!(
                "Catalog audit did not receive source locale `{}`.",
                options.source_locale
            ),
            None,
            Some(options.source_locale.to_owned()),
        ));
        finalize_summary(&mut report, 0, 0);
        return Ok(report);
    };

    let source_keys = active_message_keys(source_catalog);
    let target_locales = select_target_locales(&catalog_index, options, &mut report);
    let source_locale = source_catalog.parsed_catalog().locale.as_deref();

    if options.checks.obsolete_entries || options.checks.icu_syntax {
        audit_catalog_entries(
            source_catalog,
            source_locale,
            true,
            options,
            icu_options,
            &mut report,
        );
    }
    if options.checks.semantic_metadata {
        audit_metadata(options.metadata, &source_keys, &mut report);
    }

    for target_locale in &target_locales {
        let Some(target_catalog) = catalog_index.get(target_locale.as_str()).copied() else {
            continue;
        };
        audit_catalog_entries(
            target_catalog,
            Some(target_locale),
            false,
            options,
            icu_options,
            &mut report,
        );
        audit_target_catalog(
            target_catalog,
            target_locale,
            &source_keys,
            options,
            icu_options,
            &mut report,
        );
    }

    finalize_summary(&mut report, source_keys.len(), target_locales.len());
    Ok(report)
}

fn index_catalogs<'a>(
    catalogs: &'a [&'a NormalizedParsedCatalog],
) -> Result<BTreeMap<String, &'a NormalizedParsedCatalog>, ApiError> {
    let mut index = BTreeMap::new();
    for catalog in catalogs {
        let locale = catalog
            .parsed_catalog()
            .locale
            .as_deref()
            .filter(|locale| !locale.trim().is_empty())
            .ok_or_else(|| {
                ApiError::InvalidArguments(
                    "audit_catalogs requires every catalog to declare a locale".to_owned(),
                )
            })?;
        if index.insert(locale.to_owned(), *catalog).is_some() {
            return Err(ApiError::InvalidArguments(format!(
                "audit_catalogs received duplicate catalog locale {locale:?}"
            )));
        }
    }
    Ok(index)
}

fn select_target_locales(
    catalog_index: &BTreeMap<String, &NormalizedParsedCatalog>,
    options: &CatalogAuditOptions<'_>,
    report: &mut CatalogAuditReport,
) -> Vec<String> {
    if options.locales.is_empty() {
        return catalog_index
            .keys()
            .filter(|locale| locale.as_str() != options.source_locale)
            .cloned()
            .collect();
    }

    let mut seen = BTreeSet::new();
    let mut locales = Vec::new();
    for locale in options.locales {
        if !seen.insert((*locale).to_owned()) {
            continue;
        }
        if catalog_index.contains_key(*locale) {
            if *locale != options.source_locale {
                locales.push((*locale).to_owned());
            }
        } else {
            report.diagnostics.push(CatalogAuditDiagnostic::new(
                DiagnosticSeverity::Error,
                diagnostic_codes::catalog::MISSING_LOCALE,
                format!("Catalog audit did not receive requested locale `{locale}`."),
                None,
                Some((*locale).to_owned()),
            ));
        }
    }
    locales
}

fn audit_catalog_entries(
    catalog: &NormalizedParsedCatalog,
    locale: Option<&str>,
    validate_source_identity: bool,
    options: &CatalogAuditOptions<'_>,
    icu_options: &CatalogAuditIcuOptions,
    report: &mut CatalogAuditReport,
) {
    for (key, message) in catalog.iter() {
        let message_ref = CatalogAuditMessageRef::new(locale, key);
        if options.checks.obsolete_entries && message.obsolete.is_some() {
            report.diagnostics.push(CatalogAuditDiagnostic::new(
                DiagnosticSeverity::Info,
                diagnostic_codes::catalog::OBSOLETE_ENTRY,
                "Catalog contains an obsolete entry.",
                Some(message_ref.clone()),
                None,
            ));
        }
        if options.checks.icu_syntax && message.obsolete.is_none() {
            audit_icu_syntax_for_message(
                message,
                validate_source_identity,
                icu_options.syntax_policy,
                &message_ref,
                report,
            );
        }
    }
}

fn audit_target_catalog(
    target_catalog: &NormalizedParsedCatalog,
    target_locale: &str,
    source_keys: &BTreeSet<CatalogMessageKey>,
    options: &CatalogAuditOptions<'_>,
    icu_options: &CatalogAuditIcuOptions,
    report: &mut CatalogAuditReport,
) {
    if options.checks.completeness {
        for key in source_keys {
            let message_ref = CatalogAuditMessageRef::new(Some(target_locale), key);
            match classify_expected_message(target_catalog, key) {
                CatalogMessageStatus::Missing | CatalogMessageStatus::Obsolete => {
                    report.diagnostics.push(CatalogAuditDiagnostic::new(
                        DiagnosticSeverity::Error,
                        diagnostic_codes::catalog::MISSING_TRANSLATION,
                        format!(
                            "Locale `{target_locale}` is missing translation for source message."
                        ),
                        Some(message_ref),
                        Some(target_locale.to_owned()),
                    ));
                }
                CatalogMessageStatus::Empty => {
                    report.diagnostics.push(CatalogAuditDiagnostic::new(
                        DiagnosticSeverity::Error,
                        diagnostic_codes::catalog::EMPTY_TRANSLATION,
                        format!("Locale `{target_locale}` has an empty translation."),
                        Some(message_ref),
                        Some(target_locale.to_owned()),
                    ));
                }
                CatalogMessageStatus::Translated => {}
                CatalogMessageStatus::Extra => {
                    unreachable!("expected source-key classification cannot produce extra status")
                }
            }
        }
    }

    if options.checks.extra_messages {
        for (key, message) in target_catalog.iter() {
            if is_extra_target_message(source_keys, key, message) {
                report.diagnostics.push(CatalogAuditDiagnostic::new(
                    DiagnosticSeverity::Warning,
                    diagnostic_codes::catalog::EXTRA_TRANSLATION,
                    format!(
                        "Locale `{target_locale}` contains an active message that is not present in the source catalog."
                    ),
                    Some(CatalogAuditMessageRef::new(Some(target_locale), key)),
                    Some(target_locale.to_owned()),
                ));
            }
        }
    }

    if options.checks.icu_compatibility {
        audit_icu_compatibility(
            target_catalog,
            target_locale,
            source_keys,
            icu_options,
            report,
        );
    }
}

fn audit_icu_syntax_for_message(
    message: &CatalogMessage,
    validate_source_identity: bool,
    syntax_policy: IcuSyntaxPolicy,
    message_ref: &CatalogAuditMessageRef,
    report: &mut CatalogAuditReport,
) {
    for value in message_strings(message, validate_source_identity) {
        if value.trim().is_empty() {
            continue;
        }
        if let Err(error) = parse_icu_with_syntax_policy(value, syntax_policy) {
            report.diagnostics.push(CatalogAuditDiagnostic::new(
                DiagnosticSeverity::Error,
                diagnostic_codes::icu::INVALID_SYNTAX,
                format!("Catalog message is not valid ICU MessageFormat v1: {error}"),
                Some(message_ref.clone()),
                None,
            ));
        }
    }
}

fn audit_icu_compatibility(
    target_catalog: &NormalizedParsedCatalog,
    target_locale: &str,
    source_keys: &BTreeSet<CatalogMessageKey>,
    icu_options: &CatalogAuditIcuOptions,
    report: &mut CatalogAuditReport,
) {
    for key in source_keys {
        let Some(target_message) = target_catalog
            .get(key)
            .filter(|message| message.obsolete.is_none())
        else {
            continue;
        };
        let Some(target_value) =
            singular_translation(target_message).filter(|value| !value.trim().is_empty())
        else {
            continue;
        };

        let Ok(source) = parse_icu_with_syntax_policy(&key.msgid, icu_options.syntax_policy) else {
            continue;
        };
        let Ok(translation) = parse_icu_with_syntax_policy(target_value, icu_options.syntax_policy)
        else {
            continue;
        };
        let compatibility =
            compare_icu_messages(&source, &translation, &IcuCompatibilityOptions::default());
        for diagnostic in compatibility.diagnostics {
            report.diagnostics.push(CatalogAuditDiagnostic::new(
                severity_from_icu(diagnostic.severity),
                diagnostic.code,
                diagnostic.message,
                Some(CatalogAuditMessageRef::new(Some(target_locale), key)),
                diagnostic.name,
            ));
        }
    }
}

fn audit_metadata(
    metadata: &[MessageMetadataInput],
    source_keys: &BTreeSet<CatalogMessageKey>,
    report: &mut CatalogAuditReport,
) {
    let mut seen = BTreeSet::<CatalogMessageKey>::new();
    for input in metadata {
        let key = CatalogMessageKey::new(input.msgid.clone(), input.msgctxt.clone());
        let source_ref = CatalogAuditMessageRef::new(None, &key);
        if !seen.insert(key.clone()) {
            report.diagnostics.push(CatalogAuditDiagnostic::new(
                DiagnosticSeverity::Error,
                diagnostic_codes::catalog::DUPLICATE_METADATA,
                "Semantic metadata contains a duplicate message identity.",
                Some(source_ref.clone()),
                None,
            ));
        }
        if !source_keys.contains(&key) {
            report.diagnostics.push(CatalogAuditDiagnostic::new(
                DiagnosticSeverity::Warning,
                diagnostic_codes::catalog::METADATA_UNKNOWN_MESSAGE,
                "Semantic metadata refers to a message that is not active in the source catalog.",
                Some(source_ref.clone()),
                None,
            ));
        }
        if let Err(error) = normalize_message_metadata(input.clone()) {
            report.diagnostics.push(CatalogAuditDiagnostic::new(
                DiagnosticSeverity::Error,
                diagnostic_codes::metadata::INVALID_MSGID,
                format!("Semantic metadata `msgid` is not valid ICU MessageFormat v1: {error}"),
                Some(source_ref.clone()),
                Some("msgid".to_owned()),
            ));
            continue;
        }
        let metadata_report = validate_message_metadata(input);
        for diagnostic in metadata_report.diagnostics {
            report.diagnostics.push(CatalogAuditDiagnostic::new(
                severity_from_icu(diagnostic.severity),
                diagnostic.code,
                diagnostic.message,
                Some(source_ref.clone()),
                diagnostic.name,
            ));
        }
    }
}

fn message_strings(message: &CatalogMessage, include_msgid: bool) -> Vec<&str> {
    let mut values = Vec::new();
    if include_msgid {
        push_unique(&mut values, message.msgid.as_str());
    }
    match message.effective_translation() {
        EffectiveTranslationRef::Singular(value) => push_unique(&mut values, value),
        EffectiveTranslationRef::Plural(translations) => {
            for value in translations.values().map(String::as_str) {
                push_unique(&mut values, value);
            }
        }
    }
    values
}

fn push_unique<'a>(values: &mut Vec<&'a str>, value: &'a str) {
    if !values.contains(&value) {
        values.push(value);
    }
}

#[cfg(test)]
mod tests {
    use ferrocat_icu::MessageMetadataInput;

    use super::{CatalogAuditChecks, CatalogAuditOptions};

    #[test]
    fn audit_option_builders_set_fields() {
        let metadata = [MessageMetadataInput::new("Checkout")];
        let locales = ["de", "fr"];
        let checks = CatalogAuditChecks::default()
            .with_completeness(false)
            .with_extra_messages(false)
            .with_icu_syntax(false)
            .with_icu_compatibility(false)
            .with_semantic_metadata(false)
            .with_obsolete_entries(false);

        assert!(!checks.completeness);
        assert!(!checks.extra_messages);
        assert!(!checks.icu_syntax);
        assert!(!checks.icu_compatibility);
        assert!(!checks.semantic_metadata);
        assert!(!checks.obsolete_entries);

        let options = CatalogAuditOptions::new("en")
            .with_locales(&locales)
            .with_metadata(&metadata)
            .with_checks(checks);

        assert_eq!(options.source_locale, "en");
        assert_eq!(options.locales, &locales);
        assert_eq!(options.metadata, &metadata);
        assert_eq!(options.checks, checks);
    }
}

fn singular_translation(message: &CatalogMessage) -> Option<&str> {
    match message.effective_translation() {
        EffectiveTranslationRef::Singular(value) => Some(value),
        EffectiveTranslationRef::Plural(_) => None,
    }
}

fn severity_from_icu(severity: IcuDiagnosticSeverity) -> DiagnosticSeverity {
    match severity {
        IcuDiagnosticSeverity::Info => DiagnosticSeverity::Info,
        IcuDiagnosticSeverity::Warning => DiagnosticSeverity::Warning,
        IcuDiagnosticSeverity::Error => DiagnosticSeverity::Error,
    }
}

fn finalize_summary(
    report: &mut CatalogAuditReport,
    source_messages: usize,
    target_locales: usize,
) {
    let mut summary = CatalogAuditSummary {
        source_messages,
        target_locales,
        diagnostics: report.diagnostics.len(),
        ..CatalogAuditSummary::default()
    };
    for diagnostic in &report.diagnostics {
        match diagnostic.severity {
            DiagnosticSeverity::Info => summary.infos += 1,
            DiagnosticSeverity::Warning => summary.warnings += 1,
            DiagnosticSeverity::Error => summary.errors += 1,
        }
    }
    report.summary = summary;
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::{
        CatalogAuditDiagnostic, CatalogAuditMessageRef, CatalogAuditReport, CatalogAuditSummary,
    };
    use crate::api::DiagnosticSeverity;

    #[test]
    fn catalog_audit_report_serde_round_trips_ci_report_shape() {
        let report = CatalogAuditReport {
            summary: CatalogAuditSummary {
                source_messages: 1,
                target_locales: 1,
                diagnostics: 1,
                errors: 1,
                warnings: 0,
                infos: 0,
            },
            diagnostics: vec![CatalogAuditDiagnostic {
                severity: DiagnosticSeverity::Error,
                code: "catalog.missing_translation".into(),
                message: "missing target translation".to_owned(),
                source_key: Some(CatalogAuditMessageRef {
                    locale: Some("de".to_owned()),
                    msgid: "Checkout".to_owned(),
                    msgctxt: None,
                }),
                name: Some("de".to_owned()),
            }],
        };

        let json = serde_json::to_value(&report).expect("audit report serialization must succeed");
        assert_eq!(json["diagnostics"][0]["severity"], "error");
        assert_eq!(json["summary"]["errors"], 1);

        let roundtrip: CatalogAuditReport =
            serde_json::from_value(json).expect("audit report deserialization must succeed");
        assert_eq!(roundtrip, report);
    }
}
