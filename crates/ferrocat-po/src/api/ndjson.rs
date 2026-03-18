use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::catalog::{
    CanonicalMessage, CanonicalTranslation, Catalog, append_placeholder_comments,
    plural_source_branches, split_placeholder_comments, textual_translation_from_strings,
};
use super::plural::synthesize_icu_plural;
use super::{ApiError, CatalogOrigin, PlaceholderCommentMode, PluralEncoding};

const FRONTMATTER_DELIMITER: &str = "---";
const NDJSON_FORMAT: &str = "ferrocat.ndjson.v1";

#[derive(Debug, Default)]
struct Frontmatter {
    locale: Option<String>,
    source_locale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NdjsonRecord {
    id: String,
    str: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ctx: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    comments: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    origin: Vec<NdjsonOrigin>,
    #[serde(default, skip_serializing_if = "is_false")]
    obsolete: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    extra: Option<NdjsonExtra>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NdjsonOrigin {
    file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    line: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct NdjsonExtra {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    translator_comments: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    flags: Vec<String>,
}

pub(super) fn parse_catalog_to_internal_ndjson(
    content: &str,
    locale_override: Option<&str>,
    source_locale: &str,
    strict: bool,
) -> Result<Catalog, ApiError> {
    let normalized = normalize_input(content);
    let (frontmatter, body) = parse_frontmatter(normalized.as_ref())?;
    if let Some(header_source_locale) = &frontmatter.source_locale
        && header_source_locale != source_locale
    {
        return Err(ApiError::InvalidArguments(format!(
            "NDJSON source_locale {:?} did not match requested source_locale {:?}",
            header_source_locale, source_locale
        )));
    }

    let locale = locale_override.map(str::to_owned).or(frontmatter.locale);

    let mut diagnostics = Vec::new();
    let mut seen = BTreeSet::<(String, Option<String>)>::new();
    let mut messages = Vec::new();

    for (index, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let record = serde_json::from_str::<NdjsonRecord>(trimmed).map_err(|error| {
            ApiError::InvalidArguments(format!(
                "invalid NDJSON record on line {}: {error}",
                index + 1
            ))
        })?;
        let key = (record.id.clone(), record.ctx.clone());
        if !seen.insert(key.clone()) {
            return Err(ApiError::Conflict(format!(
                "duplicate NDJSON message for id {:?} and context {:?}",
                key.0, key.1
            )));
        }

        let (comments, placeholders) = split_placeholder_comments(record.comments);
        let translation = textual_translation_from_strings(
            &record.id,
            record.ctx.as_deref(),
            &record.str,
            PluralEncoding::Icu,
            strict,
            &mut diagnostics,
        )?;

        let extra = record.extra.unwrap_or_default();
        messages.push(CanonicalMessage {
            msgid: record.id,
            msgctxt: record.ctx,
            translation,
            comments,
            origins: record
                .origin
                .into_iter()
                .map(|origin| CatalogOrigin {
                    file: origin.file,
                    line: origin.line,
                })
                .collect(),
            placeholders,
            obsolete: record.obsolete,
            translator_comments: extra.translator_comments,
            flags: extra.flags,
        });
    }

    Ok(Catalog {
        locale,
        headers: BTreeMap::new(),
        file_comments: Vec::new(),
        file_extracted_comments: Vec::new(),
        messages,
        diagnostics,
    })
}

pub(super) fn stringify_catalog_ndjson(
    catalog: &Catalog,
    locale: Option<&str>,
    source_locale: &str,
    placeholder_comment_mode: &PlaceholderCommentMode,
) -> String {
    let mut rendered = String::new();
    rendered.push_str(FRONTMATTER_DELIMITER);
    rendered.push('\n');
    rendered.push_str("format: ");
    rendered.push_str(NDJSON_FORMAT);
    rendered.push('\n');
    if let Some(locale) = locale {
        rendered.push_str("locale: ");
        rendered.push_str(locale);
        rendered.push('\n');
    }
    rendered.push_str("source_locale: ");
    rendered.push_str(source_locale);
    rendered.push('\n');
    rendered.push_str(FRONTMATTER_DELIMITER);
    rendered.push('\n');

    for message in &catalog.messages {
        let mut comments = message.comments.clone();
        append_placeholder_comments(
            &mut comments,
            &message.placeholders,
            placeholder_comment_mode,
        );

        let record = NdjsonRecord {
            id: ndjson_id(message),
            str: ndjson_translation(message),
            ctx: message.msgctxt.clone(),
            comments,
            origin: message
                .origins
                .iter()
                .map(|origin| NdjsonOrigin {
                    file: origin.file.clone(),
                    line: origin.line,
                })
                .collect(),
            obsolete: message.obsolete,
            extra: ndjson_extra(message),
        };
        rendered.push_str(
            &serde_json::to_string(&record).expect("NDJSON record serialization must succeed"),
        );
        rendered.push('\n');
    }

    rendered
}

fn ndjson_id(message: &CanonicalMessage) -> String {
    match &message.translation {
        CanonicalTranslation::Singular { .. } => message.msgid.clone(),
        CanonicalTranslation::Plural {
            source, variable, ..
        } => synthesize_icu_plural(variable, &plural_source_branches(source)),
    }
}

fn ndjson_translation(message: &CanonicalMessage) -> String {
    match &message.translation {
        CanonicalTranslation::Singular { value } => value.clone(),
        CanonicalTranslation::Plural {
            translation_by_category,
            variable,
            ..
        } => synthesize_icu_plural(variable, translation_by_category),
    }
}

fn ndjson_extra(message: &CanonicalMessage) -> Option<NdjsonExtra> {
    if message.translator_comments.is_empty() && message.flags.is_empty() {
        None
    } else {
        Some(NdjsonExtra {
            translator_comments: message.translator_comments.clone(),
            flags: message.flags.clone(),
        })
    }
}

fn parse_frontmatter(input: &str) -> Result<(Frontmatter, &str), ApiError> {
    let Some((first_line, mut cursor)) = take_line(input, 0) else {
        return Err(ApiError::InvalidArguments(
            "NDJSON catalog must start with a frontmatter block".to_owned(),
        ));
    };
    if first_line.trim() != FRONTMATTER_DELIMITER {
        return Err(ApiError::InvalidArguments(
            "NDJSON catalog must start with `---`".to_owned(),
        ));
    }

    let mut header = Frontmatter::default();
    let mut seen = BTreeSet::new();

    while let Some((line, next_cursor)) = take_line(input, cursor) {
        if line.trim() == FRONTMATTER_DELIMITER {
            let body = input.get(next_cursor..).unwrap_or_default();
            if !seen.contains("format") {
                return Err(ApiError::InvalidArguments(
                    "NDJSON frontmatter is missing required `format`".to_owned(),
                ));
            }
            return Ok((header, body));
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            cursor = next_cursor;
            continue;
        }
        let (key, value) = trimmed.split_once(':').ok_or_else(|| {
            ApiError::InvalidArguments(format!("invalid NDJSON frontmatter line: {trimmed:?}"))
        })?;
        let key = key.trim();
        let value = value.trim();
        if !seen.insert(key.to_owned()) {
            return Err(ApiError::InvalidArguments(format!(
                "duplicate NDJSON frontmatter key {key:?}"
            )));
        }

        match key {
            "format" => {
                if value != NDJSON_FORMAT {
                    return Err(ApiError::InvalidArguments(format!(
                        "unsupported NDJSON format {:?}; expected {:?}",
                        value, NDJSON_FORMAT
                    )));
                }
            }
            "locale" => header.locale = Some(value.to_owned()),
            "source_locale" => header.source_locale = Some(value.to_owned()),
            other => {
                return Err(ApiError::InvalidArguments(format!(
                    "unknown NDJSON frontmatter key {other:?}"
                )));
            }
        }
        cursor = next_cursor;
    }

    Err(ApiError::InvalidArguments(
        "NDJSON frontmatter was not closed with `---`".to_owned(),
    ))
}

fn normalize_input(input: &str) -> std::borrow::Cow<'_, str> {
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    if input.as_bytes().contains(&b'\r') {
        std::borrow::Cow::Owned(input.replace("\r\n", "\n").replace('\r', "\n"))
    } else {
        std::borrow::Cow::Borrowed(input)
    }
}

const fn is_false(value: &bool) -> bool {
    !*value
}

fn take_line(input: &str, start: usize) -> Option<(&str, usize)> {
    if start >= input.len() {
        return None;
    }
    match input[start..].find('\n') {
        Some(offset) => {
            let end = start + offset;
            Some((&input[start..end], end + 1))
        }
        None => Some((&input[start..], input.len())),
    }
}
