use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::message_status::{active_message_keys, classify_expected_message};
use super::{
    ApiError, CatalogCoverageOptions, CatalogLocaleCoverage, CatalogMessage, CatalogMessageKey,
    CatalogMessageStatus, EffectiveTranslationRef, NormalizedParsedCatalog, catalog_coverage,
    machine_translation_hash, validate_source_locale,
};

/// Options controlling catalog review reports.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CatalogReviewOptions<'a> {
    /// Source locale whose active identities define the expected current set.
    pub source_locale: &'a str,
    /// Optional target locale filter. Empty means all current non-source locales.
    pub locales: &'a [&'a str],
    /// Whether detail vectors should be populated in addition to counters.
    pub include_details: bool,
}

impl<'a> CatalogReviewOptions<'a> {
    /// Creates review options with the required source locale set.
    #[must_use]
    pub fn new(source_locale: &'a str) -> Self {
        Self {
            source_locale,
            ..Self::default()
        }
    }

    /// Returns options that include source, translation, and metadata detail rows.
    #[must_use]
    pub const fn with_details(mut self, include_details: bool) -> Self {
        self.include_details = include_details;
        self
    }
}

/// Read-only catalog review report comparing two normalized catalog states.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CatalogReviewReport {
    /// Aggregate counters across the compared catalog states.
    pub summary: CatalogReviewSummary,
    /// Source identity additions and removals.
    pub source_changes: CatalogSourceChangeReport,
    /// Per-locale target review sections.
    pub locales: Vec<CatalogLocaleReview>,
}

/// Aggregate counters for a catalog review report.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CatalogReviewSummary {
    /// Active source identities added in the current state.
    pub source_added: usize,
    /// Active source identities removed from the current state.
    pub source_removed: usize,
    /// Target locales included in the report.
    pub target_locales: usize,
    /// Target translations that changed between previous and current states.
    pub translation_changed: usize,
    /// Current active target messages with valid current machine-translation metadata.
    pub machine_translation_current: usize,
    /// Current active target messages with stale machine-translation metadata.
    pub machine_translation_stale: usize,
    /// Current active target messages without machine-translation metadata.
    pub machine_translation_absent: usize,
    /// Current active target messages with invalid machine-translation metadata.
    pub machine_translation_invalid: usize,
}

/// Source identity add/remove summary.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CatalogSourceChangeReport {
    /// Number of active source identities added in the current state.
    pub added: usize,
    /// Number of active source identities removed from the current state.
    pub removed: usize,
    /// Optional source change details.
    pub details: Vec<CatalogSourceChange>,
}

/// One source identity change.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CatalogSourceChange {
    /// Canonical gettext identity that changed.
    pub source_key: CatalogMessageKey,
    /// Add/remove classification.
    pub kind: CatalogSourceChangeKind,
}

/// Kind of source identity change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum CatalogSourceChangeKind {
    /// Identity exists in current source but not previous source.
    Added,
    /// Identity existed in previous source but not current source.
    Removed,
}

/// Review details for one target locale.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CatalogLocaleReview {
    /// Locale represented by this review section.
    pub locale: String,
    /// Current coverage/status rollup for this locale.
    pub coverage: CatalogLocaleCoverage,
    /// Translation changes against the previous state.
    pub translations: CatalogTranslationChangeReport,
    /// Machine-translation metadata state in the current target catalog.
    pub machine_translation: CatalogMachineTranslationReview,
}

/// Translation change summary for one locale.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CatalogTranslationChangeReport {
    /// Number of active translations whose effective value changed.
    pub changed: usize,
    /// Optional changed translation details.
    pub details: Vec<CatalogTranslationChange>,
}

/// One target translation change.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CatalogTranslationChange {
    /// Target locale whose translation changed.
    pub locale: String,
    /// Canonical gettext identity for the changed translation.
    pub source_key: CatalogMessageKey,
    /// Previous effective translation value.
    pub previous: CatalogReviewTranslation,
    /// Current effective translation value.
    pub current: CatalogReviewTranslation,
}

