//! Small collection helpers used by the catalog API.
//!
//! Most comment/origin/placeholder lists in real catalogs are tiny and already
//! unique, so these helpers first handle the common shapes (empty input, an
//! empty merge target, nothing to deduplicate) without allocating anything.
//! Only inputs large enough for the linear scans to dominate fall back to a
//! hash set, which is used purely for membership: the emitted order always
//! comes from the input order, never from the set.

use std::collections::BTreeMap;

use rustc_hash::{FxBuildHasher, FxHashSet};

use super::CatalogOrigin;
use crate::PoVec;

/// Incoming lists up to this length stay on linear scans. A membership set
/// costs one hash plus one insert per element on *both* sides, so it only pays
/// off once the incoming list is long enough that the scans dominate.
const MAX_LINEAR_INCOMING: usize = 4;

/// Combined length below which a merge or dedupe stays on linear scans even
/// when the incoming list is longer than [`MAX_LINEAR_INCOMING`].
const MAX_LINEAR_TOTAL: usize = 8;

/// Deduplicates strings while preserving first-seen order.
pub(super) fn dedupe_strings(mut values: Vec<String>) -> Vec<String> {
    dedupe_strings_in_place(&mut values);
    values
}

/// Deduplicates `values` in place, preserving first-seen order.
///
/// Nothing is allocated and nothing is moved when the list is already unique,
/// which is the overwhelmingly common case for extracted comments and
/// placeholder examples.
fn dedupe_strings_in_place(values: &mut Vec<String>) {
    if values.len() < 2 {
        return;
    }

    if values.len() < MAX_LINEAR_TOTAL {
        let mut kept = 0;
        for index in 0..values.len() {
            if !push_unique_string(&values[..kept], &values[index]) {
                values.swap(kept, index);
                kept += 1;
            }
        }
        values.truncate(kept);
        return;
    }

    dedupe_strings_with_seen(values);
}

fn dedupe_strings_with_seen(values: &mut Vec<String>) {
    let mut selected_indexes = Vec::with_capacity(values.len());
    {
        let mut seen = FxHashSet::with_capacity_and_hasher(values.len(), FxBuildHasher);
        for (index, value) in values.iter().enumerate() {
            if seen.insert(value.as_str()) {
                selected_indexes.push(index);
            }
        }
    }

    if selected_indexes.len() == values.len() {
        return;
    }

    // `selected_indexes` is strictly increasing, so `kept` never runs ahead of
    // the index being read and the retained entries keep their first-seen
    // order. Duplicates are swapped into the tail and dropped by `truncate`.
    let mut kept = 0;
    for index in selected_indexes {
        values.swap(kept, index);
        kept += 1;
    }
    values.truncate(kept);
}

/// Merges strings into `target` without reordering existing entries.
pub(super) fn merge_unique_strings(target: &mut Vec<String>, incoming: Vec<String>) {
    if incoming.is_empty() {
        return;
    }

    if target.is_empty() {
        *target = dedupe_strings(incoming);
        return;
    }

    if incoming.len() <= MAX_LINEAR_INCOMING || target.len() + incoming.len() < MAX_LINEAR_TOTAL {
        for value in incoming {
            if !push_unique_string(target, &value) {
                target.push(value);
            }
        }
        return;
    }

    let mut selected_indexes = Vec::with_capacity(incoming.len());
    {
        let mut seen = target.iter().map(String::as_str).collect::<FxHashSet<_>>();
        for (index, value) in incoming.iter().enumerate() {
            if seen.insert(value.as_str()) {
                selected_indexes.push(index);
            }
        }
    }

    if selected_indexes.len() == incoming.len() {
        target.extend(incoming);
        return;
    }

    let mut selected_indexes = selected_indexes.into_iter();
    let mut next_selected = selected_indexes.next();
    for (index, value) in incoming.into_iter().enumerate() {
        if next_selected == Some(index) {
            target.push(value);
            next_selected = selected_indexes.next();
        }
    }
}

/// Fast membership check used by the small-vector path above.
pub(super) fn push_unique_string(target: &[String], value: &str) -> bool {
    target.iter().any(|existing| existing == value)
}

