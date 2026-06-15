use std::collections::{BTreeMap, BTreeSet};

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer, XmlVersion};

use super::{
    ApiError, CatalogMessage, CatalogMessageExtra, CatalogMessageKey, Diagnostic,
    DiagnosticSeverity, ParsedCatalog, TranslationShape,
};

/// Options for exporting host-neutral catalog data as XLIFF 1.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XliffExportOptions<'a> {
    /// Source catalog that defines the message identities to export.
    pub source: &'a ParsedCatalog,
    /// Optional target catalog used to populate `<target>` values.
    pub target: Option<&'a ParsedCatalog>,
    /// BCP-47-like source locale written to the XLIFF `<file>` element.
    pub source_locale: &'a str,
    /// BCP-47-like target locale written to the XLIFF `<file>` element.
    pub target_locale: &'a str,
    /// XLIFF `original` label for the generated `<file>` element.
    pub original: &'a str,
    /// Whether obsolete source messages should be exported.
    pub include_obsolete: bool,
}

impl<'a> XliffExportOptions<'a> {
    /// Creates export options with required fields set.
    #[must_use]
    pub const fn new(
        source: &'a ParsedCatalog,
        target: Option<&'a ParsedCatalog>,
        source_locale: &'a str,
        target_locale: &'a str,
    ) -> Self {
        Self {
            source,
            target,
            source_locale,
            target_locale,
            original: "ferrocat",
            include_obsolete: false,
        }
    }
}

/// Counters collected while exporting XLIFF.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct XliffExportStats {
    /// Singular `trans-unit` entries written.
    pub units: usize,
    /// Source plural messages skipped by the MVP bridge.
    pub skipped_plural: usize,
    /// Obsolete source messages skipped by default.
    pub skipped_obsolete: usize,
    /// Empty target values written with `state="needs-translation"`.
    pub empty_targets: usize,
}

/// Result returned by [`export_xliff`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XliffExportResult {
    /// Generated XLIFF 1.2 document.
    pub content: String,
    /// Export counters.
    pub stats: XliffExportStats,
    /// Non-fatal diagnostics collected during export.
    pub diagnostics: Vec<Diagnostic>,
}

/// Options for importing a minimal XLIFF 1.2 document into catalog data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XliffImportOptions<'a> {
    /// XLIFF 1.2 document content.
    pub content: &'a str,
    /// Optional existing catalog to update in memory.
    pub existing: Option<&'a ParsedCatalog>,
    /// Source locale associated with the XLIFF source side.
    pub source_locale: &'a str,
    /// Target locale to assign to the imported catalog.
    pub target_locale: &'a str,
}

impl<'a> XliffImportOptions<'a> {
    /// Creates import options with required fields set.
    #[must_use]
    pub const fn new(content: &'a str, source_locale: &'a str, target_locale: &'a str) -> Self {
        Self {
            content,
            existing: None,
            source_locale,
            target_locale,
        }
    }
}

/// Counters collected while importing XLIFF.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct XliffImportStats {
    /// `trans-unit` entries read from the XLIFF document.
    pub units: usize,
    /// New singular catalog messages added.
    pub added: usize,
    /// Existing singular catalog messages changed.
    pub updated: usize,
    /// Existing singular catalog messages already matching the imported value.
    pub unchanged: usize,
    /// Existing plural messages intentionally skipped by the MVP bridge.
    pub skipped_plural: usize,
    /// Imported units with an empty target value.
    pub empty_targets: usize,
}

/// Result returned by [`import_xliff`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XliffImportResult {
    /// Imported or updated catalog.
    pub catalog: ParsedCatalog,
    /// Import counters.
    pub stats: XliffImportStats,
    /// Non-fatal diagnostics collected during import.
    pub diagnostics: Vec<Diagnostic>,
}

