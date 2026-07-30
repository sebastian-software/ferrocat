//! Shared plural and ICU projection helpers for the catalog API.
//!
//! The key design goal in this module is conservative interoperability: we use
//! locale-aware plural categories when they are safe to apply, and otherwise we
//! fall back to predictable synthetic category sets instead of guessing.

use std::collections::{BTreeMap, HashMap};
use std::mem;
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

pub(super) type PluralCategoryCache = Mutex<HashMap<String, Option<Vec<&'static str>>>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PluralProfile {
    categories: Vec<&'static str>,
    gettext_header: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GettextPluralRule {
    locale: &'static str,
    categories: &'static [&'static str],
    header: &'static str,
}

impl GettextPluralRule {
    const fn nplurals(self) -> usize {
        self.categories.len()
    }
}

const ONE_FORM_CATEGORIES: &[&str] = &["other"];
const TWO_FORM_CATEGORIES: &[&str] = &["one", "other"];
const THREE_FORM_CATEGORIES: &[&str] = &["one", "few", "other"];
const SIX_FORM_CATEGORIES: &[&str] = &["zero", "one", "two", "few", "many", "other"];

const GETTEXT_ONE_FORM_HEADER: &str = "nplurals=1; plural=0;";
const GETTEXT_ONE_ONLY_HEADER: &str = "nplurals=2; plural=(n != 1);";
const GETTEXT_ZERO_ONE_HEADER: &str = "nplurals=2; plural=(n > 1);";
const GETTEXT_POLISH_HEADER: &str = "nplurals=3; plural=(n == 1 ? 0 : (n % 10 >= 2 && n % 10 <= 4 && (n % 100 < 10 || n % 100 >= 20)) ? 1 : 2);";
const GETTEXT_SLAVIC_THREE_FORM_HEADER: &str = "nplurals=3; plural=(n % 10 == 1 && n % 100 != 11 ? 0 : (n % 10 >= 2 && n % 10 <= 4 && (n % 100 < 10 || n % 100 >= 20)) ? 1 : 2);";
const GETTEXT_CZECH_SLOVAK_HEADER: &str =
    "nplurals=3; plural=(n == 1 ? 0 : (n >= 2 && n <= 4) ? 1 : 2);";
const GETTEXT_ARABIC_HEADER: &str = "nplurals=6; plural=(n == 0 ? 0 : n == 1 ? 1 : n == 2 ? 2 : (n % 100 >= 3 && n % 100 <= 10) ? 3 : (n % 100 >= 11) ? 4 : 5);";

