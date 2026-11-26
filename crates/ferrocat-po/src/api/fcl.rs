//! FCL (Ferrocat Catalog Lines) codec — see `docs/fcl-format.md`.
//!
//! A line-oriented, machine-owned catalog encoding optimized for git merge and
//! fast parsing: one entry per line, deterministically sorted by collated
//! `(id, ctxt)`, with minimal escaping (`\n \r \t \\`). The codec parses and serializes
//! [`CanonicalMessage`] directly, reusing the shared plural/placeholder/MT
//! helpers so it stays equivalent to the catalog's canonical representation.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::BTreeMap;

use rustc_hash::{FxHashMap, FxHashSet};

use super::catalog::{
    CanonicalMessage, CanonicalTranslation, Catalog, OpaqueCapture, OpaqueMetadata, parse_origin,
    parse_placeholder_comment,
};
use super::collation::{
    CollationPrefix, MAX_CONTINUATION_ROUNDS, PREFIX_BYTES, collect_colliding_runs,
    storage_collation_prefix, storage_collation_prefix_from,
};
use super::export::for_each_placeholder_comment_with_scratch;
use super::helpers::dedupe_placeholders;
use super::mt::{
    MachineMetadata, format_ai_descriptor, machine_translation_hash, parse_ai_descriptor,
    validate_machine_metadata,
};
use super::plural::{synthesize_icu_plural, synthesize_icu_plural_source};
use super::{
    ApiError, CatalogOrigin, CatalogSemantics, EffectiveTranslationRef, ObsoleteInfo, RenderOptions,
};
use crate::PoVec;
use crate::collation::{CollationKey, storage_collation_key};
use crate::scan::{find_byte, find_fcl_escapable_byte, split_once_byte};
use crate::utf8::input_slice_as_str;

const FCL_MAGIC: &str = "%FCL1";
const FCL2_MAGIC: &str = "%FCL2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FclVersion {
    V1,
    V2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FclOrder {
    LegacyBytewise,
    Collated,
}

struct FclHeader {
    version: FclVersion,
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

