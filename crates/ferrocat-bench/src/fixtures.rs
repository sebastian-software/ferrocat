use std::borrow::Cow;
use std::collections::BTreeMap;

use ferrocat_po::{
    CatalogOrigin, ExtractedMessage, ExtractedPluralMessage, ExtractedSingularMessage,
    MergeExtractedMessage, PluralSource, parse_po,
};

const TINY_FIXTURE: &str = include_str!("../fixtures/tiny.po");
const REALISTIC_FIXTURE: &str = include_str!("../fixtures/realistic.po");
const STRESS_FIXTURE: &str = include_str!("../fixtures/stress.po");

#[derive(Debug, Clone, Copy)]
pub struct FixtureStats {
    pub entries: usize,
    pub plural_entries: usize,
    pub translator_comments: usize,
    pub extracted_comments: usize,
    pub references: usize,
    pub contexts: usize,
    pub metadata_comments: usize,
    pub obsolete_entries: usize,
    pub multiline_entries: usize,
    pub escaped_entries: usize,
}

pub struct Fixture {
    name: Cow<'static, str>,
    kind: &'static str,
    content: Cow<'static, str>,
    stats: FixtureStats,
}

pub struct IcuFixture {
    name: Cow<'static, str>,
    kind: &'static str,
    messages: Vec<String>,
    total_bytes: usize,
}

pub struct MergeFixture {
    name: Cow<'static, str>,
    kind: &'static str,
    existing_po: Cow<'static, str>,
    extracted_messages: Vec<MergeExtractedMessage<'static>>,
    api_extracted_messages: Vec<ExtractedMessage>,
    existing_entries: usize,
}

#[derive(Clone, Copy)]
enum IcuFixtureKind {
    Literal,
    Args,
    Formatters,
    Plural,
    Select,
    Nested,
    Tags,
}

#[derive(Clone, Copy)]
enum CatalogIcuFixtureKind {
    Light,
    Heavy,
    Projectable,
    Unsupported,
}

#[derive(Clone, Copy)]
enum GettextFixtureFamily {
    Ui,
    Commerce,
    Saas,
    Content,
}

#[derive(Clone, Copy)]
struct GettextLocaleProfile {
    id: &'static str,
    language: &'static str,
    plural_forms: &'static str,
    nplurals: usize,
}

impl MergeFixture {
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn kind(&self) -> &str {
        self.kind
    }

    pub fn existing_po(&self) -> &str {
        self.existing_po.as_ref()
    }

    pub fn extracted_messages(&self) -> &[MergeExtractedMessage<'static>] {
        &self.extracted_messages
    }

    pub fn existing_entries(&self) -> usize {
        self.existing_entries
    }

    pub fn extracted_entries(&self) -> usize {
        self.extracted_messages.len()
    }

    pub fn api_extracted_messages(&self) -> &[ExtractedMessage] {
        &self.api_extracted_messages
    }
}

impl Fixture {
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn kind(&self) -> &str {
        self.kind
    }

    pub fn content(&self) -> &str {
        self.content.as_ref()
    }

    pub fn stats(&self) -> FixtureStats {
        self.stats
    }
}

impl IcuFixture {
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn kind(&self) -> &str {
        self.kind
    }

    pub fn messages(&self) -> &[String] {
        &self.messages
    }

    pub fn entries(&self) -> usize {
        self.messages.len()
    }

    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }
}

pub fn fixture_by_name(name: &str) -> Option<Fixture> {
    match name {
        "tiny" => Some(static_fixture("tiny", TINY_FIXTURE)),
        "realistic" => Some(static_fixture("realistic", REALISTIC_FIXTURE)),
        "stress" => Some(static_fixture("stress", STRESS_FIXTURE)),
        "mixed-1000" => Some(generated_fixture(1_000)),
        "mixed-10000" => Some(generated_fixture(10_000)),
        _ => parse_gettext_fixture_name(name)
            .map(|(family, locale, entries)| generated_gettext_fixture(family, locale, entries)),
    }
}

pub fn icu_fixture_by_name(name: &str) -> Option<IcuFixture> {
    match name {
        "icu-literal-1000" => Some(generated_icu_fixture(IcuFixtureKind::Literal, 1_000)),
        "icu-literal-10000" => Some(generated_icu_fixture(IcuFixtureKind::Literal, 10_000)),
        "icu-args-1000" => Some(generated_icu_fixture(IcuFixtureKind::Args, 1_000)),
        "icu-args-10000" => Some(generated_icu_fixture(IcuFixtureKind::Args, 10_000)),
        "icu-formatters-1000" => Some(generated_icu_fixture(IcuFixtureKind::Formatters, 1_000)),
        "icu-formatters-10000" => Some(generated_icu_fixture(IcuFixtureKind::Formatters, 10_000)),
        "icu-plural-1000" => Some(generated_icu_fixture(IcuFixtureKind::Plural, 1_000)),
        "icu-plural-10000" => Some(generated_icu_fixture(IcuFixtureKind::Plural, 10_000)),
        "icu-select-1000" => Some(generated_icu_fixture(IcuFixtureKind::Select, 1_000)),
        "icu-select-10000" => Some(generated_icu_fixture(IcuFixtureKind::Select, 10_000)),
        "icu-nested-1000" => Some(generated_icu_fixture(IcuFixtureKind::Nested, 1_000)),
        "icu-nested-10000" => Some(generated_icu_fixture(IcuFixtureKind::Nested, 10_000)),
        "icu-tags-1000" => Some(generated_icu_fixture(IcuFixtureKind::Tags, 1_000)),
        "icu-tags-10000" => Some(generated_icu_fixture(IcuFixtureKind::Tags, 10_000)),
        _ => None,
    }
}

pub fn merge_fixture_by_name(name: &str) -> Option<MergeFixture> {
    match name {
        "mixed-1000" => Some(generated_merge_fixture(1_000)),
        "mixed-10000" => Some(generated_merge_fixture(10_000)),
        "catalog-icu-light" => Some(generated_catalog_icu_fixture(
            CatalogIcuFixtureKind::Light,
            1_000,
        )),
        "catalog-icu-heavy" => Some(generated_catalog_icu_fixture(
            CatalogIcuFixtureKind::Heavy,
            1_000,
        )),
        "catalog-icu-projectable" => Some(generated_catalog_icu_fixture(
            CatalogIcuFixtureKind::Projectable,
            1_000,
        )),
        "catalog-icu-unsupported" => Some(generated_catalog_icu_fixture(
            CatalogIcuFixtureKind::Unsupported,
            1_000,
        )),
        _ => parse_gettext_fixture_name(name).map(|(family, locale, entries)| {
            generated_gettext_merge_fixture(family, locale, entries)
        }),
    }
}

fn static_fixture(name: &'static str, content: &'static str) -> Fixture {
    Fixture {
        name: Cow::Borrowed(name),
        kind: "static",
        content: Cow::Borrowed(content),
        stats: scan_stats(content),
    }
}

fn generated_fixture(entries: usize) -> Fixture {
    let content = build_mixed_fixture(entries);
    let stats = scan_stats(&content);
    Fixture {
        name: Cow::Owned(format!("mixed-{entries}")),
        kind: "generated",
        content: Cow::Owned(content),
        stats,
    }
}

fn generated_gettext_fixture(
    family: GettextFixtureFamily,
    locale: GettextLocaleProfile,
    entries: usize,
) -> Fixture {
    let content = build_gettext_fixture(family, locale, entries);
    let stats = scan_stats(&content);
    Fixture {
        name: Cow::Owned(format!(
            "gettext-{}-{}-{entries}",
            gettext_family_name(family),
            locale.id
        )),
        kind: "generated-gettext",
        content: Cow::Owned(content),
        stats,
    }
}

