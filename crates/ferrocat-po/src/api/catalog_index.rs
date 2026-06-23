use std::collections::{BTreeMap, BTreeSet};

use super::{ApiError, NormalizedParsedCatalog};

pub(super) fn index_catalogs<'a>(
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

pub(super) fn select_target_locales(
    catalog_index: &BTreeMap<String, &NormalizedParsedCatalog>,
    source_locale: &str,
    requested_locales: &[&str],
    label: &str,
) -> Result<Vec<String>, ApiError> {
    if requested_locales.is_empty() {
        return Ok(catalog_index
            .keys()
            .filter(|locale| locale.as_str() != source_locale)
            .cloned()
            .collect());
    }

    let mut seen = BTreeSet::new();
    let mut locales = Vec::new();
    for locale in requested_locales {
        if *locale == source_locale || !seen.insert((*locale).to_owned()) {
            continue;
        }
        if !catalog_index.contains_key(*locale) {
            return Err(ApiError::InvalidArguments(format!(
                "{label} did not receive requested locale {locale:?}"
            )));
        }
        locales.push((*locale).to_owned());
    }
    Ok(locales)
}
