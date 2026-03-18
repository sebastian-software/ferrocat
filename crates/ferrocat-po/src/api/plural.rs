//! Shared plural and ICU projection helpers for the catalog API.
//!
//! The key design goal in this module is conservative interoperability: we use
//! locale-aware plural categories when they are safe to apply, and otherwise we
//! fall back to predictable synthetic category sets instead of guessing.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, OnceLock};

use ferrocat_icu::{IcuMessage, IcuNode, IcuPluralKind, parse_icu};
use icu_locale::Locale;
use icu_plurals::{PluralCategory, PluralRules};
use memchr::memchr;

use super::PluralSource;

#[allow(
    dead_code,
    reason = "ICU projection remains available for lazy/on-demand bridges."
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ParsedIcuPlural {
    pub(super) variable: String,
    pub(super) branches: BTreeMap<String, String>,
}

#[allow(
    dead_code,
    reason = "ICU projection remains available for lazy/on-demand bridges."
)]
pub(super) enum IcuPluralProjection {
    NotPlural,
    Projected(ParsedIcuPlural),
    Unsupported(&'static str),
    Malformed,
}

pub(super) type PluralCategoryCache = Mutex<HashMap<String, Option<Vec<String>>>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PluralProfile {
    categories: Vec<String>,
}

impl PluralProfile {
    /// Builds the plural-category profile used for one import/export operation.
    ///
    /// Locale-derived categories are preferred when they match the observed
    /// gettext slot count; otherwise we fall back to a synthetic category list
    /// so we do not silently mislabel translator-provided slots.
    fn new(locale: Option<&str>, nplurals: Option<usize>) -> Self {
        let categories = locale.and_then(icu_plural_categories_for).map_or_else(
            || fallback_plural_categories(nplurals),
            |locale_categories| {
                if nplurals.is_none() || nplurals == Some(locale_categories.len()) {
                    locale_categories
                } else {
                    fallback_plural_categories(nplurals)
                }
            },
        );

        Self { categories }
    }

    pub(super) fn for_locale(locale: Option<&str>) -> Self {
        Self::new(locale, None)
    }

    pub(super) fn for_gettext_slots(locale: Option<&str>, nplurals: Option<usize>) -> Self {
        Self::new(locale, nplurals)
    }

    pub(super) fn for_translation(
        locale: Option<&str>,
        translation_by_category: &BTreeMap<String, String>,
    ) -> Self {
        Self::new(locale, Some(translation_by_category.len()))
    }

    pub(super) fn categories(&self) -> &[String] {
        &self.categories
    }

    pub(super) fn nplurals(&self) -> usize {
        self.categories.len().max(1)
    }

    pub(super) fn materialize_translation(
        &self,
        translation: &BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        self.categories
            .iter()
            .map(|category| {
                (
                    category.clone(),
                    translation.get(category).cloned().unwrap_or_default(),
                )
            })
            .collect()
    }

    pub(super) fn source_locale_translation(
        &self,
        source: &PluralSource,
    ) -> BTreeMap<String, String> {
        let mut translation = BTreeMap::new();
        for category in &self.categories {
            let value = match category.as_str() {
                "one" => source.one.clone().unwrap_or_else(|| source.other.clone()),
                _ => source.other.clone(),
            };
            translation.insert(category.clone(), value);
        }
        translation
    }

    pub(super) fn empty_translation(&self) -> BTreeMap<String, String> {
        self.categories
            .iter()
            .map(|category| (category.clone(), String::new()))
            .collect()
    }

    pub(super) fn gettext_values(&self, translation: &BTreeMap<String, String>) -> Vec<String> {
        self.categories
            .iter()
            .map(|category| translation.get(category).cloned().unwrap_or_default())
            .collect()
    }

    pub(super) fn gettext_header(&self) -> Option<String> {
        match self.nplurals() {
            1 => Some("nplurals=1; plural=0;".to_owned()),
            2 => Some("nplurals=2; plural=(n != 1);".to_owned()),
            _ => None,
        }
    }
}

/// Materializes a sparse plural category map against an explicit category order.
///
/// Missing categories become empty strings so downstream export and fallback
/// code can treat the map as dense without extra branching.
#[allow(
    dead_code,
    reason = "ICU projection remains available for lazy/on-demand bridges."
)]
pub(super) fn materialize_plural_categories(
    categories: &[String],
    translation: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    categories
        .iter()
        .map(|category| {
            (
                category.clone(),
                translation.get(category).cloned().unwrap_or_default(),
            )
        })
        .collect()
}

