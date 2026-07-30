use std::collections::BTreeSet;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::{CatalogMessage, CatalogMessageKey, EffectiveTranslationRef, NormalizedParsedCatalog};

/// Canonical catalog message status used by coverage and review reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum CatalogMessageStatus {
    /// Active target message exists and has a non-empty, non-fuzzy translation.
    Translated,
    /// No active or obsolete target entry exists for the active source identity.
    Missing,
    /// Active target entry exists, but its effective translation is empty.
    Empty,
    /// Target entry exists for the active source identity, but is obsolete.
    Obsolete,
    /// Active target entry is not present in the active source identity set.
    Extra,
    /// Active target message exists, has a non-empty translation, and carries
    /// the semantic `fuzzy` review marker.
    Fuzzy,
}

pub(super) fn active_message_keys(
    catalog: &NormalizedParsedCatalog,
) -> BTreeSet<CatalogMessageKey> {
    catalog
        .iter()
        .filter_map(|(key, message)| (message.obsolete.is_none()).then_some(key.clone()))
        .collect()
}

pub(super) fn classify_expected_message(
    target_catalog: &NormalizedParsedCatalog,
    key: &CatalogMessageKey,
) -> CatalogMessageStatus {
    let Some(message) = target_catalog.get(key) else {
        return CatalogMessageStatus::Missing;
    };
    if message.obsolete.is_some() {
        return CatalogMessageStatus::Obsolete;
    }
    if translation_is_empty(message) {
        return CatalogMessageStatus::Empty;
    }
    if target_catalog.is_fuzzy(key) {
        return CatalogMessageStatus::Fuzzy;
    }
    CatalogMessageStatus::Translated
}

pub(super) fn is_extra_target_message(
    source_keys: &BTreeSet<CatalogMessageKey>,
    key: &CatalogMessageKey,
    message: &CatalogMessage,
) -> bool {
    message.obsolete.is_none() && !source_keys.contains(key)
}

pub(super) fn translation_is_empty(message: &CatalogMessage) -> bool {
    match message.effective_translation() {
        EffectiveTranslationRef::Singular(value) => value.trim().is_empty(),
        EffectiveTranslationRef::Plural(translations) => {
            translations.is_empty() || translations.values().any(|value| value.trim().is_empty())
        }
    }
}
