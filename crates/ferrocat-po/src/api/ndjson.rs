use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, Write};

use serde::{Deserialize, Serialize};

use super::catalog::{
    CanonicalMessage, CanonicalTranslation, Catalog, public_message_from_canonical,
    split_placeholder_comments,
};
use super::export::{append_placeholder_comments, plural_source_branches};
use super::mt::{
    MachineTranslationMetadata, machine_translation_hash, validate_machine_translation_metadata,
};
use super::plural::synthesize_icu_plural;
use super::{
    ApiError, CatalogMessage, CatalogMessageExtra, CatalogOrigin, CatalogSemantics,
    EffectiveTranslationRef, PlaceholderCommentMode, TranslationShape,
};

const FRONTMATTER_DELIMITER: &str = "---";
const NDJSON_FORMAT_V1: &str = "ferrocat.ndjson.v1";

/// Options for constructing an [`NdjsonCatalogReader`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NdjsonCatalogReaderOptions<'a> {
    /// Optional explicit locale override. When `None`, the reader uses the
    /// locale declared in NDJSON frontmatter.
    pub locale: Option<&'a str>,
    /// Source locale expected by the caller.
    pub source_locale: &'a str,
}

impl<'a> NdjsonCatalogReaderOptions<'a> {
    /// Creates reader options with the required source locale set.
    #[must_use]
    pub const fn new(source_locale: &'a str) -> Self {
        Self {
            locale: None,
            source_locale,
        }
    }
}

/// Streaming reader for Ferrocat NDJSON catalog records.
///
/// The reader consumes and validates frontmatter during construction, then
/// yields one [`CatalogMessage`] per non-empty NDJSON body line.
pub struct NdjsonCatalogReader<R> {
    inner: NdjsonCanonicalReader<R>,
}

impl<R: BufRead> NdjsonCatalogReader<R> {
    /// Creates a streaming reader with the required source locale.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when frontmatter cannot be read, the source locale
    /// is empty, or a declared NDJSON `source_locale` does not match.
    pub fn new(reader: R, source_locale: &str) -> Result<Self, ApiError> {
        Self::with_options(reader, NdjsonCatalogReaderOptions::new(source_locale))
    }

    /// Creates a streaming reader with explicit options.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when frontmatter cannot be read, the source locale
    /// is empty, or a declared NDJSON `source_locale` does not match.
    pub fn with_options(
        reader: R,
        options: NdjsonCatalogReaderOptions<'_>,
    ) -> Result<Self, ApiError> {
        Ok(Self {
            inner: NdjsonCanonicalReader::with_options(reader, options)?,
        })
    }

    /// Returns the effective catalog locale after applying the optional
    /// override.
    #[must_use]
    pub fn locale(&self) -> Option<&str> {
        self.inner.locale.as_deref()
    }

    /// Returns a shared reference to the wrapped reader.
    #[must_use]
    pub const fn get_ref(&self) -> &R {
        &self.inner.reader
    }

    /// Returns a mutable reference to the wrapped reader.
    pub const fn get_mut(&mut self) -> &mut R {
        &mut self.inner.reader
    }

    /// Consumes the streaming reader and returns the wrapped reader.
    #[must_use]
    pub fn into_inner(self) -> R {
        self.inner.reader
    }
}

impl<R: BufRead> Iterator for NdjsonCatalogReader<R> {
    type Item = Result<CatalogMessage, ApiError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|message| message.map(public_message_from_canonical))
    }
}

/// Options for constructing an [`NdjsonCatalogWriter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NdjsonCatalogWriterOptions<'a> {
    /// Locale to write into NDJSON frontmatter.
    pub locale: Option<&'a str>,
    /// Source locale to write into NDJSON frontmatter.
    pub source_locale: &'a str,
}

impl<'a> NdjsonCatalogWriterOptions<'a> {
    /// Creates writer options with the required source locale set.
    #[must_use]
    pub const fn new(source_locale: &'a str) -> Self {
        Self {
            locale: None,
            source_locale,
        }
    }
}