/// Deduplicates origins while preserving first-seen order.
///
/// Origin lists are short, so the linear membership scan stays cheaper than any
/// set. Reserving up front keeps the single-origin case inline and avoids the
/// regrowth steps for the rare longer lists.
pub(super) fn dedupe_origins(
    values: impl IntoIterator<Item = CatalogOrigin>,
) -> PoVec<CatalogOrigin> {
    let values = values.into_iter();
    let mut out = PoVec::with_capacity(values.size_hint().0);
    for value in values {
        if !push_unique_origin(&out, &value) {
            out.push(value);
        }
    }
    out
}

/// Merges origins into `target` without reordering existing entries.
pub(super) fn merge_unique_origins(
    target: &mut PoVec<CatalogOrigin>,
    incoming: PoVec<CatalogOrigin>,
) {
    if incoming.is_empty() {
        return;
    }

    if target.is_empty() {
        *target = dedupe_origins(incoming);
        return;
    }

    if incoming.len() <= MAX_LINEAR_INCOMING || target.len() + incoming.len() < MAX_LINEAR_TOTAL {
        for value in incoming {
            if !push_unique_origin(target, &value) {
                target.push(value);
            }
        }
        return;
    }

    let mut selected_indexes = Vec::with_capacity(incoming.len());
    {
        let mut seen = target
            .iter()
            .map(|origin| (origin.file.as_str(), origin.scope.as_deref()))
            .collect::<FxHashSet<_>>();
        for (index, value) in incoming.iter().enumerate() {
            if seen.insert((value.file.as_str(), value.scope.as_deref())) {
                selected_indexes.push(index);
            }
        }
    }

    if selected_indexes.len() == incoming.len() {
        target.extend(incoming);
        return;
    }

    let mut selected_indexes = selected_indexes.into_iter();
    let mut next_selected = selected_indexes.next();
    for (index, value) in incoming.into_iter().enumerate() {
        if next_selected == Some(index) {
            target.push(value);
            next_selected = selected_indexes.next();
        }
    }
}

/// Fast membership check used by the small-origin merge path above.
pub(super) fn push_unique_origin(target: &[CatalogOrigin], value: &CatalogOrigin) -> bool {
    target.iter().any(|origin| origin == value)
}