// Seeded from GNU gettext's documented plural-form families for common locales.
const GETTEXT_PLURAL_RULES: &[GettextPluralRule] = &[
    GettextPluralRule {
        locale: "ar",
        categories: SIX_FORM_CATEGORIES,
        header: GETTEXT_ARABIC_HEADER,
    },
    GettextPluralRule {
        locale: "be",
        categories: THREE_FORM_CATEGORIES,
        header: GETTEXT_SLAVIC_THREE_FORM_HEADER,
    },
    GettextPluralRule {
        locale: "bg",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ONE_ONLY_HEADER,
    },
    GettextPluralRule {
        locale: "cs",
        categories: THREE_FORM_CATEGORIES,
        header: GETTEXT_CZECH_SLOVAK_HEADER,
    },
    GettextPluralRule {
        locale: "da",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ONE_ONLY_HEADER,
    },
    GettextPluralRule {
        locale: "de",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ONE_ONLY_HEADER,
    },
    GettextPluralRule {
        locale: "el",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ONE_ONLY_HEADER,
    },
    GettextPluralRule {
        locale: "en",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ONE_ONLY_HEADER,
    },
    GettextPluralRule {
        locale: "eo",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ONE_ONLY_HEADER,
    },
    GettextPluralRule {
        locale: "es",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ONE_ONLY_HEADER,
    },
    GettextPluralRule {
        locale: "et",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ONE_ONLY_HEADER,
    },
    GettextPluralRule {
        locale: "fi",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ONE_ONLY_HEADER,
    },
    GettextPluralRule {
        locale: "fr",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ZERO_ONE_HEADER,
    },
    GettextPluralRule {
        locale: "he",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ONE_ONLY_HEADER,
    },
    GettextPluralRule {
        locale: "hr",
        categories: THREE_FORM_CATEGORIES,
        header: GETTEXT_SLAVIC_THREE_FORM_HEADER,
    },
    GettextPluralRule {
        locale: "hu",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ONE_ONLY_HEADER,
    },
    GettextPluralRule {
        locale: "id",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ONE_ONLY_HEADER,
    },
    GettextPluralRule {
        locale: "it",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ONE_ONLY_HEADER,
    },
    GettextPluralRule {
        locale: "ja",
        categories: ONE_FORM_CATEGORIES,
        header: GETTEXT_ONE_FORM_HEADER,
    },
    GettextPluralRule {
        locale: "ko",
        categories: ONE_FORM_CATEGORIES,
        header: GETTEXT_ONE_FORM_HEADER,
    },
    GettextPluralRule {
        locale: "nb",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ONE_ONLY_HEADER,
    },
    GettextPluralRule {
        locale: "nl",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ONE_ONLY_HEADER,
    },
    GettextPluralRule {
        locale: "nn",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ONE_ONLY_HEADER,
    },
    GettextPluralRule {
        locale: "no",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ONE_ONLY_HEADER,
    },
    GettextPluralRule {
        locale: "pl",
        categories: THREE_FORM_CATEGORIES,
        header: GETTEXT_POLISH_HEADER,
    },
    GettextPluralRule {
        locale: "pt",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ONE_ONLY_HEADER,
    },
    GettextPluralRule {
        locale: "pt-br",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ZERO_ONE_HEADER,
    },
    GettextPluralRule {
        locale: "ru",
        categories: THREE_FORM_CATEGORIES,
        header: GETTEXT_SLAVIC_THREE_FORM_HEADER,
    },
    GettextPluralRule {
        locale: "sk",
        categories: THREE_FORM_CATEGORIES,
        header: GETTEXT_CZECH_SLOVAK_HEADER,
    },
    GettextPluralRule {
        locale: "sr",
        categories: THREE_FORM_CATEGORIES,
        header: GETTEXT_SLAVIC_THREE_FORM_HEADER,
    },
    GettextPluralRule {
        locale: "sv",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ONE_ONLY_HEADER,
    },
    GettextPluralRule {
        locale: "th",
        categories: ONE_FORM_CATEGORIES,
        header: GETTEXT_ONE_FORM_HEADER,
    },
    GettextPluralRule {
        locale: "tr",
        categories: TWO_FORM_CATEGORIES,
        header: GETTEXT_ONE_ONLY_HEADER,
    },
    GettextPluralRule {
        locale: "uk",
        categories: THREE_FORM_CATEGORIES,
        header: GETTEXT_SLAVIC_THREE_FORM_HEADER,
    },
    GettextPluralRule {
        locale: "vi",
        categories: ONE_FORM_CATEGORIES,
        header: GETTEXT_ONE_FORM_HEADER,
    },
    GettextPluralRule {
        locale: "zh",
        categories: ONE_FORM_CATEGORIES,
        header: GETTEXT_ONE_FORM_HEADER,
    },
];

impl PluralProfile {
    /// Builds the plural-category profile used for one import/export operation.
    ///
    /// Locale-derived categories are preferred when they match the observed
    /// gettext slot count; otherwise we fall back to a synthetic category list
    /// so we do not silently mislabel translator-provided slots.
    fn new(locale: Option<&str>, nplurals: Option<usize>) -> Self {
        let normalized_locale = normalized_locale(locale);
        let categories = normalized_locale
            .as_deref()
            .and_then(icu_plural_categories_for)
            .map_or_else(
                || fallback_plural_categories(nplurals),
                |locale_categories| {
                    if nplurals.is_none() || nplurals == Some(locale_categories.len()) {
                        locale_categories
                    } else {
                        fallback_plural_categories(nplurals)
                    }
                },
            );
        let gettext_header =
            gettext_header_for_categories(normalized_locale.as_deref(), categories.len());

        Self {
            categories,
            gettext_header,
        }
    }

    pub(super) fn for_locale(locale: Option<&str>) -> Self {
        Self::new(locale, None)
    }