/// Streaming writer for Ferrocat NDJSON catalog records.
///
/// The writer emits frontmatter during construction and then appends one JSON
/// record per [`CatalogMessage`] written.
pub struct NdjsonCatalogWriter<W> {
    writer: W,
}

impl<W: Write> NdjsonCatalogWriter<W> {
    /// Creates a streaming writer with the required source locale.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the source locale is empty or the frontmatter
    /// cannot be written.
    pub fn new(writer: W, source_locale: &str) -> Result<Self, ApiError> {
        Self::with_options(writer, NdjsonCatalogWriterOptions::new(source_locale))
    }

    /// Creates a streaming writer with explicit options.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the source locale is empty or the frontmatter
    /// cannot be written.
    pub fn with_options(
        mut writer: W,
        options: NdjsonCatalogWriterOptions<'_>,
    ) -> Result<Self, ApiError> {
        super::validate_source_locale(options.source_locale)?;
        write_frontmatter(&mut writer, options.locale, options.source_locale)?;
        Ok(Self { writer })
    }

    /// Writes one catalog message as an NDJSON body record.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the underlying writer fails.
    pub fn write_message(&mut self, message: &CatalogMessage) -> Result<(), ApiError> {
        let record = ndjson_record_from_public_message(message);
        write_record(&mut self.writer, &record)
    }

    /// Returns a shared reference to the wrapped writer.
    #[must_use]
    pub const fn get_ref(&self) -> &W {
        &self.writer
    }

    /// Returns a mutable reference to the wrapped writer.
    pub const fn get_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    /// Flushes and returns the wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when flushing the underlying writer fails.
    pub fn finish(mut self) -> Result<W, ApiError> {
        self.writer.flush()?;
        Ok(self.writer)
    }
}

#[derive(Debug, Default)]
struct Frontmatter {
    locale: Option<String>,
    source_locale: Option<String>,
}

struct NdjsonCanonicalReader<R> {
    reader: R,
    locale: Option<String>,
    line_number: usize,
    seen: BTreeSet<(String, Option<String>)>,
}

impl<R: BufRead> NdjsonCanonicalReader<R> {
    fn with_options(
        mut reader: R,
        options: NdjsonCatalogReaderOptions<'_>,
    ) -> Result<Self, ApiError> {
        super::validate_source_locale(options.source_locale)?;
        let mut line_number = 0;
        let frontmatter = read_frontmatter(&mut reader, &mut line_number)?;
        if let Some(header_source_locale) = &frontmatter.source_locale
            && header_source_locale != options.source_locale
        {
            return Err(ApiError::InvalidArguments(format!(
                "NDJSON source_locale {:?} did not match requested source_locale {:?}",
                header_source_locale, options.source_locale
            )));
        }

        Ok(Self {
            reader,
            locale: options.locale.map(str::to_owned).or(frontmatter.locale),
            line_number,
            seen: BTreeSet::new(),
        })
    }
}

impl<R: BufRead> Iterator for NdjsonCanonicalReader<R> {
    type Item = Result<CanonicalMessage, ApiError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let line = match read_line(&mut self.reader, &mut self.line_number) {
                Ok(Some(line)) => line,
                Ok(None) => return None,
                Err(error) => return Some(Err(error)),
            };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let record = match serde_json::from_str::<NdjsonRecord>(trimmed) {
                Ok(record) => record,
                Err(error) => {
                    return Some(Err(ApiError::InvalidArguments(format!(
                        "invalid NDJSON record on line {}: {error}",
                        self.line_number
                    ))));
                }
            };
            let key = (record.id.clone(), record.ctx.clone());
            if !self.seen.insert(key.clone()) {
                return Some(Err(ApiError::Conflict(format!(
                    "duplicate NDJSON message for id {:?} and context {:?}",
                    key.0, key.1
                ))));
            }

