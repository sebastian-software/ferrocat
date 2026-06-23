#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::catalog_index::{index_catalogs, select_target_locales};
use super::message_status::{
    CatalogMessageStatus, active_message_keys, classify_expected_message, is_extra_target_message,
};
use super::{ApiError, CatalogMessageKey, NormalizedParsedCatalog, validate_source_locale};

/// Options controlling catalog coverage reports.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CatalogCoverageOptions<'a> {
    /// Source locale whose active message identities define expected coverage.
    pub source_locale: &'a str,
    /// Optional target locale filter. Empty means all non-source locales present in `catalogs`.
    pub locales: &'a [&'a str],
    /// Whether the report should include one detail row per classified message.
    pub include_details: bool,
}

impl<'a> CatalogCoverageOptions<'a> {
    /// Creates coverage options with the required source locale set.
    #[must_use]
    pub fn new(source_locale: &'a str) -> Self {
        Self {
            source_locale,
            ..Self::default()
        }
    }

    /// Returns options that include per-message detail rows.
    #[must_use]
    pub const fn with_details(mut self, include_details: bool) -> Self {
        self.include_details = include_details;
        self
    }
}

/// Structured catalog coverage report.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CatalogCoverageReport {
    /// Active source messages considered expected by the report.
    pub source_messages: usize,
    /// Target locales included in the report.
    pub target_locales: usize,
    /// Per-locale coverage rollups in deterministic locale order.
    pub locales: Vec<CatalogLocaleCoverage>,
}

/// Coverage counters for one target locale.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CatalogLocaleCoverage {
    /// Target locale represented by these counters.
    pub locale: String,
    /// Active source messages expected for this locale.
    pub total: usize,
    /// Expected messages with non-empty, non-fuzzy active translations.
    pub translated: usize,
    /// Expected messages with no target entry.
    pub missing: usize,
    /// Expected messages with an empty effective translation.
    pub empty: usize,
    /// Expected messages with an active fuzzy target entry.
    pub fuzzy: usize,
    /// Expected messages with only an obsolete target entry.
    pub obsolete: usize,
    /// Active target messages that are not present in the active source set.
    pub extra: usize,
    /// Optional per-message detail rows.
    pub details: Vec<CatalogCoverageMessage>,
}

impl CatalogLocaleCoverage {
    /// Returns messages that still need translator attention.
    #[must_use]
    pub const fn incomplete(&self) -> usize {
        self.total.saturating_sub(self.translated)
    }

    /// Returns completion as a `0.0..=1.0` ratio.
    #[must_use]
    pub fn completion_ratio(&self) -> f64 {
        if self.total == 0 {
            1.0
        } else {
            self.translated as f64 / self.total as f64
        }
    }

    /// Returns completion as a `0.0..=100.0` percentage.
    #[must_use]
    pub fn completion_percent(&self) -> f64 {
        self.completion_ratio() * 100.0
    }
}

/// One classified message row in a coverage report.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CatalogCoverageMessage {
    /// Locale associated with this row.
    pub locale: String,
    /// Canonical gettext identity for the row.
    pub source_key: CatalogMessageKey,
    /// Canonical status assigned by the shared message-status classifier.
    pub status: CatalogMessageStatus,
}

/// Builds a read-only completeness and coverage report for normalized catalogs.
///
/// The report uses the source locale's active messages as the expected set.
/// Fuzzy, empty, obsolete, and absent target messages do not count as
/// translated. Active target-only messages are counted as `extra` and do not
/// affect the completion denominator.
///
/// # Errors
///
/// Returns [`ApiError::InvalidArguments`] when locales are missing, empty,
/// duplicated, or when a requested target locale is absent from the catalog set.
pub fn catalog_coverage(
    catalogs: &[&NormalizedParsedCatalog],
    options: &CatalogCoverageOptions<'_>,
) -> Result<CatalogCoverageReport, ApiError> {
    validate_source_locale(options.source_locale)?;
    let catalog_index = index_catalogs(catalogs, "catalog_coverage")?;
    let source_catalog = catalog_index
        .get(options.source_locale)
        .copied()
        .ok_or_else(|| {
            ApiError::InvalidArguments(format!(
                "catalog_coverage did not receive source locale {:?}",
                options.source_locale
            ))
        })?;
    let source_keys = active_message_keys(source_catalog);
    let target_locales = select_target_locales(
        &catalog_index,
        options.source_locale,
        options.locales,
        "catalog_coverage",
    )?;
    let mut locale_reports = Vec::with_capacity(target_locales.len());

    for target_locale in &target_locales {
        let target_catalog = catalog_index
            .get(target_locale.as_str())
            .expect("selected target locale must exist");
        locale_reports.push(coverage_for_locale(
            target_locale,
            target_catalog,
            &source_keys,
            options.include_details,
        ));
    }

    Ok(CatalogCoverageReport {
        source_messages: source_keys.len(),
        target_locales: target_locales.len(),
        locales: locale_reports,
    })
}

fn coverage_for_locale(
    locale: &str,
    target_catalog: &NormalizedParsedCatalog,
    source_keys: &std::collections::BTreeSet<CatalogMessageKey>,
    include_details: bool,
) -> CatalogLocaleCoverage {
    let mut coverage = CatalogLocaleCoverage {
        locale: locale.to_owned(),
        total: source_keys.len(),
        ..CatalogLocaleCoverage::default()
    };

    for source_key in source_keys {
        let status = classify_expected_message(target_catalog, source_key);
        increment_status(&mut coverage, status);
        if include_details {
            coverage.details.push(CatalogCoverageMessage {
                locale: locale.to_owned(),
                source_key: source_key.clone(),
                status,
            });
        }
    }

    for (key, message) in target_catalog.iter() {
        if is_extra_target_message(source_keys, key, message) {
            increment_status(&mut coverage, CatalogMessageStatus::Extra);
            if include_details {
                coverage.details.push(CatalogCoverageMessage {
                    locale: locale.to_owned(),
                    source_key: key.clone(),
                    status: CatalogMessageStatus::Extra,
                });
            }
        }
    }

    coverage
}

fn increment_status(coverage: &mut CatalogLocaleCoverage, status: CatalogMessageStatus) {
    match status {
        CatalogMessageStatus::Translated => coverage.translated += 1,
        CatalogMessageStatus::Fuzzy => coverage.fuzzy += 1,
        CatalogMessageStatus::Missing => coverage.missing += 1,
        CatalogMessageStatus::Empty => coverage.empty += 1,
        CatalogMessageStatus::Obsolete => coverage.obsolete += 1,
        CatalogMessageStatus::Extra => coverage.extra += 1,
    }
}
