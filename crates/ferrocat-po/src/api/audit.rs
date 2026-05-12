use std::collections::{BTreeMap, BTreeSet};

use ferrocat_icu::{
    IcuCompatibilityOptions, IcuDiagnosticSeverity, MessageMetadataInput, compare_icu_messages,
    normalize_message_metadata, parse_icu, validate_message_metadata,
};

use super::{
    ApiError, CatalogMessage, CatalogMessageKey, DiagnosticSeverity, EffectiveTranslationRef,
    NormalizedParsedCatalog, validate_source_locale,
};

/// Options controlling catalog audit checks.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CatalogAuditOptions<'a> {
    /// Source locale used as the expected message set.
    pub source_locale: &'a str,
    /// Optional target locale filter. Empty means all non-source locales present in `catalogs`.
    pub locales: &'a [&'a str],
    /// Optional source-side semantic metadata records.
    pub metadata: &'a [MessageMetadataInput],
    /// Individual audit checks to run.
    pub checks: CatalogAuditChecks,
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
}

/// Enables or disables individual catalog audit checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Report existing `fuzzy` flags.
    pub fuzzy_flags: bool,
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
            fuzzy_flags: true,
            obsolete_entries: true,
        }
    }
}

/// Summary counters for a catalog audit report.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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
pub struct CatalogAuditDiagnostic {
    /// Severity for the diagnostic.
    pub severity: DiagnosticSeverity,
    /// Stable machine-readable diagnostic code.
    pub code: String,
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
        code: impl Into<String>,
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
pub fn audit_catalogs(
    catalogs: &[&NormalizedParsedCatalog],
    options: &CatalogAuditOptions<'_>,
) -> Result<CatalogAuditReport, ApiError> {
    validate_source_locale(options.source_locale)?;
    let catalog_index = index_catalogs(catalogs)?;
    let mut report = CatalogAuditReport::default();

    let Some(source_catalog) = catalog_index.get(options.source_locale).copied() else {
        report.diagnostics.push(CatalogAuditDiagnostic::new(
            DiagnosticSeverity::Error,
            "catalog.missing_source_locale",
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

    let source_keys = active_keys(source_catalog);
    let target_locales = select_target_locales(&catalog_index, options, &mut report);
    let source_locale = source_catalog.parsed_catalog().locale.as_deref();

    if options.checks.fuzzy_flags || options.checks.obsolete_entries || options.checks.icu_syntax {
        audit_catalog_entries(source_catalog, source_locale, true, options, &mut report);
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
            &mut report,
        );
        audit_target_catalog(
            target_catalog,
            target_locale,
            &source_keys,
            options,
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
                "catalog.missing_locale",
                format!("Catalog audit did not receive requested locale `{locale}`."),
                None,
                Some((*locale).to_owned()),
            ));
        }
    }
    locales
}

fn active_keys(catalog: &NormalizedParsedCatalog) -> BTreeSet<CatalogMessageKey> {
    catalog
        .iter()
        .filter_map(|(key, message)| (!message.obsolete).then_some(key.clone()))
        .collect()
}

fn audit_catalog_entries(
    catalog: &NormalizedParsedCatalog,
    locale: Option<&str>,
    validate_source_identity: bool,
    options: &CatalogAuditOptions<'_>,
    report: &mut CatalogAuditReport,
) {
    for (key, message) in catalog.iter() {
        let message_ref = CatalogAuditMessageRef::new(locale, key);
        if options.checks.obsolete_entries && message.obsolete {
            report.diagnostics.push(CatalogAuditDiagnostic::new(
                DiagnosticSeverity::Info,
                "catalog.obsolete_entry",
                "Catalog contains an obsolete entry.",
                Some(message_ref.clone()),
                None,
            ));
        }
        if options.checks.fuzzy_flags && message_has_fuzzy_flag(message) {
            report.diagnostics.push(CatalogAuditDiagnostic::new(
                DiagnosticSeverity::Info,
                "catalog.fuzzy_flag",
                "Catalog entry carries a fuzzy flag.",
                Some(message_ref.clone()),
                Some("fuzzy".to_owned()),
            ));
        }
        if options.checks.icu_syntax && !message.obsolete {
            audit_icu_syntax_for_message(message, validate_source_identity, &message_ref, report);
        }
    }
}

fn audit_target_catalog(
    target_catalog: &NormalizedParsedCatalog,
    target_locale: &str,
    source_keys: &BTreeSet<CatalogMessageKey>,
    options: &CatalogAuditOptions<'_>,
    report: &mut CatalogAuditReport,
) {
    if options.checks.completeness {
        for key in source_keys {
            let message_ref = CatalogAuditMessageRef::new(Some(target_locale), key);
            let Some(target_message) = target_catalog.get(key).filter(|message| !message.obsolete)
            else {
                report.diagnostics.push(CatalogAuditDiagnostic::new(
                    DiagnosticSeverity::Error,
                    "catalog.missing_translation",
                    format!("Locale `{target_locale}` is missing translation for source message."),
                    Some(message_ref),
                    Some(target_locale.to_owned()),
                ));
                continue;
            };
            if translation_is_empty(target_message) {
                report.diagnostics.push(CatalogAuditDiagnostic::new(
                    DiagnosticSeverity::Error,
                    "catalog.empty_translation",
                    format!("Locale `{target_locale}` has an empty translation."),
                    Some(message_ref),
                    Some(target_locale.to_owned()),
                ));
            }
        }
    }

    if options.checks.extra_messages {
        for (key, message) in target_catalog.iter() {
            if !message.obsolete && !source_keys.contains(key) {
                report.diagnostics.push(CatalogAuditDiagnostic::new(
                    DiagnosticSeverity::Warning,
                    "catalog.extra_translation",
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
        audit_icu_compatibility(target_catalog, target_locale, source_keys, report);
    }
}

fn audit_icu_syntax_for_message(
    message: &CatalogMessage,
    validate_source_identity: bool,
    message_ref: &CatalogAuditMessageRef,
    report: &mut CatalogAuditReport,
) {
    for value in message_strings(message, validate_source_identity) {
        if value.trim().is_empty() {
            continue;
        }
        if let Err(error) = parse_icu(value) {
            report.diagnostics.push(CatalogAuditDiagnostic::new(
                DiagnosticSeverity::Error,
                "icu.invalid_syntax",
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
    report: &mut CatalogAuditReport,
) {
    for key in source_keys {
        let Some(target_message) = target_catalog.get(key).filter(|message| !message.obsolete)
        else {
            continue;
        };
        let Some(target_value) =
            singular_translation(target_message).filter(|value| !value.trim().is_empty())
        else {
            continue;
        };

        let Ok(source) = parse_icu(&key.msgid) else {
            continue;
        };
        let Ok(translation) = parse_icu(target_value) else {
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
                "catalog.duplicate_metadata",
                "Semantic metadata contains a duplicate message identity.",
                Some(source_ref.clone()),
                None,
            ));
        }
        if !source_keys.contains(&key) {
            report.diagnostics.push(CatalogAuditDiagnostic::new(
                DiagnosticSeverity::Warning,
                "catalog.metadata_unknown_message",
                "Semantic metadata refers to a message that is not active in the source catalog.",
                Some(source_ref.clone()),
                None,
            ));
        }
        if let Err(error) = normalize_message_metadata(input.clone()) {
            report.diagnostics.push(CatalogAuditDiagnostic::new(
                DiagnosticSeverity::Error,
                "metadata.invalid_msgid",
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

fn singular_translation(message: &CatalogMessage) -> Option<&str> {
    match message.effective_translation() {
        EffectiveTranslationRef::Singular(value) => Some(value),
        EffectiveTranslationRef::Plural(_) => None,
    }
}

fn translation_is_empty(message: &CatalogMessage) -> bool {
    match message.effective_translation() {
        EffectiveTranslationRef::Singular(value) => value.trim().is_empty(),
        EffectiveTranslationRef::Plural(translations) => {
            translations.is_empty() || translations.values().any(|value| value.trim().is_empty())
        }
    }
}

fn message_has_fuzzy_flag(message: &CatalogMessage) -> bool {
    message
        .extra
        .as_ref()
        .is_some_and(|extra| extra.flags.iter().any(|flag| flag == "fuzzy"))
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