fn generated_icu_fixture(kind: IcuFixtureKind, entries: usize) -> IcuFixture {
    let messages = (0..entries)
        .map(|index| build_icu_message(kind, index))
        .collect::<Vec<_>>();
    let total_bytes = messages.iter().map(|message| message.len()).sum();

    IcuFixture {
        name: Cow::Owned(format!("{}-{entries}", icu_fixture_kind_name(kind))),
        kind: "generated",
        messages,
        total_bytes,
    }
}

fn generated_merge_fixture(entries: usize) -> MergeFixture {
    merge_fixture_from_existing(
        format!("merge-mixed-{entries}"),
        "generated",
        build_mixed_fixture(entries),
    )
}

fn generated_gettext_merge_fixture(
    family: GettextFixtureFamily,
    locale: GettextLocaleProfile,
    entries: usize,
) -> MergeFixture {
    merge_fixture_from_existing(
        format!(
            "gettext-{}-{}-{entries}",
            gettext_family_name(family),
            locale.id
        ),
        "generated-gettext",
        build_gettext_fixture(family, locale, entries),
    )
}

fn merge_fixture_from_existing(
    name: String,
    kind: &'static str,
    existing_po: String,
) -> MergeFixture {
    let parsed = parse_po(&existing_po).expect("generated merge fixture must parse");

    let mut extracted_messages = Vec::with_capacity((parsed.items.len() * 9) / 10);
    let mut api_extracted_messages = Vec::with_capacity((parsed.items.len() * 9) / 10);
    let mut active_index = 0usize;
    for item in &parsed.items {
        if item.obsolete {
            continue;
        }
        active_index += 1;
        if active_index % 5 == 0 {
            continue;
        }

        let reference = format!(
            "src/merged_{:04}.rs:{}",
            active_index,
            (active_index % 200) + 1
        );
        let extracted_comment = (active_index % 7 == 0)
            .then(|| format!("Merged extractor comment {}", active_index % 13));
        let msgctxt = item.msgctxt.clone();
        let msgid = item.msgid.clone();
        let msgid_plural = item.msgid_plural.clone();

        extracted_messages.push(MergeExtractedMessage {
            msgctxt: msgctxt.clone().map(Cow::Owned),
            msgid: Cow::Owned(msgid.clone()),
            msgid_plural: msgid_plural.clone().map(Cow::Owned),
            references: vec![Cow::Owned(reference.clone())],
            extracted_comments: extracted_comment
                .clone()
                .into_iter()
                .map(Cow::Owned)
                .collect(),
            flags: if active_index % 11 == 0 {
                vec![Cow::Borrowed("c-format")]
            } else {
                Vec::new()
            },
        });

        api_extracted_messages.push(if let Some(msgid_plural) = msgid_plural {
            ExtractedMessage::Plural(ExtractedPluralMessage {
                msgid,
                msgctxt,
                source: PluralSource {
                    one: Some(item.msgid.clone()),
                    other: msgid_plural,
                },
                comments: extracted_comment.into_iter().collect(),
                origin: vec![parse_origin(&reference)],
                placeholders: Default::default(),
            })
        } else {
            ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid,
                msgctxt,
                comments: extracted_comment.into_iter().collect(),
                origin: vec![parse_origin(&reference)],
                placeholders: Default::default(),
            })
        });
    }

    for index in 0..(parsed.items.len() / 10).max(1) {
        let message_index = parsed.items.len() + index;
        let msgctxt =
            (message_index % 9 == 0).then(|| format!("merge-context-{}", message_index % 5));
        let msgid = format!("Merged message {}", message_index);
        let msgid_plural =
            (message_index % 8 == 0).then(|| format!("Merged messages {}", message_index));
        let reference = format!(
            "src/new_merge_{:04}.rs:{}",
            message_index,
            (message_index % 200) + 1
        );
        let extracted_comment = (message_index % 6 == 0).then(|| "newly extracted".to_owned());

        extracted_messages.push(MergeExtractedMessage {
            msgctxt: msgctxt.clone().map(Cow::Owned),
            msgid: Cow::Owned(msgid.clone()),
            msgid_plural: msgid_plural.clone().map(Cow::Owned),
            references: vec![Cow::Owned(reference.clone())],
            extracted_comments: extracted_comment
                .clone()
                .into_iter()
                .map(Cow::Owned)
                .collect(),
            flags: if message_index % 10 == 0 {
                vec![Cow::Borrowed("fuzzy")]
            } else {
                Vec::new()
            },
        });

        api_extracted_messages.push(if let Some(msgid_plural) = msgid_plural {
            ExtractedMessage::Plural(ExtractedPluralMessage {
                msgid: msgid.clone(),
                msgctxt: msgctxt.clone(),
                source: PluralSource {
                    one: Some(msgid),
                    other: msgid_plural,
                },
                comments: extracted_comment.into_iter().collect(),
                origin: vec![parse_origin(&reference)],
                placeholders: Default::default(),
            })
        } else {
            ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid,
                msgctxt,
                comments: extracted_comment.into_iter().collect(),
                origin: vec![parse_origin(&reference)],
                placeholders: Default::default(),
            })
        });
    }

    MergeFixture {
        name: Cow::Owned(name),
        kind,
        existing_entries: parsed.items.len(),
        existing_po: Cow::Owned(existing_po),
        extracted_messages,
        api_extracted_messages,
    }
}

fn generated_catalog_icu_fixture(kind: CatalogIcuFixtureKind, entries: usize) -> MergeFixture {
    let mut existing_po = String::with_capacity(entries * 160);
    existing_po.push_str("# ICU-heavy benchmark catalog\n");
    existing_po.push_str("msgid \"\"\n");
    existing_po.push_str("msgstr \"\"\n");
    existing_po.push_str("\"Project-Id-Version: ferrocat icu benchmark\\n\"\n");
    existing_po.push_str("\"Language: de\\n\"\n");
    existing_po.push_str("\"Content-Type: text/plain; charset=UTF-8\\n\"\n");
    existing_po.push_str("\"Content-Transfer-Encoding: 8bit\\n\"\n\n");

    let mut extracted_messages = Vec::with_capacity(entries);
    let mut api_extracted_messages = Vec::with_capacity(entries);

    for index in 0..entries {
        let msgctxt = (index % 9 == 0).then(|| format!("icu-context-{}", index % 5));
        let reference = format!("src/icu_{:04}.tsx:{}", index, (index % 200) + 1);
        let comments = if index % 7 == 0 {
            vec![format!("ICU benchmark extractor note {}", index % 11)]
        } else {
            Vec::new()
        };
        let origin = vec![parse_origin(&reference)];
        let flavor = catalog_icu_flavor(kind, index);
        let (msgid, msgstr, api_message, merge_message) =
            build_catalog_icu_entry(flavor, index, msgctxt.clone(), comments, origin, reference);

        if let Some(ref msgctxt) = msgctxt {
            push_keyword(&mut existing_po, "", "msgctxt", msgctxt);
        }
        push_keyword(&mut existing_po, "", "msgid", &msgid);
        push_keyword(&mut existing_po, "", "msgstr", &msgstr);
        existing_po.push('\n');

        api_extracted_messages.push(api_message);
        extracted_messages.push(merge_message);
    }

    MergeFixture {
        name: Cow::Borrowed(match kind {
            CatalogIcuFixtureKind::Light => "catalog-icu-light",
            CatalogIcuFixtureKind::Heavy => "catalog-icu-heavy",
            CatalogIcuFixtureKind::Projectable => "catalog-icu-projectable",
            CatalogIcuFixtureKind::Unsupported => "catalog-icu-unsupported",
        }),
        kind: "generated",
        existing_entries: entries,
        existing_po: Cow::Owned(existing_po),
        extracted_messages,
        api_extracted_messages,
    }
}