            return Some(canonical_message_from_record(record));
        }
    }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mt: Option<MachineTranslationMetadata>,
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
    semantics: CatalogSemantics,
    _strict: bool,
) -> Result<Catalog, ApiError> {
    if semantics != CatalogSemantics::IcuNative {
        return Err(ApiError::Unsupported(
            "CatalogSemantics::GettextCompat is not supported for NDJSON catalogs".to_owned(),
        ));
    }

    let normalized = normalize_input(content);
    let mut reader = NdjsonCanonicalReader::with_options(
        std::io::Cursor::new(normalized.as_bytes()),
        NdjsonCatalogReaderOptions {
            locale: locale_override,
            source_locale,
        },
    )?;
    let locale = reader.locale.clone();
    let mut messages = Vec::new();
    for message in &mut reader {
        messages.push(message?);
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

pub(super) fn stringify_catalog_ndjson(
    catalog: &Catalog,
    locale: Option<&str>,
    source_locale: &str,
    placeholder_comment_mode: &PlaceholderCommentMode,
) -> String {
    let mut rendered = Vec::new();
    write_frontmatter(&mut rendered, locale, source_locale)
        .expect("writing NDJSON frontmatter into a Vec must succeed");
    for message in &catalog.messages {
        let record = ndjson_record_from_canonical(message, placeholder_comment_mode);
        write_record(&mut rendered, &record)
            .expect("writing NDJSON record into a Vec must succeed");
    }
    String::from_utf8(rendered).expect("NDJSON renderer writes UTF-8")
}

fn canonical_message_from_record(record: NdjsonRecord) -> Result<CanonicalMessage, ApiError> {
    let (comments, placeholders) = split_placeholder_comments(record.comments);
    let extra = record.extra.unwrap_or_default();
    if let Some(metadata) = &record.mt {
        validate_machine_translation_metadata(metadata)?;
    }
    Ok(CanonicalMessage {
        msgid: record.id,
        msgctxt: record.ctx,
        translation: CanonicalTranslation::Singular { value: record.str },
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
        machine_translation: record.mt,
        translator_comments: extra.translator_comments,
        flags: extra.flags,
    })
}

fn ndjson_record_from_canonical(
    message: &CanonicalMessage,
    placeholder_comment_mode: &PlaceholderCommentMode,
) -> NdjsonRecord {
    let mut comments = message.comments.clone();
    append_placeholder_comments(
        &mut comments,
        &message.placeholders,
        placeholder_comment_mode,
    );

    NdjsonRecord {
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
        mt: ndjson_machine_translation(message),
    }
}

fn ndjson_record_from_public_message(message: &CatalogMessage) -> NdjsonRecord {
    NdjsonRecord {
        id: ndjson_public_id(message),
        str: ndjson_public_translation(message),
        ctx: message.msgctxt.clone(),
        comments: message.comments.clone(),
        origin: message
            .origin
            .iter()
            .map(|origin| NdjsonOrigin {
                file: origin.file.clone(),
                line: origin.line,
            })
            .collect(),
        obsolete: message.obsolete,
        extra: ndjson_public_extra(message.extra.as_ref()),
        mt: ndjson_public_machine_translation(message),
    }
}

fn ndjson_machine_translation(message: &CanonicalMessage) -> Option<MachineTranslationMetadata> {
    let metadata = message.machine_translation.as_ref()?;
    if validate_machine_translation_metadata(metadata).is_err() {
        return None;
    }
    (metadata.hash == machine_translation_hash(ndjson_translation_ref(message)))
        .then(|| metadata.clone())
}

fn ndjson_translation_ref(message: &CanonicalMessage) -> EffectiveTranslationRef<'_> {
    match &message.translation {
        CanonicalTranslation::Singular { value } => EffectiveTranslationRef::Singular(value),
        CanonicalTranslation::Plural {
            translation_by_category,
            ..
        } => EffectiveTranslationRef::Plural(translation_by_category),
    }
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

fn ndjson_public_machine_translation(
    message: &CatalogMessage,
) -> Option<MachineTranslationMetadata> {
    let metadata = message.machine_translation.as_ref()?;
    if validate_machine_translation_metadata(metadata).is_err() {
        return None;
    }
    (metadata.hash == machine_translation_hash(message.effective_translation()))
        .then(|| metadata.clone())
}

fn ndjson_public_id(message: &CatalogMessage) -> String {
    match &message.translation {
        TranslationShape::Singular { .. } => message.msgid.clone(),
        TranslationShape::Plural {
            source, variable, ..
        } => synthesize_icu_plural(variable, &plural_source_branches(source)),
    }
}

fn ndjson_public_translation(message: &CatalogMessage) -> String {
    match &message.translation {
        TranslationShape::Singular { value } => value.clone(),
        TranslationShape::Plural {
            translation,
            variable,
            ..
        } => synthesize_icu_plural(variable, translation),
    }
}

fn ndjson_public_extra(extra: Option<&CatalogMessageExtra>) -> Option<NdjsonExtra> {
    let extra = extra?;
    if extra.translator_comments.is_empty() && extra.flags.is_empty() {
        None
    } else {
        Some(NdjsonExtra {
            translator_comments: extra.translator_comments.clone(),
            flags: extra.flags.clone(),
        })
    }
}

fn write_frontmatter<W: Write>(
    writer: &mut W,
    locale: Option<&str>,
    source_locale: &str,
) -> Result<(), ApiError> {
    writer.write_all(FRONTMATTER_DELIMITER.as_bytes())?;
    writer.write_all(b"\nformat: ")?;
    writer.write_all(NDJSON_FORMAT_V1.as_bytes())?;
    writer.write_all(b"\n")?;
    if let Some(locale) = locale {
        writer.write_all(b"locale: ")?;
        writer.write_all(locale.as_bytes())?;
        writer.write_all(b"\n")?;
    }
    writer.write_all(b"source_locale: ")?;
    writer.write_all(source_locale.as_bytes())?;
    writer.write_all(b"\n")?;
    writer.write_all(FRONTMATTER_DELIMITER.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

fn write_record<W: Write>(writer: &mut W, record: &NdjsonRecord) -> Result<(), ApiError> {
    let line = serde_json::to_string(record).expect("NDJSON record serialization must succeed");
    writer.write_all(line.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

fn read_frontmatter<R: BufRead>(
    reader: &mut R,
    line_number: &mut usize,
) -> Result<Frontmatter, ApiError> {
    let Some(first_line) = read_line(reader, line_number)? else {
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

    while let Some(line) = read_line(reader, line_number)? {
        if line.trim() == FRONTMATTER_DELIMITER {
            if !seen.contains("format") {
                return Err(ApiError::InvalidArguments(
                    "NDJSON frontmatter is missing required `format`".to_owned(),
                ));
            }
            return Ok(header);
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
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
                if value != NDJSON_FORMAT_V1 {
                    return Err(ApiError::InvalidArguments(format!(
                        "unsupported NDJSON format {:?}; expected {:?}",
                        value, NDJSON_FORMAT_V1
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
    }

    Err(ApiError::InvalidArguments(
        "NDJSON frontmatter was not closed with `---`".to_owned(),
    ))
}

fn read_line<R: BufRead>(
    reader: &mut R,
    line_number: &mut usize,
) -> Result<Option<String>, ApiError> {
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Ok(None);
    }
    *line_number += 1;
    if *line_number == 1 {
        line = line.strip_prefix('\u{feff}').unwrap_or(&line).to_owned();
    }
    if line.ends_with('\n') {
        line.pop();
    }
    if line.ends_with('\r') {
        line.pop();
    }
    Ok(Some(line))
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

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        io::{Cursor, Read},
    };

    use super::{
        CanonicalMessage, CanonicalTranslation, Catalog, CatalogMessage, CatalogMessageExtra,
        CatalogOrigin, CatalogSemantics, EffectiveTranslationRef, MachineTranslationMetadata,
        NDJSON_FORMAT_V1, NdjsonCatalogReader, NdjsonCatalogReaderOptions, NdjsonCatalogWriter,
        NdjsonCatalogWriterOptions, PlaceholderCommentMode, TranslationShape,
        machine_translation_hash, normalize_input, parse_catalog_to_internal_ndjson,
        read_frontmatter, read_line, stringify_catalog_ndjson,
    };

    fn sample_catalog() -> Catalog {
        Catalog {
            locale: Some("de".to_owned()),
            headers: BTreeMap::new(),
            file_comments: Vec::new(),
            file_extracted_comments: Vec::new(),
            messages: vec![
                CanonicalMessage {
                    msgid: "About us".to_owned(),
                    msgctxt: Some("nav".to_owned()),
                    translation: CanonicalTranslation::Singular {
                        value: "Ueber uns".to_owned(),
                    },
                    comments: vec!["Shown in nav".to_owned()],
                    origins: vec![CatalogOrigin {
                        file: "src/nav.rs".to_owned(),
                        line: Some(4),
                    }],
                    placeholders: BTreeMap::new(),
                    obsolete: false,
                    machine_translation: None,
                    translator_comments: vec!["Keep short".to_owned()],
                    flags: vec!["fuzzy".to_owned()],
                },
                CanonicalMessage {
                    msgid: "files".to_owned(),
                    msgctxt: None,
                    translation: CanonicalTranslation::Plural {
                        source: super::super::PluralSource {
                            one: Some("# file".to_owned()),
                            other: "# files".to_owned(),
                        },
                        translation_by_category: BTreeMap::from([
                            ("one".to_owned(), "# Datei".to_owned()),
                            ("other".to_owned(), "# Dateien".to_owned()),
                        ]),
                        variable: "count".to_owned(),
                    },
                    comments: Vec::new(),
                    origins: Vec::new(),
                    placeholders: BTreeMap::new(),
                    obsolete: true,
                    machine_translation: None,
                    translator_comments: Vec::new(),
                    flags: Vec::new(),
                },
            ],
            diagnostics: Vec::new(),
        }
    }

    #[test]
    fn frontmatter_parser_accepts_valid_blocks_and_rejects_invalid_ones() {
        let mut reader = Cursor::new(concat!(
            "---\n",
            "format: ferrocat.ndjson.v1\n",
            "locale: de\n",
            "source_locale: en\n",
            "---\n",
            "{\"id\":\"About us\",\"str\":\"Ueber uns\"}\n",
        ));
        let mut line_number = 0;
        let frontmatter = read_frontmatter(&mut reader, &mut line_number).expect("frontmatter");
        assert_eq!(frontmatter.locale.as_deref(), Some("de"));
        assert_eq!(frontmatter.source_locale.as_deref(), Some("en"));
        assert_eq!(line_number, 5);
        let mut body = String::new();
        reader.read_to_string(&mut body).expect("body");
        assert!(body.contains("\"About us\""));

        for invalid in [
            "format: ferrocat.ndjson.v1\n---\n",
            "---\nlocale: de\n---\n",
            "---\nformat: wrong\n---\n",
            "---\nformat: ferrocat.ndjson.v1\nformat: ferrocat.ndjson.v1\n---\n",
            "---\nformat: ferrocat.ndjson.v1\nunknown: value\n---\n",
            "---\nformat: ferrocat.ndjson.v1\n",
        ] {
            let mut reader = Cursor::new(invalid);
            let mut line_number = 0;
            assert!(
                read_frontmatter(&mut reader, &mut line_number).is_err(),
                "{invalid:?}"
            );
        }
    }

    #[test]
    fn normalize_input_and_streaming_line_reader_handle_bom_and_crlf() {
        assert_eq!(normalize_input("\u{feff}a\r\nb\r").as_ref(), "a\nb\n");

        let mut reader = Cursor::new("\u{feff}alpha\r\nbeta\n");
        let mut line_number = 0;
        assert_eq!(
            read_line(&mut reader, &mut line_number).expect("line"),
            Some("alpha".to_owned())
        );
        assert_eq!(
            read_line(&mut reader, &mut line_number).expect("line"),
            Some("beta".to_owned())
        );
        assert_eq!(read_line(&mut reader, &mut line_number).expect("eof"), None);
        assert_eq!(line_number, 2);
    }

    #[test]
    fn ndjson_roundtrip_keeps_comments_metadata_and_plural_rendering() {
        let rendered = stringify_catalog_ndjson(
            &sample_catalog(),
            Some("de"),
            "en",
            &PlaceholderCommentMode::Disabled,
        );
        assert!(rendered.contains(&format!("format: {NDJSON_FORMAT_V1}")));
        assert!(rendered.contains("\"ctx\":\"nav\""));
        assert!(rendered.contains("\"translator_comments\":[\"Keep short\"]"));
        assert!(rendered.contains("{count, plural, one {# Datei} other {# Dateien}}"));

        let parsed = parse_catalog_to_internal_ndjson(
            &rendered,
            None,
            "en",
            CatalogSemantics::IcuNative,
            false,
        )
        .expect("roundtrip parse");

        assert_eq!(parsed.locale.as_deref(), Some("de"));
        assert_eq!(parsed.messages.len(), 2);
        assert_eq!(parsed.messages[0].msgctxt.as_deref(), Some("nav"));
        assert_eq!(parsed.messages[0].origins[0].file, "src/nav.rs");
        assert_eq!(
            parsed.messages[0].translator_comments,
            vec!["Keep short".to_owned()]
        );
        assert_eq!(parsed.messages[0].flags, vec!["fuzzy".to_owned()]);
        assert!(parsed.messages[1].obsolete);
    }

    #[test]
    fn ndjson_streaming_reader_yields_public_catalog_messages() {
        let rendered = stringify_catalog_ndjson(
            &sample_catalog(),
            Some("de"),
            "en",
            &PlaceholderCommentMode::Disabled,
        );
        let mut reader = NdjsonCatalogReader::with_options(
            Cursor::new(rendered.as_bytes()),
            NdjsonCatalogReaderOptions {
                locale: None,
                source_locale: "en",
            },
        )
        .expect("streaming reader");

        assert_eq!(reader.locale(), Some("de"));
        let messages = reader
            .by_ref()
            .collect::<Result<Vec<_>, _>>()
            .expect("stream messages");

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].msgid, "About us");
        assert_eq!(messages[0].msgctxt.as_deref(), Some("nav"));
        assert!(matches!(
            messages[0].translation,
            TranslationShape::Singular { ref value } if value == "Ueber uns"
        ));
        assert_eq!(messages[0].origin[0].file, "src/nav.rs");
        assert_eq!(
            messages[0]
                .extra
                .as_ref()
                .expect("extra")
                .translator_comments,
            vec!["Keep short".to_owned()]
        );
    }

    #[test]
    fn ndjson_streaming_writer_emits_parseable_records() {
        let mut writer = NdjsonCatalogWriter::with_options(
            Vec::new(),
            NdjsonCatalogWriterOptions {
                locale: Some("de"),
                source_locale: "en",
            },
        )
        .expect("streaming writer");

        writer
            .write_message(&CatalogMessage {
                msgid: "Checkout".to_owned(),
                msgctxt: Some("button".to_owned()),
                translation: TranslationShape::Singular {
                    value: "Zur Kasse".to_owned(),
                },
                comments: vec!["Short button label".to_owned()],
                origin: vec![CatalogOrigin {
                    file: "src/checkout.rs".to_owned(),
                    line: Some(12),
                }],
                obsolete: false,
                machine_translation: None,
                extra: Some(CatalogMessageExtra {
                    translator_comments: vec!["Keep concise".to_owned()],
                    flags: vec!["rust-format".to_owned()],
                }),
            })
            .expect("write message");

        let rendered = String::from_utf8(writer.finish().expect("finish")).expect("utf8");
        assert!(rendered.starts_with("---\nformat: ferrocat.ndjson.v1\n"));
        assert!(rendered.contains("\"ctx\":\"button\""));
        assert!(rendered.contains("\"translator_comments\":[\"Keep concise\"]"));

        let parsed = parse_catalog_to_internal_ndjson(
            &rendered,
            None,
            "en",
            CatalogSemantics::IcuNative,
            false,
        )
        .expect("parse streamed output");
        assert_eq!(parsed.locale.as_deref(), Some("de"));
        assert_eq!(parsed.messages[0].msgid, "Checkout");
        assert_eq!(parsed.messages[0].flags, vec!["rust-format".to_owned()]);
    }

    #[test]
    fn ndjson_streaming_reader_convenience_methods_expose_inner_reader() {
        let input = concat!(
            "---\n",
            "format: ferrocat.ndjson.v1\n",
            "source_locale: en\n",
            "---\n",
            "{\"id\":\"Checkout\",\"str\":\"Zur Kasse\"}\n",
        );
        let mut reader = NdjsonCatalogReader::new(Cursor::new(input.as_bytes()), "en")
            .expect("streaming reader");

        assert!(reader.get_ref().position() > 0);
        assert!(reader.get_mut().position() > 0);
        let message = reader
            .next()
            .expect("record")
            .expect("streamed catalog message");
        assert_eq!(message.msgid, "Checkout");
        assert_eq!(reader.into_inner().position() as usize, input.len());
    }

    #[test]
    fn ndjson_streaming_writer_convenience_methods_and_plural_records_work() {
        let mut writer = NdjsonCatalogWriter::new(Vec::new(), "en").expect("streaming writer");
        assert!(
            writer
                .get_ref()
                .starts_with(b"---\nformat: ferrocat.ndjson.v1")
        );
        assert!(!writer.get_mut().is_empty());

        let translation = BTreeMap::from([
            ("one".to_owned(), "# Datei".to_owned()),
            ("other".to_owned(), "# Dateien".to_owned()),
        ]);
        let hash = machine_translation_hash(EffectiveTranslationRef::Plural(&translation));
        writer
            .write_message(&CatalogMessage {
                msgid: "files".to_owned(),
                msgctxt: None,
                translation: TranslationShape::Plural {
                    source: super::super::PluralSource {
                        one: Some("# file".to_owned()),
                        other: "# files".to_owned(),
                    },
                    translation,
                    variable: "count".to_owned(),
                },
                comments: Vec::new(),
                origin: Vec::new(),
                obsolete: false,
                machine_translation: Some(MachineTranslationMetadata {
                    model: "test/model".to_owned(),
                    modified: None,
                    confidence: Some(90),
                    hash,
                }),
                extra: None,
            })
            .expect("write plural");

        let rendered = String::from_utf8(writer.finish().expect("finish")).expect("utf8");
        assert!(rendered.contains("{count, plural, one {# file} other {# files}}"));
        assert!(rendered.contains("{count, plural, one {# Datei} other {# Dateien}}"));
        assert!(rendered.contains("\"mt\":{\"model\":\"test/model\""));
    }

    #[test]
    fn ndjson_parser_rejects_invalid_semantics_duplicates_and_bad_json() {
        let duplicate = concat!(
            "---\n",
            "format: ferrocat.ndjson.v1\n",
            "source_locale: en\n",
            "---\n",
            "{\"id\":\"About us\",\"str\":\"A\"}\n",
            "{\"id\":\"About us\",\"str\":\"B\"}\n",
        );
        assert!(
            parse_catalog_to_internal_ndjson(
                duplicate,
                None,
                "en",
                CatalogSemantics::IcuNative,
                false
            )
            .is_err()
        );

        assert!(
            parse_catalog_to_internal_ndjson(
                concat!(
                    "---\n",
                    "format: ferrocat.ndjson.v1\n",
                    "source_locale: en\n",
                    "---\n",
                    "{\"id\":\"About us\",\"str\":\"A\",\n",
                ),
                None,
                "en",
                CatalogSemantics::IcuNative,
                false
            )
            .is_err()
        );

        assert!(
            parse_catalog_to_internal_ndjson(
                concat!(
                    "---\n",
                    "format: ferrocat.ndjson.v1\n",
                    "source_locale: en\n",
                    "---\n",
                    "{\"id\":\"About us\",\"str\":\"A\"}\n",
                ),
                None,
                "en",
                CatalogSemantics::GettextCompat,
                false
            )
            .is_err()
        );
    }
}
