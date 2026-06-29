//! FCL (Ferrocat Catalog Lines) codec — see `docs/fcl-format.md`.
//!
//! A line-oriented, machine-owned catalog encoding optimized for git merge and
//! fast parsing: one entry per line, deterministically sorted by `(id, ctxt)`,
//! minimal escaping (`\n \t \\`). FCL is a different *encoding* of the same
//! record the NDJSON path uses, so it reuses
//! [`canonical_message_from_record`]/[`ndjson_record_from_canonical`] and stays
//! equivalent to NDJSON by construction.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use super::catalog::Catalog;
use super::mt::MachineTranslationMetadata;
use super::ndjson::{
    NdjsonOrigin, NdjsonRecord, canonical_message_from_record, ndjson_record_from_canonical,
};
use super::{ApiError, CatalogSemantics, PlaceholderCommentMode};

const FCL_MAGIC: &str = "%FCL1";

// --- escaping -------------------------------------------------------------

fn escape_into(out: &mut String, value: &str) {
    if !value
        .bytes()
        .any(|byte| matches!(byte, b'\\' | b'\t' | b'\n'))
    {
        out.push_str(value);
        return;
    }
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            other => out.push(other),
        }
    }
}

fn unescape(value: &str) -> Result<Cow<'_, str>, ApiError> {
    if !value.contains('\\') {
        return Ok(Cow::Borrowed(value));
    }
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some(other) => {
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
    Ok(Cow::Owned(out))
}

// --- serialize ------------------------------------------------------------

fn write_tag(out: &mut String, key: &str, value: &str) {
    out.push('\t');
    out.push_str(key);
    out.push('=');
    escape_into(out, value);
}

fn write_entry(out: &mut String, record: &NdjsonRecord) {
    escape_into(out, &record.id);
    out.push('\t');
    escape_into(out, record.ctx.as_deref().unwrap_or(""));
    out.push('\t');
    escape_into(out, &record.str);

    // Canonical tag order: r (sorted), c, tc, f (sorted), o, mt.*
    let mut refs = record
        .origin
        .iter()
        .map(|origin| match origin.line {
            Some(line) => format!("{}:{line}", origin.file),
            None => origin.file.clone(),
        })
        .collect::<Vec<_>>();
    refs.sort_unstable();
    for reference in &refs {
        write_tag(out, "r", reference);
    }
    for comment in &record.comments {
        write_tag(out, "c", comment);
    }
    if let Some(extra) = &record.extra {
        for comment in &extra.translator_comments {
            write_tag(out, "tc", comment);
        }
        let mut flags = extra.flags.clone();
        flags.sort_unstable();
        for flag in &flags {
            write_tag(out, "f", flag);
        }
    }
    if record.obsolete {
        out.push_str("\to");
    }
    if let Some(mt) = &record.mt {
        write_tag(out, "mt.model", &mt.model);
        if let Some(confidence) = mt.confidence {
            out.push_str("\tmt.conf=");
            out.push_str(&confidence.to_string());
        }
        write_tag(out, "mt.hash", &mt.hash);
    }
    out.push('\n');
}

/// Renders an internal [`Catalog`] as FCL text.
pub(super) fn stringify_catalog_fcl(
    catalog: &Catalog,
    locale: Option<&str>,
    source_locale: &str,
    placeholder_comment_mode: &PlaceholderCommentMode,
) -> String {
    let mut out = String::new();
    out.push_str(FCL_MAGIC);
    write_tag(&mut out, "source", source_locale);
    if let Some(locale) = locale {
        write_tag(&mut out, "locale", locale);
    }
    out.push('\n');

    let mut records = catalog
        .messages
        .iter()
        .map(|message| ndjson_record_from_canonical(message, placeholder_comment_mode))
        .collect::<Vec<_>>();
    records.sort_by(|a, b| (&a.id, &a.ctx).cmp(&(&b.id, &b.ctx)));
    for record in &records {
        write_entry(&mut out, record);
    }
    out
}

// --- parse ----------------------------------------------------------------

fn is_conflict_marker(line: &str) -> bool {
    line.starts_with("<<<<<<<") || line.starts_with("=======") || line.starts_with(">>>>>>>")
}