fn icu_fixture_kind_name(kind: IcuFixtureKind) -> &'static str {
    match kind {
        IcuFixtureKind::Literal => "icu-literal",
        IcuFixtureKind::Args => "icu-args",
        IcuFixtureKind::Formatters => "icu-formatters",
        IcuFixtureKind::Plural => "icu-plural",
        IcuFixtureKind::Select => "icu-select",
        IcuFixtureKind::Nested => "icu-nested",
        IcuFixtureKind::Tags => "icu-tags",
    }
}

fn build_icu_message(kind: IcuFixtureKind, index: usize) -> String {
    match kind {
        IcuFixtureKind::Literal => {
            format!("Static localized copy for benchmark entry {index} without ICU placeholders.")
        }
        IcuFixtureKind::Args => {
            format!("Hello {{name}}, benchmark item {index} has {{count}} values and {{value}}.")
        }
        IcuFixtureKind::Formatters => {
            "On {date, date, short} at {time, time, ::HHmm} {name} saw {count, number, integer} items in list {items, list, disjunction}.".to_string()
        }
        IcuFixtureKind::Plural => icu_top_level_plural(
            "count",
            &format!("{index} file for {{name}}"),
            &format!("{index} files for {{name}}"),
        ),
        IcuFixtureKind::Select => {
            "{gender, select, male {He has {count, number} files for {name}} female {She has {count, number} files for {name}} other {They have {count, number} files for {name}}}".to_string()
        }
        IcuFixtureKind::Nested => {
            "{gender, select, male {{count, plural, one {He opened one alert} other {He opened # alerts for {name}}}} female {{count, plural, one {She opened one alert} other {She opened # alerts for {name}}}} other {{count, plural, one {They opened one alert} other {They opened # alerts for {name}}}}}".to_string()
        }
        IcuFixtureKind::Tags => format!(
            "<link>{{name}}</link> has <b>{{count, plural, one {{# alert}} other {{# alerts}}}}</b> in benchmark entry {index}."
        ),
    }
}

fn parse_gettext_fixture_name(
    name: &str,
) -> Option<(GettextFixtureFamily, GettextLocaleProfile, usize)> {
    let mut parts = name.split('-');
    let prefix = parts.next()?;
    let family = parts.next()?;
    let locale = parts.next()?;
    let entries = parts.next()?;

    if prefix != "gettext" || parts.next().is_some() {
        return None;
    }

    Some((
        parse_gettext_family(family)?,
        gettext_locale_profile(locale)?,
        entries.parse::<usize>().ok()?,
    ))
}

fn parse_gettext_family(name: &str) -> Option<GettextFixtureFamily> {
    match name {
        "ui" => Some(GettextFixtureFamily::Ui),
        "commerce" => Some(GettextFixtureFamily::Commerce),
        "saas" => Some(GettextFixtureFamily::Saas),
        "content" => Some(GettextFixtureFamily::Content),
        _ => None,
    }
}

fn gettext_locale_profile(id: &str) -> Option<GettextLocaleProfile> {
    match id {
        "de" => Some(GettextLocaleProfile {
            id: "de",
            language: "de",
            plural_forms: "nplurals=2; plural=(n != 1);",
            nplurals: 2,
        }),
        "fr" => Some(GettextLocaleProfile {
            id: "fr",
            language: "fr",
            plural_forms: "nplurals=2; plural=(n > 1);",
            nplurals: 2,
        }),
        "pl" => Some(GettextLocaleProfile {
            id: "pl",
            language: "pl",
            plural_forms: "nplurals=3; plural=(n == 1 ? 0 : (n % 10 >= 2 && n % 10 <= 4 && (n % 100 < 10 || n % 100 >= 20)) ? 1 : 2);",
            nplurals: 3,
        }),
        "ar" => Some(GettextLocaleProfile {
            id: "ar",
            language: "ar",
            plural_forms: "nplurals=6; plural=(n == 0 ? 0 : n == 1 ? 1 : n == 2 ? 2 : (n % 100 >= 3 && n % 100 <= 10) ? 3 : (n % 100 >= 11 && n % 100 <= 99) ? 4 : 5);",
            nplurals: 6,
        }),
        _ => None,
    }
}

fn gettext_family_name(family: GettextFixtureFamily) -> &'static str {
    match family {
        GettextFixtureFamily::Ui => "ui",
        GettextFixtureFamily::Commerce => "commerce",
        GettextFixtureFamily::Saas => "saas",
        GettextFixtureFamily::Content => "content",
    }
}

fn build_gettext_fixture(
    family: GettextFixtureFamily,
    locale: GettextLocaleProfile,
    entries: usize,
) -> String {
    let mut out = String::with_capacity(entries * 160);
    out.push_str("# gettext compatibility benchmark corpus\n");
    out.push_str("msgid \"\"\n");
    out.push_str("msgstr \"\"\n");
    out.push_str("\"Project-Id-Version: ferrocat gettext compat benchmark\\n\"\n");
    out.push_str(&format!("\"Language: {}\\n\"\n", locale.language));
    out.push_str("\"Content-Type: text/plain; charset=UTF-8\\n\"\n");
    out.push_str("\"Content-Transfer-Encoding: 8bit\\n\"\n");
    out.push_str(&format!("\"Plural-Forms: {}\\n\"\n\n", locale.plural_forms));

    for index in 0..entries {
        let shape = gettext_feature_shape(family, index);
        let effective_multiline = shape.is_multiline && !(shape.is_plural && locale.nplurals > 2);
        let effective_escape = shape.has_escape && !(shape.is_plural && locale.nplurals > 2);
        if shape.has_translator_comment {
            push_line(
                &mut out,
                "",
                gettext_translator_comment(family, index, shape.is_plural),
            );
        }
        if shape.has_extracted_comment {
            push_line(
                &mut out,
                "",
                gettext_extracted_comment(family, index, shape.is_plural),
            );
        }
        if shape.has_references {
            push_line(&mut out, "", &gettext_reference_line(family, index));
        }
        if shape.has_fuzzy {
            push_line(&mut out, "", "#, fuzzy");
        } else if shape.has_c_format {
            push_line(&mut out, "", "#, c-format");
        }
        if shape.has_context {
            push_keyword(&mut out, "", "msgctxt", &gettext_context(family, index));
        }

        let subject = gettext_subject(family, index);
        let source = gettext_source_message(
            family,
            &subject,
            index,
            shape.is_plural,
            effective_multiline,
            effective_escape,
        );
        push_keyword(&mut out, "", "msgid", &source.msgid);
        if let Some(msgid_plural) = source.msgid_plural.as_deref() {
            push_keyword(&mut out, "", "msgid_plural", msgid_plural);
            for (slot, value) in gettext_plural_translations(
                locale,
                family,
                &subject,
                index,
                effective_multiline,
                effective_escape,
            )
            .into_iter()
            .enumerate()
            {
                push_indexed_keyword(&mut out, "", "msgstr", slot, &value);
            }
        } else {
            push_keyword(
                &mut out,
                "",
                "msgstr",
                &gettext_singular_translation(
                    locale,
                    family,
                    &subject,
                    index,
                    effective_multiline,
                    effective_escape,
                ),
            );
        }
        out.push('\n');
    }

    out
}