/// Owned translation value used in review reports.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "kind", content = "value", rename_all = "snake_case")
)]
pub enum CatalogReviewTranslation {
    /// Singular translation value.
    Singular(String),
    /// Plural translation values keyed by plural category.
    Plural(BTreeMap<String, String>),
}

/// Machine-translation metadata summary for one locale.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CatalogMachineTranslationReview {
    /// Active target messages whose metadata hash matches the current translation.
    pub current: usize,
    /// Active target messages whose metadata hash no longer matches the translation.
    pub stale: usize,
    /// Active target messages without machine-translation metadata.
    pub absent: usize,
    /// Active target messages with invalid metadata, when detectable.
    pub invalid: usize,
    /// Optional per-message metadata detail rows.
    pub details: Vec<CatalogMachineTranslationMessage>,
}

/// One machine-translation metadata classification.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CatalogMachineTranslationMessage {
    /// Target locale associated with the metadata row.
    pub locale: String,
    /// Canonical gettext identity for the metadata row.
    pub source_key: CatalogMessageKey,
    /// Machine-translation metadata status.
    pub status: CatalogMachineTranslationStatus,
}

/// Machine-translation metadata freshness status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum CatalogMachineTranslationStatus {
    /// Metadata hash matches the current effective translation.
    Current,
    /// Metadata hash does not match the current effective translation.
    Stale,
    /// No machine-translation metadata is present.
    Absent,
    /// Metadata is invalid in a way the parsed catalog representation can expose.
    Invalid,
}

/// Compares previous and current normalized catalog states for translator review.
///
/// Source changes are deterministic identity additions/removals by
/// `msgctxt + msgid`; semantic rename detection is intentionally out of scope.
/// Target status rollups reuse [`CatalogMessageStatus`] and therefore match
/// [`super::audit_catalogs`] and [`super::catalog_coverage`] semantics.
///
/// # Errors
///
/// Returns [`ApiError::InvalidArguments`] when either catalog state is missing
/// required locales, contains duplicate locales, or a requested target locale is
/// not available in the current catalog state.
pub fn catalog_review(
    previous_catalogs: &[&NormalizedParsedCatalog],
    current_catalogs: &[&NormalizedParsedCatalog],
    options: &CatalogReviewOptions<'_>,
) -> Result<CatalogReviewReport, ApiError> {
    validate_source_locale(options.source_locale)?;
    let previous_index = index_catalogs(previous_catalogs, "catalog_review previous")?;
    let current_index = index_catalogs(current_catalogs, "catalog_review current")?;
    let previous_source = previous_index
        .get(options.source_locale)
        .copied()
        .ok_or_else(|| missing_source_error("previous", options.source_locale))?;
    let current_source = current_index
        .get(options.source_locale)
        .copied()
        .ok_or_else(|| missing_source_error("current", options.source_locale))?;
    let previous_source_keys = active_message_keys(previous_source);
    let current_source_keys = active_message_keys(current_source);
    let target_locales = select_target_locales(&current_index, options)?;
    let source_changes = source_change_report(
        &previous_source_keys,
        &current_source_keys,
        options.include_details,
    );
    let coverage = catalog_coverage(
        current_catalogs,
        &CatalogCoverageOptions {
            source_locale: options.source_locale,
            locales: options.locales,
            include_details: options.include_details,
        },
    )?;
    let mut locales = Vec::with_capacity(target_locales.len());

    for locale in target_locales {
        let current_target = current_index
            .get(locale.as_str())
            .expect("selected target locale must exist");
        let previous_target = previous_index.get(locale.as_str()).copied();
        let locale_coverage = coverage
            .locales
            .iter()
            .find(|entry| entry.locale == locale)
            .expect("coverage locale must exist")
            .clone();
        let translations = translation_change_report(
            &locale,
            previous_target,
            current_target,
            &current_source_keys,
            options.include_details,
        );
        let machine_translation =
            machine_translation_review(&locale, current_target, options.include_details);
        locales.push(CatalogLocaleReview {
            locale,
            coverage: locale_coverage,
            translations,
            machine_translation,
        });
    }

    let summary = review_summary(&source_changes, &locales);
    Ok(CatalogReviewReport {
        summary,
        source_changes,
        locales,
    })
}