    pub(super) fn for_gettext_slots(locale: Option<&str>, nplurals: Option<usize>) -> Self {
        Self::new_gettext(locale, nplurals)
    }

    pub(super) fn for_gettext_locale(locale: Option<&str>) -> Self {
        Self::new_gettext(locale, None)
    }

    fn new_gettext(locale: Option<&str>, nplurals: Option<usize>) -> Self {
        let normalized_locale = normalized_locale(locale);
        if let Some(rule) = normalized_locale
            .as_deref()
            .and_then(gettext_plural_rule_for_normalized)
            .filter(|rule| nplurals.is_none() || nplurals == Some(rule.nplurals()))
        {
            return Self {
                categories: rule.categories.to_vec(),
                gettext_header: Some(rule.header),
            };
        }

        let categories = normalized_locale
            .as_deref()
            .and_then(icu_plural_categories_for)
            .map_or_else(
                || fallback_plural_categories(nplurals),
                |locale_categories| {
                    if nplurals.is_none() || nplurals == Some(locale_categories.len()) {
                        locale_categories
                    } else {
                        fallback_plural_categories(nplurals)
                    }
                },
            );
        let gettext_header = if normalized_locale.is_none() {
            generic_gettext_header_for_nplurals(categories.len())
        } else {
            None
        };

        Self {
            categories,
            gettext_header,
        }
    }

    pub(super) fn categories(&self) -> &[&'static str] {
        &self.categories
    }

    pub(super) fn nplurals(&self) -> usize {
        self.categories.len().max(1)
    }

    /// Materializes a translation and fills missing or empty categories from the
    /// source forms in a single pass.
    pub(super) fn source_fallback_translation(
        &self,
        translation: &BTreeMap<String, String>,
        source: &PluralSource,
    ) -> BTreeMap<String, String> {
        self.categories
            .iter()
            .map(|category| {
                let value = translation
                    .get(*category)
                    .filter(|value| !value.is_empty())
                    .cloned()
                    .unwrap_or_else(|| self.source_locale_value(category, source));
                ((*category).to_owned(), value)
            })
            .collect()
    }

    /// Materializes an existing translation map against this profile's
    /// categories without rebuilding it when it already matches.
    ///
    /// Returns whether the map changed. The merge path uses that to classify a
    /// message as unchanged instead of comparing against a retained copy.
    pub(super) fn materialize_translation_in_place(
        &self,
        translation: &mut BTreeMap<String, String>,
    ) -> bool {
        if translation.len() == self.categories.len()
            && self
                .categories
                .iter()
                .all(|category| translation.contains_key(*category))
        {
            return false;
        }

        let mut previous = mem::take(translation);
        for category in &self.categories {
            let (key, value) = previous
                .remove_entry(*category)
                .unwrap_or_else(|| ((*category).to_owned(), String::new()));
            translation.insert(key, value);
        }
        true
    }

    pub(super) fn source_locale_translation(
        &self,
        source: &PluralSource,
    ) -> BTreeMap<String, String> {
        let mut translation = BTreeMap::new();
        for category in &self.categories {
            translation.insert(
                (*category).to_owned(),
                self.source_locale_value(category, source),
            );
        }
        translation
    }

    pub(super) fn source_locale_value(&self, category: &str, source: &PluralSource) -> String {
        match category {
            "one" => source.one.clone().unwrap_or_else(|| source.other.clone()),
            _ => source.other.clone(),
        }
    }

    pub(super) fn empty_translation(&self) -> BTreeMap<String, String> {
        self.categories
            .iter()
            .map(|category| ((*category).to_owned(), String::new()))
            .collect()
    }

    /// Borrows the gettext slot values in category order; missing categories
    /// render as empty slots.
    pub(super) fn gettext_values<'a>(
        &self,
        translation: &'a BTreeMap<String, String>,
    ) -> Vec<&'a str> {
        self.categories
            .iter()
            .map(|category| translation.get(*category).map_or("", String::as_str))
            .collect()
    }

    pub(super) fn gettext_header(&self) -> Option<String> {
        self.gettext_header.map(str::to_owned)
    }
}