#[derive(Clone, Copy)]
struct GettextFeatureShape {
    is_plural: bool,
    is_multiline: bool,
    has_escape: bool,
    has_context: bool,
    has_references: bool,
    has_translator_comment: bool,
    has_extracted_comment: bool,
    has_fuzzy: bool,
    has_c_format: bool,
}

fn gettext_feature_shape(family: GettextFixtureFamily, index: usize) -> GettextFeatureShape {
    let plural_mod = match family {
        GettextFixtureFamily::Ui => 8,
        GettextFixtureFamily::Commerce => 4,
        GettextFixtureFamily::Saas => 6,
        GettextFixtureFamily::Content => 7,
    };
    let multiline_mod = match family {
        GettextFixtureFamily::Ui => 21,
        GettextFixtureFamily::Commerce => 18,
        GettextFixtureFamily::Saas => 15,
        GettextFixtureFamily::Content => 5,
    };
    let context_mod = match family {
        GettextFixtureFamily::Ui => 4,
        GettextFixtureFamily::Commerce => 6,
        GettextFixtureFamily::Saas => 5,
        GettextFixtureFamily::Content => 9,
    };

    GettextFeatureShape {
        is_plural: index % plural_mod == 0,
        is_multiline: index % multiline_mod == 0,
        has_escape: index % 17 == 0,
        has_context: index % context_mod == 0,
        has_references: index % 2 == 0,
        has_translator_comment: index % 9 == 0,
        has_extracted_comment: index % 11 == 0,
        has_fuzzy: index % 19 == 0,
        has_c_format: true,
    }
}

struct GettextSourceMessage {
    msgid: String,
    msgid_plural: Option<String>,
}

fn gettext_source_message(
    family: GettextFixtureFamily,
    subject: &str,
    index: usize,
    is_plural: bool,
    is_multiline: bool,
    has_escape: bool,
) -> GettextSourceMessage {
    if is_plural {
        let (msgid, msgid_plural) = match family {
            GettextFixtureFamily::Ui => (
                format!("%d pending {subject} alert for workspace %s"),
                format!("%d pending {subject} alerts for workspace %s"),
            ),
            GettextFixtureFamily::Commerce => (
                format!("%d {subject} item in cart for order %s"),
                format!("%d {subject} items in cart for order %s"),
            ),
            GettextFixtureFamily::Saas => (
                format!("%d team member assigned to {subject} in %s"),
                format!("%d team members assigned to {subject} in %s"),
            ),
            GettextFixtureFamily::Content => (
                format!("%d revised paragraph in {subject} for publication %s"),
                format!("%d revised paragraphs in {subject} for publication %s"),
            ),
        };
        return GettextSourceMessage {
            msgid: maybe_multiline_or_escaped(msgid, index, is_multiline, has_escape),
            msgid_plural: Some(maybe_multiline_or_escaped(
                msgid_plural,
                index,
                is_multiline,
                has_escape,
            )),
        };
    }

    let msgid = match family {
        GettextFixtureFamily::Ui => {
            format!("Open the {subject} panel for workspace %s")
        }
        GettextFixtureFamily::Commerce => {
            format!("Review the {subject} step for order %s")
        }
        GettextFixtureFamily::Saas => {
            format!("Invite %s to the {subject} workspace")
        }
        GettextFixtureFamily::Content => {
            format!("Publish the {subject} article for channel %s")
        }
    };

    GettextSourceMessage {
        msgid: maybe_multiline_or_escaped(msgid, index, is_multiline, has_escape),
        msgid_plural: None,
    }
}

fn maybe_multiline_or_escaped(
    base: String,
    index: usize,
    is_multiline: bool,
    has_escape: bool,
) -> String {
    if is_multiline {
        format!("{base}.\nReview the current value set before saving entry {index}.")
    } else if has_escape {
        format!("\"{base}\" shortcut for entry {index}")
    } else {
        base
    }
}

fn gettext_singular_translation(
    locale: GettextLocaleProfile,
    family: GettextFixtureFamily,
    subject: &str,
    index: usize,
    is_multiline: bool,
    has_escape: bool,
) -> String {
    let base = match (locale.id, family) {
        ("de", GettextFixtureFamily::Ui) => {
            format!("Oeffne den Bereich {subject} fuer Workspace %s")
        }
        ("de", GettextFixtureFamily::Commerce) => {
            format!("Pruefe den Schritt {subject} fuer Bestellung %s")
        }
        ("de", GettextFixtureFamily::Saas) => format!("Lade %s in den Bereich {subject} ein"),
        ("de", GettextFixtureFamily::Content) => {
            format!("Veroeffentliche den Artikel {subject} fuer Kanal %s")
        }
        ("fr", GettextFixtureFamily::Ui) => format!("Ouvrir la section {subject} pour espace %s"),
        ("fr", GettextFixtureFamily::Commerce) => {
            format!("Verifier letape {subject} pour commande %s")
        }
        ("fr", GettextFixtureFamily::Saas) => format!("Inviter %s dans lespace {subject}"),
        ("fr", GettextFixtureFamily::Content) => {
            format!("Publier larticle {subject} pour canal %s")
        }
        ("pl", GettextFixtureFamily::Ui) => format!("Otworz sekcje {subject} dla workspace %s"),
        ("pl", GettextFixtureFamily::Commerce) => {
            format!("Sprawdz etap {subject} dla zamowienia %s")
        }
        ("pl", GettextFixtureFamily::Saas) => format!("Zaproś %s do obszaru {subject}"),
        ("pl", GettextFixtureFamily::Content) => {
            format!("Opublikuj artykul {subject} dla kanalu %s")
        }
        ("ar", GettextFixtureFamily::Ui) => format!("Iftah qism {subject} li workspace %s"),
        ("ar", GettextFixtureFamily::Commerce) => format!("Muraja khatwa {subject} li order %s"),
        ("ar", GettextFixtureFamily::Saas) => format!("Uda %s ila masaha {subject}"),
        ("ar", GettextFixtureFamily::Content) => format!("Unshur maqal {subject} li channel %s"),
        _ => format!("Translate {subject} entry {index} for %s"),
    };

    maybe_multiline_or_escaped(base, index, is_multiline, has_escape)
}

