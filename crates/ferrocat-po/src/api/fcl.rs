//! FCL (Ferrocat Catalog Lines) codec — see `docs/fcl-format.md`.
//!
//! A line-oriented, machine-owned catalog encoding optimized for git merge and
//! fast parsing: one entry per line, deterministically sorted by collated
//! `(id, ctxt)`, with minimal escaping (`\n \t \\`). The codec parses and serializes
//! [`CanonicalMessage`] directly, reusing the shared plural/placeholder/MT
//! helpers so it stays equivalent to the catalog's canonical representation.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use super::catalog::{
    CanonicalMessage, CanonicalTranslation, Catalog, parse_origin, split_placeholder_comments,
};
use super::collation::{CollationKey, CollationPrefix, collation_key, collation_prefix};
use super::export::for_each_placeholder_comment;
use super::mt::{
    MachineMetadata, format_ai_descriptor, machine_translation_hash, parse_ai_descriptor,
    validate_machine_metadata,
};
use super::plural::{synthesize_icu_plural, synthesize_icu_plural_source};
use super::{
    ApiError, CatalogOrigin, CatalogSemantics, EffectiveTranslationRef, ObsoleteInfo, RenderOptions,
};
use crate::PoVec;
use crate::scan::{find_byte, find_fcl_escapable_byte, split_once_byte};
use crate::utf8::input_slice_as_str;

const FCL_MAGIC: &str = "%FCL1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FclOrder {
    LegacyBytewise,
    Collated,
}

struct FclHeader {
    locale: Option<String>,
    order: FclOrder,
}

// --- escaping -------------------------------------------------------------

fn escape_into(out: &mut String, value: &str) {
    let bytes = value.as_bytes();
    let Some(first) = find_fcl_escapable_byte(bytes) else {
        out.push_str(value);
        return;
    };

    // Escapable bytes (`\`, `\t`, `\n`) are ASCII, so every index is a UTF-8
    // boundary and the unescaped runs between them can be pushed as whole slices.
    let mut start = 0;
    let mut at = first;
    loop {
        out.push_str(&value[start..at]);
        out.push_str(match bytes[at] {
            b'\\' => "\\\\",
            b'\t' => "\\t",
            _ => "\\n",
        });
        start = at + 1;
        match find_fcl_escapable_byte(&bytes[start..]) {
            Some(relative) => at = start + relative,
            None => {
                out.push_str(&value[start..]);
                return;
            }
        }
    }
}

fn unescape(value: &str) -> Result<Cow<'_, str>, ApiError> {
    let bytes = value.as_bytes();
    let Some(first) = find_byte(b'\\', bytes) else {
        return Ok(Cow::Borrowed(value));
    };

    let mut out = String::with_capacity(value.len());
    let mut start = 0;
    let mut at = first;
    loop {
        // `\` is ASCII, so `value[start..at]` is always a valid UTF-8 slice.
        out.push_str(&value[start..at]);
        match bytes.get(at + 1) {
            Some(b'\\') => out.push('\\'),
            Some(b't') => out.push('\t'),
            Some(b'n') => out.push('\n'),
            Some(_) => {
                let other = value[at + 1..].chars().next().unwrap_or('\u{fffd}');
                return Err(ApiError::InvalidArguments(format!(
                    "invalid FCL escape `\\{other}`"
                )));
            }
            None => {
                return Err(ApiError::InvalidArguments(
                    "dangling `\\` at end of FCL value".to_owned(),
                ));
            }
        }
        start = at + 2;
        match find_byte(b'\\', &bytes[start..]) {
            Some(relative) => at = start + relative,
            None => {
                out.push_str(&value[start..]);
                return Ok(Cow::Owned(out));
            }
        }
    }
}

// --- serialize ------------------------------------------------------------

fn write_tag(out: &mut String, key: &str, value: &str) {
    out.push('\t');
    out.push_str(key);
    out.push('=');
    escape_into(out, value);
}

/// The FCL `id` column for a message: the source msgid for singular messages,
/// or the ICU plural string synthesized from the source forms for plurals
/// (matching how the catalog round-trips ICU-native plurals).
fn fcl_id(message: &CanonicalMessage) -> Cow<'_, str> {
    match &message.translation {
        CanonicalTranslation::Singular { .. } => Cow::Borrowed(message.msgid.as_str()),
        CanonicalTranslation::Plural {
            source, variable, ..
        } => Cow::Owned(synthesize_icu_plural_source(variable, source)),
    }
}