fn missing_source_error(state: &str, source_locale: &str) -> ApiError {
    ApiError::InvalidArguments(format!(
        "catalog_review {state} catalogs did not receive source locale {source_locale:?}"
    ))
}

fn source_change_report(
    previous_keys: &BTreeSet<CatalogMessageKey>,
    current_keys: &BTreeSet<CatalogMessageKey>,
    include_details: bool,
) -> CatalogSourceChangeReport {
    let added_keys = current_keys.difference(previous_keys);
    let removed_keys = previous_keys.difference(current_keys);
    let mut report = CatalogSourceChangeReport {
        added: added_keys.clone().count(),
        removed: removed_keys.clone().count(),
        details: Vec::new(),
    };

    if include_details {
        report
            .details
            .extend(added_keys.map(|source_key| CatalogSourceChange {
                source_key: source_key.clone(),
                kind: CatalogSourceChangeKind::Added,
            }));
        report
            .details
            .extend(removed_keys.map(|source_key| CatalogSourceChange {
                source_key: source_key.clone(),
                kind: CatalogSourceChangeKind::Removed,
            }));
    }

    report
}

fn translation_change_report(
    locale: &str,
    previous_target: Option<&NormalizedParsedCatalog>,
    current_target: &NormalizedParsedCatalog,
    current_source_keys: &BTreeSet<CatalogMessageKey>,
    include_details: bool,
) -> CatalogTranslationChangeReport {
    let Some(previous_target) = previous_target else {
        return CatalogTranslationChangeReport::default();
    };
    let mut report = CatalogTranslationChangeReport::default();

    for source_key in current_source_keys {
        if classify_expected_message(current_target, source_key) != CatalogMessageStatus::Translated
        {
            continue;
        }
        let Some(previous_message) = previous_target
            .get(source_key)
            .filter(|message| !message.obsolete)
        else {
            continue;
        };
        let Some(current_message) = current_target
            .get(source_key)
            .filter(|message| !message.obsolete)
        else {
            continue;
        };
        let previous = previous_message.effective_translation();
        let current = current_message.effective_translation();
        if previous == current {
            continue;
        }
        report.changed += 1;
        if include_details {
            report.details.push(CatalogTranslationChange {
                locale: locale.to_owned(),
                source_key: source_key.clone(),
                previous: owned_translation(previous),
                current: owned_translation(current),
            });
        }
    }

    report
}

fn machine_translation_review(
    locale: &str,
    current_target: &NormalizedParsedCatalog,
    include_details: bool,
) -> CatalogMachineTranslationReview {
    let mut report = CatalogMachineTranslationReview::default();

    for (source_key, message) in current_target.iter() {
        if message.obsolete {
            continue;
        }
        let status = machine_translation_status(message);
        increment_machine_translation_status(&mut report, status);
        if include_details {
            report.details.push(CatalogMachineTranslationMessage {
                locale: locale.to_owned(),
                source_key: source_key.clone(),
                status,
            });
        }
    }

    report
}

fn machine_translation_status(message: &CatalogMessage) -> CatalogMachineTranslationStatus {
    let Some(metadata) = message.machine_translation.as_ref() else {
        return CatalogMachineTranslationStatus::Absent;
    };
    if metadata.hash == machine_translation_hash(message.effective_translation()) {
        CatalogMachineTranslationStatus::Current
    } else {
        CatalogMachineTranslationStatus::Stale
    }
}