pub(super) fn icu_plural_categories_for(locale: &str) -> Option<Vec<String>> {
    static CACHE: OnceLock<PluralCategoryCache> = OnceLock::new();

    cached_icu_plural_categories_for(locale, CACHE.get_or_init(|| Mutex::new(HashMap::new())))
}

/// Resolves CLDR cardinal categories for a locale and caches both hits and misses.
///
/// The poisoned-lock path intentionally still returns or writes through the
/// inner map so that one panicking caller does not disable plural-category
/// caching for the whole process.
pub(super) fn cached_icu_plural_categories_for(
    locale: &str,
    cache: &PluralCategoryCache,
) -> Option<Vec<String>> {
    let normalized = normalize_plural_locale(locale);
    if normalized.is_empty() {
        return None;
    }

    let cached = match cache.lock() {
        Ok(guard) => guard.get(&normalized).cloned(),
        Err(poisoned) => poisoned.into_inner().get(&normalized).cloned(),
    };
    if let Some(cached) = cached {
        return cached;
    }

    let resolved = normalized
        .parse::<Locale>()
        .ok()
        .and_then(|locale| PluralRules::try_new_cardinal(locale.into()).ok())
        .map(|rules| {
            rules
                .categories()
                .map(plural_category_name)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        });

    match cache.lock() {
        Ok(mut guard) => {
            guard.insert(normalized, resolved.clone());
        }
        Err(poisoned) => {
            poisoned.into_inner().insert(normalized, resolved.clone());
        }
    }

    resolved
}

fn normalize_plural_locale(locale: &str) -> String {
    locale.trim().replace('_', "-")
}

const fn plural_category_name(category: PluralCategory) -> &'static str {
    match category {
        PluralCategory::Zero => "zero",
        PluralCategory::One => "one",
        PluralCategory::Two => "two",
        PluralCategory::Few => "few",
        PluralCategory::Many => "many",
        PluralCategory::Other => "other",
    }
}

/// Produces a deterministic fallback category order when locale-derived CLDR
/// categories are unavailable or incompatible with the observed slot count.
pub(super) fn fallback_plural_categories(nplurals: Option<usize>) -> Vec<String> {
    let categories = match nplurals.unwrap_or(2) {
        0 | 1 => vec!["other"],
        2 => vec!["one", "other"],
        3 => vec!["one", "few", "other"],
        4 => vec!["one", "few", "many", "other"],
        5 => vec!["zero", "one", "few", "many", "other"],
        _ => vec!["zero", "one", "two", "few", "many", "other"],
    };

    categories.into_iter().map(str::to_owned).collect()
}

/// Keeps plural categories in the canonical CLDR-like order expected by
/// import/export code and guarantees that `other` is present at the end.
pub(super) fn sorted_plural_keys(map: &BTreeMap<String, String>) -> Vec<String> {
    let mut keys: Vec<_> = map.keys().cloned().collect();
    keys.sort_by_key(|key| plural_key_rank(key));
    if !keys.iter().any(|key| key == "other") {
        keys.push("other".to_owned());
    }
    keys
}

fn plural_key_rank(key: &str) -> usize {
    match key {
        "zero" => 0,
        "one" => 1,
        "two" => 2,
        "few" => 3,
        "many" => 4,
        "other" => 5,
        _ => 6,
    }
}

/// Derives the best plural variable candidate from extracted placeholders.
///
/// We prefer `count` when present and only infer another name when there is a
/// single unambiguous named placeholder.
pub(super) fn derive_plural_variable(
    placeholders: &BTreeMap<String, Vec<String>>,
) -> Option<String> {
    if placeholders.contains_key("count") {
        return Some("count".to_owned());
    }

    let mut named = placeholders
        .keys()
        .filter(|key| !key.chars().all(|ch| ch.is_ascii_digit()))
        .cloned();
    let first = named.next()?;
    if named.next().is_none() {
        Some(first)
    } else {
        None
    }
}

/// Re-synthesizes a structured plural map into a top-level ICU plural string.
pub(super) fn synthesize_icu_plural(variable: &str, branches: &BTreeMap<String, String>) -> String {
    let mut out = String::new();
    out.push('{');
    out.push_str(variable);
    out.push_str(", plural,");
    for category in sorted_plural_keys(branches) {
        let value = branches
            .get(&category)
            .expect("sorted plural keys must exist in the branch map");
        out.push(' ');
        out.push_str(&category);
        out.push_str(" {");
        out.push_str(value);
        out.push('}');
    }
    out.push('}');
    out
}