    // Escapable bytes (`\`, `\t`, `\n`, `\r`) are ASCII, so every index is a UTF-8
    // boundary and the unescaped runs between them can be pushed as whole slices.
    let mut start = 0;
    let mut at = first;
    loop {
        out.push_str(&value[start..at]);
        out.push_str(match bytes[at] {
            b'\\' => "\\\\",
            b'\t' => "\\t",
            b'\r' => "\\r",
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

fn unescape(value: &str, version: FclVersion) -> Result<Cow<'_, str>, ApiError> {
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
            Some(b'r') if version == FclVersion::V2 => out.push('\r'),
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

#[inline]
fn unescape_maybe(
    value: &str,
    version: FclVersion,
    has_backslash: bool,
) -> Result<Cow<'_, str>, ApiError> {
    if !has_backslash {
        return Ok(Cow::Borrowed(value));
    }
    unescape(value, version)
}

#[inline]
fn validate_escapes_maybe(
    value: &str,
    version: FclVersion,
    has_backslash: bool,
) -> Result<(), ApiError> {
    if !has_backslash {
        return Ok(());
    }
    validate_escapes(value, version)
}

fn validate_escapes(value: &str, version: FclVersion) -> Result<(), ApiError> {
    let bytes = value.as_bytes();
    let mut start = 0;
    while let Some(relative) = find_byte(b'\\', &bytes[start..]) {
        let at = start + relative;
        match bytes.get(at + 1) {
            Some(b'\\' | b't' | b'n') => start = at + 2,
            Some(b'r') if version == FclVersion::V2 => start = at + 2,
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
    }
    Ok(())
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
        CanonicalTranslation::Plural(plural) => Cow::Owned(synthesize_icu_plural_source(
            &plural.variable,
            &plural.source,
        )),
    }
}

/// The FCL `target` column: the translation value, or the synthesized ICU plural.
fn fcl_target(message: &CanonicalMessage) -> Cow<'_, str> {
    match &message.translation {
        CanonicalTranslation::Singular { value } => Cow::Borrowed(value.as_str()),
        CanonicalTranslation::Plural(plural) => Cow::Owned(synthesize_icu_plural(
            &plural.variable,
            &plural.translation_by_category,
        )),
    }
}

fn write_origin_tag(out: &mut String, file: &str, scope: Option<&str>) {
    out.push_str("\tr=");
    escape_into(out, file);
    if let Some(scope) = scope {
        out.push('#');
        escape_into(out, scope);
    }
}

fn write_entry<'a>(
    out: &mut String,
    message: &'a CanonicalMessage,
    render: &RenderOptions<'_>,
    origins_scratch: &mut Vec<(&'a str, Option<&'a str>)>,
    placeholder_scratch: &mut String,
) {
    escape_into(out, &fcl_id(message));
    out.push('\t');
    match &message.msgctxt {
        None => out.push('n'),
        Some(context) => {
            out.push('s');
            escape_into(out, context);
        }
    }
    out.push('\t');
    escape_into(out, &fcl_target(message));

    // Canonical tag order: r (sorted), c, tc, f, o, lock, ai.
    if render.include_origins {
        // References are file-only; sort for determinism and drop duplicates that
        // distinct origins can now collapse to.
        match message.origins.as_slice() {
            [] => {}
            [origin] => write_origin_tag(out, &origin.file, origin.scope.as_deref()),
            origins => {
                origins_scratch.clear();
                origins_scratch.extend(
                    origins
                        .iter()
                        .map(|origin| (origin.file.as_str(), origin.scope.as_deref())),
                );
                origins_scratch.sort_unstable();
                origins_scratch.dedup();
                for &(file, scope) in origins_scratch.iter() {
                    write_origin_tag(out, file, scope);
                }
            }
        }
    }

    // Extracted comments plus the placeholder comments folded back in, matching
    // the catalog's comment round-trip.
    for comment in &message.comments {
        write_tag(out, "c", comment);
    }
    for_each_placeholder_comment_with_scratch(
        &message.comments,
        &message.placeholders,
        &render.print_placeholders_in_comments,
        placeholder_scratch,
        |comment| {
            write_tag(out, "c", comment);
        },
    );

    // Translator-owned notes and flags stay separate from `c` and keep their
    // stored order. Export treats them as opaque even though the dedicated
    // review projection observes exact `fuzzy`.
    for comment in message.translator_comments() {
        write_tag(out, "tc", comment);
    }
    for flag in message.flags() {
        write_tag(out, "f", flag);
    }

    if let Some(info) = &message.obsolete {
        match &info.since {
            Some(since) => write_tag(out, "o", since),
            None => out.push_str("\to"),
        }
    }

    // The lock/ai block is only emitted when the lock still matches the value
    // (stale machine metadata is dropped).
    if let Some(machine) = message.machine.as_deref() {
        let translation_ref = match &message.translation {
            CanonicalTranslation::Singular { value } => EffectiveTranslationRef::Singular(value),
            CanonicalTranslation::Plural(plural) => {
                EffectiveTranslationRef::Plural(&plural.translation_by_category)
            }
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
}

fn fcl_collated_key(id: &str, message: &CanonicalMessage) -> FclCollatedKey {
    FclCollatedKey {
        message: storage_collation_key(id),
        context: storage_collation_key(message.msgctxt.as_deref().unwrap_or("")),
    }
}

/// Compares two FCL2 entries whose initial `storage_collation_prefix` tied,
/// using continuation prefixes before falling back to full collation keys
/// without cloning `msgid` or `msgctxt`.
fn cmp_fcl_collated_on_prefix_tie(
    left: &CanonicalMessage,
    right: &CanonicalMessage,
    prefix: &CollationPrefix,
) -> Ordering {
    if left.msgid != right.msgid {
        if prefix[PREFIX_BYTES - 1] != 0 {
            for round in 1..=MAX_CONTINUATION_ROUNDS {
                let skip = round * PREFIX_BYTES;
                let left_cont = storage_collation_prefix_from(&left.msgid, skip);
                let right_cont = storage_collation_prefix_from(&right.msgid, skip);
                match left_cont.cmp(&right_cont) {
                    Ordering::Equal => {
                        if left_cont[PREFIX_BYTES - 1] == 0 {
                            break;
                        }
                    }
                    non_eq => return non_eq,
                }
            }
        }
        let msg_ord = storage_collation_key(&left.msgid).cmp(&storage_collation_key(&right.msgid));
        if msg_ord != Ordering::Equal {
            return msg_ord;
        }
    }

    if left.msgctxt != right.msgctxt {
        let left_ctx = left.msgctxt.as_deref().unwrap_or("");
        let right_ctx = right.msgctxt.as_deref().unwrap_or("");
        let ctx_prefix_ord =
            storage_collation_prefix(left_ctx).cmp(&storage_collation_prefix(right_ctx));
        if ctx_prefix_ord != Ordering::Equal {
            return ctx_prefix_ord;
        }
        let ctx_ord = storage_collation_key(left_ctx).cmp(&storage_collation_key(right_ctx));
        if ctx_ord != Ordering::Equal {
            return ctx_ord;
        }
    }

    left.msgid
        .cmp(&right.msgid)
        .then_with(|| left.msgctxt.cmp(&right.msgctxt))
        .then_with(|| left.obsolete.is_some().cmp(&right.obsolete.is_some()))
}

fn ensure_cached_fcl_ids(
    messages: &[CanonicalMessage],
    indices: impl IntoIterator<Item = usize>,
    plural_ids: &mut FxHashMap<usize, String>,
) {
    for index in indices {
        if let CanonicalTranslation::Plural(plural) = &messages[index].translation {
            plural_ids
                .entry(index)
                .or_insert_with(|| synthesize_icu_plural_source(&plural.variable, &plural.source));
        }
    }
}

fn fcl_id_with_cache<'a>(
    messages: &'a [CanonicalMessage],
    index: usize,
    plural_ids: &'a FxHashMap<usize, String>,
) -> &'a str {
    plural_ids
        .get(&index)
        .map_or(messages[index].msgid.as_str(), String::as_str)
}

fn sort_fcl_run_by_full_key(
    messages: &[CanonicalMessage],
    run: &mut [usize],
    plural_ids: &mut FxHashMap<usize, String>,
) -> Result<(), ApiError> {
    ensure_cached_fcl_ids(messages, run.iter().copied(), plural_ids);
    let mut keyed: Vec<(FclCollatedKey, &str, usize)> = run
        .iter()
        .map(|&index| {
            let message = &messages[index];
            let id = fcl_id_with_cache(messages, index, plural_ids);
            let key = fcl_collated_key(id, message);
            (key, id, index)
        })
        .collect();
    keyed.sort_by(|left, right| {
        let left_message = &messages[left.2];
        let right_message = &messages[right.2];
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.cmp(right.1))
            .then_with(|| left_message.msgctxt.cmp(&right_message.msgctxt))
            .then_with(|| {
                left_message
                    .obsolete
                    .is_some()
                    .cmp(&right_message.obsolete.is_some())
            })
    });
    // Every duplicate FCL `(id, msgctxt)` identity collides on all prefix rounds,
    // so it is guaranteed to land adjacent inside one unresolved run here.
    for pair in keyed.windows(2) {
        if pair[0].1 == pair[1].1 && messages[pair[0].2].msgctxt == messages[pair[1].2].msgctxt {
            return Err(ApiError::Conflict(format!(
                "catalog contains duplicate FCL identity for id {:?} and context {:?}",
                pair[1].1, messages[pair[1].2].msgctxt
            )));
        }
    }
    for (slot, (_, _, index)) in run.iter_mut().zip(keyed) {
        *slot = index;
    }
    Ok(())
}