fn gettext_plural_translations(
    locale: GettextLocaleProfile,
    family: GettextFixtureFamily,
    subject: &str,
    index: usize,
    is_multiline: bool,
    has_escape: bool,
) -> Vec<String> {
    (0..locale.nplurals)
        .map(|slot| {
            let base = match (locale.id, family) {
                ("de", GettextFixtureFamily::Ui) => {
                    format!("%d offener Hinweis {subject} fuer Workspace %s [slot {slot}]")
                }
                ("de", GettextFixtureFamily::Commerce) => {
                    format!("%d Artikel {subject} im Warenkorb fuer Bestellung %s [slot {slot}]")
                }
                ("de", GettextFixtureFamily::Saas) => {
                    format!("%d Teammitglieder in {subject} fuer %s [slot {slot}]")
                }
                ("de", GettextFixtureFamily::Content) => {
                    format!("%d Abschnitte in {subject} fuer Veroeffentlichung %s [slot {slot}]")
                }
                ("fr", GettextFixtureFamily::Ui) => {
                    format!("%d alerte {subject} pour espace %s [slot {slot}]")
                }
                ("fr", GettextFixtureFamily::Commerce) => {
                    format!("%d article {subject} dans le panier %s [slot {slot}]")
                }
                ("fr", GettextFixtureFamily::Saas) => {
                    format!("%d membres dans {subject} pour %s [slot {slot}]")
                }
                ("fr", GettextFixtureFamily::Content) => {
                    format!("%d paragraphes dans {subject} pour publication %s [slot {slot}]")
                }
                ("pl", GettextFixtureFamily::Ui) => {
                    format!("%d alerty {subject} dla workspace %s [slot {slot}]")
                }
                ("pl", GettextFixtureFamily::Commerce) => {
                    format!("%d elementy {subject} w koszyku %s [slot {slot}]")
                }
                ("pl", GettextFixtureFamily::Saas) => {
                    format!("%d osoby w {subject} dla %s [slot {slot}]")
                }
                ("pl", GettextFixtureFamily::Content) => {
                    format!("%d akapity w {subject} dla publikacji %s [slot {slot}]")
                }
                ("ar", GettextFixtureFamily::Ui) => {
                    format!("%d tanbih {subject} li workspace %s [slot {slot}]")
                }
                ("ar", GettextFixtureFamily::Commerce) => {
                    format!("%d item {subject} fi cart %s [slot {slot}]")
                }
                ("ar", GettextFixtureFamily::Saas) => {
                    format!("%d member fi {subject} li %s [slot {slot}]")
                }
                ("ar", GettextFixtureFamily::Content) => {
                    format!("%d paragraph fi {subject} li publish %s [slot {slot}]")
                }
                _ => format!("%d translated {subject} slot {slot} for %s"),
            };
            maybe_multiline_or_escaped(base, index, is_multiline, has_escape)
        })
        .collect()
}

fn gettext_subject(family: GettextFixtureFamily, index: usize) -> String {
    const UI: [&str; 8] = [
        "notifications",
        "billing",
        "security",
        "preferences",
        "dashboard",
        "integrations",
        "team access",
        "release notes",
    ];
    const COMMERCE: [&str; 8] = [
        "cart", "shipment", "payment", "discount", "invoice", "wishlist", "refund", "address",
    ];
    const SAAS: [&str; 8] = [
        "workspace",
        "project",
        "environment",
        "audit log",
        "role mapping",
        "team",
        "service account",
        "incident policy",
    ];
    const CONTENT: [&str; 8] = [
        "article",
        "guide",
        "campaign",
        "newsletter",
        "landing page",
        "knowledge base",
        "release email",
        "help center",
    ];

    let pool = match family {
        GettextFixtureFamily::Ui => UI.as_slice(),
        GettextFixtureFamily::Commerce => COMMERCE.as_slice(),
        GettextFixtureFamily::Saas => SAAS.as_slice(),
        GettextFixtureFamily::Content => CONTENT.as_slice(),
    };
    format!("{} {:04}", pool[index % pool.len()], index)
}

fn gettext_context(family: GettextFixtureFamily, index: usize) -> String {
    match family {
        GettextFixtureFamily::Ui => {
            ["button", "menu", "dialog.title", "toast", "empty-state"][index % 5].to_owned()
        }
        GettextFixtureFamily::Commerce => [
            "checkout.step",
            "cart.sidebar",
            "receipt.email",
            "invoice.pdf",
            "promo.banner",
        ][index % 5]
            .to_owned(),
        GettextFixtureFamily::Saas => [
            "settings.page",
            "invite.modal",
            "audit.table",
            "billing.notice",
            "access.review",
        ][index % 5]
            .to_owned(),
        GettextFixtureFamily::Content => [
            "editor.toolbar",
            "email.subject",
            "email.body",
            "help.article",
            "cms.sidebar",
        ][index % 5]
            .to_owned(),
    }
}

fn gettext_reference_line(family: GettextFixtureFamily, index: usize) -> String {
    let path = match family {
        GettextFixtureFamily::Ui => format!("src/ui/view_{:04}.tsx", index % 320),
        GettextFixtureFamily::Commerce => format!("src/checkout/flow_{:04}.tsx", index % 280),
        GettextFixtureFamily::Saas => format!("src/settings/page_{:04}.tsx", index % 260),
        GettextFixtureFamily::Content => format!("src/content/panel_{:04}.tsx", index % 240),
    };
    format!("#: {path}:{}", (index % 180) + 1)
}

fn gettext_translator_comment(
    family: GettextFixtureFamily,
    index: usize,
    is_plural: bool,
) -> &'static str {
    match (family, is_plural, index % 3) {
        (GettextFixtureFamily::Ui, false, _) => "# Keep the label short for narrow sidebars",
        (GettextFixtureFamily::Ui, true, _) => {
            "# Plural wording appears in the notification center"
        }
        (GettextFixtureFamily::Commerce, false, _) => "# Used in the checkout confirmation flow",
        (GettextFixtureFamily::Commerce, true, _) => "# Keep quantity wording natural for the cart",
        (GettextFixtureFamily::Saas, false, _) => "# Shown to administrators in account settings",
        (GettextFixtureFamily::Saas, true, _) => "# Used in team membership summary rows",
        (GettextFixtureFamily::Content, false, _) => "# Appears in long-form publishing workflows",
        (GettextFixtureFamily::Content, true, _) => {
            "# Plural text is rendered in editorial summaries"
        }
    }
}

fn gettext_extracted_comment(
    family: GettextFixtureFamily,
    index: usize,
    is_plural: bool,
) -> &'static str {
    match (family, is_plural, index % 3) {
        (GettextFixtureFamily::Ui, false, _) => {
            "#. UI label rendered in the main application shell"
        }
        (GettextFixtureFamily::Ui, true, _) => "#. Quantity label rendered in the alert overview",
        (GettextFixtureFamily::Commerce, false, _) => {
            "#. Checkout label rendered during order review"
        }
        (GettextFixtureFamily::Commerce, true, _) => {
            "#. Quantity summary rendered in the cart sidebar"
        }
        (GettextFixtureFamily::Saas, false, _) => {
            "#. Settings label rendered in multi-tenant admin screens"
        }
        (GettextFixtureFamily::Saas, true, _) => {
            "#. Membership summary rendered in access review lists"
        }
        (GettextFixtureFamily::Content, false, _) => {
            "#. Publishing label rendered in content operations tools"
        }
        (GettextFixtureFamily::Content, true, _) => {
            "#. Quantity summary rendered in editorial digests"
        }
    }
}

fn icu_top_level_plural(variable: &str, one: &str, other: &str) -> String {
    let mut out = String::new();
    out.push('{');
    out.push_str(variable);
    out.push_str(", plural, one {");
    out.push_str(one);
    out.push_str("} other {");
    out.push_str(other);
    out.push_str("}}");
    out
}

fn catalog_icu_flavor(kind: CatalogIcuFixtureKind, index: usize) -> CatalogIcuFlavor {
    match kind {
        CatalogIcuFixtureKind::Light => {
            if index % 12 == 0 {
                CatalogIcuFlavor::ProjectablePlural
            } else if index % 5 == 0 {
                CatalogIcuFlavor::Formatters
            } else {
                CatalogIcuFlavor::Args
            }
        }
        CatalogIcuFixtureKind::Heavy => match index % 6 {
            0 | 1 => CatalogIcuFlavor::ProjectablePlural,
            2 => CatalogIcuFlavor::NestedUnsupported,
            3 => CatalogIcuFlavor::SelectUnsupported,
            4 => CatalogIcuFlavor::TagsProjectable,
            _ => CatalogIcuFlavor::Formatters,
        },
        CatalogIcuFixtureKind::Projectable => match index % 3 {
            0 => CatalogIcuFlavor::ProjectablePlural,
            1 => CatalogIcuFlavor::TagsProjectable,
            _ => CatalogIcuFlavor::FormatterPlural,
        },
        CatalogIcuFixtureKind::Unsupported => match index % 3 {
            0 => CatalogIcuFlavor::NestedUnsupported,
            1 => CatalogIcuFlavor::ExactSelectorUnsupported,
            _ => CatalogIcuFlavor::OffsetUnsupported,
        },
    }
}