/// Caches gettext plural profiles for the duration of one catalog operation.
///
/// A profile is fully determined by the locale and the observed slot count, and
/// both stay stable across most messages of a catalog. Building it per message
/// only repeated the same locale lookup and category allocations.
#[derive(Debug)]
pub(super) struct GettextPluralProfiles<'a> {
    locale: Option<&'a str>,
    profiles: Vec<(Option<usize>, PluralProfile)>,
}

impl<'a> GettextPluralProfiles<'a> {
    pub(super) fn new(locale: Option<&'a str>) -> Self {
        Self {
            locale,
            profiles: Vec::new(),
        }
    }

    /// Returns the profile for `nplurals`, building it on first use.
    pub(super) fn for_slots(&mut self, nplurals: Option<usize>) -> &PluralProfile {
        let index = match self
            .profiles
            .iter()
            .position(|(slots, _)| *slots == nplurals)
        {
            Some(index) => index,
            None => {
                self.profiles.push((
                    nplurals,
                    PluralProfile::for_gettext_slots(self.locale, nplurals),
                ));
                self.profiles.len() - 1
            }
        };

        &self.profiles[index].1
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
    categories: &[&'static str],
    translation: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    categories
        .iter()
        .map(|category| {
            (
                (*category).to_owned(),
                translation.get(*category).cloned().unwrap_or_default(),
            )
        })
        .collect()
}

pub(super) fn icu_plural_categories_for(locale: &str) -> Option<Vec<&'static str>> {
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
) -> Option<Vec<&'static str>> {
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
        .map(|rules| rules.categories().map(plural_category_name).collect());

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

fn normalized_locale(locale: Option<&str>) -> Option<String> {
    let normalized = normalize_plural_locale(locale?);
    (!normalized.is_empty()).then_some(normalized)
}

fn gettext_plural_rule_for_normalized(locale: &str) -> Option<GettextPluralRule> {
    let normalized = locale.to_ascii_lowercase();
    gettext_plural_rule_for_key(&normalized).or_else(|| {
        normalized
            .split_once('-')
            .and_then(|(language, _)| gettext_plural_rule_for_key(language))
    })
}

fn gettext_plural_rule_for_key(locale: &str) -> Option<GettextPluralRule> {
    GETTEXT_PLURAL_RULES
        .iter()
        .copied()
        .find(|rule| rule.locale == locale)
}

fn gettext_header_for_categories(locale: Option<&str>, nplurals: usize) -> Option<&'static str> {
    locale
        .and_then(gettext_plural_rule_for_normalized)
        .filter(|rule| rule.nplurals() == nplurals)
        .map(|rule| rule.header)
        .or_else(|| {
            locale
                .is_none()
                .then(|| generic_gettext_header_for_nplurals(nplurals))
                .flatten()
        })
}

fn generic_gettext_header_for_nplurals(nplurals: usize) -> Option<&'static str> {
    match nplurals {
        1 => Some(GETTEXT_ONE_FORM_HEADER),
        2 => Some(GETTEXT_ONE_ONLY_HEADER),
        _ => None,
    }
}

pub(super) fn expected_gettext_nplurals_for_locale(locale: Option<&str>) -> Option<usize> {
    let normalized = normalized_locale(locale)?;
    gettext_plural_rule_for_normalized(&normalized)
        .map(GettextPluralRule::nplurals)
        .or_else(|| icu_plural_categories_for(&normalized).map(|categories| categories.len()))
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
pub(super) fn fallback_plural_categories(nplurals: Option<usize>) -> Vec<&'static str> {
    match nplurals.unwrap_or(2) {
        0 | 1 => vec!["other"],
        2 => vec!["one", "other"],
        3 => vec!["one", "few", "other"],
        4 => vec!["one", "few", "many", "other"],
        5 => vec!["zero", "one", "few", "many", "other"],
        _ => vec!["zero", "one", "two", "few", "many", "other"],
    }
}

/// Keeps plural branches in the canonical CLDR-like order expected by
/// import/export code and guarantees that `other` is present at the end.
///
/// Branches borrow from `map` so that ordering never clones catalog strings.
fn ordered_plural_branches(map: &BTreeMap<String, String>) -> Vec<(&str, &str)> {
    let mut branches: Vec<(&str, &str)> = map
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    branches.sort_by_key(|(key, _)| plural_key_rank(key));
    if !branches.iter().any(|(key, _)| *key == "other") {
        branches.push(("other", ""));
    }
    branches
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
    render_icu_plural(variable, &ordered_plural_branches(branches))
}