/// Applies the same packed-prefix and multi-round continuation strategy as the
/// shared catalog sort, using FCL's serialized `id` column so plural IDs
/// validate after a round trip. Rejects duplicate FCL `(id, msgctxt)`
/// identities without building a separate full-catalog hash set.
fn fcl_collated_order(messages: &[CanonicalMessage]) -> Result<Vec<usize>, ApiError> {
    let mut order: Vec<usize> = (0..messages.len()).collect();
    if messages.len() < 2 {
        return Ok(order);
    }

    let mut keyed: Vec<(CollationPrefix, usize)> = messages
        .iter()
        .enumerate()
        .map(|(index, message)| (storage_collation_prefix(&fcl_id(message)), index))
        .collect();
    keyed.sort_unstable_by_key(|&(prefix, _)| prefix);

    let mut pending = Vec::new();
    let mut unresolved = Vec::new();
    collect_colliding_runs(&keyed, 0, &mut pending, &mut unresolved);

    let mut plural_ids = FxHashMap::default();
    let mut split = Vec::new();
    for round in 1..=MAX_CONTINUATION_ROUNDS {
        if pending.is_empty() {
            break;
        }
        let skip = round * PREFIX_BYTES;
        for (start, end) in pending.drain(..) {
            ensure_cached_fcl_ids(
                messages,
                keyed[start..end].iter().map(|&(_, index)| index),
                &mut plural_ids,
            );
            let first_id = fcl_id_with_cache(messages, keyed[start].1, &plural_ids);
            if keyed[start + 1..end]
                .iter()
                .all(|&(_, index)| fcl_id_with_cache(messages, index, &plural_ids) == first_id)
            {
                unresolved.push((start, end));
                continue;
            }
            for entry in &mut keyed[start..end] {
                let id = fcl_id_with_cache(messages, entry.1, &plural_ids);
                entry.0 = storage_collation_prefix_from(id, skip);
            }
            keyed[start..end].sort_unstable_by_key(|&(prefix, _)| prefix);
            collect_colliding_runs(&keyed[start..end], start, &mut split, &mut unresolved);
        }
        std::mem::swap(&mut pending, &mut split);
    }
    unresolved.append(&mut pending);

    for (slot, &(_, index)) in order.iter_mut().zip(keyed.iter()) {
        *slot = index;
    }
    for (start, end) in unresolved {
        sort_fcl_run_by_full_key(messages, &mut order[start..end], &mut plural_ids)?;
    }
    Ok(order)
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
) -> Result<String, ApiError> {
    let order = fcl_collated_order(&catalog.messages)?;

    // Reserve a rough output budget up front so growth does not repeatedly
    // reallocate; the estimate only needs to be in the right ballpark.
    let mut out = String::with_capacity(catalog.messages.len() * 128 + 64);
    out.push_str(FCL2_MAGIC);
    write_tag(&mut out, "source", source_locale);
    if let Some(locale) = locale {
        write_tag(&mut out, "locale", locale);
    }
    write_tag(&mut out, "order", "collated");
    out.push('\n');

    let mut origins_scratch = Vec::new();
    let mut placeholder_scratch = String::new();
    for index in order {
        write_entry(
            &mut out,
            &catalog.messages[index],
            render,
            &mut origins_scratch,
            &mut placeholder_scratch,
        );
    }
    Ok(out)
}

// --- parse ----------------------------------------------------------------

#[inline]
fn is_conflict_marker(line: &str) -> bool {
    // Guard on the first byte so normal entries skip all four marker checks.
    // A valid entry has at least three tab-separated fields, even when its ID
    // begins with marker characters. Diff3 also uses a `|||||||` base marker.
    let Some(&first) = line.as_bytes().first() else {
        return false;
    };
    let matches_marker = match first {
        b'<' => line.starts_with("<<<<<<<"),
        b'|' => line.starts_with("|||||||"),
        b'=' => line.starts_with("======="),
        b'>' => line.starts_with(">>>>>>>"),
        _ => false,
    };
    matches_marker && line.bytes().filter(|&byte| byte == b'\t').take(2).count() < 2
}