#[derive(Clone, Copy)]
enum CatalogIcuFlavor {
    Args,
    Formatters,
    ProjectablePlural,
    FormatterPlural,
    TagsProjectable,
    NestedUnsupported,
    SelectUnsupported,
    ExactSelectorUnsupported,
    OffsetUnsupported,
}

fn build_catalog_icu_entry(
    flavor: CatalogIcuFlavor,
    index: usize,
    msgctxt: Option<String>,
    comments: Vec<String>,
    origin: Vec<CatalogOrigin>,
    reference: String,
) -> (
    String,
    String,
    ExtractedMessage,
    MergeExtractedMessage<'static>,
) {
    let merge_comments = comments.iter().cloned().map(Cow::Owned).collect::<Vec<_>>();
    let merge_reference = vec![Cow::Owned(reference)];
    match flavor {
        CatalogIcuFlavor::Args => {
            let msgid = format!("Bench {index}: Hello {{name}}, you have {{count}} items.");
            let msgstr = format!("Lauf {index}: Hallo {{name}}, du hast {{count}} Einträge.");
            let placeholders = placeholder_map(&[("name", "name"), ("count", "count")]);
            let api = ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: msgid.clone(),
                msgctxt: msgctxt.clone(),
                comments,
                origin,
                placeholders,
            });
            let merge = MergeExtractedMessage {
                msgctxt: msgctxt.clone().map(Cow::Owned),
                msgid: Cow::Owned(msgid.clone()),
                msgid_plural: None,
                references: merge_reference,
                extracted_comments: merge_comments,
                flags: Vec::new(),
            };
            (msgid, msgstr, api, merge)
        }
        CatalogIcuFlavor::Formatters => {
            let msgid = format!(
                "Bench {index}: {{count, number, integer}} items on {{date, date, short}} for {{name}}."
            );
            let msgstr = format!(
                "Lauf {index}: {{count, number, integer}} Einträge am {{date, date, short}} für {{name}}."
            );
            let placeholders =
                placeholder_map(&[("count", "count"), ("date", "date"), ("name", "name")]);
            let api = ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: msgid.clone(),
                msgctxt: msgctxt.clone(),
                comments,
                origin,
                placeholders,
            });
            let merge = MergeExtractedMessage {
                msgctxt: msgctxt.clone().map(Cow::Owned),
                msgid: Cow::Owned(msgid.clone()),
                msgid_plural: None,
                references: merge_reference,
                extracted_comments: merge_comments,
                flags: Vec::new(),
            };
            (msgid, msgstr, api, merge)
        }
        CatalogIcuFlavor::ProjectablePlural
        | CatalogIcuFlavor::FormatterPlural
        | CatalogIcuFlavor::TagsProjectable => {
            let (msgid, msgstr, one, other) = match flavor {
                CatalogIcuFlavor::ProjectablePlural => (
                    icu_top_level_plural(
                        "count",
                        &format!("{index} file for {{name}}"),
                        &format!("{index} files for {{name}}"),
                    ),
                    icu_top_level_plural(
                        "count",
                        &format!("{index} Datei für {{name}}"),
                        &format!("{index} Dateien für {{name}}"),
                    ),
                    format!("{index} file for {{name}}"),
                    format!("{index} files for {{name}}"),
                ),
                CatalogIcuFlavor::FormatterPlural => (
                    icu_top_level_plural(
                        "count",
                        &format!("{index} file on {{date, date, short}}"),
                        &format!("{index} files on {{date, date, short}}"),
                    ),
                    icu_top_level_plural(
                        "count",
                        &format!("{index} Datei am {{date, date, short}}"),
                        &format!("{index} Dateien am {{date, date, short}}"),
                    ),
                    format!("{index} file on {{date, date, short}}"),
                    format!("{index} files on {{date, date, short}}"),
                ),
                CatalogIcuFlavor::TagsProjectable => (
                    format!(
                        "{{count, plural, one {{<link>{index} alert</link>}} other {{<link>{index} alerts</link>}}}}"
                    ),
                    format!(
                        "{{count, plural, one {{<link>{index} Hinweis</link>}} other {{<link>{index} Hinweise</link>}}}}"
                    ),
                    format!("<link>{index} alert</link>"),
                    format!("<link>{index} alerts</link>"),
                ),
                _ => unreachable!(),
            };
            let mut placeholders = placeholder_map(&[("count", "count")]);
            if matches!(flavor, CatalogIcuFlavor::ProjectablePlural) {
                placeholders.insert("name".to_owned(), vec!["name".to_owned()]);
            }
            if matches!(flavor, CatalogIcuFlavor::FormatterPlural) {
                placeholders.insert("date".to_owned(), vec!["date".to_owned()]);
            }
            let api = ExtractedMessage::Plural(ExtractedPluralMessage {
                msgid: msgid.clone(),
                msgctxt: msgctxt.clone(),
                source: PluralSource {
                    one: Some(one),
                    other,
                },
                comments,
                origin,
                placeholders,
            });
            let merge = MergeExtractedMessage {
                msgctxt: msgctxt.clone().map(Cow::Owned),
                msgid: Cow::Owned(msgid.clone()),
                msgid_plural: None,
                references: merge_reference,
                extracted_comments: merge_comments,
                flags: Vec::new(),
            };
            (msgid, msgstr, api, merge)
        }
        CatalogIcuFlavor::NestedUnsupported => {
            let msgid =
                "{count, plural, one {{name, select, short {One short file} other {One file for {name}}}} other {{name, select, short {# short files} other {# files for {name}}}}}".to_string();
            let msgstr =
                "{count, plural, one {{name, select, short {Eine kurze Datei} other {Eine Datei für {name}}}} other {{name, select, short {# kurze Dateien} other {# Dateien für {name}}}}}".to_string();
            let placeholders = placeholder_map(&[("count", "count"), ("name", "name")]);
            let api = ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: msgid.clone(),
                msgctxt: msgctxt.clone(),
                comments,
                origin,
                placeholders,
            });
            let merge = MergeExtractedMessage {
                msgctxt: msgctxt.clone().map(Cow::Owned),
                msgid: Cow::Owned(msgid.clone()),
                msgid_plural: None,
                references: merge_reference,
                extracted_comments: merge_comments,
                flags: Vec::new(),
            };
            (msgid, msgstr, api, merge)
        }
        CatalogIcuFlavor::SelectUnsupported => {
            let msgid =
                "{choice, select, a {{count, plural, one {One A} other {# A items}}} other {{count, plural, one {One other} other {# other items}}}}".to_string();
            let msgstr =
                "{choice, select, a {{count, plural, one {Ein A} other {# A-Einträge}}} other {{count, plural, one {Ein anderer} other {# andere Einträge}}}}".to_string();
            let placeholders = placeholder_map(&[("choice", "choice"), ("count", "count")]);
            let api = ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: msgid.clone(),
                msgctxt: msgctxt.clone(),
                comments,
                origin,
                placeholders,
            });
            let merge = MergeExtractedMessage {
                msgctxt: msgctxt.clone().map(Cow::Owned),
                msgid: Cow::Owned(msgid.clone()),
                msgid_plural: None,
                references: merge_reference,
                extracted_comments: merge_comments,
                flags: Vec::new(),
            };
            (msgid, msgstr, api, merge)
        }
        CatalogIcuFlavor::ExactSelectorUnsupported => {
            let msgid = "{count, plural, =0 {No files} one {One file} other {# files}}".to_owned();
            let msgstr =
                "{count, plural, =0 {Keine Dateien} one {Eine Datei} other {# Dateien}}".to_owned();
            let placeholders = placeholder_map(&[("count", "count")]);
            let api = ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: msgid.clone(),
                msgctxt: msgctxt.clone(),
                comments,
                origin,
                placeholders,
            });
            let merge = MergeExtractedMessage {
                msgctxt: msgctxt.clone().map(Cow::Owned),
                msgid: Cow::Owned(msgid.clone()),
                msgid_plural: None,
                references: merge_reference,
                extracted_comments: merge_comments,
                flags: Vec::new(),
            };
            (msgid, msgstr, api, merge)
        }
        CatalogIcuFlavor::OffsetUnsupported => {
            let msgid = "{count, plural, offset:1 one {One guest} other {# guests}}".to_owned();
            let msgstr = "{count, plural, offset:1 one {Ein Gast} other {# Gäste}}".to_owned();
            let placeholders = placeholder_map(&[("count", "count")]);
            let api = ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: msgid.clone(),
                msgctxt: msgctxt.clone(),
                comments,
                origin,
                placeholders,
            });
            let merge = MergeExtractedMessage {
                msgctxt: msgctxt.map(Cow::Owned),
                msgid: Cow::Owned(msgid.clone()),
                msgid_plural: None,
                references: merge_reference,
                extracted_comments: merge_comments,
                flags: Vec::new(),
            };
            (msgid, msgstr, api, merge)
        }
    }
}