/// Exports singular catalog messages as a minimal XLIFF 1.2 document.
///
/// The MVP mapping preserves identity through `<source>` text plus an optional
/// `<note from="msgctxt">...` entry. Plural messages are reported and skipped
/// so that a later plural-specific mapping can be introduced without silently
/// flattening plural slots.
///
/// # Errors
///
/// Returns [`ApiError`] when required locales are empty, the source or target
/// catalog contains duplicate identities, or XML writing fails unexpectedly.
pub fn export_xliff(options: XliffExportOptions<'_>) -> Result<XliffExportResult, ApiError> {
    validate_xliff_locale("source_locale", options.source_locale)?;
    validate_xliff_locale("target_locale", options.target_locale)?;

    let target_map = options
        .target
        .map(catalog_message_map)
        .transpose()?
        .unwrap_or_default();
    let mut seen_source_keys = BTreeSet::new();
    let mut diagnostics = Vec::new();
    let mut stats = XliffExportStats::default();

    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(xml_write_error)?;
    write_start(
        &mut writer,
        "xliff",
        &[
            ("version", "1.2"),
            ("xmlns", "urn:oasis:names:tc:xliff:document:1.2"),
        ],
    )?;
    write_start(
        &mut writer,
        "file",
        &[
            ("source-language", options.source_locale),
            ("target-language", options.target_locale),
            ("datatype", "plaintext"),
            ("original", options.original),
        ],
    )?;
    write_start(&mut writer, "body", &[])?;

    for source_message in &options.source.messages {
        let key = source_message.key();
        if !seen_source_keys.insert(key.clone()) {
            return Err(ApiError::Conflict(format!(
                "duplicate source catalog message for msgid {:?} and context {:?}",
                key.msgid, key.msgctxt
            )));
        }
        if source_message.obsolete && !options.include_obsolete {
            stats.skipped_obsolete += 1;
            continue;
        }
        if !matches!(
            source_message.translation,
            TranslationShape::Singular { .. }
        ) {
            stats.skipped_plural += 1;
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticSeverity::Warning,
                    "xliff.export.skipped_plural",
                    "XLIFF MVP export skips plural messages.",
                )
                .with_identity(&source_message.msgid, source_message.msgctxt.as_deref()),
            );
            continue;
        }

        let (target_value, fuzzy) = target_map
            .get(&key)
            .and_then(|target_message| match &target_message.translation {
                TranslationShape::Singular { value } => {
                    Some((value.as_str(), message_has_fuzzy_flag(target_message)))
                }
                TranslationShape::Plural { .. } => {
                    diagnostics.push(
                        Diagnostic::new(
                            DiagnosticSeverity::Warning,
                            "xliff.export.skipped_plural_target",
                            "XLIFF MVP export cannot use plural target values for a singular unit.",
                        )
                        .with_identity(&source_message.msgid, source_message.msgctxt.as_deref()),
                    );
                    None
                }
            })
            .unwrap_or(("", false));

        if target_value.is_empty() {
            stats.empty_targets += 1;
        }
        stats.units += 1;
        write_trans_unit(
            &mut writer,
            stats.units,
            source_message,
            target_value,
            target_state(target_value, fuzzy),
        )?;
    }

    write_end(&mut writer, "body")?;
    write_end(&mut writer, "file")?;
    write_end(&mut writer, "xliff")?;

    let content = String::from_utf8(writer.into_inner()).map_err(|error| {
        ApiError::InvalidArguments(format!("generated XLIFF was not valid UTF-8: {error}"))
    })?;

    Ok(XliffExportResult {
        content,
        stats,
        diagnostics,
    })
}