/// Deduplicates placeholder example values per placeholder name.
///
/// The map is deduplicated in place: the keys are untouched, so the tree does
/// not have to be rebuilt just to normalize the value lists.
pub(super) fn dedupe_placeholders(
    mut placeholders: BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, Vec<String>> {
    for values in placeholders.values_mut() {
        dedupe_strings_in_place(values);
    }
    placeholders
}

/// Merges placeholder example values per placeholder name while preserving order.
pub(super) fn merge_placeholders(
    target: &mut BTreeMap<String, Vec<String>>,
    incoming: BTreeMap<String, Vec<String>>,
) {
    if incoming.is_empty() {
        return;
    }

    // Merging into an empty map can only ever produce the deduplicated incoming
    // map, so take it wholesale instead of inserting key by key.
    if target.is_empty() {
        *target = dedupe_placeholders(incoming);
        return;
    }

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
    use crate::PoVec;
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
        assert_eq!(
            dedupe_strings(vec![
                "a".to_owned(),
                "b".to_owned(),
                "c".to_owned(),
                "d".to_owned(),
                "e".to_owned(),
                "f".to_owned(),
                "g".to_owned(),
                "h".to_owned(),
                "a".to_owned(),
                "i".to_owned(),
            ]),
            vec![
                "a".to_owned(),
                "b".to_owned(),
                "c".to_owned(),
                "d".to_owned(),
                "e".to_owned(),
                "f".to_owned(),
                "g".to_owned(),
                "h".to_owned(),
                "i".to_owned(),
            ]
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
            scope: None,
        };
        let origin_b = CatalogOrigin {
            file: "src/b.rs".to_owned(),
            scope: None,
        };

        assert_eq!(
            dedupe_origins(vec![origin_a.clone(), origin_b.clone(), origin_a.clone()]).as_slice(),
            vec![origin_a.clone(), origin_b.clone()].as_slice()
        );

        let mut merged = PoVec::from(vec![origin_a.clone()]);
        merge_unique_origins(
            &mut merged,
            vec![origin_a.clone(), origin_b.clone(), origin_b.clone()].into(),
        );
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0], origin_a);
        assert_eq!(merged[1], origin_b);
        assert!(push_unique_origin(&merged, &origin_b));
        assert!(!push_unique_origin(
            &merged,
            &CatalogOrigin {
                file: "src/c.rs".to_owned(),
                scope: None,
            }
        ));
    }

    #[test]
    fn merge_origins_uses_set_path_for_larger_inputs() {
        let mut merged = (0..6)
            .map(|index| CatalogOrigin {
                file: format!("src/{index}.rs"),
                scope: None,
            })
            .collect::<PoVec<_>>();

        merge_unique_origins(
            &mut merged,
            vec![
                CatalogOrigin {
                    file: "src/1.rs".to_owned(),
                    scope: None,
                },
                CatalogOrigin {
                    file: "src/6.rs".to_owned(),
                    scope: None,
                },
                CatalogOrigin {
                    file: "src/7.rs".to_owned(),
                    scope: None,
                },
            ]
            .into(),
        );

        assert_eq!(merged.len(), 8);
        assert_eq!(merged[6].file, "src/6.rs");
        assert_eq!(merged[7].file, "src/7.rs");

        // Long incoming lists take the membership-set path; scopes stay part of
        // the identity there, so `src/1.rs#scope` is a distinct origin.
        let mut merged = (0..6)
            .map(|index| CatalogOrigin {
                file: format!("src/{index}.rs"),
                scope: None,
            })
            .collect::<PoVec<_>>();
        let incoming = (0..10)
            .map(|index| CatalogOrigin {
                file: format!("src/{index}.rs"),
                scope: (index == 1).then(|| "scope".to_owned()),
            })
            .collect::<PoVec<_>>();
        merge_unique_origins(&mut merged, incoming);

        assert_eq!(
            merged
                .iter()
                .map(|origin| (origin.file.as_str(), origin.scope.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("src/0.rs", None),
                ("src/1.rs", None),
                ("src/2.rs", None),
                ("src/3.rs", None),
                ("src/4.rs", None),
                ("src/5.rs", None),
                ("src/1.rs", Some("scope")),
                ("src/6.rs", None),
                ("src/7.rs", None),
                ("src/8.rs", None),
                ("src/9.rs", None),
            ]
        );
    }

    /// Reference implementation of the shared contract: keep the first
    /// occurrence of every value, in input order.
    fn reference_merge(target: &[String], incoming: &[String]) -> Vec<String> {
        let mut out = target.to_vec();
        for value in incoming {
            if !out.iter().any(|existing| existing == value) {
                out.push(value.clone());
            }
        }
        out
    }

    #[test]
    fn merge_and_dedupe_match_the_reference_across_all_size_paths() {
        // Covers the empty, small-linear, and membership-set paths of both the
        // string and origin helpers, including inputs that are already unique.
        for target_len in [0usize, 1, 3, 7, 8, 20] {
            for incoming_len in [0usize, 1, 4, 5, 9, 20] {
                for duplicate_stride in [1usize, 2, 3, 97] {
                    let target = (0..target_len)
                        .map(|index| format!("target-{}", index % 5))
                        .collect::<Vec<_>>();
                    let incoming = (0..incoming_len)
                        .map(|index| {
                            if index.is_multiple_of(duplicate_stride) {
                                format!("target-{}", index % 7)
                            } else {
                                format!("incoming-{}", index % 4)
                            }
                        })
                        .collect::<Vec<_>>();

                    let expected = reference_merge(&dedupe_strings(target.clone()), &incoming);

                    let mut merged = dedupe_strings(target.clone());
                    merge_unique_strings(&mut merged, incoming.clone());
                    assert_eq!(merged, expected, "strings {target:?} + {incoming:?}");

                    let as_origin = |value: &String| CatalogOrigin {
                        file: value.clone(),
                        scope: None,
                    };
                    let mut merged_origins =
                        dedupe_origins(target.iter().map(as_origin).collect::<Vec<_>>());
                    merge_unique_origins(
                        &mut merged_origins,
                        incoming.iter().map(as_origin).collect::<Vec<_>>().into(),
                    );
                    assert_eq!(
                        merged_origins
                            .iter()
                            .map(|origin| origin.file.clone())
                            .collect::<Vec<_>>(),
                        expected,
                        "origins {target:?} + {incoming:?}"
                    );

                    let mut all = target.clone();
                    all.extend(incoming.iter().cloned());
                    assert_eq!(
                        dedupe_strings(all.clone()),
                        reference_merge(&[], &all),
                        "dedupe {all:?}"
                    );
                }
            }
        }
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