fn placeholder_map(entries: &[(&str, &str)]) -> BTreeMap<String, Vec<String>> {
    let mut map = BTreeMap::new();
    for (name, value) in entries {
        map.entry((*name).to_owned())
            .or_insert_with(Vec::new)
            .push((*value).to_owned());
    }
    map
}

fn parse_origin(reference: &str) -> CatalogOrigin {
    match reference.rsplit_once(':') {
        Some((file, line)) if line.chars().all(|ch| ch.is_ascii_digit()) => CatalogOrigin {
            file: file.to_owned(),
            line: line.parse::<u32>().ok(),
        },
        _ => CatalogOrigin {
            file: reference.to_owned(),
            line: None,
        },
    }
}

fn build_mixed_fixture(entries: usize) -> String {
    let mut out = String::with_capacity(entries * 120);
    out.push_str("# Benchmark corpus for ferrocat\n");
    out.push_str("# Mixed feature distribution, deterministic generation\n");
    out.push_str("msgid \"\"\n");
    out.push_str("msgstr \"\"\n");
    out.push_str("\"Project-Id-Version: ferrocat benchmark\\n\"\n");
    out.push_str("\"Language: de\\n\"\n");
    out.push_str("\"Content-Type: text/plain; charset=UTF-8\\n\"\n");
    out.push_str("\"Content-Transfer-Encoding: 8bit\\n\"\n");
    out.push_str("\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\n");

    for index in 0..entries {
        let is_plural = index % 10 == 0;
        let has_comment = index % 20 == 0;
        let has_extracted = index % 25 == 0;
        let has_references = index % 3 == 0;
        let has_context = index % 12 == 0;
        let has_metadata = index % 50 == 0;
        let is_obsolete = index > 0 && index % 100 == 0;
        let is_multiline = index % 33 == 0;
        let has_escape = index % 40 == 0;
        let prefix = if is_obsolete { "#~ " } else { "" };

        if has_comment {
            push_line(&mut out, prefix, "# Translator note for entry");
        }
        if has_extracted {
            push_line(&mut out, prefix, "#. Extracted from benchmark source");
        }
        if has_metadata {
            push_line(&mut out, prefix, "#@ domain: benchmark");
        }
        if has_references {
            push_line(
                &mut out,
                prefix,
                &format!("#: src/feature_{:04}.rs:{}", index, (index % 200) + 1),
            );
        }
        if index % 18 == 0 {
            push_line(&mut out, prefix, "#, fuzzy");
        }
        if has_context {
            push_keyword(
                &mut out,
                prefix,
                "msgctxt",
                &format!("context-{}", index % 7),
            );
        }

        let msgid = if is_multiline {
            format!("Entry {index} first line\nEntry {index} second line with placeholder {{name}}")
        } else if has_escape {
            format!("Entry {index} contains \\\"quotes\\\" and \\\\slashes\\\\")
        } else {
            format!("Entry {index} simple benchmark message")
        };

        push_keyword(&mut out, prefix, "msgid", &msgid);

        if is_plural {
            let plural = if is_multiline {
                format!("Entry {index} plural first line\nEntry {index} plural second line")
            } else {
                format!("Entry {index} plural benchmark messages")
            };
            push_keyword(&mut out, prefix, "msgid_plural", &plural);

            let singular_translation = if is_multiline {
                format!("Eintrag {index} erste Zeile\nEintrag {index} zweite Zeile")
            } else {
                format!("Eintrag {index} einzelne Benchmark-Nachricht")
            };
            let plural_translation = if has_escape {
                format!("Eintrag {index} plural mit \\\"Zitat\\\" und \\\\Pfad\\\\")
            } else {
                format!("Eintrag {index} mehrere Benchmark-Nachrichten")
            };

            push_indexed_keyword(&mut out, prefix, "msgstr", 0, &singular_translation);
            push_indexed_keyword(&mut out, prefix, "msgstr", 1, &plural_translation);
        } else {
            let msgstr = if is_multiline {
                format!("Eintrag {index} erste Zeile\nEintrag {index} zweite Zeile")
            } else if has_escape {
                format!("Eintrag {index} mit \\\"Zitat\\\" und \\\\Pfad\\\\")
            } else {
                format!("Eintrag {index} einfache Benchmark-Nachricht")
            };
            push_keyword(&mut out, prefix, "msgstr", &msgstr);
        }

        out.push('\n');
    }

    out
}

fn push_line(out: &mut String, prefix: &str, line: &str) {
    out.push_str(prefix);
    out.push_str(line);
    out.push('\n');
}

fn push_keyword(out: &mut String, prefix: &str, keyword: &str, value: &str) {
    if !value.contains('\n') {
        out.push_str(prefix);
        out.push_str(keyword);
        out.push_str(" \"");
        out.push_str(&escape_po(value));
        out.push_str("\"\n");
        return;
    }

    let mut parts = value.split('\n').peekable();
    out.push_str(prefix);
    out.push_str(keyword);
    out.push_str(" \"\"\n");
    while let Some(part) = parts.next() {
        out.push_str(prefix);
        out.push('"');
        out.push_str(&escape_po(part));
        if parts.peek().is_some() {
            out.push_str("\\n");
        }
        out.push_str("\"\n");
    }
}

fn push_indexed_keyword(out: &mut String, prefix: &str, keyword: &str, index: usize, value: &str) {
    if !value.contains('\n') {
        out.push_str(prefix);
        out.push_str(keyword);
        out.push('[');
        out.push_str(&index.to_string());
        out.push_str("] \"");
        out.push_str(&escape_po(value));
        out.push_str("\"\n");
        return;
    }

    let mut parts = value.split('\n').peekable();
    out.push_str(prefix);
    out.push_str(keyword);
    out.push('[');
    out.push_str(&index.to_string());
    out.push_str("] \"\"\n");
    while let Some(part) = parts.next() {
        out.push_str(prefix);
        out.push('"');
        out.push_str(&escape_po(part));
        if parts.peek().is_some() {
            out.push_str("\\n");
        }
        out.push_str("\"\n");
    }
}