/// Imports a minimal XLIFF 1.2 document into a host-neutral parsed catalog.
///
/// The import updates singular messages by `msgid + msgctxt`. Existing plural
/// messages are left untouched and reported as skipped.
///
/// # Errors
///
/// Returns [`ApiError`] when required locales are empty, the XML is malformed,
/// or duplicate XLIFF units disagree for the same `msgid + msgctxt`.
pub fn import_xliff(options: XliffImportOptions<'_>) -> Result<XliffImportResult, ApiError> {
    validate_xliff_locale("source_locale", options.source_locale)?;
    validate_xliff_locale("target_locale", options.target_locale)?;

    let units = parse_xliff_units(options.content)?;
    let mut diagnostics = Vec::new();
    let mut stats = XliffImportStats {
        units: units.len(),
        ..XliffImportStats::default()
    };
    let mut catalog = options.existing.cloned().unwrap_or_else(|| ParsedCatalog {
        locale: Some(options.target_locale.to_owned()),
        semantics: super::CatalogSemantics::IcuNative,
        headers: BTreeMap::from([
            ("Language".to_owned(), options.target_locale.to_owned()),
            (
                "X-Ferrocat-XLIFF-Source-Language".to_owned(),
                options.source_locale.to_owned(),
            ),
        ]),
        messages: Vec::new(),
        diagnostics: Vec::new(),
    });
    catalog.locale = Some(options.target_locale.to_owned());
    catalog
        .headers
        .insert("Language".to_owned(), options.target_locale.to_owned());

    let mut key_index = catalog_message_index(&catalog)?;
    let mut seen_units = BTreeMap::<CatalogMessageKey, (String, bool)>::new();

    for unit in units {
        if unit.source.is_empty() {
            diagnostics.push(Diagnostic::new(
                DiagnosticSeverity::Warning,
                "xliff.import.skipped_empty_source",
                "XLIFF trans-unit without source text was skipped.",
            ));
            continue;
        }

        let key = CatalogMessageKey::new(unit.source.clone(), unit.msgctxt.clone());
        let fuzzy = unit.is_review_state();
        if unit.target.is_empty() {
            stats.empty_targets += 1;
        }
        if let Some((previous_target, previous_fuzzy)) =
            seen_units.insert(key.clone(), (unit.target.clone(), fuzzy))
        {
            if previous_target != unit.target || previous_fuzzy != fuzzy {
                return Err(ApiError::Conflict(format!(
                    "conflicting XLIFF trans-unit for msgid {:?} and context {:?}",
                    key.msgid, key.msgctxt
                )));
            }
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticSeverity::Info,
                    "xliff.import.duplicate_identical_unit",
                    "Duplicate XLIFF trans-unit with identical target was ignored.",
                )
                .with_identity(&key.msgid, key.msgctxt.as_deref()),
            );
            continue;
        }

        if let Some(index) = key_index.get(&key).copied() {
            let message = &mut catalog.messages[index];
            let existing_fuzzy = message_has_fuzzy_flag(message);
            match &mut message.translation {
                TranslationShape::Singular { value } => {
                    if *value == unit.target && existing_fuzzy == fuzzy {
                        stats.unchanged += 1;
                    } else {
                        *value = unit.target;
                        set_message_fuzzy_flag(message, fuzzy);
                        stats.updated += 1;
                    }
                }
                TranslationShape::Plural { .. } => {
                    stats.skipped_plural += 1;
                    diagnostics.push(
                        Diagnostic::new(
                            DiagnosticSeverity::Warning,
                            "xliff.import.skipped_plural",
                            "XLIFF MVP import does not overwrite existing plural messages.",
                        )
                        .with_identity(&key.msgid, key.msgctxt.as_deref()),
                    );
                }
            }
            continue;
        }

        let message = CatalogMessage {
            msgid: key.msgid.clone(),
            msgctxt: key.msgctxt.clone(),
            translation: TranslationShape::Singular { value: unit.target },
            comments: Vec::new(),
            origin: Vec::new(),
            obsolete: false,
            machine_translation: None,
            extra: Some(CatalogMessageExtra {
                translator_comments: Vec::new(),
                flags: fuzzy.then(|| "fuzzy".to_owned()).into_iter().collect(),
            }),
        };
        catalog.messages.push(message);
        key_index.insert(key, catalog.messages.len() - 1);
        stats.added += 1;
    }

    catalog.diagnostics.extend(diagnostics.clone());

    Ok(XliffImportResult {
        catalog,
        stats,
        diagnostics,
    })
}

