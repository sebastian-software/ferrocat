//! Small collection helpers used by the catalog API.
//!
//! These helpers intentionally keep tiny collections on simple linear scans
//! before switching to `BTreeSet`-based deduplication. Most comment/origin/
//! placeholder lists in real catalogs are small, so this avoids needless
//! allocation on the common path.

use std::collections::{BTreeMap, BTreeSet};

use super::CatalogOrigin;

/// Deduplicates strings while preserving first-seen order.
pub(super) fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        if !push_unique_string(&out, &value) {
            out.push(value);
        }
    }
    out
}

/// Merges strings into `target` without reordering existing entries.
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

/// Fast membership check used by the small-vector path above.
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

/// Deduplicates origins while preserving first-seen order.
pub(super) fn dedupe_origins(values: Vec<CatalogOrigin>) -> Vec<CatalogOrigin> {
    let mut out = Vec::new();
    for value in values {
        if !push_unique_origin(&out, &value) {
            out.push(value);
        }
    }
    out
}

/// Merges origins into `target` without reordering existing entries.
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

/// Fast membership check used by the small-origin merge path above.
pub(super) fn push_unique_origin(target: &[CatalogOrigin], value: &CatalogOrigin) -> bool {
    target
        .iter()
        .any(|origin| origin.file == value.file && origin.line == value.line)
}

/// Deduplicates placeholder example values per placeholder name.
pub(super) fn dedupe_placeholders(
    placeholders: BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, Vec<String>> {
    placeholders
        .into_iter()
        .map(|(key, values)| (key, dedupe_strings(values)))
        .collect()
}

/// Merges placeholder example values per placeholder name while preserving order.
pub(super) fn merge_placeholders(
    target: &mut BTreeMap<String, Vec<String>>,
    incoming: BTreeMap<String, Vec<String>>,
) {
    for (key, values) in incoming {
        merge_unique_strings(target.entry(key).or_default(), values);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        dedupe_origins, dedupe_placeholders, dedupe_strings, merge_placeholders,
        merge_unique_origins, merge_unique_strings, push_unique_origin, push_unique_string,
    };
    use crate::api::CatalogOrigin;

    #[test]
    fn dedupe_and_merge_strings_preserve_first_seen_order() {
        assert_eq!(
            dedupe_strings(vec![
                "alpha".to_owned(),
                "beta".to_owned(),
                "alpha".to_owned(),
                "gamma".to_owned(),
            ]),
            vec!["alpha".to_owned(), "beta".to_owned(), "gamma".to_owned(),]
        );

        let mut small = vec!["alpha".to_owned()];
        merge_unique_strings(
            &mut small,
            vec!["alpha".to_owned(), "beta".to_owned(), "beta".to_owned()],
        );
        assert_eq!(small, vec!["alpha".to_owned(), "beta".to_owned()]);

        let mut large = vec![
            "a".to_owned(),
            "b".to_owned(),
            "c".to_owned(),
            "d".to_owned(),
            "e".to_owned(),
            "f".to_owned(),
        ];
        merge_unique_strings(
            &mut large,
            vec!["b".to_owned(), "g".to_owned(), "h".to_owned()],
        );
        assert_eq!(
            large,
            vec![
                "a".to_owned(),
                "b".to_owned(),
                "c".to_owned(),
                "d".to_owned(),
                "e".to_owned(),
                "f".to_owned(),
                "g".to_owned(),
                "h".to_owned(),
            ]
        );
        assert!(push_unique_string(&large, "h"));
        assert!(!push_unique_string(&large, "missing"));
    }

    #[test]
    fn dedupe_and_merge_origins_keep_unique_entries() {
        let origin_a = CatalogOrigin {
            file: "src/a.rs".to_owned(),
            line: Some(1),
        };
        let origin_b = CatalogOrigin {
            file: "src/b.rs".to_owned(),
            line: None,
        };

        assert_eq!(
            dedupe_origins(vec![origin_a.clone(), origin_b.clone(), origin_a.clone()]),
            vec![origin_a.clone(), origin_b.clone()]
        );

        let mut merged = vec![origin_a.clone()];
        merge_unique_origins(
            &mut merged,
            vec![origin_a.clone(), origin_b.clone(), origin_b.clone()],
        );
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0], origin_a);
        assert_eq!(merged[1], origin_b);
        assert!(push_unique_origin(&merged, &origin_b));
        assert!(!push_unique_origin(
            &merged,
            &CatalogOrigin {
                file: "src/c.rs".to_owned(),
                line: Some(2),
            }
        ));
    }

    #[test]
    fn placeholder_helpers_dedupe_and_merge_per_key() {
        let deduped = dedupe_placeholders(BTreeMap::from([
            (
                "count".to_owned(),
                vec!["1".to_owned(), "2".to_owned(), "1".to_owned()],
            ),
            ("name".to_owned(), vec!["Ada".to_owned(), "Ada".to_owned()]),
        ]));
        assert_eq!(
            deduped,
            BTreeMap::from([
                ("count".to_owned(), vec!["1".to_owned(), "2".to_owned()]),
                ("name".to_owned(), vec!["Ada".to_owned()]),
            ])
        );

        let mut merged = BTreeMap::from([("count".to_owned(), vec!["1".to_owned()])]);
        merge_placeholders(
            &mut merged,
            BTreeMap::from([
                (
                    "count".to_owned(),
                    vec!["1".to_owned(), "3".to_owned(), "3".to_owned()],
                ),
                ("name".to_owned(), vec!["Ada".to_owned()]),
            ]),
        );
        assert_eq!(
            merged,
            BTreeMap::from([
                ("count".to_owned(), vec!["1".to_owned(), "3".to_owned()]),
                ("name".to_owned(), vec!["Ada".to_owned()]),
            ])
        );
    }
}