fn increment_machine_translation_status(
    report: &mut CatalogMachineTranslationReview,
    status: CatalogMachineTranslationStatus,
) {
    match status {
        CatalogMachineTranslationStatus::Current => report.current += 1,
        CatalogMachineTranslationStatus::Stale => report.stale += 1,
        CatalogMachineTranslationStatus::Absent => report.absent += 1,
        CatalogMachineTranslationStatus::Invalid => report.invalid += 1,
    }
}

fn review_summary(
    source_changes: &CatalogSourceChangeReport,
    locales: &[CatalogLocaleReview],
) -> CatalogReviewSummary {
    let mut summary = CatalogReviewSummary {
        source_added: source_changes.added,
        source_removed: source_changes.removed,
        target_locales: locales.len(),
        ..CatalogReviewSummary::default()
    };
    for locale in locales {
        summary.translation_changed += locale.translations.changed;
        summary.machine_translation_current += locale.machine_translation.current;
        summary.machine_translation_stale += locale.machine_translation.stale;
        summary.machine_translation_absent += locale.machine_translation.absent;
        summary.machine_translation_invalid += locale.machine_translation.invalid;
    }
    summary
}

fn owned_translation(value: EffectiveTranslationRef<'_>) -> CatalogReviewTranslation {
    match value {
        EffectiveTranslationRef::Singular(value) => {
            CatalogReviewTranslation::Singular(value.to_owned())
        }
        EffectiveTranslationRef::Plural(values) => CatalogReviewTranslation::Plural(values.clone()),
    }
}

fn index_catalogs<'a>(
    catalogs: &'a [&'a NormalizedParsedCatalog],
    label: &str,
) -> Result<BTreeMap<String, &'a NormalizedParsedCatalog>, ApiError> {
    let mut index = BTreeMap::new();
    for catalog in catalogs {
        let locale = catalog
            .parsed_catalog()
            .locale
            .as_deref()
            .filter(|locale| !locale.trim().is_empty())
            .ok_or_else(|| {
                ApiError::InvalidArguments(format!(
                    "{label} requires every catalog to declare a locale"
                ))
            })?;
        if index.insert(locale.to_owned(), *catalog).is_some() {
            return Err(ApiError::InvalidArguments(format!(
                "{label} received duplicate catalog locale {locale:?}"
            )));
        }
    }
    Ok(index)
}

fn select_target_locales(
    catalog_index: &BTreeMap<String, &NormalizedParsedCatalog>,
    options: &CatalogReviewOptions<'_>,
) -> Result<Vec<String>, ApiError> {
    if options.locales.is_empty() {
        return Ok(catalog_index
            .keys()
            .filter(|locale| locale.as_str() != options.source_locale)
            .cloned()
            .collect());
    }

    let mut seen = BTreeSet::new();
    let mut locales = Vec::new();
    for locale in options.locales {
        if *locale == options.source_locale || !seen.insert((*locale).to_owned()) {
            continue;
        }
        if !catalog_index.contains_key(*locale) {
            return Err(ApiError::InvalidArguments(format!(
                "catalog_review did not receive requested locale {locale:?}"
            )));
        }
        locales.push((*locale).to_owned());
    }
    Ok(locales)
}

#[cfg(test)]
mod tests {
    use super::{
        CatalogMachineTranslationStatus, CatalogReviewOptions, CatalogReviewTranslation,
        CatalogSourceChangeKind, catalog_review,
    };
    use crate::api::{
        CatalogMessageKey, CatalogMode, EffectiveTranslationRef, ParseCatalogOptions,
        machine_translation_hash, parse_catalog,
    };

    fn catalog(content: &str, locale: &str) -> crate::api::NormalizedParsedCatalog {
        parse_catalog(ParseCatalogOptions {
            locale: Some(locale),
            mode: CatalogMode::IcuPo,
            ..ParseCatalogOptions::new(content, "en")
        })
        .expect("parse catalog")
        .into_normalized_view()
        .expect("normalize catalog")
    }