fn validate_xliff_locale(field: &str, locale: &str) -> Result<(), ApiError> {
    if locale.trim().is_empty() {
        return Err(ApiError::InvalidArguments(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

fn catalog_message_map(
    catalog: &ParsedCatalog,
) -> Result<BTreeMap<CatalogMessageKey, &CatalogMessage>, ApiError> {
    let mut map = BTreeMap::new();
    for message in &catalog.messages {
        let key = message.key();
        if map.insert(key.clone(), message).is_some() {
            return Err(ApiError::Conflict(format!(
                "duplicate catalog message for msgid {:?} and context {:?}",
                key.msgid, key.msgctxt
            )));
        }
    }
    Ok(map)
}

fn catalog_message_index(
    catalog: &ParsedCatalog,
) -> Result<BTreeMap<CatalogMessageKey, usize>, ApiError> {
    let mut map = BTreeMap::new();
    for (index, message) in catalog.messages.iter().enumerate() {
        let key = message.key();
        if map.insert(key.clone(), index).is_some() {
            return Err(ApiError::Conflict(format!(
                "duplicate catalog message for msgid {:?} and context {:?}",
                key.msgid, key.msgctxt
            )));
        }
    }
    Ok(map)
}

fn write_trans_unit(
    writer: &mut Writer<Vec<u8>>,
    index: usize,
    source_message: &CatalogMessage,
    target_value: &str,
    state: &str,
) -> Result<(), ApiError> {
    let id = format!("u{index}");
    write_start(
        writer,
        "trans-unit",
        &[
            ("id", id.as_str()),
            ("resname", source_message.msgid.as_str()),
        ],
    )?;
    write_text_element(writer, "source", &source_message.msgid, &[])?;
    write_text_element(writer, "target", target_value, &[("state", state)])?;
    if let Some(msgctxt) = &source_message.msgctxt {
        write_text_element(writer, "note", msgctxt, &[("from", "msgctxt")])?;
    }
    write_end(writer, "trans-unit")?;
    Ok(())
}

fn write_start(
    writer: &mut Writer<Vec<u8>>,
    name: &str,
    attributes: &[(&str, &str)],
) -> Result<(), ApiError> {
    let mut start = BytesStart::new(name);
    start.extend_attributes(attributes.iter().copied());
    writer
        .write_event(Event::Start(start))
        .map_err(xml_write_error)
}

fn write_end(writer: &mut Writer<Vec<u8>>, name: &str) -> Result<(), ApiError> {
    writer
        .write_event(Event::End(BytesEnd::new(name)))
        .map_err(xml_write_error)
}

fn write_text_element(
    writer: &mut Writer<Vec<u8>>,
    name: &str,
    value: &str,
    attributes: &[(&str, &str)],
) -> Result<(), ApiError> {
    let mut start = BytesStart::new(name);
    start.extend_attributes(attributes.iter().copied());
    writer
        .write_event(Event::Start(start.borrow()))
        .map_err(xml_write_error)?;
    writer
        .write_event(Event::Text(BytesText::new(value)))
        .map_err(xml_write_error)?;
    writer
        .write_event(Event::End(start.to_end()))
        .map_err(xml_write_error)
}

fn target_state(target_value: &str, fuzzy: bool) -> &'static str {
    if target_value.is_empty() {
        "needs-translation"
    } else if fuzzy {
        "needs-review-translation"
    } else {
        "translated"
    }
}

fn message_has_fuzzy_flag(message: &CatalogMessage) -> bool {
    message
        .extra
        .as_ref()
        .is_some_and(|extra| extra.flags.iter().any(|flag| flag == "fuzzy"))
}

fn set_message_fuzzy_flag(message: &mut CatalogMessage, fuzzy: bool) {
    let extra = message
        .extra
        .get_or_insert_with(CatalogMessageExtra::default);
    if fuzzy {
        if !extra.flags.iter().any(|flag| flag == "fuzzy") {
            extra.flags.push("fuzzy".to_owned());
        }
    } else {
        extra.flags.retain(|flag| flag != "fuzzy");
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct ImportedUnit {
    source: String,
    target: String,
    target_state: Option<String>,
    msgctxt: Option<String>,
}

impl ImportedUnit {
    fn is_review_state(&self) -> bool {
        self.target_state
            .as_deref()
            .is_some_and(|state| state.contains("needs-review"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextTarget {
    Source,
    Target,
    MsgctxtNote,
    OtherNote,
}

fn parse_xliff_units(content: &str) -> Result<Vec<ImportedUnit>, ApiError> {
    let mut reader = Reader::from_str(content);
    let mut units = Vec::new();
    let mut current_unit: Option<ImportedUnit> = None;
    let mut text_target: Option<TextTarget> = None;

    loop {
        match reader.read_event().map_err(xml_read_error)? {
            Event::Start(event) if local_name(event.local_name().as_ref()) == b"trans-unit" => {
                current_unit = Some(ImportedUnit::default());
            }
            Event::Start(event) if current_unit.is_some() => {
                match local_name(event.local_name().as_ref()) {
                    b"source" => text_target = Some(TextTarget::Source),
                    b"target" => {
                        if let Some(unit) = &mut current_unit {
                            unit.target_state = attr_value(&event, b"state")?;
                        }
                        text_target = Some(TextTarget::Target);
                    }
                    b"note" => {
                        text_target = if attr_value(&event, b"from")?.as_deref() == Some("msgctxt")
                        {
                            Some(TextTarget::MsgctxtNote)
                        } else {
                            Some(TextTarget::OtherNote)
                        };
                    }
                    _ => {}
                }
            }
            Event::Empty(event) if current_unit.is_some() => {
                if local_name(event.local_name().as_ref()) == b"target"
                    && let Some(unit) = &mut current_unit
                {
                    unit.target_state = attr_value(&event, b"state")?;
                }
            }
            Event::Text(event) => {
                if let (Some(unit), Some(target)) = (&mut current_unit, text_target) {
                    push_text(unit, target, decode_text(&event)?)?;
                }
            }
            Event::GeneralRef(event) => {
                if let (Some(unit), Some(target)) = (&mut current_unit, text_target) {
                    push_text(unit, target, decode_general_ref(&event)?)?;
                }
            }
            Event::CData(event) => {
                if let (Some(unit), Some(target)) = (&mut current_unit, text_target) {
                    push_text(
                        unit,
                        target,
                        event.decode().map_err(xml_decode_error)?.into_owned(),
                    )?;
                }
            }
            Event::End(event) => match local_name(event.local_name().as_ref()) {
                b"source" | b"target" | b"note" => text_target = None,
                b"trans-unit" => {
                    if let Some(unit) = current_unit.take() {
                        units.push(unit);
                    }
                    text_target = None;
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(units)
}

fn push_text(unit: &mut ImportedUnit, target: TextTarget, value: String) -> Result<(), ApiError> {
    match target {
        TextTarget::Source => unit.source.push_str(&value),
        TextTarget::Target => unit.target.push_str(&value),
        TextTarget::MsgctxtNote => {
            let context = unit.msgctxt.get_or_insert_with(String::new);
            context.push_str(&value);
        }
        TextTarget::OtherNote => {}
    }
    Ok(())
}

fn attr_value(event: &BytesStart<'_>, name: &[u8]) -> Result<Option<String>, ApiError> {
    for attribute in event.attributes() {
        let attribute = attribute.map_err(|error| {
            ApiError::InvalidArguments(format!("invalid XLIFF XML attribute: {error}"))
        })?;
        if local_name(attribute.key.as_ref()) == name {
            return attribute
                .normalized_value(XmlVersion::Implicit1_0)
                .map(|value| Some(value.into_owned()))
                .map_err(xml_read_error);
        }
    }
    Ok(None)
}

fn decode_text(event: &BytesText<'_>) -> Result<String, ApiError> {
    let decoded = event.decode().map_err(xml_decode_error)?;
    quick_xml::escape::unescape(decoded.as_ref())
        .map(|value| value.into_owned())
        .map_err(|error| ApiError::InvalidArguments(format!("invalid XLIFF text escape: {error}")))
}

fn decode_general_ref(event: &quick_xml::events::BytesRef<'_>) -> Result<String, ApiError> {
    let decoded = event.decode().map_err(xml_decode_error)?;
    let value = match decoded.as_ref() {
        "amp" => "&".to_owned(),
        "lt" => "<".to_owned(),
        "gt" => ">".to_owned(),
        "quot" => "\"".to_owned(),
        "apos" => "'".to_owned(),
        numeric if numeric.starts_with("#x") => decode_numeric_ref(&numeric[2..], 16)?,
        numeric if numeric.starts_with('#') => decode_numeric_ref(&numeric[1..], 10)?,
        other => {
            return Err(ApiError::InvalidArguments(format!(
                "unsupported XLIFF entity reference &{other};"
            )));
        }
    };
    Ok(value)
}

fn decode_numeric_ref(raw: &str, radix: u32) -> Result<String, ApiError> {
    let value = u32::from_str_radix(raw, radix).map_err(|error| {
        ApiError::InvalidArguments(format!("invalid XLIFF numeric entity reference: {error}"))
    })?;
    let ch = char::from_u32(value).ok_or_else(|| {
        ApiError::InvalidArguments("invalid XLIFF numeric entity scalar value".to_owned())
    })?;
    Ok(ch.to_string())
}

fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

fn xml_write_error(error: std::io::Error) -> ApiError {
    ApiError::Io(error)
}

fn xml_read_error(error: impl std::fmt::Display) -> ApiError {
    ApiError::InvalidArguments(format!("invalid XLIFF XML: {error}"))
}

fn xml_decode_error(error: impl std::fmt::Display) -> ApiError {
    ApiError::InvalidArguments(format!("invalid XLIFF text encoding: {error}"))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{XliffExportOptions, XliffImportOptions, export_xliff, import_xliff};
    use crate::{
        CatalogMessage, CatalogMessageExtra, CatalogSemantics, ParsedCatalog, PluralSource,
        TranslationShape,
    };

    fn catalog(messages: Vec<CatalogMessage>) -> ParsedCatalog {
        ParsedCatalog {
            locale: Some("de".to_owned()),
            semantics: CatalogSemantics::IcuNative,
            headers: BTreeMap::new(),
            messages,
            diagnostics: Vec::new(),
        }
    }

    fn singular(msgid: &str, msgctxt: Option<&str>, value: &str, fuzzy: bool) -> CatalogMessage {
        CatalogMessage {
            msgid: msgid.to_owned(),
            msgctxt: msgctxt.map(str::to_owned),
            translation: TranslationShape::Singular {
                value: value.to_owned(),
            },
            comments: Vec::new(),
            origin: Vec::new(),
            obsolete: false,
            machine_translation: None,
            extra: Some(CatalogMessageExtra {
                translator_comments: Vec::new(),
                flags: fuzzy.then(|| "fuzzy".to_owned()).into_iter().collect(),
            }),
        }
    }

    fn plural(msgid: &str) -> CatalogMessage {
        CatalogMessage {
            msgid: msgid.to_owned(),
            msgctxt: None,
            translation: TranslationShape::Plural {
                source: PluralSource {
                    one: Some("{count} file".to_owned()),
                    other: "{count} files".to_owned(),
                },
                translation: BTreeMap::from([
                    ("one".to_owned(), "{count} Datei".to_owned()),
                    ("other".to_owned(), "{count} Dateien".to_owned()),
                ]),
                variable: "count".to_owned(),
            },
            comments: Vec::new(),
            origin: Vec::new(),
            obsolete: false,
            machine_translation: None,
            extra: None,
        }
    }

    #[test]
    fn exports_and_imports_singular_context_fuzzy_and_xml_escaping() {
        let source = catalog(vec![singular("Save & <close>", Some("button"), "", false)]);
        let target = catalog(vec![singular(
            "Save & <close>",
            Some("button"),
            "Speichern & schließen",
            true,
        )]);
        let exported = export_xliff(XliffExportOptions::new(&source, Some(&target), "en", "de"))
            .expect("export");

        assert!(exported.content.contains("Save &amp; &lt;close&gt;"));
        assert!(exported.content.contains("needs-review-translation"));

        let imported =
            import_xliff(XliffImportOptions::new(&exported.content, "en", "de")).expect("import");
        let message = &imported.catalog.messages[0];
        assert_eq!(message.msgid, "Save & <close>");
        assert_eq!(message.msgctxt.as_deref(), Some("button"));
        assert!(matches!(
            &message.translation,
            TranslationShape::Singular { value } if value == "Speichern & schließen"
        ));
        assert!(
            message
                .extra
                .as_ref()
                .is_some_and(|extra| extra.flags == ["fuzzy"])
        );
    }

    #[test]
    fn export_marks_empty_targets_and_skips_plural_messages() {
        let source = catalog(vec![singular("Hello", None, "", false), plural("Files")]);
        let exported =
            export_xliff(XliffExportOptions::new(&source, None, "en", "de")).expect("export");

        assert_eq!(exported.stats.units, 1);
        assert_eq!(exported.stats.empty_targets, 1);
        assert_eq!(exported.stats.skipped_plural, 1);
        assert!(exported.content.contains("needs-translation"));
        assert!(
            exported
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "xliff.export.skipped_plural")
        );
    }

    #[test]
    fn import_detects_conflicting_duplicate_units() {
        let content = concat!(
            r#"<?xml version="1.0" encoding="UTF-8"?>"#,
            r#"<xliff version="1.2"><file><body>"#,
            r#"<trans-unit id="a"><source>Hello</source><target>Hallo</target></trans-unit>"#,
            r#"<trans-unit id="b"><source>Hello</source><target>Servus</target></trans-unit>"#,
            r#"</body></file></xliff>"#,
        );

        let error = import_xliff(XliffImportOptions::new(content, "en", "de"))
            .expect_err("duplicate should conflict");
        assert!(error.to_string().contains("conflicting XLIFF trans-unit"));
    }

    #[test]
    fn import_does_not_overwrite_existing_plural_messages() {
        let existing = catalog(vec![plural("Files")]);
        let content = concat!(
            r#"<xliff version="1.2"><file><body>"#,
            r#"<trans-unit id="files"><source>Files</source><target>Dateien</target></trans-unit>"#,
            r#"</body></file></xliff>"#,
        );

        let mut options = XliffImportOptions::new(content, "en", "de");
        options.existing = Some(&existing);
        let imported = import_xliff(options).expect("import");

        assert_eq!(imported.stats.skipped_plural, 1);
        assert!(matches!(
            imported.catalog.messages[0].translation,
            TranslationShape::Plural { .. }
        ));
        assert!(
            imported
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "xliff.import.skipped_plural")
        );
    }
}
