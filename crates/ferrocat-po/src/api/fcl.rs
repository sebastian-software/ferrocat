//! FCL (Ferrocat Catalog Lines) codec — see `docs/fcl-format.md`.
//!
//! A line-oriented, machine-owned catalog encoding optimized for git merge and
//! fast parsing: one entry per line, deterministically sorted by `(id, ctxt)`,
//! minimal escaping (`\n \t \\`). The codec parses and serializes
//! [`CanonicalMessage`] directly, reusing the shared plural/placeholder/MT
//! helpers so it stays equivalent to the catalog's canonical representation.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::BTreeMap;

use super::catalog::{CanonicalMessage, CanonicalTranslation, Catalog, split_placeholder_comments};
use super::export::{append_placeholder_comments, plural_source_branches};
use super::mt::{
    MachineTranslationMetadata, machine_translation_hash, validate_machine_translation_metadata,
};
use super::plural::synthesize_icu_plural;
use super::{ApiError, CatalogOrigin, CatalogSemantics, EffectiveTranslationRef, RenderOptions};
use crate::PoVec;
use crate::scan::{find_byte, find_fcl_escapable_byte, split_once_byte};
use crate::utf8::input_slice_as_str;

const FCL_MAGIC: &str = "%FCL1";

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
        } => Cow::Owned(synthesize_icu_plural(
            variable,
            &plural_source_branches(source),
        )),
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
        let mut refs = message
            .origins
            .iter()
            .map(|origin| origin.file.as_str())
            .collect::<Vec<_>>();
        refs.sort_unstable();
        refs.dedup();
        for reference in &refs {
            write_tag(out, "r", reference);
        }
    }

    // Extracted comments plus the placeholder comments folded back in, matching
    // the catalog's comment round-trip.
    let mut comments = message.comments.clone();
    append_placeholder_comments(
        &mut comments,
        &message.placeholders,
        &render.print_placeholders_in_comments,
    );
    for comment in &comments {
        write_tag(out, "c", comment);
    }

    for comment in &message.translator_comments {
        write_tag(out, "tc", comment);
    }
    let mut flags = message.flags.clone();
    flags.sort_unstable();
    for flag in &flags {
        write_tag(out, "f", flag);
    }
    if message.obsolete {
        out.push_str("\to");
    }

    // The MT block is only emitted when its hash still matches the translation
    // (stale machine translations are dropped).
    if let Some(mt) = message.machine_translation.as_ref() {
        let translation_ref = match &message.translation {
            CanonicalTranslation::Singular { value } => EffectiveTranslationRef::Singular(value),
            CanonicalTranslation::Plural {
                translation_by_category,
                ..
            } => EffectiveTranslationRef::Plural(translation_by_category),
        };
        if validate_machine_translation_metadata(mt).is_ok()
            && mt.hash == machine_translation_hash(translation_ref)
        {
            write_tag(out, "mt.model", &mt.model);
            if let Some(confidence) = mt.confidence {
                out.push_str("\tmt.conf=");
                out.push_str(&confidence.to_string());
            }
            write_tag(out, "mt.hash", &mt.hash);
        }
    }
    out.push('\n');
}

/// Renders an internal [`Catalog`] as FCL text, sorted canonically by (id, ctxt).
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
    out.push('\n');

    let mut messages: Vec<&CanonicalMessage> = catalog.messages.iter().collect();
    messages.sort_by_cached_key(|message| (fcl_id(message).into_owned(), message.msgctxt.clone()));
    for message in messages {
        write_entry(&mut out, message, render);
    }
    out
}

// --- parse ----------------------------------------------------------------

fn is_conflict_marker(line: &str) -> bool {
    line.starts_with("<<<<<<<") || line.starts_with("=======") || line.starts_with(">>>>>>>")
}

