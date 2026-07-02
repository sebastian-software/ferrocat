use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::catalog_index::{index_catalogs, select_target_locales};
use super::message_status::{active_message_keys, classify_expected_message};
use super::mt::validate_machine_metadata;
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
/// Translation change details are limited to source identities whose current
/// target status is [`CatalogMessageStatus::Translated`]; missing, empty, and
/// obsolete current entries are surfaced by the coverage counters.
///
/// # Examples
///
/// ```rust
/// use ferrocat_po::{CatalogReviewOptions, ParseCatalogOptions, catalog_review, parse_catalog};
///
/// let previous_source = parse_catalog(
///     ParseCatalogOptions::new("msgid \"Save\"\nmsgstr \"\"\n", "en").with_locale("en"),
/// )?
/// .into_normalized_view()?;
/// let previous_target = parse_catalog(
///     ParseCatalogOptions::new("msgid \"Save\"\nmsgstr \"Speichern\"\n", "en")
///         .with_locale("de"),
/// )?
/// .into_normalized_view()?;
///
/// let current_source = parse_catalog(
///     ParseCatalogOptions::new(
///         "msgid \"Save\"\nmsgstr \"\"\n\nmsgid \"Cancel\"\nmsgstr \"\"\n",
///         "en",
///     )
///     .with_locale("en"),
/// )?
/// .into_normalized_view()?;
/// let current_target = parse_catalog(
///     ParseCatalogOptions::new(
///         "msgid \"Save\"\nmsgstr \"Sichern\"\n\nmsgid \"Cancel\"\nmsgstr \"\"\n",
///         "en",
///     )
///     .with_locale("de"),
/// )?
/// .into_normalized_view()?;
///
/// let options = CatalogReviewOptions::new("en").with_details(true);
/// let report = catalog_review(
///     &[&previous_source, &previous_target],
///     &[&current_source, &current_target],
///     &options,
/// )?;
///
/// assert_eq!(report.summary.source_added, 1);
/// assert_eq!(report.summary.translation_changed, 1);
/// assert_eq!(report.locales[0].coverage.empty, 1);
/// # Ok::<(), ferrocat_po::ApiError>(())
/// ```
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
    let target_locales = select_target_locales(
        &current_index,
        options.source_locale,
        options.locales,
        "catalog_review",
    )?;
    let source_changes = source_change_report(
        &previous_source_keys,
        &current_source_keys,
        options.include_details,
    );
    let coverage_options = CatalogCoverageOptions {
        source_locale: options.source_locale,
        locales: options.locales,
        include_details: options.include_details,
    };
    let coverage = catalog_coverage(current_catalogs, &coverage_options)?;
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
            .filter(|message| message.obsolete.is_none())
        else {
            continue;
        };
        let current_message = current_target
            .get(source_key)
            .filter(|message| message.obsolete.is_none())
            .expect("translated classification must have an active current message");
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
        if message.obsolete.is_some() {
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
    let Some(metadata) = message.machine.as_ref() else {
        return CatalogMachineTranslationStatus::Absent;
    };
    if validate_machine_metadata(metadata).is_err() {
        return CatalogMachineTranslationStatus::Invalid;
    }
    if metadata.lock == machine_translation_hash(message.effective_translation()) {
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        CatalogMachineTranslationStatus, CatalogReviewOptions, CatalogReviewTranslation,
        CatalogSourceChangeKind, catalog_review,
    };
    use crate::api::{
        AiProvenance, ApiError, CatalogMessage, CatalogMessageKey, CatalogMode, CatalogSemantics,
        EffectiveTranslationRef, MachineMetadata, ParseCatalogOptions, ParsedCatalog,
        TranslationShape, machine_translation_hash, parse_catalog,
    };

    fn catalog(content: &str, locale: &str) -> crate::api::NormalizedParsedCatalog {
        catalog_with_mode(content, Some(locale), CatalogMode::IcuPo)
    }

    fn catalog_with_locale(
        content: &str,
        locale: Option<&str>,
    ) -> crate::api::NormalizedParsedCatalog {
        catalog_with_mode(content, locale, CatalogMode::IcuPo)
    }

    fn gettext_catalog(content: &str, locale: &str) -> crate::api::NormalizedParsedCatalog {
        catalog_with_mode(content, Some(locale), CatalogMode::GettextPo)
    }

    fn catalog_with_mode(
        content: &str,
        locale: Option<&str>,
        mode: CatalogMode,
    ) -> crate::api::NormalizedParsedCatalog {
        parse_catalog(ParseCatalogOptions {
            locale,
            mode,
            ..ParseCatalogOptions::new(content, "en")
        })
        .expect("parse catalog")
        .into_normalized_view()
        .expect("normalize catalog")
    }

    fn catalog_with_messages(
        locale: &str,
        messages: Vec<CatalogMessage>,
    ) -> crate::api::NormalizedParsedCatalog {
        ParsedCatalog {
            locale: Some(locale.to_owned()),
            semantics: CatalogSemantics::IcuNative,
            headers: BTreeMap::new(),
            messages,
            diagnostics: Vec::new(),
        }
        .into_normalized_view()
        .expect("normalize catalog")
    }

    fn error_debug(error: ApiError) -> String {
        format!("{error:?}")
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
                    "#@ lock: {}\n#@ ai: openai/gpt-5.5-high\n",
                    "msgid \"Hello\"\nmsgstr \"Hallo\"\n\n",
                    "#@ lock: old\n#@ ai: openai/gpt-5.5-high\n",
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
    fn catalog_review_reports_invalid_machine_translation_metadata() {
        let source = catalog("msgid \"Hello\"\nmsgstr \"Hello\"\n", "en");
        let target = catalog_with_messages(
            "de",
            vec![CatalogMessage {
                msgid: "Hello".to_owned(),
                msgctxt: None,
                translation: TranslationShape::Singular {
                    value: "Hallo".to_owned(),
                },
                comments: Vec::new(),
                origin: crate::PoVec::new(),
                obsolete: None,
                machine: Some(MachineMetadata {
                    lock: machine_translation_hash(EffectiveTranslationRef::Singular("Hallo")),
                    ai: Some(AiProvenance {
                        model: String::new(),
                        confidence: None,
                    }),
                }),
            }],
        );

        let report = catalog_review(
            &[&source, &target],
            &[&source, &target],
            &CatalogReviewOptions::new("en").with_details(true),
        )
        .expect("review");
        let machine_translation = &report.locales[0].machine_translation;

        assert_eq!(machine_translation.invalid, 1);
        assert_eq!(report.summary.machine_translation_invalid, 1);
        assert!(machine_translation.details.iter().any(|detail| {
            detail.source_key == CatalogMessageKey::new("Hello", None)
                && detail.status == CatalogMachineTranslationStatus::Invalid
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

    #[test]
    fn catalog_review_rejects_invalid_locale_inputs() {
        let source = catalog("msgid \"Hello\"\nmsgstr \"Hello\"\n", "en");
        let duplicate_source = catalog("msgid \"Bye\"\nmsgstr \"Bye\"\n", "en");
        let missing_locale = catalog_with_locale("msgid \"Hello\"\nmsgstr \"Hallo\"\n", None);
        let target = catalog("msgid \"Hello\"\nmsgstr \"Hallo\"\n", "de");
        let requested = ["fr"];

        let missing_previous_source = catalog_review(
            &[&target],
            &[&source, &target],
            &CatalogReviewOptions::new("en"),
        )
        .expect_err("missing previous source should fail");
        assert!(error_debug(missing_previous_source).contains("previous catalogs"));

        let missing_current_source = catalog_review(
            &[&source, &target],
            &[&target],
            &CatalogReviewOptions::new("en"),
        )
        .expect_err("missing current source should fail");
        assert!(error_debug(missing_current_source).contains("current catalogs"));

        let undeclared_locale = catalog_review(
            &[&missing_locale],
            &[&source, &target],
            &CatalogReviewOptions::new("en"),
        )
        .expect_err("missing declared locale should fail");
        assert!(error_debug(undeclared_locale).contains("declare a locale"));

        let duplicate_locale = catalog_review(
            &[&source, &duplicate_source],
            &[&source, &target],
            &CatalogReviewOptions::new("en"),
        )
        .expect_err("duplicate locale should fail");
        assert!(error_debug(duplicate_locale).contains("duplicate catalog locale"));

        let missing_requested = catalog_review(
            &[&source, &target],
            &[&source, &target],
            &CatalogReviewOptions {
                locales: &requested,
                ..CatalogReviewOptions::new("en")
            },
        )
        .expect_err("missing requested locale should fail");
        assert!(error_debug(missing_requested).contains("requested locale"));
    }

    #[test]
    fn catalog_review_filters_requested_locales_and_handles_new_target_locale() {
        let source = catalog("msgid \"Hello\"\nmsgstr \"Hello\"\n", "en");
        let de = catalog("msgid \"Hello\"\nmsgstr \"Hallo\"\n", "de");
        let fr = catalog("msgid \"Hello\"\nmsgstr \"Bonjour\"\n", "fr");
        let requested = ["en", "de", "de", "fr"];

        let report = catalog_review(
            &[&source],
            &[&source, &de, &fr],
            &CatalogReviewOptions {
                locales: &requested,
                ..CatalogReviewOptions::new("en")
            },
        )
        .expect("review");

        assert_eq!(report.summary.target_locales, 2);
        assert_eq!(report.locales[0].locale, "de");
        assert_eq!(report.locales[1].locale, "fr");
        assert_eq!(report.summary.translation_changed, 0);
        assert_eq!(report.locales[0].translations.changed, 0);
    }

    #[test]
    fn catalog_review_ignores_new_messages_when_tracking_translation_changes() {
        let previous_source = catalog("msgid \"Hello\"\nmsgstr \"Hello\"\n", "en");
        let current_source = catalog(
            "msgid \"Hello\"\nmsgstr \"Hello\"\n\nmsgid \"Added\"\nmsgstr \"Added\"\n",
            "en",
        );
        let previous_target = catalog("msgid \"Hello\"\nmsgstr \"Hallo\"\n", "de");
        let current_target = catalog(
            "msgid \"Hello\"\nmsgstr \"Hallo\"\n\nmsgid \"Added\"\nmsgstr \"Neu\"\n",
            "de",
        );

        let report = catalog_review(
            &[&previous_source, &previous_target],
            &[&current_source, &current_target],
            &CatalogReviewOptions::new("en").with_details(true),
        )
        .expect("review");

        assert_eq!(report.locales[0].translations.changed, 0);
        assert!(report.locales[0].translations.details.is_empty());
    }

    #[test]
    fn catalog_review_reports_plural_translation_changes() {
        let previous_source = gettext_catalog(
            concat!(
                "msgid \"book\"\n",
                "msgid_plural \"books\"\n",
                "msgstr[0] \"book\"\n",
                "msgstr[1] \"books\"\n",
            ),
            "en",
        );
        let current_source = gettext_catalog(
            concat!(
                "msgid \"book\"\n",
                "msgid_plural \"books\"\n",
                "msgstr[0] \"book\"\n",
                "msgstr[1] \"books\"\n",
            ),
            "en",
        );
        let previous_target = gettext_catalog(
            concat!(
                "msgid \"book\"\n",
                "msgid_plural \"books\"\n",
                "msgstr[0] \"Buch\"\n",
                "msgstr[1] \"Buecher\"\n",
            ),
            "de",
        );
        let current_target = gettext_catalog(
            concat!(
                "msgid \"book\"\n",
                "msgid_plural \"books\"\n",
                "msgstr[0] \"Buch\"\n",
                "msgstr[1] \"Buecher neu\"\n",
            ),
            "de",
        );

        let report = catalog_review(
            &[&previous_source, &previous_target],
            &[&current_source, &current_target],
            &CatalogReviewOptions::new("en").with_details(true),
        )
        .expect("review");
        let detail = &report.locales[0].translations.details[0];

        assert_eq!(report.locales[0].translations.changed, 1);
        assert!(matches!(
            detail.current,
            CatalogReviewTranslation::Plural(_)
        ));
    }

    #[test]
    fn catalog_review_skips_obsolete_machine_translation_entries() {
        let source = catalog("msgid \"Hello\"\nmsgstr \"Hello\"\n", "en");
        let target = catalog(
            "msgid \"Hello\"\nmsgstr \"Hallo\"\n\n#~ msgid \"Old\"\n#~ msgstr \"Alt\"\n",
            "de",
        );

        let report = catalog_review(
            &[&source, &target],
            &[&source, &target],
            &CatalogReviewOptions::new("en").with_details(true),
        )
        .expect("review");
        let machine_translation = &report.locales[0].machine_translation;

        assert_eq!(machine_translation.absent, 1);
        assert_eq!(machine_translation.details.len(), 1);
        assert_eq!(
            machine_translation.details[0].source_key,
            CatalogMessageKey::new("Hello", None)
        );
    }
}