    #[test]
    fn catalog_review_reports_source_and_target_changes() {
        let previous_source = catalog(
            "msgid \"Hello\"\nmsgstr \"Hello\"\n\nmsgid \"Removed\"\nmsgstr \"Removed\"\n",
            "en",
        );
        let current_source = catalog(
            "msgid \"Hello\"\nmsgstr \"Hello\"\n\nmsgid \"Added\"\nmsgstr \"Added\"\n",
            "en",
        );
        let previous_target = catalog(
            "msgid \"Hello\"\nmsgstr \"Hallo\"\n\nmsgid \"Removed\"\nmsgstr \"Entfernt\"\n",
            "de",
        );
        let current_target = catalog(
            "msgid \"Hello\"\nmsgstr \"Hallo neu\"\n\nmsgid \"Added\"\nmsgstr \"\"\n\nmsgid \"Extra\"\nmsgstr \"Extra\"\n",
            "de",
        );

        let report = catalog_review(
            &[&previous_source, &previous_target],
            &[&current_source, &current_target],
            &CatalogReviewOptions::new("en").with_details(true),
        )
        .expect("review");
        let locale = &report.locales[0];

        assert_eq!(report.summary.source_added, 1);
        assert_eq!(report.summary.source_removed, 1);
        assert_eq!(report.summary.translation_changed, 1);
        assert!(report.source_changes.details.iter().any(|change| {
            change.source_key == CatalogMessageKey::new("Added", None)
                && change.kind == CatalogSourceChangeKind::Added
        }));
        assert_eq!(locale.coverage.empty, 1);
        assert_eq!(locale.coverage.extra, 1);
        assert_eq!(
            locale.translations.details[0].current,
            CatalogReviewTranslation::Singular("Hallo neu".to_owned())
        );
    }

    #[test]
    fn catalog_review_reports_machine_translation_freshness() {
        let hash = machine_translation_hash(EffectiveTranslationRef::Singular("Hallo"));
        let source = catalog(
            "msgid \"Hello\"\nmsgstr \"Hello\"\n\nmsgid \"Stale\"\nmsgstr \"Stale\"\n\nmsgid \"Absent\"\nmsgstr \"Absent\"\n",
            "en",
        );
        let target = catalog(
            &format!(
                concat!(
                    "#@ ferrocat-mt model=openai/gpt-5.5-high hash={}\n",
                    "msgid \"Hello\"\nmsgstr \"Hallo\"\n\n",
                    "#@ ferrocat-mt model=openai/gpt-5.5-high hash=old\n",
                    "msgid \"Stale\"\nmsgstr \"Alt\"\n\n",
                    "msgid \"Absent\"\nmsgstr \"Ohne\"\n",
                ),
                hash
            ),
            "de",
        );

        let report = catalog_review(
            &[&source, &target],
            &[&source, &target],
            &CatalogReviewOptions::new("en").with_details(true),
        )
        .expect("review");
        let machine_translation = &report.locales[0].machine_translation;

        assert_eq!(machine_translation.current, 1);
        assert_eq!(machine_translation.stale, 1);
        assert_eq!(machine_translation.absent, 1);
        assert!(machine_translation.details.iter().any(|detail| {
            detail.source_key == CatalogMessageKey::new("Stale", None)
                && detail.status == CatalogMachineTranslationStatus::Stale
        }));
    }

    #[test]
    fn catalog_review_can_return_summary_only() {
        let source = catalog("msgid \"Hello\"\nmsgstr \"Hello\"\n", "en");
        let target = catalog("msgid \"Hello\"\nmsgstr \"Hallo\"\n", "de");

        let report = catalog_review(
            &[&source, &target],
            &[&source, &target],
            &CatalogReviewOptions::new("en"),
        )
        .expect("review");

        assert!(report.source_changes.details.is_empty());
        assert!(report.locales[0].translations.details.is_empty());
        assert!(report.locales[0].machine_translation.details.is_empty());
        assert!(report.locales[0].coverage.details.is_empty());
        assert_eq!(report.locales[0].coverage.translated, 1);
    }
}