/// Synthesizes the ICU plural string for the source forms without building an
/// intermediate branch map.
pub(super) fn synthesize_icu_plural_source(variable: &str, source: &PluralSource) -> String {
    match source.one.as_deref() {
        Some(one) => render_icu_plural(variable, &[("one", one), ("other", &source.other)]),
        None => render_icu_plural(variable, &[("other", &source.other)]),
    }
}

/// Renders already-ordered `(category, value)` branches, presizing the output so
/// the buffer never has to grow.
fn render_icu_plural(variable: &str, branches: &[(&str, &str)]) -> String {
    // `{` + variable + `, plural,` + branches + `}` is 11 fixed bytes plus the
    // variable, where every branch costs ` ` + category + ` {` + value + `}`.
    let capacity = variable.len()
        + 11
        + branches
            .iter()
            .map(|(category, value)| category.len() + value.len() + 4)
            .sum::<usize>();

    let mut out = String::with_capacity(capacity);
    out.push('{');
    out.push_str(variable);
    out.push_str(", plural,");
    for (category, value) in branches {
        out.push(' ');
        out.push_str(category);
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
            if children.is_empty() {
                out.push_str("/>");
            } else {
                out.push('>');
                for child in children {
                    render_projectable_icu_node(child, out)?;
                }
                out.push_str("</");
                out.push_str(name);
                out.push('>');
            }
        }
        IcuNode::Select { .. } | IcuNode::Plural { .. } => {
            return Err(
                "Nested ICU select/plural structures are not projected into the current catalog plural model.",
            );
        }
        #[allow(
            unreachable_patterns,
            reason = "cargo package verifies ferrocat-po against the latest published ferrocat-icu until this release publishes the dependency crate first."
        )]
        _ => {
            return Err(
                "Unsupported ICU node is not projected into the current catalog plural model.",
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
    use std::collections::{BTreeMap, HashMap};
    use std::sync::Mutex;

    use super::{
        GETTEXT_ARABIC_HEADER, GETTEXT_ONE_FORM_HEADER, GETTEXT_POLISH_HEADER,
        GETTEXT_SLAVIC_THREE_FORM_HEADER, GETTEXT_ZERO_ONE_HEADER, GettextPluralProfiles,
        IcuPluralProjection, PluralProfile, cached_icu_plural_categories_for,
        derive_plural_variable, expected_gettext_nplurals_for_locale, fallback_plural_categories,
        looks_like_projectable_icu_plural, materialize_plural_categories, normalize_plural_locale,
        ordered_plural_branches, project_icu_plural, split_icu_kind, synthesize_icu_plural,
        synthesize_icu_plural_source,
    };

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

    #[test]
    fn plural_profiles_and_category_helpers_fill_expected_shapes() {
        let mut profiles = GettextPluralProfiles::new(Some("fr"));
        let profile = profiles.for_slots(Some(2)).clone();
        assert_eq!(profile.nplurals(), 2);
        assert_eq!(profile.categories(), &["one", "other"]);
        let mut sparse = BTreeMap::from([("other".to_owned(), "autres".to_owned())]);
        assert!(profile.materialize_translation_in_place(&mut sparse));
        assert_eq!(
            sparse,
            BTreeMap::from([
                ("one".to_owned(), String::new()),
                ("other".to_owned(), "autres".to_owned()),
            ])
        );
        assert!(!profile.materialize_translation_in_place(&mut sparse));
        assert_eq!(
            profile.source_locale_translation(&super::PluralSource {
                one: Some("one-file".to_owned()),
                other: "many-files".to_owned(),
            }),
            BTreeMap::from([
                ("one".to_owned(), "one-file".to_owned()),
                ("other".to_owned(), "many-files".to_owned()),
            ])
        );
        assert_eq!(
            profile.empty_translation(),
            BTreeMap::from([
                ("one".to_owned(), String::new()),
                ("other".to_owned(), String::new()),
            ])
        );
        assert_eq!(
            profile.gettext_values(&BTreeMap::from([
                ("one".to_owned(), "eins".to_owned()),
                ("other".to_owned(), "viele".to_owned()),
            ])),
            vec!["eins", "viele"]
        );
        assert_eq!(
            profile.gettext_header().as_deref(),
            Some(GETTEXT_ZERO_ONE_HEADER)
        );
        assert_eq!(
            materialize_plural_categories(
                &["one", "other"],
                &BTreeMap::from([("one".to_owned(), "eins".to_owned())]),
            ),
            BTreeMap::from([
                ("one".to_owned(), "eins".to_owned()),
                ("other".to_owned(), String::new()),
            ])
        );

        // Repeated lookups reuse the cached profile instead of rebuilding it.
        assert_eq!(profiles.for_slots(Some(2)), &profile);
    }

    #[test]
    fn gettext_plural_profiles_use_safe_locale_table() {
        let cases = [
            ("fr", GETTEXT_ZERO_ONE_HEADER, &["one", "other"][..]),
            ("pt_BR", GETTEXT_ZERO_ONE_HEADER, &["one", "other"]),
            ("pl", GETTEXT_POLISH_HEADER, &["one", "few", "other"]),
            (
                "ru",
                GETTEXT_SLAVIC_THREE_FORM_HEADER,
                &["one", "few", "other"],
            ),
            (
                "ar",
                GETTEXT_ARABIC_HEADER,
                &["zero", "one", "two", "few", "many", "other"],
            ),
            ("ja", GETTEXT_ONE_FORM_HEADER, &["other"]),
        ];

        for (locale, header, categories) in cases {
            let profile = PluralProfile::for_gettext_locale(Some(locale));
            assert_eq!(profile.gettext_header().as_deref(), Some(header));
            assert_eq!(profile.categories(), categories);
            assert_eq!(
                expected_gettext_nplurals_for_locale(Some(locale)),
                Some(categories.len())
            );
        }
    }

    #[test]
    fn gettext_plural_profiles_do_not_guess_headers_for_unlisted_locales() {
        let profile = PluralProfile::for_gettext_locale(Some("ga"));

        assert_eq!(profile.gettext_header(), None);
    }

    #[test]
    fn plural_category_fallbacks_and_sorting_are_deterministic() {
        assert_eq!(fallback_plural_categories(Some(1)), vec!["other"]);
        assert_eq!(
            fallback_plural_categories(Some(3)),
            vec!["one", "few", "other"]
        );
        assert_eq!(
            fallback_plural_categories(Some(7)),
            vec!["zero", "one", "two", "few", "many", "other"]
        );
        assert_eq!(
            ordered_plural_branches(&BTreeMap::from([
                ("many".to_owned(), "viele".to_owned()),
                ("one".to_owned(), "eins".to_owned()),
            ])),
            vec![("one", "eins"), ("many", "viele"), ("other", "")]
        );
    }

    #[test]
    fn derive_plural_variable_prefers_count_and_rejects_ambiguous_sets() {
        assert_eq!(
            derive_plural_variable(&BTreeMap::from([
                ("count".to_owned(), vec!["1".to_owned()]),
                ("items".to_owned(), vec!["files".to_owned()]),
            ])),
            Some("count".to_owned())
        );
        assert_eq!(
            derive_plural_variable(&BTreeMap::from([(
                "items".to_owned(),
                vec!["files".to_owned()],
            )])),
            Some("items".to_owned())
        );
        assert_eq!(
            derive_plural_variable(&BTreeMap::from([
                ("1".to_owned(), vec!["eins".to_owned()]),
                ("2".to_owned(), vec!["zwei".to_owned()]),
            ])),
            None
        );
        assert_eq!(
            derive_plural_variable(&BTreeMap::from([
                ("files".to_owned(), vec!["many".to_owned()]),
                ("items".to_owned(), vec!["many".to_owned()]),
            ])),
            None
        );
    }

    #[test]
    fn synthesize_and_project_icu_plural_cover_supported_and_unsupported_cases() {
        let synthesized = synthesize_icu_plural(
            "count",
            &BTreeMap::from([
                ("other".to_owned(), "# files".to_owned()),
                ("one".to_owned(), "# file".to_owned()),
            ]),
        );
        assert_eq!(synthesized, "{count, plural, one {# file} other {# files}}");
        assert_eq!(
            synthesize_icu_plural_source(
                "count",
                &super::PluralSource {
                    one: Some("# file".to_owned()),
                    other: "# files".to_owned(),
                },
            ),
            synthesized
        );
        assert_eq!(
            synthesize_icu_plural_source(
                "count",
                &super::PluralSource {
                    one: None,
                    other: "# files".to_owned(),
                },
            ),
            "{count, plural, other {# files}}"
        );

        // The capacity estimate must cover the rendered output exactly, so the
        // synthesis buffer never grows. `String::with_capacity` keeps the
        // requested capacity when no reallocation happens, so equality with the
        // final length pins the no-growth contract.
        assert_eq!(synthesized.capacity(), synthesized.len());

        assert!(matches!(
            project_icu_plural("{count, plural, offset:1 one {# file} other {# files}}"),
            IcuPluralProjection::Unsupported(message) if message.contains("offset")
        ));
        assert!(matches!(
            project_icu_plural("{count, plural, =0 {none} other {# files}}"),
            IcuPluralProjection::Unsupported(message) if message.contains("exact-match")
        ));
        assert!(matches!(
            project_icu_plural("{count, plural, one {{gender, select, other {# file}}} other {# files}}"),
            IcuPluralProjection::Unsupported(message) if message.contains("Nested ICU")
        ));
        assert!(matches!(
            project_icu_plural("{count, plural, one {# file}}"),
            IcuPluralProjection::Malformed
        ));
        assert!(matches!(
            project_icu_plural("plain text"),
            IcuPluralProjection::NotPlural
        ));
    }

    #[test]
    fn project_icu_plural_renders_supported_nested_leaf_nodes() {
        let projection = project_icu_plural(
            "{count, plural, one {It''s '{'one'}' <b>{name}</b><0/> {price, number, integer} {created, date, short} {time, time, HH:mm} {items, list, conjunction} {elapsed, duration} {since, ago} {person, name}} other {# files}}",
        );

        match projection {
            IcuPluralProjection::Projected(parsed) => {
                assert_eq!(parsed.variable, "count");
                assert_eq!(
                    parsed.branches.get("one").map(String::as_str),
                    Some(
                        "It''s '{'one'}' <b>{name}</b><0/> {price, number, integer} {created, date, short} {time, time, HH:mm} {items, list, conjunction} {elapsed, duration} {since, ago} {person, name}"
                    )
                );
                assert_eq!(
                    parsed.branches.get("other").map(String::as_str),
                    Some("# files")
                );
            }
            _ => panic!("expected projected plural"),
        }
    }

    #[test]
    fn project_icu_plural_reports_malformed_and_non_plural_candidates() {
        assert!(matches!(
            project_icu_plural("{count, plural, one {# file} other {# files}"),
            IcuPluralProjection::Malformed
        ));
        assert!(matches!(
            project_icu_plural("{rank, selectordinal, one {#st} other {#th}}"),
            IcuPluralProjection::NotPlural
        ));
        assert!(!looks_like_projectable_icu_plural(b""));
        assert!(!looks_like_projectable_icu_plural(b"{"));
        assert!(looks_like_projectable_icu_plural(b"{, plural"));
        assert!(looks_like_projectable_icu_plural(b"{count,"));
        assert!(looks_like_projectable_icu_plural(b"{count, unknown"));
    }

    #[test]
    fn locale_normalization_and_cache_helpers_cover_hits_and_misses() {
        assert_eq!(normalize_plural_locale(" pt_BR "), "pt-BR");
        assert_eq!(
            split_icu_kind(b"plural, one {x}").map(|(kind, _)| kind),
            Some(&b"plural"[..])
        );
        assert_eq!(split_icu_kind(b"").map(|(kind, _)| kind), None);

        let cache = Mutex::new(HashMap::new());
        assert_eq!(cached_icu_plural_categories_for("   ", &cache), None);
        assert!(
            cached_icu_plural_categories_for("de", &cache)
                .expect("categories")
                .contains(&"other")
        );
        let synthetic = PluralProfile::for_gettext_slots(Some("fr"), Some(4));
        assert_eq!(synthetic.categories(), &["one", "few", "many", "other"]);
        assert_eq!(
            PluralProfile::for_gettext_slots(Some("und"), Some(1)).nplurals(),
            1
        );
    }
}