/// Projects the narrow ICU plural subset that Ferrocat can round-trip through
/// the current catalog plural model.
///
/// Unsupported but valid ICU constructs report `Unsupported` so callers can
/// keep the message as singular with a targeted diagnostic instead of failing
/// or guessing.
#[allow(
    dead_code,
    reason = "ICU projection remains available for lazy/on-demand bridges."
)]
pub(super) fn project_icu_plural(input: &str) -> IcuPluralProjection {
    if !looks_like_projectable_icu_plural(input.as_bytes()) {
        return IcuPluralProjection::NotPlural;
    }

    let Ok(message) = parse_icu(input) else {
        return IcuPluralProjection::Malformed;
    };

    let Some(IcuNode::Plural {
        name,
        kind: IcuPluralKind::Cardinal,
        offset,
        options,
    }) = only_node(&message)
    else {
        return IcuPluralProjection::NotPlural;
    };

    if *offset != 0 {
        return IcuPluralProjection::Unsupported(
            "ICU plural offset syntax is not projected into the current catalog plural model.",
        );
    }

    let mut branches = BTreeMap::new();
    for option in options {
        if option.selector.starts_with('=') {
            return IcuPluralProjection::Unsupported(
                "ICU exact-match plural selectors are not projected into the current catalog plural model.",
            );
        }

        let value = match render_projectable_icu_nodes(&option.value) {
            Ok(value) => value,
            Err(message) => return IcuPluralProjection::Unsupported(message),
        };
        branches.insert(option.selector.clone(), value);
    }

    if !branches.contains_key("other") {
        return IcuPluralProjection::Malformed;
    }

    IcuPluralProjection::Projected(ParsedIcuPlural {
        variable: name.clone(),
        branches,
    })
}

#[inline]
#[allow(
    dead_code,
    reason = "ICU projection remains available for lazy/on-demand bridges."
)]
fn looks_like_projectable_icu_plural(input: &[u8]) -> bool {
    let input = trim_ascii(input);
    let Some(first) = input.first().copied() else {
        return false;
    };

    match first {
        b'<' => return false,
        b'{' => {}
        _ => return false,
    }

    let Some(after_open) = input.get(1..) else {
        return false;
    };
    let Some(first_comma) = memchr(b',', after_open) else {
        return false;
    };
    if first_comma == 0 {
        return true;
    }

    let after_name = trim_ascii_start(&after_open[first_comma + 1..]);
    let Some((kind, _tail)) = split_icu_kind(after_name) else {
        return true;
    };

    if kind == b"plural" {
        return true;
    }

    !matches!(
        kind,
        b"number"
            | b"date"
            | b"time"
            | b"list"
            | b"duration"
            | b"ago"
            | b"name"
            | b"select"
            | b"selectordinal"
    )
}

#[allow(
    dead_code,
    reason = "ICU projection remains available for lazy/on-demand bridges."
)]
fn trim_ascii(input: &[u8]) -> &[u8] {
    trim_ascii_end(trim_ascii_start(input))
}

#[allow(
    dead_code,
    reason = "ICU projection remains available for lazy/on-demand bridges."
)]
fn trim_ascii_start(input: &[u8]) -> &[u8] {
    let start = input
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(input.len());
    &input[start..]
}

#[allow(
    dead_code,
    reason = "ICU projection remains available for lazy/on-demand bridges."
)]
fn trim_ascii_end(input: &[u8]) -> &[u8] {
    let end = input
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map(|index| index + 1)
        .unwrap_or(0);
    &input[..end]
}

#[allow(
    dead_code,
    reason = "ICU projection remains available for lazy/on-demand bridges."
)]
fn split_icu_kind(input: &[u8]) -> Option<(&[u8], &[u8])> {
    let token_end = input
        .iter()
        .position(|byte| byte.is_ascii_whitespace() || matches!(byte, b',' | b'}'))
        .unwrap_or(input.len());
    let kind = input.get(..token_end)?;
    if kind.is_empty() {
        return None;
    }
    Some((kind, input.get(token_end..).unwrap_or_default()))
}

#[allow(
    dead_code,
    reason = "ICU projection remains available for lazy/on-demand bridges."
)]
fn only_node(message: &IcuMessage) -> Option<&IcuNode> {
    match message.nodes.as_slice() {
        [node] => Some(node),
        _ => None,
    }
}

/// Re-renders a projected ICU subtree back into a string while rejecting nested
/// select/plural constructs that the catalog model cannot represent.
#[allow(
    dead_code,
    reason = "ICU projection remains available for lazy/on-demand bridges."
)]
fn render_projectable_icu_nodes(nodes: &[IcuNode]) -> Result<String, &'static str> {
    let mut out = String::new();
    for node in nodes {
        render_projectable_icu_node(node, &mut out)?;
    }
    Ok(out)
}