fn escape_po(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
    out
}

fn scan_stats(content: &str) -> FixtureStats {
    let mut stats = FixtureStats {
        entries: 0,
        plural_entries: 0,
        translator_comments: 0,
        extracted_comments: 0,
        references: 0,
        contexts: 0,
        metadata_comments: 0,
        obsolete_entries: 0,
        multiline_entries: 0,
        escaped_entries: 0,
    };

    let mut in_header = true;
    let mut saw_multiline_for_current = false;
    let mut saw_escape_for_current = false;
    for line in content.lines() {
        if line.starts_with("msgid \"\"") && in_header {
            continue;
        }
        if line.starts_with("msgstr \"\"") && in_header {
            continue;
        }
        if in_header && line.starts_with('"') {
            continue;
        }
        if in_header && line.is_empty() {
            in_header = false;
            continue;
        }

        if line.starts_with("msgid ") || line.starts_with("#~ msgid ") {
            if stats.entries > 0 {
                stats.multiline_entries += usize::from(saw_multiline_for_current);
                stats.escaped_entries += usize::from(saw_escape_for_current);
            }
            stats.entries += 1;
            stats.obsolete_entries += usize::from(line.starts_with("#~ "));
            saw_multiline_for_current = line.ends_with("\"\"");
            saw_escape_for_current = line.contains("\\\"") || line.contains("\\\\");
            continue;
        }

        if line.starts_with("msgid_plural ") || line.starts_with("#~ msgid_plural ") {
            stats.plural_entries += 1;
            saw_multiline_for_current |= line.ends_with("\"\"");
            saw_escape_for_current |= line.contains("\\\"") || line.contains("\\\\");
            continue;
        }

        if line.starts_with("# ") || line == "#" || line.starts_with("#~ # ") || line == "#~ #" {
            stats.translator_comments += 1;
            continue;
        }
        if line.starts_with("#. ") || line == "#." || line.starts_with("#~ #. ") || line == "#~ #."
        {
            stats.extracted_comments += 1;
            continue;
        }
        if line.starts_with("#: ") || line.starts_with("#~ #: ") {
            stats.references += 1;
            continue;
        }
        if line.starts_with("#@ ") || line.starts_with("#~ #@ ") {
            stats.metadata_comments += 1;
            continue;
        }
        if line.starts_with("msgctxt ") || line.starts_with("#~ msgctxt ") {
            stats.contexts += 1;
            continue;
        }
        if line.starts_with('"') || line.starts_with("#~ \"") {
            saw_multiline_for_current = true;
            saw_escape_for_current |= line.contains("\\\"") || line.contains("\\\\");
        }
    }

    if stats.entries > 0 {
        stats.multiline_entries += usize::from(saw_multiline_for_current);
        stats.escaped_entries += usize::from(saw_escape_for_current);
    }

    stats
}

#[cfg(test)]
mod tests {
    use ferrocat_icu::parse_icu;
    use ferrocat_po::{PluralEncoding, UpdateCatalogOptions, update_catalog};

    use super::{fixture_by_name, icu_fixture_by_name, merge_fixture_by_name};

    #[test]
    fn builds_mixed_1000_fixture_with_expected_shape() {
        let fixture = fixture_by_name("mixed-1000").expect("fixture exists");
        let stats = fixture.stats();

        assert_eq!(fixture.kind(), "generated");
        assert_eq!(stats.entries, 1000);
        assert_eq!(stats.plural_entries, 100);
        assert!(stats.translator_comments >= 50);
        assert!(stats.extracted_comments >= 40);
        assert!(stats.references >= 300);
        assert!(stats.contexts >= 80);
        assert!(stats.metadata_comments >= 20);
        assert!(stats.obsolete_entries >= 9);
        assert!(stats.multiline_entries >= 30);
        assert!(stats.escaped_entries >= 20);
    }

    #[test]
    fn builds_gettext_compat_fixtures_with_expected_shape() {
        for name in [
            "gettext-ui-de-1000",
            "gettext-commerce-pl-1000",
            "gettext-saas-fr-1000",
            "gettext-content-ar-1000",
        ] {
            let fixture = fixture_by_name(name).expect("gettext fixture exists");
            let stats = fixture.stats();
            assert_eq!(fixture.kind(), "generated-gettext");
            assert_eq!(stats.entries, 1000);
            assert!(stats.plural_entries > 0);
            assert!(stats.references > 0);
            assert!(stats.contexts > 0);
            assert_eq!(stats.metadata_comments, 0);
            assert_eq!(stats.obsolete_entries, 0);
        }
    }

    #[test]
    fn builds_parseable_icu_fixtures() {
        for name in [
            "icu-literal-1000",
            "icu-args-1000",
            "icu-formatters-1000",
            "icu-plural-1000",
            "icu-select-1000",
            "icu-nested-1000",
            "icu-tags-1000",
        ] {
            let fixture = icu_fixture_by_name(name).expect("icu fixture exists");
            assert_eq!(fixture.entries(), 1000);
            for message in fixture.messages().iter().take(32) {
                parse_icu(message).expect("generated icu fixture must parse");
            }
        }
    }

    #[test]
    fn builds_catalog_icu_fixtures() {
        for name in [
            "catalog-icu-light",
            "catalog-icu-heavy",
            "catalog-icu-projectable",
            "catalog-icu-unsupported",
        ] {
            let fixture = merge_fixture_by_name(name).expect("catalog icu fixture exists");
            assert!(fixture.existing_po().contains("msgid"));
            assert_eq!(fixture.extracted_entries(), 1000);
        }
    }

    #[test]
    fn builds_gettext_merge_fixtures() {
        for name in [
            "gettext-ui-de-1000",
            "gettext-commerce-pl-1000",
            "gettext-saas-fr-1000",
            "gettext-content-ar-1000",
        ] {
            let fixture = merge_fixture_by_name(name).expect("gettext merge fixture exists");
            assert!(fixture.existing_po().contains("Plural-Forms"));
            assert_eq!(fixture.existing_entries(), 1000);
            assert!(fixture.extracted_entries() > 0);
        }
    }

    #[test]
    fn update_catalog_accepts_projectable_icu_fixture() {
        let fixture = merge_fixture_by_name("catalog-icu-projectable").expect("fixture exists");
        let result = update_catalog(UpdateCatalogOptions {
            locale: Some("de".to_owned()),
            source_locale: "en".to_owned(),
            input: fixture.api_extracted_messages().to_vec().into(),
            existing: Some(fixture.existing_po().to_owned()),
            plural_encoding: PluralEncoding::Icu,
            ..UpdateCatalogOptions::default()
        })
        .expect("update catalog");

        assert!(result.content.contains("plural"));
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn update_catalog_preserves_diagnostics_for_unsupported_icu_fixture() {
        let fixture = merge_fixture_by_name("catalog-icu-unsupported").expect("fixture exists");
        let result = update_catalog(UpdateCatalogOptions {
            locale: Some("de".to_owned()),
            source_locale: "en".to_owned(),
            input: fixture.api_extracted_messages().to_vec().into(),
            existing: Some(fixture.existing_po().to_owned()),
            plural_encoding: PluralEncoding::Icu,
            ..UpdateCatalogOptions::default()
        })
        .expect("update catalog");

        assert!(!result.diagnostics.is_empty());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "plural.unsupported_icu_projection")
        );
    }
}