fn parse_header(line: &str, source_locale: &str) -> Result<Option<String>, ApiError> {
    let mut fields = line.split('\t');
    if fields.next() != Some(FCL_MAGIC) {
        return Err(ApiError::InvalidArguments(
            "FCL catalog must start with the `%FCL1` header".to_owned(),
        ));
    }
    let mut locale = None;
    let mut declared_source: Option<String> = None;
    for tag in fields {
        let (key, value) = tag
            .split_once('=')
            .ok_or_else(|| ApiError::InvalidArguments(format!("invalid FCL header tag {tag:?}")))?;
        let value = unescape(value)?.into_owned();
        match key {
            "source" => declared_source = Some(value),
            "locale" => locale = Some(value),
            other => {
                return Err(ApiError::InvalidArguments(format!(
                    "unknown FCL header key {other:?}"
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

fn parse_entry(line: &str) -> Result<NdjsonRecord, ApiError> {
    let mut fields = line.split('\t');
    let id = unescape(field(&mut fields, "id")?)?.into_owned();
    let ctx_raw = unescape(field(&mut fields, "ctxt")?)?.into_owned();
    let target = unescape(field(&mut fields, "target")?)?.into_owned();

    let mut record = NdjsonRecord {
        id,
        str: target,
        ctx: (!ctx_raw.is_empty()).then_some(ctx_raw),
        comments: Vec::new(),
        origin: Vec::new(),
        obsolete: false,
        extra: None,
        mt: None,
    };
    let mut mt_model = None;
    let mut mt_conf = None;
    let mut mt_hash = None;

    for tag in fields {
        if tag == "o" {
            record.obsolete = true;
            continue;
        }
        let (key, value) = tag
            .split_once('=')
            .ok_or_else(|| ApiError::InvalidArguments(format!("invalid FCL tag {tag:?}")))?;
        let value = unescape(value)?.into_owned();
        match key {
            "r" => record.origin.push(parse_origin(value)),
            "c" => record.comments.push(value),
            "tc" => record
                .extra
                .get_or_insert_with(Default::default)
                .translator_comments
                .push(value),
            "f" => record
                .extra
                .get_or_insert_with(Default::default)
                .flags
                .push(value),
            "mt.model" => mt_model = Some(value),
            "mt.conf" => {
                mt_conf = Some(value.parse::<u8>().map_err(|_| {
                    ApiError::InvalidArguments(format!("invalid FCL mt.conf value {value:?}"))
                })?);
            }
            "mt.hash" => mt_hash = Some(value),
            other => {
                return Err(ApiError::InvalidArguments(format!(
                    "unknown FCL tag key {other:?}"
                )));
            }
        }
    }

    if mt_model.is_some() || mt_hash.is_some() || mt_conf.is_some() {
        record.mt = Some(MachineTranslationMetadata {
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
        });
    }

    Ok(record)
}

fn field<'a>(fields: &mut impl Iterator<Item = &'a str>, name: &str) -> Result<&'a str, ApiError> {
    fields
        .next()
        .ok_or_else(|| ApiError::InvalidArguments(format!("FCL entry is missing the {name} field")))
}

fn parse_origin(value: String) -> NdjsonOrigin {
    match value.rsplit_once(':') {
        Some((file, line))
            if !line.is_empty() && line.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            NdjsonOrigin {
                file: file.to_owned(),
                line: line.parse().ok(),
            }
        }
        _ => NdjsonOrigin {
            file: value,
            line: None,
        },
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
    let frontmatter_locale = parse_header(header, source_locale)?;
    let locale = locale_override.map(str::to_owned).or(frontmatter_locale);

    let mut messages = Vec::new();
    let mut seen = BTreeSet::<(String, Option<String>)>::new();
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
        let record = parse_entry(line).map_err(|error| {
            ApiError::InvalidArguments(format!("invalid FCL entry on line {}: {error}", index + 1))
        })?;
        if !seen.insert((record.id.clone(), record.ctx.clone())) {
            return Err(ApiError::Conflict(format!(
                "duplicate FCL entry for id {:?} and context {:?}",
                record.id, record.ctx
            )));
        }
        messages.push(canonical_message_from_record(record)?);
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
    use crate::api::types::PlaceholderCommentMode;

    fn roundtrip(text: &str) -> String {
        let catalog =
            parse_catalog_to_internal_fcl(text, None, "en", CatalogSemantics::IcuNative, false)
                .expect("parse FCL");
        stringify_catalog_fcl(
            &catalog,
            catalog.locale.as_deref(),
            "en",
            &PlaceholderCommentMode::default(),
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
             greeting\t\tHallo {{name}}\tr=src/a.tsx:12\tmt.model=openai/gpt-5.5\tmt.conf=88\tmt.hash={hash}\n\
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
    fn entries_are_sorted_by_id_then_ctxt() {
        let text = format!("{FCL_MAGIC}\tsource=en\nbeta\t\tB\nalpha\t\tA\nalpha\tmenu\tA2\n");
        let sorted = roundtrip(&text);
        let ids: Vec<&str> = sorted
            .lines()
            .skip(1)
            .map(|line| line.split('\t').next().unwrap_or_default())
            .collect();
        assert_eq!(ids, ["alpha", "alpha", "beta"]);
    }
}