#[allow(
    dead_code,
    reason = "ICU projection remains available for lazy/on-demand bridges."
)]
fn render_projectable_icu_node(node: &IcuNode, out: &mut String) -> Result<(), &'static str> {
    match node {
        IcuNode::Literal(value) => append_escaped_icu_literal(out, value),
        IcuNode::Argument { name } => {
            out.push('{');
            out.push_str(name);
            out.push('}');
        }
        IcuNode::Number { name, style } => render_formatter("number", name, style.as_deref(), out),
        IcuNode::Date { name, style } => render_formatter("date", name, style.as_deref(), out),
        IcuNode::Time { name, style } => render_formatter("time", name, style.as_deref(), out),
        IcuNode::List { name, style } => render_formatter("list", name, style.as_deref(), out),
        IcuNode::Duration { name, style } => {
            render_formatter("duration", name, style.as_deref(), out);
        }
        IcuNode::Ago { name, style } => render_formatter("ago", name, style.as_deref(), out),
        IcuNode::Name { name, style } => render_formatter("name", name, style.as_deref(), out),
        IcuNode::Pound => out.push('#'),
        IcuNode::Tag { name, children } => {
            out.push('<');
            out.push_str(name);
            out.push('>');
            for child in children {
                render_projectable_icu_node(child, out)?;
            }
            out.push_str("</");
            out.push_str(name);
            out.push('>');
        }
        IcuNode::Select { .. } | IcuNode::Plural { .. } => {
            return Err(
                "Nested ICU select/plural structures are not projected into the current catalog plural model.",
            );
        }
    }

    Ok(())
}

#[allow(
    dead_code,
    reason = "ICU projection remains available for lazy/on-demand bridges."
)]
fn render_formatter(kind: &str, name: &str, style: Option<&str>, out: &mut String) {
    out.push('{');
    out.push_str(name);
    out.push_str(", ");
    out.push_str(kind);
    if let Some(style) = style {
        out.push_str(", ");
        out.push_str(style);
    }
    out.push('}');
}

/// Escapes ICU-sensitive literal characters only when needed, keeping the
/// common literal path allocation-light.
#[allow(
    dead_code,
    reason = "ICU projection remains available for lazy/on-demand bridges."
)]
fn append_escaped_icu_literal(out: &mut String, value: &str) {
    if !value
        .as_bytes()
        .iter()
        .any(|byte| matches!(byte, b'\'' | b'{' | b'}' | b'#' | b'<' | b'>'))
    {
        out.push_str(value);
        return;
    }

    for ch in value.chars() {
        match ch {
            '\'' => out.push_str("''"),
            '{' | '}' | '#' | '<' | '>' => {
                out.push('\'');
                out.push(ch);
                out.push('\'');
            }
            _ => out.push(ch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IcuPluralProjection, looks_like_projectable_icu_plural, project_icu_plural};

    #[test]
    fn plural_fast_scan_skips_plain_and_mixed_messages() {
        assert!(!looks_like_projectable_icu_plural(
            b"Bench 1: Hello {name}, you have {count} items."
        ));
        assert!(!looks_like_projectable_icu_plural(
            b"<link>{name}</link> updated benchmark entry."
        ));
        assert!(!looks_like_projectable_icu_plural(
            b"{count, number, integer}"
        ));
        assert!(!looks_like_projectable_icu_plural(b"{name}"));
    }

    #[test]
    fn plural_fast_scan_keeps_plural_candidates() {
        assert!(looks_like_projectable_icu_plural(
            b"{count, plural, one {# item} other {# items}}"
        ));
        assert!(looks_like_projectable_icu_plural(
            b"{count,plural,one {# item} other {# items}}"
        ));
        assert!(looks_like_projectable_icu_plural(
            b"{count, plural one {# item} other {# items}}"
        ));
        assert!(looks_like_projectable_icu_plural(b"{count, plura"));
    }

    #[test]
    fn project_icu_plural_keeps_formatter_messages_singular() {
        assert!(matches!(
            project_icu_plural("Bench 1: {count, number, integer} items for {name}."),
            IcuPluralProjection::NotPlural
        ));
        assert!(matches!(
            project_icu_plural("<link>{name}</link> updated benchmark entry."),
            IcuPluralProjection::NotPlural
        ));
        assert!(matches!(
            project_icu_plural("{count, number, integer}"),
            IcuPluralProjection::NotPlural
        ));
    }
}