/// The FCL `target` column: the translation value, or the synthesized ICU plural.
fn fcl_target(message: &CanonicalMessage) -> Cow<'_, str> {
    match &message.translation {
        CanonicalTranslation::Singular { value } => Cow::Borrowed(value.as_str()),
        CanonicalTranslation::Plural {
            translation_by_category,
            variable,
            ..
        } => Cow::Owned(synthesize_icu_plural(variable, translation_by_category)),
    }
}

fn write_entry(out: &mut String, message: &CanonicalMessage, render: &RenderOptions<'_>) {
    escape_into(out, &fcl_id(message));
    out.push('\t');
    escape_into(out, message.msgctxt.as_deref().unwrap_or(""));
    out.push('\t');
    escape_into(out, &fcl_target(message));

    // Canonical tag order: r (sorted), c, tc, f (sorted), o, mt.*
    if render.include_origins {
        // References are file-only; sort for determinism and drop duplicates that
        // distinct origins can now collapse to.
        let refs = message
            .origins
            .iter()
            .map(|origin| (origin.file.as_str(), origin.scope.as_deref()))
            .collect::<BTreeSet<_>>();
        let mut reference = String::new();
        for (file, scope) in refs {
            reference.clear();
            reference.push_str(file);
            if let Some(scope) = scope {
                reference.push('#');
                reference.push_str(scope);
            }
            write_tag(out, "r", &reference);
        }
    }

    // Extracted comments plus the placeholder comments folded back in, matching
    // the catalog's comment round-trip.
    for comment in &message.comments {
        write_tag(out, "c", comment);
    }
    for_each_placeholder_comment(
        &message.comments,
        &message.placeholders,
        &render.print_placeholders_in_comments,
        |comment| {
            write_tag(out, "c", comment);
        },
    );

    if let Some(info) = &message.obsolete {
        match &info.since {
            Some(since) => write_tag(out, "o", since),
            None => out.push_str("\to"),
        }
    }

    // The lock/ai block is only emitted when the lock still matches the value
    // (stale machine metadata is dropped).
    if let Some(machine) = message.machine.as_ref() {
        let translation_ref = match &message.translation {
            CanonicalTranslation::Singular { value } => EffectiveTranslationRef::Singular(value),
            CanonicalTranslation::Plural {
                translation_by_category,
                ..
            } => EffectiveTranslationRef::Plural(translation_by_category),
        };
        if validate_machine_metadata(machine).is_ok()
            && machine.lock == machine_translation_hash(translation_ref)
        {
            write_tag(out, "lock", &machine.lock);
            if let Some(ai) = &machine.ai {
                write_tag(out, "ai", &format_ai_descriptor(ai));
            }
        }
    }
    out.push('\n');
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct FclCollatedKey {
    message: CollationKey,
    context: CollationKey,
    raw_message: String,
    raw_context: Option<String>,
    obsolete: bool,
}

fn fcl_collated_key(message: &CanonicalMessage) -> FclCollatedKey {
    let id = fcl_id(message);
    FclCollatedKey {
        message: collation_key(&id),
        context: collation_key(message.msgctxt.as_deref().unwrap_or("")),
        raw_message: id.into_owned(),
        raw_context: message.msgctxt.clone(),
        obsolete: message.obsolete.is_some(),
    }
}

/// Applies the same packed-prefix strategy as the shared catalog sort, using
/// FCL's serialized `id` column so plural IDs validate after a round trip.
fn sort_fcl_messages_collated(messages: &mut [&CanonicalMessage]) {
    if messages.len() < 2 {
        return;
    }

    let prefixes: Vec<CollationPrefix> = messages
        .iter()
        .map(|message| collation_prefix(&fcl_id(message)))
        .collect();
    let mut order: Vec<usize> = (0..messages.len()).collect();
    order.sort_unstable_by_key(|&index| prefixes[index]);

    let mut start = 0;
    while start < order.len() {
        let prefix = prefixes[order[start]];
        let mut end = start + 1;
        while end < order.len() && prefixes[order[end]] == prefix {
            end += 1;
        }
        if end - start > 1 {
            let mut run: Vec<(FclCollatedKey, usize)> = order[start..end]
                .iter()
                .map(|&index| (fcl_collated_key(messages[index]), index))
                .collect();
            run.sort_by(|left, right| left.0.cmp(&right.0));
            for (slot, (_, index)) in order[start..end].iter_mut().zip(run) {
                *slot = index;
            }
        }
        start = end;
    }

    let mut destination = vec![0_usize; order.len()];
    for (position, &source) in order.iter().enumerate() {
        destination[source] = position;
    }
    for position in 0..destination.len() {
        while destination[position] != position {
            let target = destination[position];
            messages.swap(position, target);
            destination.swap(position, target);
        }
    }
}

/// Renders an internal [`Catalog`] as FCL text.
///
/// FCL always declares and uses collated `(id, ctxt)` order because line order
/// is part of its storage invariant. PO-specific origin ordering does not alter
/// the FCL representation.
pub(super) fn stringify_catalog_fcl(
    catalog: &Catalog,
    locale: Option<&str>,
    source_locale: &str,
    render: &RenderOptions<'_>,
) -> String {
    // Reserve a rough output budget up front so growth does not repeatedly
    // reallocate; the estimate only needs to be in the right ballpark.
    let mut out = String::with_capacity(catalog.messages.len() * 128 + 64);
    out.push_str(FCL_MAGIC);
    write_tag(&mut out, "source", source_locale);
    if let Some(locale) = locale {
        write_tag(&mut out, "locale", locale);
    }
    write_tag(&mut out, "order", "collated");
    out.push('\n');

    let mut messages: Vec<&CanonicalMessage> = catalog.messages.iter().collect();
    sort_fcl_messages_collated(&mut messages);
    for message in messages {
        write_entry(&mut out, message, render);
    }
    out
}

// --- parse ----------------------------------------------------------------

fn is_conflict_marker(line: &str) -> bool {
    line.starts_with("<<<<<<<") || line.starts_with("=======") || line.starts_with(">>>>>>>")
}

fn parse_header(line: &str, source_locale: &str) -> Result<FclHeader, ApiError> {
    let mut fields = SplitTab::new(line.as_bytes());
    if fields.next() != Some(FCL_MAGIC.as_bytes()) {
        return Err(ApiError::InvalidArguments(
            "FCL catalog must start with the `%FCL1` header".to_owned(),
        ));
    }
    let mut locale = None;
    let mut declared_source: Option<String> = None;
    let mut order = FclOrder::LegacyBytewise;
    let mut declared_order = false;
    for tag in fields {
        let (key, value) = split_once_byte(tag, b'=').ok_or_else(|| {
            ApiError::InvalidArguments(format!(
                "invalid FCL header tag {:?}",
                input_slice_as_str(tag)
            ))
        })?;
        let value = unescape(input_slice_as_str(value))?.into_owned();
        match key {
            b"source" => declared_source = Some(value),
            b"locale" => locale = Some(value),
            b"order" => {
                if declared_order {
                    return Err(ApiError::InvalidArguments(
                        "duplicate FCL header key `order`".to_owned(),
                    ));
                }
                declared_order = true;
                order = match value.as_str() {
                    "collated" => FclOrder::Collated,
                    other => {
                        return Err(ApiError::InvalidArguments(format!(
                            "unknown FCL order {other:?}"
                        )));
                    }
                };
            }
            other => {
                return Err(ApiError::InvalidArguments(format!(
                    "unknown FCL header key {:?}",
                    input_slice_as_str(other)
                )));
            }
        }
    }
    if let Some(declared) = &declared_source
        && declared != source_locale
    {
        return Err(ApiError::InvalidArguments(format!(
            "FCL source {declared:?} did not match requested source_locale {source_locale:?}"
        )));
    }
    Ok(FclHeader { locale, order })
}

/// Parses one FCL entry line directly into a [`CanonicalMessage`], building the
/// owned fields in a single pass without an intermediate record.
fn parse_entry(line: &str) -> Result<CanonicalMessage, ApiError> {
    let mut fields = SplitTab::new(line.as_bytes());
    let msgid = unescape(field(&mut fields, "id")?)?.into_owned();
    let ctx_raw = unescape(field(&mut fields, "ctxt")?)?.into_owned();
    let value = unescape(field(&mut fields, "target")?)?.into_owned();

    let msgctxt = (!ctx_raw.is_empty()).then_some(ctx_raw);
    let mut origins: PoVec<CatalogOrigin> = PoVec::new();
    let mut raw_comments: Vec<Cow<'_, str>> = Vec::new();
    let mut obsolete: Option<ObsoleteInfo> = None;
    let mut lock = None;
    let mut ai = None;
    let mut last_tag_rank = 0_u8;

    for tag in fields {
        if tag == b"o" {
            validate_tag_order(&mut last_tag_rank, 2, "o")?;
            set_obsolete(&mut obsolete, None)?;
            continue;
        }
        let (key, raw_value) = split_once_byte(tag, b'=').ok_or_else(|| {
            ApiError::InvalidArguments(format!("invalid FCL tag {:?}", input_slice_as_str(tag)))
        })?;
        // Keep the unescaped value borrowed; only allocate (`into_owned`) for tags
        // whose value is stored.
        let value = unescape(input_slice_as_str(raw_value))?;
        match key {
            b"r" => {
                validate_tag_order(&mut last_tag_rank, 0, "r")?;
                origins.push(parse_origin(value));
            }
            b"c" => {
                validate_tag_order(&mut last_tag_rank, 1, "c")?;
                raw_comments.push(value);
            }
            b"o" => {
                validate_tag_order(&mut last_tag_rank, 2, "o")?;
                set_obsolete(&mut obsolete, Some(value.into_owned()))?;
            }
            b"lock" => {
                validate_tag_order(&mut last_tag_rank, 3, "lock")?;
                if lock.is_some() {
                    return Err(ApiError::InvalidArguments(
                        "duplicate FCL tag `lock`".to_owned(),
                    ));
                }
                lock = Some(value.into_owned());
            }
            b"ai" => {
                validate_tag_order(&mut last_tag_rank, 4, "ai")?;
                if ai.is_some() {
                    return Err(ApiError::InvalidArguments(
                        "duplicate FCL tag `ai`".to_owned(),
                    ));
                }
                ai = Some(parse_ai_descriptor(&value));
            }
            other => {
                return Err(ApiError::InvalidArguments(format!(
                    "unknown FCL tag key {:?}",
                    input_slice_as_str(other)
                )));
            }
        }
    }

    let machine = if let Some(lock) = lock {
        let metadata = MachineMetadata { lock, ai };
        validate_machine_metadata(&metadata)?;
        Some(metadata)
    } else if ai.is_some() {
        return Err(ApiError::InvalidArguments(
            "FCL `ai` tag requires a `lock` tag".to_owned(),
        ));
    } else {
        None
    };

    let (comments, placeholders) = split_placeholder_comments(raw_comments);

    Ok(CanonicalMessage {
        msgid,
        msgctxt,
        translation: CanonicalTranslation::Singular { value },
        comments,
        origins,
        placeholders,
        obsolete,
        machine,
    })
}

fn validate_tag_order(last_rank: &mut u8, rank: u8, key: &str) -> Result<(), ApiError> {
    if rank < *last_rank {
        return Err(ApiError::InvalidArguments(format!(
            "FCL tag `{key}` is out of canonical order"
        )));
    }
    *last_rank = rank;
    Ok(())
}

/// Sets the obsolete state from a bare `o` (`since = None`) or `o=<date>` tag,
/// rejecting a second `o` tag on the same entry.
fn set_obsolete(
    obsolete: &mut Option<ObsoleteInfo>,
    since: Option<String>,
) -> Result<(), ApiError> {
    if obsolete.is_some() {
        return Err(ApiError::InvalidArguments(
            "duplicate FCL tag `o`".to_owned(),
        ));
    }
    *obsolete = Some(ObsoleteInfo { since });
    Ok(())
}

fn field<'a>(fields: &mut impl Iterator<Item = &'a [u8]>, name: &str) -> Result<&'a str, ApiError> {
    fields
        .next()
        .map(input_slice_as_str)
        .ok_or_else(|| ApiError::InvalidArguments(format!("FCL entry is missing the {name} field")))
}