fn parse_header(line: &str, source_locale: &str) -> Result<FclHeader, ApiError> {
    let mut fields = SplitTab::new(line.as_bytes());
    let version = match fields.next() {
        Some(magic) if magic == FCL_MAGIC.as_bytes() => FclVersion::V1,
        Some(magic) if magic == FCL2_MAGIC.as_bytes() => FclVersion::V2,
        _ => {
            return Err(ApiError::InvalidArguments(
                "FCL catalog must start with a `%FCL1` or `%FCL2` header".to_owned(),
            ));
        }
    };
    if version == FclVersion::V2 && find_byte(b'\r', line.as_bytes()).is_some() {
        return Err(ApiError::InvalidArguments(
            "invalid FCL header: raw carriage return must be escaped as \\r".to_owned(),
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
        let value = unescape(input_slice_as_str(value), version)?.into_owned();
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
    Ok(FclHeader {
        version,
        locale,
        order,
    })
}

/// Parses one FCL entry line directly into a [`CanonicalMessage`], building the
/// owned fields in a single pass without an intermediate record.
fn parse_entry(
    line: &str,
    opaque_capture: OpaqueCapture,
    version: FclVersion,
    has_backslash: bool,
) -> Result<CanonicalMessage, ApiError> {
    let mut fields = SplitTab::new(line.as_bytes());
    let msgid = unescape_maybe(field(&mut fields, "id")?, version, has_backslash)?.into_owned();
    let ctx_raw = field(&mut fields, "ctxt")?;
    let msgctxt = match version {
        FclVersion::V1 => {
            let context = unescape_maybe(ctx_raw, version, has_backslash)?.into_owned();
            (!context.is_empty()).then_some(context)
        }
        FclVersion::V2 if ctx_raw == "n" => None,
        FclVersion::V2 if ctx_raw.starts_with('s') => {
            Some(unescape_maybe(&ctx_raw[1..], version, has_backslash)?.into_owned())
        }
        FclVersion::V2 => {
            return Err(ApiError::InvalidArguments(
                "invalid FCL2 context marker; expected `n` or `s`".to_owned(),
            ));
        }
    };
    let value = unescape_maybe(field(&mut fields, "target")?, version, has_backslash)?.into_owned();
    let mut origins: PoVec<CatalogOrigin> = PoVec::new();
    let mut comments: Vec<String> = Vec::new();
    let mut placeholders: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut translator_comments: Vec<String> = Vec::new();
    let mut flags: Vec<String> = Vec::new();
    let mut obsolete: Option<ObsoleteInfo> = None;
    let mut lock = None;
    let mut ai = None;
    let mut last_tag_rank = 0_u8;

    for tag in fields {
        if tag == b"o" {
            validate_tag_order(&mut last_tag_rank, TAG_RANK_OBSOLETE, "o")?;
            set_obsolete(&mut obsolete, None)?;
            continue;
        }
        let (key, raw_value) = split_once_byte(tag, b'=').ok_or_else(|| {
            ApiError::InvalidArguments(format!("invalid FCL tag {:?}", input_slice_as_str(tag)))
        })?;
        let raw_value = input_slice_as_str(raw_value);
        match key {
            b"r" => {
                validate_tag_order(&mut last_tag_rank, TAG_RANK_REFERENCE, "r")?;
                origins.push(parse_origin(unescape_maybe(
                    raw_value,
                    version,
                    has_backslash,
                )?));
            }
            b"c" => {
                validate_tag_order(&mut last_tag_rank, TAG_RANK_COMMENT, "c")?;
                let comment = unescape_maybe(raw_value, version, has_backslash)?;
                if let Some((name, val)) = parse_placeholder_comment(&comment) {
                    placeholders
                        .entry(name.to_owned())
                        .or_default()
                        .push(val.to_owned());
                } else {
                    comments.push(comment.into_owned());
                }
            }
            b"tc" => {
                validate_tag_order(&mut last_tag_rank, TAG_RANK_TRANSLATOR_COMMENT, "tc")?;
                if opaque_capture == OpaqueCapture::Keep {
                    translator_comments
                        .push(unescape_maybe(raw_value, version, has_backslash)?.into_owned());
                } else {
                    validate_escapes_maybe(raw_value, version, has_backslash)?;
                }
            }
            b"f" => {
                validate_tag_order(&mut last_tag_rank, TAG_RANK_FLAG, "f")?;
                match opaque_capture {
                    OpaqueCapture::Keep => {
                        flags.push(unescape_maybe(raw_value, version, has_backslash)?.into_owned());
                    }
                    OpaqueCapture::ReviewState => {
                        validate_escapes_maybe(raw_value, version, has_backslash)?;
                        if raw_value == "fuzzy" {
                            flags.push("fuzzy".to_owned());
                        }
                    }
                    OpaqueCapture::Discard => {
                        validate_escapes_maybe(raw_value, version, has_backslash)?;
                    }
                }
            }
            b"o" => {
                validate_tag_order(&mut last_tag_rank, TAG_RANK_OBSOLETE, "o")?;
                set_obsolete(
                    &mut obsolete,
                    Some(unescape_maybe(raw_value, version, has_backslash)?.into_owned()),
                )?;
            }
            b"lock" => {
                validate_tag_order(&mut last_tag_rank, TAG_RANK_LOCK, "lock")?;
                if lock.is_some() {
                    return Err(ApiError::InvalidArguments(
                        "duplicate FCL tag `lock`".to_owned(),
                    ));
                }
                lock = Some(unescape_maybe(raw_value, version, has_backslash)?.into_owned());
            }
            b"ai" => {
                validate_tag_order(&mut last_tag_rank, TAG_RANK_AI, "ai")?;
                if ai.is_some() {
                    return Err(ApiError::InvalidArguments(
                        "duplicate FCL tag `ai`".to_owned(),
                    ));
                }
                let value = unescape_maybe(raw_value, version, has_backslash)?;
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
        Some(Box::new(metadata))
    } else if ai.is_some() {
        return Err(ApiError::InvalidArguments(
            "FCL `ai` tag requires a `lock` tag".to_owned(),
        ));
    } else {
        None
    };

    let placeholders = if placeholders.is_empty() {
        placeholders
    } else {
        dedupe_placeholders(placeholders)
    };

    Ok(CanonicalMessage {
        msgid,
        msgctxt,
        translation: CanonicalTranslation::Singular { value },
        comments,
        opaque: match opaque_capture {
            OpaqueCapture::Keep => OpaqueMetadata::from_parts(translator_comments, flags),
            OpaqueCapture::ReviewState => OpaqueMetadata::from_parts(Vec::new(), flags),
            OpaqueCapture::Discard => None,
        },
        origins,
        placeholders,
        obsolete,
        machine,
    })
}

/// Canonical entry-tag order, enforced on read and produced on write:
/// `r`, `c`, `tc`, `f`, `o`, `lock`, `ai`.
const TAG_RANK_REFERENCE: u8 = 0;
const TAG_RANK_COMMENT: u8 = 1;
const TAG_RANK_TRANSLATOR_COMMENT: u8 = 2;
const TAG_RANK_FLAG: u8 = 3;
const TAG_RANK_OBSOLETE: u8 = 4;
const TAG_RANK_LOCK: u8 = 5;
const TAG_RANK_AI: u8 = 6;

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

/// SIMD `memchr`-backed line splitter matching `str::lines()` semantics (`\n`
/// and `\r\n`, omitting any empty trailing slice after a final `\n`, preserving
/// interior bare `\r` bytes so FCL2 can reject them and FCL1 can retain them).
struct FclLines<'a> {
    rest: &'a [u8],
    has_cr: bool,
}

impl<'a> FclLines<'a> {
    #[inline]
    fn new(bytes: &'a [u8], has_cr: bool) -> Self {
        Self {
            rest: bytes,
            has_cr,
        }
    }
}

impl<'a> Iterator for FclLines<'a> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        if self.rest.is_empty() {
            return None;
        }
        match find_byte(b'\n', self.rest) {
            Some(index) => {
                let mut line = &self.rest[..index];
                if self.has_cr && line.last() == Some(&b'\r') {
                    line = &line[..line.len() - 1];
                }
                self.rest = &self.rest[index + 1..];
                Some(input_slice_as_str(line))
            }
            None => {
                let line = self.rest;
                self.rest = &[];
                Some(input_slice_as_str(line))
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
    opaque_capture: OpaqueCapture,
) -> Result<Catalog, ApiError> {
    if semantics != CatalogSemantics::IcuNative {
        return Err(ApiError::Unsupported(
            "CatalogSemantics::GettextCompat is not supported for FCL catalogs".to_owned(),
        ));
    }
    super::validate_source_locale(source_locale)?;

    let bytes = content.as_bytes();
    let has_cr = find_byte(b'\r', bytes).is_some();
    let file_has_backslash = find_byte(b'\\', bytes).is_some();
    let mut lines = FclLines::new(bytes, has_cr).enumerate();
    let header = lines.next().map(|(_, line)| line).ok_or_else(|| {
        ApiError::InvalidArguments(
            "FCL catalog must start with a `%FCL1` or `%FCL2` header".to_owned(),
        )
    })?;
    let parsed_header = parse_header(header, source_locale)?;
    let locale = locale_override.map(str::to_owned).or(parsed_header.locale);

    // The entry count is at most the newline count; over-reserving by the header
    // and any blank lines only saves reallocations.
    let estimated_entries = memchr::memchr_iter(b'\n', bytes).count();
    let mut messages: Vec<CanonicalMessage> = Vec::with_capacity(estimated_entries);
    let mut previous_prefix: Option<CollationPrefix> = None;
    let check_v2_collated =
        parsed_header.order == FclOrder::Collated && parsed_header.version == FclVersion::V2;
    let check_v2_raw_cr = has_cr && parsed_header.version == FclVersion::V2;
    let legacy_collated =
        parsed_header.version == FclVersion::V1 && parsed_header.order == FclOrder::Collated;
    for (index, line) in lines {
        if check_v2_raw_cr && find_byte(b'\r', line.as_bytes()).is_some() {
            return Err(ApiError::InvalidArguments(format!(
                "invalid FCL entry on line {}: raw carriage return must be escaped as \\r",
                index + 1
            )));
        }
        if line.is_empty() {
            continue;
        }
        if is_conflict_marker(line) {
            return Err(ApiError::InvalidArguments(format!(
                "git conflict marker in FCL catalog on line {}",
                index + 1
            )));
        }
        let line_has_backslash = file_has_backslash && find_byte(b'\\', line.as_bytes()).is_some();
        let message = parse_entry(
            line,
            opaque_capture,
            parsed_header.version,
            line_has_backslash,
        )
        .map_err(|error| {
            ApiError::InvalidArguments(format!("invalid FCL entry on line {}: {error}", index + 1))
        })?;
        if check_v2_collated {
            let prefix = storage_collation_prefix(&message.msgid);
            if let (Some(prev_prefix), Some(previous)) = (previous_prefix, messages.last()) {
                match prefix.cmp(&prev_prefix) {
                    Ordering::Greater => {}
                    Ordering::Less => {
                        return Err(ApiError::InvalidArguments(format!(
                            "FCL entries must follow the declared collated order; line {} is out of order",
                            index + 1
                        )));
                    }
                    Ordering::Equal => {
                        if message.msgid == previous.msgid && message.msgctxt == previous.msgctxt {
                            return Err(ApiError::Conflict(format!(
                                "duplicate FCL entry for id {:?} and context {:?}",
                                message.msgid, message.msgctxt
                            )));
                        }
                        if cmp_fcl_collated_on_prefix_tie(&message, previous, &prefix).is_lt() {
                            return Err(ApiError::InvalidArguments(format!(
                                "FCL entries must follow the declared collated order; line {} is out of order",
                                index + 1
                            )));
                        }
                    }
                }
            }
            previous_prefix = Some(prefix);
        } else if let Some(previous) = messages.last() {
            // Tagless files retain the legacy bytewise order check.
            let identity_order = (message.msgid.as_str(), message.msgctxt.as_deref())
                .cmp(&(previous.msgid.as_str(), previous.msgctxt.as_deref()));
            if !legacy_collated && identity_order.is_eq() {
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
        messages.push(message);
    }

    if legacy_collated {
        let mut identities = FxHashSet::default();
        for message in &messages {
            if !identities.insert((message.msgid.as_str(), message.msgctxt.as_deref())) {
                return Err(ApiError::Conflict(format!(
                    "duplicate FCL entry for id {:?} and context {:?}",
                    message.msgid, message.msgctxt
                )));
            }
        }
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
    use super::{
        FCL_MAGIC, OpaqueCapture, parse_catalog_to_internal_fcl, stringify_catalog_fcl, unescape,
    };
    use crate::api::CatalogSemantics;
    use crate::api::types::RenderOptions;

    fn roundtrip(text: &str) -> String {
        let catalog = parse_catalog_to_internal_fcl(
            text,
            None,
            "en",
            CatalogSemantics::IcuNative,
            OpaqueCapture::Keep,
        )
        .expect("parse FCL");
        stringify_catalog_fcl(
            &catalog,
            catalog.locale.as_deref(),
            "en",
            &RenderOptions::default(),
        )
        .expect("serialize FCL")
    }

    #[test]
    fn escaping_roundtrips_control_chars() {
        let raw = "a\tb\nc\\d";
        let mut escaped = String::new();
        super::escape_into(&mut escaped, raw);
        assert_eq!(escaped, "a\\tb\\nc\\\\d");
        assert_eq!(
            unescape(&escaped, super::FclVersion::V2).expect("unescape"),
            raw
        );
    }

    #[test]
    fn fcl_versions_are_dispatched_strictly_and_crlf_is_supported() {
        let old = "%FCL1\tsource=en\r\na\tn\tone\r\nb\ts\ttwo\r\nc\t\tthree\r\n";
        let migrated = roundtrip(old);
        assert!(migrated.starts_with("%FCL2\tsource=en\torder=collated\n"));
        assert!(migrated.contains("a\tsn\tone\n"));
        assert!(migrated.contains("b\tss\ttwo\n"));
        assert!(migrated.contains("c\tn\tthree\n"));
        assert!(parse_err("%FCL1\tsource=en\na\t\tbad\\r\n"));
        for context in ["", "x", "nn", "nanything"] {
            assert!(parse_err(&format!("%FCL2\tsource=en\na\t{context}\tv\n")));
        }
        assert!(parse_err("%FCL2\tsource=en\na\tn\tbad\\x\n"));
        assert!(parse_err("%FCL3\tsource=en\na\tn\tv\n"));
        let encoded = "%FCL2\tsource=en\torder=collated\r\na\\r\tsctx\\r\tvalue\\r\tc=note\\r\ttc=translator\\r\tf=custom\\r\r\n";
        assert_eq!(roundtrip(encoded), encoded.replace("\r\n", "\n"));
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
            "%FCL2\tsource=en\tlocale=de\torder=collated\n\
             greeting\tn\tHallo {{name}}\tr=src/a.tsx#Greeting\tlock={hash}\tai=openai/gpt-5.5:0.88\n\
             tabbed\tsmenu\tWert mit\\tTab\n"
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
                OpaqueCapture::Keep,
            )
            .is_err()
        );

        let unknown = format!("{FCL_MAGIC}\tsource=en\nid\t\tvalue\tzz=1\n");
        assert!(
            parse_catalog_to_internal_fcl(
                &unknown,
                None,
                "en",
                CatalogSemantics::IcuNative,
                OpaqueCapture::Keep,
            )
            .is_err()
        );
    }

    #[test]
    fn accepts_legacy_bytewise_sorted_entries() {
        // Legacy FCL without an order tag remains readable when bytewise-sorted.
        // Re-serialization upgrades it to the declared collated contract.
        let text = format!("{FCL_MAGIC}\tsource=en\nalpha\t\tA\nalpha\tmenu\tA2\nbeta\t\tB\n");
        let sorted = roundtrip(&text);
        assert!(sorted.starts_with("%FCL2\tsource=en\torder=collated\n"));
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
                OpaqueCapture::Keep,
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
                OpaqueCapture::Keep,
            )
            .is_err()
        );
    }

    fn parse_err(text: &str) -> bool {
        parse_catalog_to_internal_fcl(
            text,
            None,
            "en",
            CatalogSemantics::IcuNative,
            OpaqueCapture::Keep,
        )
        .is_err()
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
            opaque: None,
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

        let text = stringify_catalog_fcl(&catalog, Some("de"), "en", &RenderOptions::default())
            .expect("serialize FCL");

        assert_eq!(text.matches("\tr=src/app.rs#Greeting").count(), 1);
        assert!(text.contains("\tc=Translator note"));
        assert!(text.contains("\tc=placeholder {0}: user name"));
    }

    #[test]
    fn roundtrips_comments_obsolete_and_escapes() {
        // Covers the c=/o serialize branches and the \t \n \\ escape paths in both
        // directions on a single entry.
        let text = "%FCL2\tsource=en\torder=collated\n\
             a.tab\\there\tn\tWert\\nZeile\\\\back\tc=note\to\n";
        assert_eq!(roundtrip(text), text);
    }

    #[test]
    fn roundtrips_reference_without_line_number() {
        // Covers the line-less origin path on both parse and serialize.
        let text = "%FCL2\tsource=en\torder=collated\nid\tn\tv\tr=README\n";
        assert_eq!(roundtrip(text), text);
    }

    #[test]
    fn roundtrips_dated_obsolete() {
        // Covers the valued `o=<since>` serialize and parse branches.
        let text = "%FCL2\tsource=en\torder=collated\nid\tn\tv\to=2026-06-30\n";
        assert_eq!(roundtrip(text), text);
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
    fn discard_projection_validates_opaque_tag_escapes_without_retaining_them() {
        let valid =
            format!("{FCL_MAGIC}\tsource=en\nid\t\tvalue\ttc=first\\tnote\tf=x\\\\custom\n");
        let catalog = parse_catalog_to_internal_fcl(
            &valid,
            None,
            "en",
            CatalogSemantics::IcuNative,
            OpaqueCapture::Discard,
        )
        .expect("discard projection should validate opaque tags");
        assert!(catalog.messages[0].translator_comments().is_empty());
        assert!(catalog.messages[0].flags().is_empty());

        for invalid in [
            format!("{FCL_MAGIC}\tsource=en\nid\t\tvalue\ttc=bad\\xescape\n"),
            format!("{FCL_MAGIC}\tsource=en\nid\t\tvalue\tf=dangling\\\n"),
        ] {
            assert!(
                parse_catalog_to_internal_fcl(
                    &invalid,
                    None,
                    "en",
                    CatalogSemantics::IcuNative,
                    OpaqueCapture::Discard,
                )
                .is_err()
            );
        }
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
        let text = "%FCL2\tsource=en\torder=collated\n\
             <0>Continue</0>\tn\tmarkup\n\
             {count, plural, one {#} other {#}}\tn\tplaceholder\n";
        assert!(parse_err(text));
    }

    #[test]
    fn storage_order_is_stable_for_uncovered_unicode_and_reads_legacy_orders() {
        let expected = "%FCL2\tsource=en\torder=collated\nΩ\tn\tomega\n꟝\tn\tletter\n";
        assert_eq!(roundtrip(expected), expected);
        for old in [
            "%FCL1\tsource=en\torder=collated\nΩ\t\tomega\n꟝\t\tletter\n",
            "%FCL1\tsource=en\torder=collated\n꟝\t\tletter\nΩ\t\tomega\n",
        ] {
            assert_eq!(roundtrip(old), expected);
        }
        assert!(parse_err(
            "%FCL1\tsource=en\torder=collated\nΩ\t\tone\n꟝\t\ttwo\nΩ\t\tthree\n"
        ));
        assert!(parse_err("%FCL1\tsource=en\n꟝\t\ttwo\nΩ\t\tone\n"));
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
    fn roundtrips_translator_comments_and_flags_byte_identically() {
        // `tc` and `f` are opaque: multiple of each are preserved verbatim, in
        // order, and re-serialize to the same bytes.
        let text = "%FCL2\tsource=en\torder=collated\n\
             id\tsbutton\tv\tr=src/a.tsx\tc=extracted\ttc=first note\ttc=second\\tnote\tf=fuzzy\tf=x-custom\to=2026-07-30\n";
        assert_eq!(roundtrip(text), text);
    }

    #[test]
    fn rejects_translator_comment_and_flag_tags_outside_canonical_order() {
        // Canonical order is `r`, `c`, `tc`, `f`, `o`, `lock`, `ai`.
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\nid\t\tv\ttc=note\tc=extracted\n"
        )));
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\nid\t\tv\tf=fuzzy\ttc=note\n"
        )));
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\nid\t\tv\to\tf=fuzzy\n"
        )));
    }

    #[test]
    fn carries_opaque_po_metadata_into_fcl_tags() {
        // A PO catalog read through the shared import path renders its
        // translator comments and flags as `tc=`/`f=` tags.
        let catalog = super::super::catalog::parse_catalog_to_internal(
            "# translator note\n#. extracted note\n#, fuzzy, x-custom\nmsgid \"Hello\"\nmsgstr \"Hallo\"\n",
            Some("de"),
            "en",
            CatalogSemantics::IcuNative,
            crate::api::PluralEncoding::Icu,
            crate::api::CatalogStorageFormat::Po,
            OpaqueCapture::Keep,
        )
        .expect("parse PO");

        let text = stringify_catalog_fcl(&catalog, Some("de"), "en", &RenderOptions::default())
            .expect("serialize FCL");

        assert!(text.contains("\tc=extracted note\ttc=translator note\tf=fuzzy\tf=x-custom\n"));
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
            translation: super::CanonicalTranslation::Plural(Box::new(
                super::super::catalog::CanonicalPlural {
                    source: crate::PluralSource {
                        one: Some("{count} file".to_owned()),
                        other: "{count} files".to_owned(),
                    },
                    translation_by_category,
                    variable: "count".to_owned(),
                },
            )),
            comments: Vec::new(),
            opaque: None,
            origins: super::PoVec::new(),
            placeholders: std::collections::BTreeMap::new(),
            obsolete: None,
            machine: Some(Box::new(super::MachineMetadata {
                lock: hash,
                ai: Some(crate::AiProvenance {
                    model: "example/mt".to_owned(),
                    confidence: Some(0.90),
                }),
            })),
        };
        let catalog = super::Catalog {
            locale: Some("de".to_owned()),
            headers: std::collections::BTreeMap::new(),
            file_comments: Vec::new(),
            file_extracted_comments: Vec::new(),
            messages: vec![message],
            diagnostics: Vec::new(),
        };

        let text = stringify_catalog_fcl(&catalog, Some("de"), "en", &RenderOptions::default())
            .expect("serialize FCL");
        assert!(text.starts_with("%FCL2\tsource=en\tlocale=de\torder=collated\n"));
        // Both id and target columns are synthesized ICU plural strings, and the
        // lock/ai block survives because the lock matches the plural translation.
        assert!(text.contains("{count, plural,"));
        assert!(text.contains("{count} Dateien"));
        assert!(text.contains("ai=example/mt:0.9"));

        // The synthesized ICU string round-trips back through the reader.
        let reparsed = parse_catalog_to_internal_fcl(
            &text,
            None,
            "en",
            CatalogSemantics::IcuNative,
            OpaqueCapture::Keep,
        )
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
            opaque: None,
            origins: super::PoVec::new(),
            placeholders: std::collections::BTreeMap::new(),
            obsolete: None,
            machine: Some(Box::new(super::MachineMetadata {
                lock: stale,
                ai: Some(crate::AiProvenance {
                    model: "example/mt".to_owned(),
                    confidence: Some(0.90),
                }),
            })),
        };
        let catalog = super::Catalog {
            locale: Some("de".to_owned()),
            headers: std::collections::BTreeMap::new(),
            file_comments: Vec::new(),
            file_extracted_comments: Vec::new(),
            messages: vec![message],
            diagnostics: Vec::new(),
        };

        let text = stringify_catalog_fcl(&catalog, Some("de"), "en", &RenderOptions::default())
            .expect("serialize FCL");
        assert!(text.contains("Hello\tn\tHallo"));
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
                OpaqueCapture::Keep,
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
            OpaqueCapture::Keep,
        )
        .expect("parse");
        assert_eq!(catalog.locale.as_deref(), Some("fr"));
        assert_eq!(catalog.messages.len(), 2);
    }

    #[test]
    fn sorts_long_colliding_structured_plurals_through_full_key_fallback() {
        let shared = "a".repeat(140);
        let plural = |suffix: &str| super::CanonicalMessage {
            msgid: format!("{shared}_{suffix}"),
            msgctxt: None,
            translation: super::CanonicalTranslation::Plural(Box::new(
                super::super::catalog::CanonicalPlural {
                    source: crate::PluralSource {
                        one: Some(format!("{shared} one {suffix}")),
                        other: format!("{shared} other {suffix}"),
                    },
                    translation_by_category: std::collections::BTreeMap::from([
                        ("one".to_owned(), format!("eins {suffix}")),
                        ("other".to_owned(), format!("viele {suffix}")),
                    ]),
                    variable: "count".to_owned(),
                },
            )),
            comments: Vec::new(),
            opaque: None,
            origins: super::PoVec::new(),
            placeholders: std::collections::BTreeMap::new(),
            obsolete: None,
            machine: None,
        };
        let catalog = super::Catalog {
            locale: Some("de".to_owned()),
            headers: std::collections::BTreeMap::new(),
            file_comments: Vec::new(),
            file_extracted_comments: Vec::new(),
            messages: vec![plural("z"), plural("a")],
            diagnostics: Vec::new(),
        };

        let text = stringify_catalog_fcl(&catalog, Some("de"), "en", &RenderOptions::default())
            .expect("serialize FCL");
        let reparsed = parse_catalog_to_internal_fcl(
            &text,
            Some("de"),
            "en",
            CatalogSemantics::IcuNative,
            OpaqueCapture::Keep,
        )
        .expect("sorted long colliding plurals must round-trip in collated order");
        assert_eq!(reparsed.messages.len(), 2);
        assert!(reparsed.messages[0].msgid.contains("one a"));
        assert!(reparsed.messages[1].msgid.contains("one z"));
    }

    #[test]
    fn fcl2_rejects_raw_carriage_return_inside_lines_while_accepting_crlf() {
        let crlf = format!(
            "{}\tsource=en\torder=collated\r\nhello\tn\thallo\r\n",
            super::FCL2_MAGIC
        );
        let parsed = parse_catalog_to_internal_fcl(
            &crlf,
            None,
            "en",
            CatalogSemantics::IcuNative,
            OpaqueCapture::Keep,
        )
        .expect("CRLF line endings are valid in FCL2");
        assert_eq!(parsed.messages.len(), 1);

        let raw_cr_in_entry = format!(
            "{}\tsource=en\torder=collated\nhello\tn\thallo\rworld\n",
            super::FCL2_MAGIC
        );
        let error = parse_catalog_to_internal_fcl(
            &raw_cr_in_entry,
            None,
            "en",
            CatalogSemantics::IcuNative,
            OpaqueCapture::Keep,
        )
        .expect_err("raw CR inside FCL2 entry line must be rejected");
        assert!(error.to_string().contains("raw carriage return"));

        let raw_cr_in_header = format!(
            "{}\tsource=en\r\torder=collated\nhello\tn\thallo\n",
            super::FCL2_MAGIC
        );
        let header_error = parse_catalog_to_internal_fcl(
            &raw_cr_in_header,
            None,
            "en",
            CatalogSemantics::IcuNative,
            OpaqueCapture::Keep,
        )
        .expect_err("raw CR inside FCL2 header must be rejected");
        assert!(header_error.to_string().contains("raw carriage return"));

        let unknown_version_error = parse_catalog_to_internal_fcl(
            "%FCL20\tsource=en\rbar\nhello\tn\thallo\n",
            None,
            "en",
            CatalogSemantics::IcuNative,
            OpaqueCapture::Keep,
        )
        .expect_err("unknown FCL version must report version error");
        assert!(unknown_version_error.to_string().contains("%FCL1"));

        let fcl1_raw_cr = format!("{FCL_MAGIC}\tsource=en\nhello\t\thallo\rworld\n");
        let fcl1_parsed = parse_catalog_to_internal_fcl(
            &fcl1_raw_cr,
            None,
            "en",
            CatalogSemantics::IcuNative,
            OpaqueCapture::Keep,
        )
        .expect("FCL1 keeps legacy interpretation");
        assert_eq!(fcl1_parsed.messages.len(), 1);
    }

    #[test]
    fn fcl_lines_matches_std_lines_across_terminators() {
        for input in [
            "",
            "\n",
            "\r\n",
            "\r",
            "a",
            "a\n",
            "a\r\n",
            "a\n\nb",
            "a\r\n\r\nb\r\n",
            "a\rb\nc\r",
            "héllo\nworld\r\n!",
        ] {
            let has_cr = crate::scan::find_byte(b'\r', input.as_bytes()).is_some();
            let got: Vec<&str> = super::FclLines::new(input.as_bytes(), has_cr).collect();
            let expected: Vec<&str> = input.lines().collect();
            assert_eq!(got, expected, "input={input:?}");
        }
    }
}
