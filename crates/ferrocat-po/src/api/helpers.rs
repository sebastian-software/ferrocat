use std::collections::{BTreeMap, BTreeSet};

use super::CatalogOrigin;

pub(super) fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        if !push_unique_string(&out, &value) {
            out.push(value);
        }
    }
    out
}

pub(super) fn merge_unique_strings(target: &mut Vec<String>, incoming: Vec<String>) {
    if target.len() + incoming.len() < 8 {
        for value in incoming {
            if !push_unique_string(target, &value) {
                target.push(value);
            }
        }
        return;
    }

    let mut seen = target.iter().cloned().collect::<BTreeSet<_>>();
    for value in incoming {
        if seen.insert(value.clone()) {
            target.push(value);
        }
    }
}

pub(super) fn push_unique_string(target: &[String], value: &str) -> bool {
    if target.len() < 8 {
        target.iter().any(|existing| existing == value)
    } else {
        target
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .contains(value)
    }
}

pub(super) fn dedupe_origins(values: Vec<CatalogOrigin>) -> Vec<CatalogOrigin> {
    let mut out = Vec::new();
    for value in values {
        if !push_unique_origin(&out, &value) {
            out.push(value);
        }
    }
    out
}

pub(super) fn merge_unique_origins(target: &mut Vec<CatalogOrigin>, incoming: Vec<CatalogOrigin>) {
    if target.len() + incoming.len() < 8 {
        for value in incoming {
            if !push_unique_origin(target, &value) {
                target.push(value);
            }
        }
        return;
    }

    let mut seen = target
        .iter()
        .map(|origin| (origin.file.clone(), origin.line))
        .collect::<BTreeSet<_>>();
    for value in incoming {
        if seen.insert((value.file.clone(), value.line)) {
            target.push(value);
        }
    }
}

pub(super) fn push_unique_origin(target: &[CatalogOrigin], value: &CatalogOrigin) -> bool {
    target
        .iter()
        .any(|origin| origin.file == value.file && origin.line == value.line)
}

pub(super) fn dedupe_placeholders(
    placeholders: BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, Vec<String>> {
    placeholders
        .into_iter()
        .map(|(key, values)| (key, dedupe_strings(values)))
        .collect()
}

pub(super) fn merge_placeholders(
    target: &mut BTreeMap<String, Vec<String>>,
    incoming: BTreeMap<String, Vec<String>>,
) {
    for (key, values) in incoming {
        merge_unique_strings(target.entry(key).or_default(), values);
    }
}