/// Splits a line into tab-delimited fields over bytes (memchr-backed), matching
/// `str::split('\t')` semantics including a trailing empty field.
struct SplitTab<'a> {
    rest: Option<&'a [u8]>,
}

impl<'a> SplitTab<'a> {
    #[inline]
    fn new(bytes: &'a [u8]) -> Self {
        Self { rest: Some(bytes) }
    }
}

impl<'a> Iterator for SplitTab<'a> {
    type Item = &'a [u8];

    #[inline]
    fn next(&mut self) -> Option<&'a [u8]> {
        let rest = self.rest?;
        match find_byte(b'\t', rest) {
            Some(index) => {
                self.rest = Some(&rest[index + 1..]);
                Some(&rest[..index])
            }
            None => {
                self.rest = None;
                Some(rest)
            }
        }
    }
}

/// Parses FCL text into the internal [`Catalog`] representation.
pub(super) fn parse_catalog_to_internal_fcl(
    content: &str,
    locale_override: Option<&str>,
    source_locale: &str,
    semantics: CatalogSemantics,
    _strict: bool,
) -> Result<Catalog, ApiError> {
    if semantics != CatalogSemantics::IcuNative {
        return Err(ApiError::Unsupported(
            "CatalogSemantics::GettextCompat is not supported for FCL catalogs".to_owned(),
        ));
    }
    super::validate_source_locale(source_locale)?;

    let mut lines = content.lines().enumerate();
    let header = lines.next().map(|(_, line)| line).ok_or_else(|| {
        ApiError::InvalidArguments("FCL catalog must start with the `%FCL1` header".to_owned())
    })?;
    let parsed_header = parse_header(header, source_locale)?;
    let locale = locale_override.map(str::to_owned).or(parsed_header.locale);

    // The entry count is at most the newline count; over-reserving by the header
    // and any blank lines only saves reallocations.
    let estimated_entries = memchr::memchr_iter(b'\n', content.as_bytes()).count();
    let mut messages: Vec<CanonicalMessage> = Vec::with_capacity(estimated_entries);
    let mut previous_collated_key: Option<FclCollatedKey> = None;
    for (index, line) in lines {
        if line.is_empty() {
            continue;
        }
        if is_conflict_marker(line) {
            return Err(ApiError::InvalidArguments(format!(
                "git conflict marker in FCL catalog on line {}",
                index + 1
            )));
        }
        let message = parse_entry(line).map_err(|error| {
            ApiError::InvalidArguments(format!("invalid FCL entry on line {}: {error}", index + 1))
        })?;
        // Legacy bytewise and declared collated order both keep equal identities
        // adjacent, so the previous entry detects duplicates without a set.
        if let Some(previous) = messages.last() {
            let identity_order = (message.msgid.as_str(), message.msgctxt.as_deref())
                .cmp(&(previous.msgid.as_str(), previous.msgctxt.as_deref()));
            if identity_order.is_eq() {
                return Err(ApiError::Conflict(format!(
                    "duplicate FCL entry for id {:?} and context {:?}",
                    message.msgid, message.msgctxt
                )));
            }
            if parsed_header.order == FclOrder::LegacyBytewise && identity_order.is_lt() {
                return Err(ApiError::InvalidArguments(format!(
                    "FCL entries must be sorted by (id, ctxt); line {} is out of order",
                    index + 1
                )));
            }
        }
        if parsed_header.order == FclOrder::Collated {
            let key = fcl_collated_key(&message);
            if previous_collated_key
                .as_ref()
                .is_some_and(|previous| key < *previous)
            {
                return Err(ApiError::InvalidArguments(format!(
                    "FCL entries must follow the declared collated order; line {} is out of order",
                    index + 1
                )));
            }
            previous_collated_key = Some(key);
        }
        messages.push(message);
    }

    Ok(Catalog {
        locale,
        headers: BTreeMap::new(),
        file_comments: Vec::new(),
        file_extracted_comments: Vec::new(),
        messages,
        diagnostics: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::{FCL_MAGIC, parse_catalog_to_internal_fcl, stringify_catalog_fcl, unescape};
    use crate::api::CatalogSemantics;
    use crate::api::types::RenderOptions;

    fn roundtrip(text: &str) -> String {
        let catalog =
            parse_catalog_to_internal_fcl(text, None, "en", CatalogSemantics::IcuNative, false)
                .expect("parse FCL");
        stringify_catalog_fcl(
            &catalog,
            catalog.locale.as_deref(),
            "en",
            &RenderOptions::default(),
        )
    }

    #[test]
    fn escaping_roundtrips_control_chars() {
        let raw = "a\tb\nc\\d";
        let mut escaped = String::new();
        super::escape_into(&mut escaped, raw);
        assert_eq!(escaped, "a\\tb\\nc\\\\d");
        assert_eq!(unescape(&escaped).expect("unescape"), raw);
    }

    #[test]
    fn collated_text_is_a_fixpoint() {
        // Header + two collated entries with tabs/newlines escaped and canonical
        // tag order. Parsing then re-serializing must reproduce it byte-for-byte.
        // The MT block is only retained when its hash matches the translation
        // (stale-protection), so compute the real hash for the target.
        let hash = crate::machine_translation_hash(crate::EffectiveTranslationRef::Singular(
            "Hallo {name}",
        ));
        let text = format!(
            "{FCL_MAGIC}\tsource=en\tlocale=de\torder=collated\n\
             greeting\t\tHallo {{name}}\tr=src/a.tsx#Greeting\tlock={hash}\tai=openai/gpt-5.5:0.88\n\
             tabbed\tmenu\tWert mit\\tTab\n"
        );
        assert_eq!(roundtrip(&text), text);
    }

    #[test]
    fn rejects_conflict_markers_and_unknown_tags() {
        let conflict = format!("{FCL_MAGIC}\tsource=en\n<<<<<<< HEAD\n");
        assert!(
            parse_catalog_to_internal_fcl(
                &conflict,
                None,
                "en",
                CatalogSemantics::IcuNative,
                false
            )
            .is_err()
        );

        let unknown = format!("{FCL_MAGIC}\tsource=en\nid\t\tvalue\tzz=1\n");
        assert!(
            parse_catalog_to_internal_fcl(&unknown, None, "en", CatalogSemantics::IcuNative, false)
                .is_err()
        );
    }

    #[test]
    fn accepts_legacy_bytewise_sorted_entries() {
        // Legacy FCL without an order tag remains readable when bytewise-sorted.
        // Re-serialization upgrades it to the declared collated contract.
        let text = format!("{FCL_MAGIC}\tsource=en\nalpha\t\tA\nalpha\tmenu\tA2\nbeta\t\tB\n");
        let sorted = roundtrip(&text);
        assert!(sorted.starts_with(&format!("{FCL_MAGIC}\tsource=en\torder=collated\n")));
        let ids: Vec<&str> = sorted
            .lines()
            .skip(1)
            .map(|line| line.split('\t').next().unwrap_or_default())
            .collect();
        assert_eq!(ids, ["alpha", "alpha", "beta"]);
    }

    #[test]
    fn rejects_out_of_order_and_duplicate_entries() {
        // FCL is a canonical, machine-owned format: the reader enforces the
        // sorted-and-unique invariant rather than silently re-sorting.
        let out_of_order = format!("{FCL_MAGIC}\tsource=en\nbeta\t\tB\nalpha\t\tA\n");
        assert!(
            parse_catalog_to_internal_fcl(
                &out_of_order,
                None,
                "en",
                CatalogSemantics::IcuNative,
                false
            )
            .is_err()
        );

        let duplicate = format!("{FCL_MAGIC}\tsource=en\nalpha\t\tA\nalpha\t\tB\n");
        assert!(
            parse_catalog_to_internal_fcl(
                &duplicate,
                None,
                "en",
                CatalogSemantics::IcuNative,
                false
            )
            .is_err()
        );
    }

    fn parse_err(text: &str) -> bool {
        parse_catalog_to_internal_fcl(text, None, "en", CatalogSemantics::IcuNative, false).is_err()
    }

    #[test]
    fn collapses_duplicate_file_references_on_serialize() {
        // References are file-only now, so two `r=` tags for the same file are
        // redundant; re-serializing normalizes them to a single reference.
        let text = format!("{FCL_MAGIC}\tsource=en\nid\t\tv\tr=src/a.rs\tr=src/a.rs\n");
        let rendered = roundtrip(&text);
        assert_eq!(rendered.matches("\tr=src/a.rs").count(), 1);
    }

    #[test]
    fn serializes_scoped_references_and_generated_placeholder_comments() {
        let message = super::CanonicalMessage {
            msgid: "Hello {0}".to_owned(),
            msgctxt: None,
            translation: super::CanonicalTranslation::Singular {
                value: "Hallo {0}".to_owned(),
            },
            comments: vec!["Translator note".to_owned()],
            origins: vec![
                super::CatalogOrigin {
                    file: "src/app.rs".to_owned(),
                    scope: Some("Greeting".to_owned()),
                },
                super::CatalogOrigin {
                    file: "src/app.rs".to_owned(),
                    scope: Some("Greeting".to_owned()),
                },
            ]
            .into(),
            placeholders: std::collections::BTreeMap::from([(
                "0".to_owned(),
                vec!["user\nname".to_owned()],
            )]),
            obsolete: None,
            machine: None,
        };
        let catalog = super::Catalog {
            locale: Some("de".to_owned()),
            headers: std::collections::BTreeMap::new(),
            file_comments: Vec::new(),
            file_extracted_comments: Vec::new(),
            messages: vec![message],
            diagnostics: Vec::new(),
        };

        let text = stringify_catalog_fcl(&catalog, Some("de"), "en", &RenderOptions::default());

        assert_eq!(text.matches("\tr=src/app.rs#Greeting").count(), 1);
        assert!(text.contains("\tc=Translator note"));
        assert!(text.contains("\tc=placeholder {0}: user name"));
    }

    #[test]
    fn roundtrips_comments_obsolete_and_escapes() {
        // Covers the c=/o serialize branches and the \t \n \\ escape paths in both
        // directions on a single entry.
        let text = format!(
            "{FCL_MAGIC}\tsource=en\torder=collated\n\
             a.tab\\there\t\tWert\\nZeile\\\\back\tc=note\to\n"
        );
        assert_eq!(roundtrip(&text), text);
    }

    #[test]
    fn roundtrips_reference_without_line_number() {
        // Covers the line-less origin path on both parse and serialize.
        let text = format!("{FCL_MAGIC}\tsource=en\torder=collated\nid\t\tv\tr=README\n");
        assert_eq!(roundtrip(&text), text);
    }

    #[test]
    fn roundtrips_dated_obsolete() {
        // Covers the valued `o=<since>` serialize and parse branches.
        let text = format!("{FCL_MAGIC}\tsource=en\torder=collated\nid\t\tv\to=2026-06-30\n");
        assert_eq!(roundtrip(&text), text);
    }

    #[test]
    fn rejects_invalid_and_dangling_escapes() {
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\nid\t\tbad\\xescape\n"
        )));
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\nid\t\ttrailing\\\n"
        )));
    }

    #[test]
    fn rejects_malformed_headers() {
        assert!(parse_err("nope\nid\t\tv\n")); // missing %FCL1 magic
        assert!(parse_err(&format!("{FCL_MAGIC}\tbogus=x\nid\t\tv\n"))); // unknown header key
        assert!(parse_err(&format!("{FCL_MAGIC}\tnoeq\nid\t\tv\n"))); // header tag without '='
        assert!(parse_err(&format!("{FCL_MAGIC}\tsource=de\nid\t\tv\n"))); // source mismatch vs "en"
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\torder=unknown\nid\t\tv\n"
        )));
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\torder=collated\torder=collated\nid\t\tv\n"
        )));
        assert!(parse_err("")); // no header at all
    }

    #[test]
    fn rejects_entries_outside_declared_collated_order() {
        // Bytewise order puts '<' before '{'; CLDR root order is the reverse.
        let text = format!(
            "{FCL_MAGIC}\tsource=en\torder=collated\n\
             <0>Continue</0>\t\tmarkup\n\
             {{count, plural, one {{#}} other {{#}}}}\t\tplaceholder\n"
        );
        assert!(parse_err(&text));
    }

    #[test]
    fn rejects_malformed_entries() {
        assert!(parse_err(&format!("{FCL_MAGIC}\tsource=en\njustid\n"))); // missing ctxt/target
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\nid\t\tv\tbogus\n"
        ))); // tag without `=`
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\nid\t\tv\tzz=1\n"
        ))); // unknown tag key
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\nid\t\tv\tai=example/mt:0.5\n"
        ))); // `ai` requires a `lock`
    }

    #[test]
    fn rejects_duplicate_singleton_and_out_of_order_tags() {
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\nid\t\tv\to\to\n"
        )));
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\nid\t\tv\tlock=a\tlock=b\n"
        ))); // duplicate `lock`
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\nid\t\tv\tlock=a\tai=x\tai=y\n"
        ))); // duplicate `ai`
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\nid\t\tv\to\tc=late-comment\n"
        ))); // `c` after `o` is out of canonical order
    }

    #[test]
    fn serializes_plural_messages_as_synthesized_icu() {
        // FCL parsing always yields singular messages (the ICU plural lives inside
        // the string), so the plural serialize arms are only reachable when a
        // catalog with gettext-style plural forms (e.g. parsed from PO) is written
        // as FCL. Build one directly and exercise that path.
        let translation_by_category = std::collections::BTreeMap::from([
            ("one".to_owned(), "{count} Datei".to_owned()),
            ("other".to_owned(), "{count} Dateien".to_owned()),
        ]);
        let hash = super::machine_translation_hash(super::EffectiveTranslationRef::Plural(
            &translation_by_category,
        ));
        let message = super::CanonicalMessage {
            msgid: "file.count".to_owned(),
            msgctxt: None,
            translation: super::CanonicalTranslation::Plural {
                source: crate::PluralSource {
                    one: Some("{count} file".to_owned()),
                    other: "{count} files".to_owned(),
                },
                translation_by_category,
                variable: "count".to_owned(),
            },
            comments: Vec::new(),
            origins: super::PoVec::new(),
            placeholders: std::collections::BTreeMap::new(),
            obsolete: None,
            machine: Some(super::MachineMetadata {
                lock: hash,
                ai: Some(crate::AiProvenance {
                    model: "example/mt".to_owned(),
                    confidence: Some(0.90),
                }),
            }),
        };
        let catalog = super::Catalog {
            locale: Some("de".to_owned()),
            headers: std::collections::BTreeMap::new(),
            file_comments: Vec::new(),
            file_extracted_comments: Vec::new(),
            messages: vec![message],
            diagnostics: Vec::new(),
        };

        let text = stringify_catalog_fcl(&catalog, Some("de"), "en", &RenderOptions::default());
        assert!(text.starts_with(&format!(
            "{FCL_MAGIC}\tsource=en\tlocale=de\torder=collated\n"
        )));
        // Both id and target columns are synthesized ICU plural strings, and the
        // lock/ai block survives because the lock matches the plural translation.
        assert!(text.contains("{count, plural,"));
        assert!(text.contains("{count} Dateien"));
        assert!(text.contains("ai=example/mt:0.9"));

        // The synthesized ICU string round-trips back through the reader.
        let reparsed =
            parse_catalog_to_internal_fcl(&text, None, "en", CatalogSemantics::IcuNative, false)
                .expect("parse plural FCL");
        assert_eq!(reparsed.messages.len(), 1);
    }

    #[test]
    fn drops_stale_machine_translation_on_serialize() {
        // A valid-format hash that belongs to different text: validation passes but
        // the hash no longer matches, so the MT block must not be written.
        let stale = super::machine_translation_hash(super::EffectiveTranslationRef::Singular(
            "some other translation",
        ));
        let message = super::CanonicalMessage {
            msgid: "Hello".to_owned(),
            msgctxt: None,
            translation: super::CanonicalTranslation::Singular {
                value: "Hallo".to_owned(),
            },
            comments: Vec::new(),
            origins: super::PoVec::new(),
            placeholders: std::collections::BTreeMap::new(),
            obsolete: None,
            machine: Some(super::MachineMetadata {
                lock: stale,
                ai: Some(crate::AiProvenance {
                    model: "example/mt".to_owned(),
                    confidence: Some(0.90),
                }),
            }),
        };
        let catalog = super::Catalog {
            locale: Some("de".to_owned()),
            headers: std::collections::BTreeMap::new(),
            file_comments: Vec::new(),
            file_extracted_comments: Vec::new(),
            messages: vec![message],
            diagnostics: Vec::new(),
        };

        let text = stringify_catalog_fcl(&catalog, Some("de"), "en", &RenderOptions::default());
        assert!(text.contains("Hello\t\tHallo"));
        assert!(!text.contains("mt."));
    }

    #[test]
    fn rejects_gettext_compat_semantics() {
        let text = format!("{FCL_MAGIC}\tsource=en\nid\t\tv\n");
        assert!(
            parse_catalog_to_internal_fcl(
                &text,
                None,
                "en",
                CatalogSemantics::GettextCompat,
                false
            )
            .is_err()
        );
    }

    #[test]
    fn locale_override_wins_and_blank_lines_are_skipped() {
        let text = format!("{FCL_MAGIC}\tsource=en\tlocale=de\n\nalpha\t\tA\n\nbeta\t\tB\n");
        let catalog = parse_catalog_to_internal_fcl(
            &text,
            Some("fr"),
            "en",
            CatalogSemantics::IcuNative,
            false,
        )
        .expect("parse");
        assert_eq!(catalog.locale.as_deref(), Some("fr"));
        assert_eq!(catalog.messages.len(), 2);
    }
}
