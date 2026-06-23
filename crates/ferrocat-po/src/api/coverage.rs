use std::collections::BTreeMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

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
    let catalog_index = index_catalogs(catalogs)?;
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
    let target_locales = select_target_locales(&catalog_index, options)?;
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
            coverage.extra += 1;
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
                    "catalog_coverage requires every catalog to declare a locale".to_owned(),
                )
            })?;
        if index.insert(locale.to_owned(), *catalog).is_some() {
            return Err(ApiError::InvalidArguments(format!(
                "catalog_coverage received duplicate catalog locale {locale:?}"
            )));
        }
    }
    Ok(index)
}

fn select_target_locales(
    catalog_index: &BTreeMap<String, &NormalizedParsedCatalog>,
    options: &CatalogCoverageOptions<'_>,
) -> Result<Vec<String>, ApiError> {
    if options.locales.is_empty() {
        return Ok(catalog_index
            .keys()
            .filter(|locale| locale.as_str() != options.source_locale)
            .cloned()
            .collect());
    }

    let mut seen = std::collections::BTreeSet::new();
    let mut locales = Vec::new();
    for locale in options.locales {
        if *locale == options.source_locale || !seen.insert((*locale).to_owned()) {
            continue;
        }
        if !catalog_index.contains_key(*locale) {
            return Err(ApiError::InvalidArguments(format!(
                "catalog_coverage did not receive requested locale {locale:?}"
            )));
        }
        locales.push((*locale).to_owned());
    }
    Ok(locales)
}

#[cfg(test)]
mod tests {
    use super::{CatalogCoverageOptions, catalog_coverage};
    use crate::api::{CatalogMessageStatus, CatalogMode, ParseCatalogOptions, parse_catalog};

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
    fn catalog_coverage_counts_locale_statuses() {
        let source = catalog(
            concat!(
                "msgid \"Hello\"\nmsgstr \"Hello\"\n\n",
                "msgid \"Empty\"\nmsgstr \"Empty\"\n\n",
                "msgid \"Fuzzy\"\nmsgstr \"Fuzzy\"\n\n",
                "msgid \"Gone\"\nmsgstr \"Gone\"\n\n",
                "msgid \"Missing\"\nmsgstr \"Missing\"\n",
            ),
            "en",
        );
        let target = catalog(
            concat!(
                "msgid \"Hello\"\nmsgstr \"Hallo\"\n\n",
                "msgid \"Empty\"\nmsgstr \"\"\n\n",
                "#, fuzzy\nmsgid \"Fuzzy\"\nmsgstr \"Unscharf\"\n\n",
                "#~ msgid \"Gone\"\n#~ msgstr \"Weg\"\n\n",
                "msgid \"Extra\"\nmsgstr \"Extra\"\n",
            ),
            "de",
        );

        let report = catalog_coverage(
            &[&source, &target],
            &CatalogCoverageOptions::new("en").with_details(true),
        )
        .expect("coverage");
        let locale = &report.locales[0];

        assert_eq!(report.source_messages, 5);
        assert_eq!(locale.translated, 1);
        assert_eq!(locale.empty, 1);
        assert_eq!(locale.fuzzy, 1);
        assert_eq!(locale.obsolete, 1);
        assert_eq!(locale.missing, 1);
        assert_eq!(locale.extra, 1);
        assert_eq!(locale.incomplete(), 4);
        assert_eq!(locale.completion_percent(), 20.0);
        assert!(
            locale
                .details
                .iter()
                .any(|detail| detail.status == CatalogMessageStatus::Extra)
        );
    }

    #[test]
    fn catalog_coverage_filters_requested_locales() {
        let source = catalog("msgid \"Hello\"\nmsgstr \"Hello\"\n", "en");
        let de = catalog("msgid \"Hello\"\nmsgstr \"Hallo\"\n", "de");
        let fr = catalog("msgid \"Hello\"\nmsgstr \"Bonjour\"\n", "fr");
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
}