fn parse_header(line: &str, source_locale: &str) -> Result<Option<String>, ApiError> {
    let mut fields = SplitTab::new(line.as_bytes());
    if fields.next() != Some(FCL_MAGIC.as_bytes()) {
        return Err(ApiError::InvalidArguments(
            "FCL catalog must start with the `%FCL1` header".to_owned(),
        ));
    }
    let mut locale = None;
    let mut declared_source: Option<String> = None;
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
    Ok(locale)
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
    let mut raw_comments: Vec<String> = Vec::new();
    let mut translator_comments: Vec<String> = Vec::new();
    let mut flags: Vec<String> = Vec::new();
    let mut obsolete = false;
    let mut mt_model = None;
    let mut mt_conf = None;
    let mut mt_hash = None;
    let mut last_tag_rank = 0_u8;

    for tag in fields {
        if tag == b"o" {
            validate_tag_order(&mut last_tag_rank, 4, "o")?;
            if obsolete {
                return Err(ApiError::InvalidArguments(
                    "duplicate FCL tag `o`".to_owned(),
                ));
            }
            obsolete = true;
            continue;
        }
        let (key, raw_value) = split_once_byte(tag, b'=').ok_or_else(|| {
            ApiError::InvalidArguments(format!("invalid FCL tag {:?}", input_slice_as_str(tag)))
        })?;
        // Keep the unescaped value borrowed; only allocate (`into_owned`) for tags
        // whose value is stored. `mt.conf` is parsed in place and never allocates.
        let value = unescape(input_slice_as_str(raw_value))?;
        match key {
            b"r" => {
                validate_tag_order(&mut last_tag_rank, 0, "r")?;
                origins.push(parse_origin(value.into_owned()));
            }
            b"c" => {
                validate_tag_order(&mut last_tag_rank, 1, "c")?;
                raw_comments.push(value.into_owned());
            }
            b"tc" => {
                validate_tag_order(&mut last_tag_rank, 2, "tc")?;
                translator_comments.push(value.into_owned());
            }
            b"f" => {
                validate_tag_order(&mut last_tag_rank, 3, "f")?;
                flags.push(value.into_owned());
            }
            b"mt.model" => {
                validate_tag_order(&mut last_tag_rank, 5, "mt.model")?;
                if mt_model.is_some() {
                    return Err(ApiError::InvalidArguments(
                        "duplicate FCL tag `mt.model`".to_owned(),
                    ));
                }
                mt_model = Some(value.into_owned());
            }
            b"mt.conf" => {
                validate_tag_order(&mut last_tag_rank, 6, "mt.conf")?;
                if mt_conf.is_some() {
                    return Err(ApiError::InvalidArguments(
                        "duplicate FCL tag `mt.conf`".to_owned(),
                    ));
                }
                mt_conf = Some(value.parse::<u8>().map_err(|_| {
                    ApiError::InvalidArguments(format!("invalid FCL mt.conf value {value:?}"))
                })?);
            }
            b"mt.hash" => {
                validate_tag_order(&mut last_tag_rank, 7, "mt.hash")?;
                if mt_hash.is_some() {
                    return Err(ApiError::InvalidArguments(
                        "duplicate FCL tag `mt.hash`".to_owned(),
                    ));
                }
                mt_hash = Some(value.into_owned());
            }
            other => {
                return Err(ApiError::InvalidArguments(format!(
                    "unknown FCL tag key {:?}",
                    input_slice_as_str(other)
                )));
            }
        }
    }

    let machine_translation = if mt_model.is_some() || mt_hash.is_some() || mt_conf.is_some() {
        let metadata = MachineTranslationMetadata {
            model: mt_model.ok_or_else(|| {
                ApiError::InvalidArguments(
                    "FCL `mt.model` is required when `mt.*` present".to_owned(),
                )
            })?,
            modified: None,
            confidence: mt_conf,
            hash: mt_hash.ok_or_else(|| {
                ApiError::InvalidArguments(
                    "FCL `mt.hash` is required when `mt.*` present".to_owned(),
                )
            })?,
        };
        validate_machine_translation_metadata(&metadata)?;
        Some(metadata)
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
        machine_translation,
        translator_comments,
        flags,
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

/// Builds a [`CatalogOrigin`] from a reference value, keeping only the file. A
/// trailing `:line` is stripped so line numbers never enter the catalog model.
fn parse_origin(mut value: String) -> CatalogOrigin {
    if let Some((file, line)) = value.rsplit_once(':')
        && !line.is_empty()
        && line.bytes().all(|byte| byte.is_ascii_digit())
    {
        let file_len = file.len();
        value.truncate(file_len);
    }
    CatalogOrigin { file: value }
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
    let frontmatter_locale = parse_header(header, source_locale)?;
    let locale = locale_override.map(str::to_owned).or(frontmatter_locale);

    // The entry count is at most the newline count; over-reserving by the header
    // and any blank lines only saves reallocations.
    let estimated_entries = memchr::memchr_iter(b'\n', content.as_bytes()).count();
    let mut messages: Vec<CanonicalMessage> = Vec::with_capacity(estimated_entries);
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
        // FCL entries are canonically sorted by (id, ctxt), so duplicates are
        // adjacent: comparing against the previous entry detects both
        // duplicates and out-of-order corruption without cloning keys or
        // maintaining a set.
        if let Some(previous) = messages.last() {
            match (message.msgid.as_str(), message.msgctxt.as_deref())
                .cmp(&(previous.msgid.as_str(), previous.msgctxt.as_deref()))
            {
                Ordering::Greater => {}
                Ordering::Equal => {
                    return Err(ApiError::Conflict(format!(
                        "duplicate FCL entry for id {:?} and context {:?}",
                        message.msgid, message.msgctxt
                    )));
                }
                Ordering::Less => {
                    return Err(ApiError::InvalidArguments(format!(
                        "FCL entries must be sorted by (id, ctxt); line {} is out of order",
                        index + 1
                    )));
                }
            }
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
    fn canonical_text_is_a_fixpoint() {
        // Header + two sorted entries with tabs/newlines escaped and canonical
        // tag order. Parsing then re-serializing must reproduce it byte-for-byte.
        // The MT block is only retained when its hash matches the translation
        // (stale-protection), so compute the real hash for the target.
        let hash = crate::machine_translation_hash(crate::EffectiveTranslationRef::Singular(
            "Hallo {name}",
        ));
        let text = format!(
            "{FCL_MAGIC}\tsource=en\tlocale=de\n\
             greeting\t\tHallo {{name}}\tr=src/a.tsx\tmt.model=openai/gpt-5.5\tmt.conf=88\tmt.hash={hash}\n\
             tabbed\tmenu\tWert mit\\tTab\tf=fuzzy\n"
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
    fn accepts_canonically_sorted_entries() {
        // Already-sorted by (id, ctxt): empty context sorts before "menu".
        let text = format!("{FCL_MAGIC}\tsource=en\nalpha\t\tA\nalpha\tmenu\tA2\nbeta\t\tB\n");
        let sorted = roundtrip(&text);
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
    fn roundtrips_comments_flags_obsolete_and_escapes() {
        // Covers the c=/tc=/o serialize branches and the \t \n \\ escape paths
        // in both directions on a single entry.
        let text = format!(
            "{FCL_MAGIC}\tsource=en\n\
             a.tab\\there\t\tWert\\nZeile\\\\back\tc=extracted\ttc=translator\to\n"
        );
        assert_eq!(roundtrip(&text), text);
    }

    #[test]
    fn roundtrips_reference_without_line_number() {
        // Covers the line-less origin path on both parse and serialize.
        let text = format!("{FCL_MAGIC}\tsource=en\nid\t\tv\tr=README\n");
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
        assert!(parse_err("")); // no header at all
    }

    #[test]
    fn rejects_malformed_entries() {
        assert!(parse_err(&format!("{FCL_MAGIC}\tsource=en\njustid\n"))); // missing ctxt/target
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\nid\t\tv\tbogus\n"
        ))); // tag without `=`
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\nid\t\tv\tmt.conf=nope\n"
        ))); // bad conf
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\nid\t\tv\tmt.model=m\n"
        ))); // mt without hash
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\nid\t\tv\tmt.hash=h\n"
        ))); // mt without model
    }

    #[test]
    fn rejects_duplicate_singleton_and_out_of_order_tags() {
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\nid\t\tv\to\to\n"
        )));
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\nid\t\tv\tmt.model=a\tmt.model=b\n"
        )));
        assert!(parse_err(&format!(
            "{FCL_MAGIC}\tsource=en\nid\t\tv\tf=fuzzy\tc=late-comment\n"
        )));
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
            obsolete: false,
            machine_translation: Some(super::MachineTranslationMetadata {
                model: "example/mt".to_owned(),
                modified: None,
                confidence: Some(90),
                hash,
            }),
            translator_comments: Vec::new(),
            flags: Vec::new(),
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
        // Both id and target columns are synthesized ICU plural strings, and the
        // MT block survives because its hash matches the plural translation.
        assert!(text.contains("{count, plural,"));
        assert!(text.contains("{count} Dateien"));
        assert!(text.contains("mt.model=example/mt"));

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
            obsolete: false,
            machine_translation: Some(super::MachineTranslationMetadata {
                model: "example/mt".to_owned(),
                modified: None,
                confidence: Some(90),
                hash: stale,
            }),
            translator_comments: Vec::new(),
            flags: Vec::new(),
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
