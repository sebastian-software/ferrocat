use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{Header, MsgStr, ParseError, PoFile, PoItem, SerializeOptions, parse_po, stringify_po};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CatalogOrigin {
    pub file: String,
    pub line: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtractedSingularMessage {
    pub msgid: String,
    pub msgctxt: Option<String>,
    pub comments: Vec<String>,
    pub origin: Vec<CatalogOrigin>,
    pub placeholders: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PluralSource {
    pub one: Option<String>,
    pub other: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtractedPluralMessage {
    pub msgid: String,
    pub msgctxt: Option<String>,
    pub source: PluralSource,
    pub comments: Vec<String>,
    pub origin: Vec<CatalogOrigin>,
    pub placeholders: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractedMessage {
    Singular(ExtractedSingularMessage),
    Plural(ExtractedPluralMessage),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranslationShape {
    Singular {
        value: String,
    },
    Plural {
        source: PluralSource,
        translation: BTreeMap<String, String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CatalogMessageExtra {
    pub translator_comments: Vec<String>,
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogMessage {
    pub msgid: String,
    pub msgctxt: Option<String>,
    pub translation: TranslationShape,
    pub comments: Vec<String>,
    pub origin: Vec<CatalogOrigin>,
    pub obsolete: bool,
    pub extra: Option<CatalogMessageExtra>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub msgid: Option<String>,
    pub msgctxt: Option<String>,
}

impl Diagnostic {
    fn new(
        severity: DiagnosticSeverity,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
            msgid: None,
            msgctxt: None,
        }
    }

    fn with_identity(mut self, msgid: &str, msgctxt: Option<&str>) -> Self {
        self.msgid = Some(msgid.to_owned());
        self.msgctxt = msgctxt.map(str::to_owned);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CatalogStats {
    pub total: usize,
    pub added: usize,
    pub changed: usize,
    pub unchanged: usize,
    pub obsolete_marked: usize,
    pub obsolete_removed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogUpdateResult {
    pub content: String,
    pub created: bool,
    pub updated: bool,
    pub stats: CatalogStats,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCatalog {
    pub locale: Option<String>,
    pub headers: BTreeMap<String, String>,
    pub messages: Vec<CatalogMessage>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PluralEncoding {
    #[default]
    Icu,
    Gettext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ObsoleteStrategy {
    #[default]
    Mark,
    Delete,
    Keep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrderBy {
    #[default]
    Msgid,
    Origin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaceholderCommentMode {
    Disabled,
    Enabled { limit: usize },
}

impl Default for PlaceholderCommentMode {
    fn default() -> Self {
        Self::Enabled { limit: 3 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCatalogOptions {
    pub locale: Option<String>,
    pub source_locale: String,
    pub extracted: Vec<ExtractedMessage>,
    pub existing: Option<String>,
    pub plural_encoding: PluralEncoding,
    pub obsolete_strategy: ObsoleteStrategy,
    pub overwrite_source_translations: bool,
    pub order_by: OrderBy,
    pub include_origins: bool,
    pub include_line_numbers: bool,
    pub print_placeholders_in_comments: PlaceholderCommentMode,
    pub custom_header_attributes: BTreeMap<String, String>,
}

impl Default for UpdateCatalogOptions {
    fn default() -> Self {
        Self {
            locale: None,
            source_locale: String::new(),
            extracted: Vec::new(),
            existing: None,
            plural_encoding: PluralEncoding::Icu,
            obsolete_strategy: ObsoleteStrategy::Mark,
            overwrite_source_translations: false,
            order_by: OrderBy::Msgid,
            include_origins: true,
            include_line_numbers: true,
            print_placeholders_in_comments: PlaceholderCommentMode::Enabled { limit: 3 },
            custom_header_attributes: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCatalogFileOptions {
    pub target_path: PathBuf,
    pub locale: Option<String>,
    pub source_locale: String,
    pub extracted: Vec<ExtractedMessage>,
    pub plural_encoding: PluralEncoding,
    pub obsolete_strategy: ObsoleteStrategy,
    pub overwrite_source_translations: bool,
    pub order_by: OrderBy,
    pub include_origins: bool,
    pub include_line_numbers: bool,
    pub print_placeholders_in_comments: PlaceholderCommentMode,
    pub custom_header_attributes: BTreeMap<String, String>,
}

impl Default for UpdateCatalogFileOptions {
    fn default() -> Self {
        Self {
            target_path: PathBuf::new(),
            locale: None,
            source_locale: String::new(),
            extracted: Vec::new(),
            plural_encoding: PluralEncoding::Icu,
            obsolete_strategy: ObsoleteStrategy::Mark,
            overwrite_source_translations: false,
            order_by: OrderBy::Msgid,
            include_origins: true,
            include_line_numbers: true,
            print_placeholders_in_comments: PlaceholderCommentMode::Enabled { limit: 3 },
            custom_header_attributes: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseCatalogOptions {
    pub content: String,
    pub locale: Option<String>,
    pub source_locale: String,
    pub plural_encoding: PluralEncoding,
    pub strict: bool,
}

impl Default for ParseCatalogOptions {
    fn default() -> Self {
        Self {
            content: String::new(),
            locale: None,
            source_locale: String::new(),
            plural_encoding: PluralEncoding::Icu,
            strict: false,
        }
    }
}

#[derive(Debug)]
pub enum ApiError {
    Parse(ParseError),
    Io(std::io::Error),
    InvalidArguments(String),
    Conflict(String),
    Unsupported(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => error.fmt(f),
            Self::Io(error) => error.fmt(f),
            Self::InvalidArguments(message)
            | Self::Conflict(message)
            | Self::Unsupported(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ApiError {}

impl From<ParseError> for ApiError {
    fn from(value: ParseError) -> Self {
        Self::Parse(value)
    }
}

impl From<std::io::Error> for ApiError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Catalog {
    locale: Option<String>,
    headers: BTreeMap<String, String>,
    file_comments: Vec<String>,
    file_extracted_comments: Vec<String>,
    messages: Vec<CanonicalMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanonicalMessage {
    msgid: String,
    msgctxt: Option<String>,
    translation: CanonicalTranslation,
    comments: Vec<String>,
    origins: Vec<CatalogOrigin>,
    placeholders: BTreeMap<String, Vec<String>>,
    obsolete: bool,
    translator_comments: Vec<String>,
    flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CanonicalTranslation {
    Singular {
        value: String,
    },
    Plural {
        source: PluralSource,
        translation_by_category: BTreeMap<String, String>,
        variable: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedMessage {
    msgid: String,
    msgctxt: Option<String>,
    kind: NormalizedKind,
    comments: Vec<String>,
    origins: Vec<CatalogOrigin>,
    placeholders: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NormalizedKind {
    Singular,
    Plural(PluralSource),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedIcuPlural {
    variable: String,
    branches: BTreeMap<String, String>,
}

pub fn update_catalog(options: UpdateCatalogOptions) -> Result<CatalogUpdateResult, ApiError> {
    validate_source_locale(&options.source_locale)?;

    let created = options.existing.is_none();
    let original = options.existing.as_deref().unwrap_or("");
    let existing = match options.existing.as_deref() {
        Some(content) if !content.is_empty() => parse_catalog_to_internal(
            content,
            options.locale.as_deref(),
            options.plural_encoding,
            false,
        )?,
        Some(_) | None => Catalog {
            locale: options.locale.clone(),
            headers: BTreeMap::new(),
            file_comments: Vec::new(),
            file_extracted_comments: Vec::new(),
            messages: Vec::new(),
        },
    };

    let locale = options
        .locale
        .clone()
        .or_else(|| existing.locale.clone())
        .or_else(|| existing.headers.get("Language").cloned());
    let normalized = normalize_extracted(&options.extracted)?;
    let mut diagnostics = Vec::new();
    let (mut merged, stats) = merge_catalogs(
        existing,
        &normalized,
        locale.as_deref(),
        &options.source_locale,
        options.overwrite_source_translations,
        options.obsolete_strategy,
        &mut diagnostics,
    );
    merged.locale = locale.clone();
    apply_header_defaults(
        &mut merged.headers,
        locale.as_deref(),
        &options.custom_header_attributes,
    );
    sort_messages(&mut merged.messages, options.order_by);
    let file = export_catalog_to_po(&merged, &options, locale.as_deref(), &mut diagnostics)?;
    let content = stringify_po(&file, &SerializeOptions::default());

    Ok(CatalogUpdateResult {
        updated: content != original,
        content,
        created,
        stats,
        diagnostics,
    })
}

pub fn update_catalog_file(
    options: UpdateCatalogFileOptions,
) -> Result<CatalogUpdateResult, ApiError> {
    validate_source_locale(&options.source_locale)?;
    if options.target_path.as_os_str().is_empty() {
        return Err(ApiError::InvalidArguments(
            "target_path must not be empty".to_owned(),
        ));
    }

    let existing = match fs::read_to_string(&options.target_path) {
        Ok(content) => Some(content),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(ApiError::Io(error)),
    };

    let result = update_catalog(UpdateCatalogOptions {
        locale: options.locale,
        source_locale: options.source_locale,
        extracted: options.extracted,
        existing,
        plural_encoding: options.plural_encoding,
        obsolete_strategy: options.obsolete_strategy,
        overwrite_source_translations: options.overwrite_source_translations,
        order_by: options.order_by,
        include_origins: options.include_origins,
        include_line_numbers: options.include_line_numbers,
        print_placeholders_in_comments: options.print_placeholders_in_comments,
        custom_header_attributes: options.custom_header_attributes,
    })?;

    if result.created || result.updated {
        atomic_write(&options.target_path, &result.content)?;
    }

    Ok(result)
}

pub fn parse_catalog(options: ParseCatalogOptions) -> Result<ParsedCatalog, ApiError> {
    validate_source_locale(&options.source_locale)?;
    let catalog = parse_catalog_to_internal(
        &options.content,
        options.locale.as_deref(),
        options.plural_encoding,
        options.strict,
    )?;
    let messages = catalog
        .messages
        .into_iter()
        .map(public_message_from_canonical)
        .collect();

    Ok(ParsedCatalog {
        locale: catalog.locale,
        headers: catalog.headers,
        messages,
        diagnostics: Vec::new(),
    })
}

fn validate_source_locale(source_locale: &str) -> Result<(), ApiError> {
    if source_locale.trim().is_empty() {
        return Err(ApiError::InvalidArguments(
            "source_locale must not be empty".to_owned(),
        ));
    }
    Ok(())
}

fn normalize_extracted(extracted: &[ExtractedMessage]) -> Result<Vec<NormalizedMessage>, ApiError> {
    let mut index = BTreeMap::<(String, Option<String>), usize>::new();
    let mut normalized = Vec::<NormalizedMessage>::new();

    for message in extracted {
        let (msgid, msgctxt, kind, comments, origins, placeholders) = match message {
            ExtractedMessage::Singular(message) => (
                message.msgid.clone(),
                message.msgctxt.clone(),
                NormalizedKind::Singular,
                message.comments.clone(),
                message.origin.clone(),
                message.placeholders.clone(),
            ),
            ExtractedMessage::Plural(message) => (
                message.msgid.clone(),
                message.msgctxt.clone(),
                NormalizedKind::Plural(message.source.clone()),
                message.comments.clone(),
                message.origin.clone(),
                message.placeholders.clone(),
            ),
        };

        if msgid.is_empty() {
            return Err(ApiError::InvalidArguments(
                "extracted msgid must not be empty".to_owned(),
            ));
        }

        let key = (msgid.clone(), msgctxt.clone());
        match index.get(&key).copied() {
            Some(existing_index) => {
                let existing = &mut normalized[existing_index];
                if existing.kind != kind {
                    return Err(ApiError::Conflict(format!(
                        "conflicting duplicate extracted message for msgid {:?}",
                        msgid
                    )));
                }
                merge_unique_strings(&mut existing.comments, comments);
                merge_unique_origins(&mut existing.origins, origins);
                merge_placeholders(&mut existing.placeholders, placeholders);
            }
            None => {
                index.insert(key, normalized.len());
                normalized.push(NormalizedMessage {
                    msgid,
                    msgctxt,
                    kind,
                    comments: dedupe_strings(comments),
                    origins: dedupe_origins(origins),
                    placeholders: dedupe_placeholders(placeholders),
                });
            }
        }
    }

    Ok(normalized)
}

fn merge_catalogs(
    existing: Catalog,
    normalized: &[NormalizedMessage],
    locale: Option<&str>,
    source_locale: &str,
    overwrite_source_translations: bool,
    obsolete_strategy: ObsoleteStrategy,
    diagnostics: &mut Vec<Diagnostic>,
) -> (Catalog, CatalogStats) {
    let is_source_locale = locale.map_or(true, |value| value == source_locale);
    let mut stats = CatalogStats::default();

    let mut existing_index = BTreeMap::<(String, Option<String>), usize>::new();
    for (index, message) in existing.messages.iter().enumerate() {
        existing_index.insert((message.msgid.clone(), message.msgctxt.clone()), index);
    }

    let mut matched = vec![false; existing.messages.len()];
    let mut messages = Vec::with_capacity(normalized.len() + existing.messages.len());

    for next in normalized {
        let key = (next.msgid.clone(), next.msgctxt.clone());
        let previous = existing_index.get(&key).copied().map(|index| {
            matched[index] = true;
            existing.messages[index].clone()
        });
        let merged = merge_message(
            previous.as_ref(),
            next,
            is_source_locale,
            locale,
            overwrite_source_translations,
            diagnostics,
        );
        if previous.is_none() {
            stats.added += 1;
        } else if previous.as_ref() == Some(&merged) {
            stats.unchanged += 1;
        } else {
            stats.changed += 1;
        }
        messages.push(merged);
    }

    for (index, message) in existing.messages.into_iter().enumerate() {
        if matched[index] {
            continue;
        }
        match obsolete_strategy {
            ObsoleteStrategy::Delete => {
                stats.obsolete_removed += 1;
            }
            ObsoleteStrategy::Mark => {
                let mut message = message;
                if !message.obsolete {
                    message.obsolete = true;
                    stats.obsolete_marked += 1;
                }
                messages.push(message);
            }
            ObsoleteStrategy::Keep => {
                let mut message = message;
                message.obsolete = false;
                messages.push(message);
            }
        }
    }

    stats.total = messages.len();
    (
        Catalog {
            locale: existing.locale,
            headers: existing.headers,
            file_comments: existing.file_comments,
            file_extracted_comments: existing.file_extracted_comments,
            messages,
        },
        stats,
    )
}

fn merge_message(
    previous: Option<&CanonicalMessage>,
    next: &NormalizedMessage,
    is_source_locale: bool,
    locale: Option<&str>,
    overwrite_source_translations: bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> CanonicalMessage {
    let expected_categories = match &next.kind {
        NormalizedKind::Singular => Vec::new(),
        NormalizedKind::Plural(_) => plural_categories_for(locale, None),
    };

    let translation = match (&next.kind, previous) {
        (NormalizedKind::Singular, Some(previous))
            if matches!(previous.translation, CanonicalTranslation::Singular { .. })
                && !(is_source_locale && overwrite_source_translations) =>
        {
            previous.translation.clone()
        }
        (NormalizedKind::Plural(source), Some(previous))
            if matches!(previous.translation, CanonicalTranslation::Plural { .. })
                && !(is_source_locale && overwrite_source_translations) =>
        {
            let previous_variable = match &previous.translation {
                CanonicalTranslation::Plural { variable, .. } => variable.clone(),
                _ => unreachable!(),
            };
            let previous_map = match &previous.translation {
                CanonicalTranslation::Plural {
                    translation_by_category,
                    ..
                } => translation_by_category.clone(),
                _ => unreachable!(),
            };
            CanonicalTranslation::Plural {
                source: source.clone(),
                translation_by_category: materialize_plural_categories(
                    &expected_categories,
                    previous_map,
                ),
                variable: previous_variable,
            }
        }
        (NormalizedKind::Singular, _) => CanonicalTranslation::Singular {
            value: if is_source_locale {
                next.msgid.clone()
            } else {
                String::new()
            },
        },
        (NormalizedKind::Plural(source), previous) => {
            let variable = previous
                .and_then(extract_plural_variable)
                .or_else(|| derive_plural_variable(&next.placeholders))
                .unwrap_or_else(|| {
                    diagnostics.push(
                        Diagnostic::new(
                            DiagnosticSeverity::Warning,
                            "plural.assumed_variable",
                            "Unable to determine plural placeholder name, assuming \"count\".",
                        )
                        .with_identity(&next.msgid, next.msgctxt.as_deref()),
                    );
                    "count".to_owned()
                });

            CanonicalTranslation::Plural {
                source: source.clone(),
                translation_by_category: if is_source_locale {
                    source_locale_plural_translation(&expected_categories, source)
                } else {
                    empty_plural_translation(&expected_categories)
                },
                variable,
            }
        }
    };

    let (translator_comments, flags, obsolete) = previous
        .map(|message| {
            (
                message.translator_comments.clone(),
                message.flags.clone(),
                false,
            )
        })
        .unwrap_or_else(|| (Vec::new(), Vec::new(), false));

    CanonicalMessage {
        msgid: next.msgid.clone(),
        msgctxt: next.msgctxt.clone(),
        translation,
        comments: next.comments.clone(),
        origins: next.origins.clone(),
        placeholders: next.placeholders.clone(),
        obsolete,
        translator_comments,
        flags,
    }
}

fn source_locale_plural_translation(
    categories: &[String],
    source: &PluralSource,
) -> BTreeMap<String, String> {
    let mut translation = BTreeMap::new();
    for category in categories {
        let value = match category.as_str() {
            "one" => source.one.clone().unwrap_or_else(|| source.other.clone()),
            "other" => source.other.clone(),
            _ => source.other.clone(),
        };
        translation.insert(category.clone(), value);
    }
    translation
}

fn empty_plural_translation(categories: &[String]) -> BTreeMap<String, String> {
    categories
        .iter()
        .map(|category| (category.clone(), String::new()))
        .collect()
}

fn materialize_plural_categories(
    categories: &[String],
    translation: BTreeMap<String, String>,
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

fn extract_plural_variable(message: &CanonicalMessage) -> Option<String> {
    match &message.translation {
        CanonicalTranslation::Plural { variable, .. } => Some(variable.clone()),
        CanonicalTranslation::Singular { .. } => None,
    }
}

fn apply_header_defaults(
    headers: &mut BTreeMap<String, String>,
    locale: Option<&str>,
    custom: &BTreeMap<String, String>,
) {
    headers
        .entry("MIME-Version".to_owned())
        .or_insert_with(|| "1.0".to_owned());
    headers
        .entry("Content-Type".to_owned())
        .or_insert_with(|| "text/plain; charset=utf-8".to_owned());
    headers
        .entry("Content-Transfer-Encoding".to_owned())
        .or_insert_with(|| "8bit".to_owned());
    headers
        .entry("X-Generator".to_owned())
        .or_insert_with(|| "ferrox".to_owned());
    if let Some(locale) = locale {
        headers.insert("Language".to_owned(), locale.to_owned());
    }
    for (key, value) in custom {
        headers.insert(key.clone(), value.clone());
    }
}

fn sort_messages(messages: &mut [CanonicalMessage], order_by: OrderBy) {
    match order_by {
        OrderBy::Msgid => messages.sort_by(|left, right| {
            left.msgid
                .cmp(&right.msgid)
                .then_with(|| left.msgctxt.cmp(&right.msgctxt))
                .then_with(|| left.obsolete.cmp(&right.obsolete))
        }),
        OrderBy::Origin => messages.sort_by(|left, right| {
            first_origin_sort_key(&left.origins)
                .cmp(&first_origin_sort_key(&right.origins))
                .then_with(|| left.msgid.cmp(&right.msgid))
                .then_with(|| left.msgctxt.cmp(&right.msgctxt))
        }),
    }
}

fn first_origin_sort_key(origins: &[CatalogOrigin]) -> (String, Option<u32>) {
    origins
        .first()
        .map(|origin| (origin.file.clone(), origin.line))
        .unwrap_or_else(|| (String::new(), None))
}

fn export_catalog_to_po(
    catalog: &Catalog,
    options: &UpdateCatalogOptions,
    locale: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<PoFile, ApiError> {
    let mut file = PoFile {
        comments: catalog.file_comments.clone(),
        extracted_comments: catalog.file_extracted_comments.clone(),
        headers: catalog
            .headers
            .iter()
            .map(|(key, value)| Header {
                key: key.clone(),
                value: value.clone(),
            })
            .collect(),
        items: Vec::with_capacity(catalog.messages.len()),
    };

    for message in &catalog.messages {
        file.items
            .push(export_message_to_po(message, options, locale, diagnostics)?);
    }

    Ok(file)
}

fn export_message_to_po(
    message: &CanonicalMessage,
    options: &UpdateCatalogOptions,
    locale: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<PoItem, ApiError> {
    let nplurals = match &message.translation {
        CanonicalTranslation::Plural {
            translation_by_category,
            ..
        } => translation_by_category.len().max(1),
        CanonicalTranslation::Singular { .. } => 1,
    };
    let mut item = PoItem::new(nplurals);
    item.msgctxt = message.msgctxt.clone();
    item.comments = message.translator_comments.clone();
    item.flags = message.flags.clone();
    item.obsolete = message.obsolete;
    item.extracted_comments = message.comments.clone();
    append_placeholder_comments(
        &mut item.extracted_comments,
        &message.placeholders,
        &options.print_placeholders_in_comments,
    );
    item.references = if options.include_origins {
        message
            .origins
            .iter()
            .map(|origin| {
                if options.include_line_numbers {
                    match origin.line {
                        Some(line) => format!("{}:{line}", origin.file),
                        None => origin.file.clone(),
                    }
                } else {
                    origin.file.clone()
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    match (&message.translation, options.plural_encoding) {
        (CanonicalTranslation::Singular { value }, _) => {
            item.msgid = message.msgid.clone();
            item.msgstr = MsgStr::from(value.clone());
        }
        (
            CanonicalTranslation::Plural {
                source,
                translation_by_category,
                variable,
            },
            PluralEncoding::Icu,
        ) => {
            item.msgid = synthesize_icu_plural(variable, &plural_source_branches(source));
            item.msgstr = MsgStr::from(synthesize_icu_plural(variable, translation_by_category));
        }
        (
            CanonicalTranslation::Plural {
                source,
                translation_by_category,
                ..
            },
            PluralEncoding::Gettext,
        ) => {
            let categories = plural_categories_for(locale, Some(translation_by_category.len()));
            if categories.is_empty() {
                return Err(ApiError::Unsupported(
                    "gettext plural export requires at least one plural category".to_owned(),
                ));
            }
            if !translation_by_category.contains_key("other") {
                diagnostics.push(
                    Diagnostic::new(
                        DiagnosticSeverity::Error,
                        "plural.unsupported_gettext_export",
                        "Plural translation is missing the required \"other\" category.",
                    )
                    .with_identity(&message.msgid, message.msgctxt.as_deref()),
                );
                return Err(ApiError::Unsupported(
                    "plural translation is missing the required \"other\" category".to_owned(),
                ));
            }
            item.msgid = source.one.clone().unwrap_or_else(|| source.other.clone());
            item.msgid_plural = Some(source.other.clone());
            item.msgstr = MsgStr::from(
                categories
                    .iter()
                    .map(|category| {
                        translation_by_category
                            .get(category)
                            .cloned()
                            .unwrap_or_default()
                    })
                    .collect::<Vec<_>>(),
            );
            item.nplurals = categories.len();
        }
    }

    Ok(item)
}

fn plural_source_branches(source: &PluralSource) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    if let Some(one) = &source.one {
        map.insert("one".to_owned(), one.clone());
    }
    map.insert("other".to_owned(), source.other.clone());
    map
}

fn append_placeholder_comments(
    comments: &mut Vec<String>,
    placeholders: &BTreeMap<String, Vec<String>>,
    mode: &PlaceholderCommentMode,
) {
    let limit = match mode {
        PlaceholderCommentMode::Disabled => return,
        PlaceholderCommentMode::Enabled { limit } => *limit,
    };

    let mut seen = comments.iter().cloned().collect::<BTreeSet<String>>();

    for (name, values) in placeholders {
        if !name.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        for value in values.iter().take(limit) {
            let comment = format!(
                "placeholder {{{name}}}: {}",
                normalize_placeholder_value(value)
            );
            if seen.insert(comment.clone()) {
                comments.push(comment);
            }
        }
    }
}

fn normalize_placeholder_value(value: &str) -> String {
    value.replace('\n', " ")
}

fn parse_catalog_to_internal(
    content: &str,
    locale_override: Option<&str>,
    plural_encoding: PluralEncoding,
    strict: bool,
) -> Result<Catalog, ApiError> {
    let file = parse_po(content)?;
    let headers = file
        .headers
        .iter()
        .map(|header| (header.key.clone(), header.value.clone()))
        .collect::<BTreeMap<_, _>>();
    let locale = locale_override
        .map(str::to_owned)
        .or_else(|| headers.get("Language").cloned());
    let nplurals = parse_nplurals_from_headers(&headers);
    let mut messages = Vec::with_capacity(file.items.len());

    for item in file.items {
        let mut conversion_diagnostics = Vec::new();
        let message = import_message_from_po(
            item,
            locale.as_deref(),
            nplurals,
            plural_encoding,
            strict,
            &mut conversion_diagnostics,
        )?;
        messages.push(message);
    }

    Ok(Catalog {
        locale,
        headers,
        file_comments: file.comments,
        file_extracted_comments: file.extracted_comments,
        messages,
    })
}

fn import_message_from_po(
    item: PoItem,
    locale: Option<&str>,
    nplurals: Option<usize>,
    plural_encoding: PluralEncoding,
    strict: bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<CanonicalMessage, ApiError> {
    let (comments, placeholders) = split_placeholder_comments(item.extracted_comments);
    let origins = item
        .references
        .iter()
        .map(|reference| parse_origin(reference))
        .collect();

    let translation = if let Some(msgid_plural) = &item.msgid_plural {
        let categories = plural_categories_for(locale, nplurals.or(Some(item.msgstr.len())));
        CanonicalTranslation::Plural {
            source: PluralSource {
                one: Some(item.msgid.clone()),
                other: msgid_plural.clone(),
            },
            translation_by_category: categories
                .iter()
                .enumerate()
                .map(|(index, category)| {
                    (
                        category.clone(),
                        item.msgstr.iter().nth(index).cloned().unwrap_or_default(),
                    )
                })
                .collect(),
            variable: "count".to_owned(),
        }
    } else {
        let msgstr = item.msgstr.first_str().unwrap_or_default().to_owned();
        if plural_encoding == PluralEncoding::Icu {
            match parse_simple_icu_plural(&item.msgid) {
                Ok(Some(source_plural)) => match parse_simple_icu_plural(&msgstr) {
                    Ok(Some(translated_plural))
                        if translated_plural.variable == source_plural.variable =>
                    {
                        CanonicalTranslation::Plural {
                            source: PluralSource {
                                one: source_plural.branches.get("one").cloned(),
                                other: source_plural
                                    .branches
                                    .get("other")
                                    .cloned()
                                    .unwrap_or_else(|| item.msgid.clone()),
                            },
                            translation_by_category: materialize_plural_categories(
                                &sorted_plural_keys(&translated_plural.branches),
                                translated_plural.branches,
                            ),
                            variable: source_plural.variable,
                        }
                    }
                    Ok(Some(_)) => {
                        if strict {
                            return Err(ApiError::Unsupported(
                                "ICU plural source and translation use different variables"
                                    .to_owned(),
                            ));
                        }
                        diagnostics.push(
                                Diagnostic::new(
                                    DiagnosticSeverity::Warning,
                                    "plural.partial_icu_parse",
                                    "Could not safely align ICU plural source and translation; keeping the message as singular.",
                                )
                                .with_identity(&item.msgid, item.msgctxt.as_deref()),
                            );
                        CanonicalTranslation::Singular { value: msgstr }
                    }
                    Ok(None) | Err(_) => {
                        if strict {
                            return Err(ApiError::Unsupported(
                                "ICU plural message could not be parsed in strict mode".to_owned(),
                            ));
                        }
                        diagnostics.push(
                                Diagnostic::new(
                                    DiagnosticSeverity::Warning,
                                    "plural.partial_icu_parse",
                                    "Could not fully parse ICU plural translation; keeping the message as singular.",
                                )
                                .with_identity(&item.msgid, item.msgctxt.as_deref()),
                            );
                        CanonicalTranslation::Singular { value: msgstr }
                    }
                },
                Ok(None) => CanonicalTranslation::Singular { value: msgstr },
                Err(_) if strict => {
                    return Err(ApiError::Unsupported(
                        "ICU plural parsing failed in strict mode".to_owned(),
                    ));
                }
                Err(_) => CanonicalTranslation::Singular { value: msgstr },
            }
        } else {
            CanonicalTranslation::Singular { value: msgstr }
        }
    };

    Ok(CanonicalMessage {
        msgid: item.msgid,
        msgctxt: item.msgctxt,
        translation,
        comments,
        origins,
        placeholders,
        obsolete: item.obsolete,
        translator_comments: item.comments,
        flags: item.flags,
    })
}

fn split_placeholder_comments(
    extracted_comments: Vec<String>,
) -> (Vec<String>, BTreeMap<String, Vec<String>>) {
    let mut comments = Vec::new();
    let mut placeholders = BTreeMap::<String, Vec<String>>::new();

    for comment in extracted_comments {
        if let Some((name, value)) = parse_placeholder_comment(&comment) {
            placeholders.entry(name).or_default().push(value);
        } else {
            comments.push(comment);
        }
    }

    (comments, dedupe_placeholders(placeholders))
}

fn parse_placeholder_comment(comment: &str) -> Option<(String, String)> {
    let rest = comment.strip_prefix("placeholder {")?;
    let end = rest.find("}: ")?;
    Some((rest[..end].to_owned(), rest[end + 3..].to_owned()))
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

fn parse_nplurals_from_headers(headers: &BTreeMap<String, String>) -> Option<usize> {
    let plural_forms = headers.get("Plural-Forms")?;
    plural_forms
        .split(';')
        .find_map(|part| part.trim().strip_prefix("nplurals=")?.trim().parse().ok())
}

fn public_message_from_canonical(message: CanonicalMessage) -> CatalogMessage {
    let translation = match message.translation {
        CanonicalTranslation::Singular { value } => TranslationShape::Singular { value },
        CanonicalTranslation::Plural {
            source,
            translation_by_category,
            ..
        } => TranslationShape::Plural {
            source,
            translation: translation_by_category,
        },
    };

    CatalogMessage {
        msgid: message.msgid,
        msgctxt: message.msgctxt,
        translation,
        comments: message.comments,
        origin: message.origins,
        obsolete: message.obsolete,
        extra: Some(CatalogMessageExtra {
            translator_comments: message.translator_comments,
            flags: message.flags,
        }),
    }
}

fn plural_categories_for(locale: Option<&str>, nplurals: Option<usize>) -> Vec<String> {
    let locale = locale.unwrap_or_default().to_ascii_lowercase();
    let categories = if locale.starts_with("cs")
        || locale.starts_with("pl")
        || locale.starts_with("ru")
        || locale.starts_with("uk")
        || locale.starts_with("sr")
    {
        vec!["one", "few", "many", "other"]
    } else if locale.starts_with("ar") {
        vec!["zero", "one", "two", "few", "many", "other"]
    } else {
        match nplurals.unwrap_or(2) {
            0 | 1 => vec!["other"],
            2 => vec!["one", "other"],
            3 => vec!["one", "few", "other"],
            4 => vec!["one", "few", "many", "other"],
            5 => vec!["zero", "one", "few", "many", "other"],
            _ => vec!["zero", "one", "two", "few", "many", "other"],
        }
    };

    categories.into_iter().map(str::to_owned).collect()
}

fn sorted_plural_keys(map: &BTreeMap<String, String>) -> Vec<String> {
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

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            out.push(value);
        }
    }
    out
}

fn merge_unique_strings(target: &mut Vec<String>, incoming: Vec<String>) {
    let mut seen = target.iter().cloned().collect::<BTreeSet<_>>();
    for value in incoming {
        if seen.insert(value.clone()) {
            target.push(value);
        }
    }
}

fn dedupe_origins(values: Vec<CatalogOrigin>) -> Vec<CatalogOrigin> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for value in values {
        if seen.insert((value.file.clone(), value.line)) {
            out.push(value);
        }
    }
    out
}

fn merge_unique_origins(target: &mut Vec<CatalogOrigin>, incoming: Vec<CatalogOrigin>) {
    let mut seen = target
        .iter()
        .map(|origin| (origin.file.clone(), origin.line))
        .collect::<BTreeSet<_>>();
    for value in incoming {
        if seen.insert((value.file.clone(), value.line)) {
            target.push(value);
        }
    }
}

fn dedupe_placeholders(
    placeholders: BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, Vec<String>> {
    placeholders
        .into_iter()
        .map(|(key, values)| (key, dedupe_strings(values)))
        .collect()
}

fn merge_placeholders(
    target: &mut BTreeMap<String, Vec<String>>,
    incoming: BTreeMap<String, Vec<String>>,
) {
    for (key, values) in incoming {
        merge_unique_strings(target.entry(key).or_default(), values);
    }
}

fn derive_plural_variable(placeholders: &BTreeMap<String, Vec<String>>) -> Option<String> {
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

fn synthesize_icu_plural(variable: &str, branches: &BTreeMap<String, String>) -> String {
    let mut out = String::new();
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

fn parse_simple_icu_plural(input: &str) -> Result<Option<ParsedIcuPlural>, ()> {
    let trimmed = input.trim();
    if !trimmed.starts_with('{') || !trimmed.ends_with('}') || !trimmed.contains(", plural,") {
        return Ok(None);
    }

    let inner = &trimmed[1..trimmed.len() - 1];
    let first_comma = inner.find(',').ok_or(())?;
    let variable = inner[..first_comma].trim();
    if variable.is_empty() {
        return Err(());
    }
    let rest = inner[first_comma + 1..].trim_start();
    let rest = rest.strip_prefix("plural,").ok_or(())?.trim_start();
    let mut branches = BTreeMap::new();
    let mut index = 0usize;
    let bytes = rest.as_bytes();

    while index < bytes.len() {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }

        let selector_start = index;
        while index < bytes.len() && !bytes[index].is_ascii_whitespace() && bytes[index] != b'{' {
            index += 1;
        }
        let selector = rest[selector_start..index].trim();
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() || bytes[index] != b'{' {
            return Err(());
        }
        index += 1;
        let value_start = index;
        let mut depth = 1usize;
        while index < bytes.len() && depth > 0 {
            match bytes[index] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
            index += 1;
        }
        if depth != 0 {
            return Err(());
        }
        let value = rest[value_start..index - 1].to_owned();
        if selector.starts_with('=') {
            return Err(());
        }
        branches.insert(selector.to_owned(), value);
    }

    if branches.contains_key("other") {
        Ok(Some(ParsedIcuPlural {
            variable: variable.to_owned(),
            branches,
        }))
    } else {
        Err(())
    }
}

fn atomic_write(path: &Path, content: &str) -> Result<(), ApiError> {
    let directory = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(directory)?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            ApiError::InvalidArguments("target_path must have a file name".to_owned())
        })?;
    let temp_path = directory.join(format!(".{file_name}.ferrox.tmp"));
    fs::write(&temp_path, content)?;
    fs::rename(&temp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        ApiError, DiagnosticSeverity, ExtractedMessage, ExtractedPluralMessage,
        ExtractedSingularMessage, ObsoleteStrategy, ParseCatalogOptions, PluralEncoding,
        PluralSource, TranslationShape, UpdateCatalogFileOptions, UpdateCatalogOptions,
        parse_catalog, update_catalog, update_catalog_file,
    };
    use crate::parse_po;
    use std::collections::BTreeMap;
    use std::fs;

    #[test]
    fn update_catalog_creates_new_source_locale_messages() {
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("en".to_owned()),
            extracted: vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            })],
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        let parsed = parse_po(&result.content).expect("parse output");
        assert_eq!(parsed.items.len(), 1);
        assert_eq!(parsed.items[0].msgid, "Hello");
        assert_eq!(parsed.items[0].msgstr[0], "Hello");
        assert!(result.created);
        assert!(result.updated);
        assert_eq!(result.stats.added, 1);
    }

    #[test]
    fn update_catalog_preserves_non_source_translations() {
        let existing = "msgid \"Hello\"\nmsgstr \"Hallo\"\n";
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("de".to_owned()),
            existing: Some(existing.to_owned()),
            extracted: vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            })],
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        let parsed = parse_po(&result.content).expect("parse output");
        assert_eq!(parsed.items[0].msgstr[0], "Hallo");
        assert_eq!(result.stats.unchanged, 1);
    }

    #[test]
    fn overwrite_source_translations_refreshes_source_locale() {
        let existing = "msgid \"Hello\"\nmsgstr \"Old\"\n";
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("en".to_owned()),
            existing: Some(existing.to_owned()),
            overwrite_source_translations: true,
            extracted: vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            })],
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        let parsed = parse_po(&result.content).expect("parse output");
        assert_eq!(parsed.items[0].msgstr[0], "Hello");
        assert_eq!(result.stats.changed, 1);
    }

    #[test]
    fn obsolete_strategy_delete_removes_missing_messages() {
        let existing = concat!("msgid \"keep\"\nmsgstr \"x\"\n\nmsgid \"drop\"\nmsgstr \"y\"\n");
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("de".to_owned()),
            existing: Some(existing.to_owned()),
            obsolete_strategy: ObsoleteStrategy::Delete,
            extracted: vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "keep".to_owned(),
                ..ExtractedSingularMessage::default()
            })],
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        let parsed = parse_po(&result.content).expect("parse output");
        assert_eq!(parsed.items.len(), 1);
        assert_eq!(result.stats.obsolete_removed, 1);
    }

    #[test]
    fn duplicate_conflicts_fail_hard() {
        let error = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("en".to_owned()),
            extracted: vec![
                ExtractedMessage::Singular(ExtractedSingularMessage {
                    msgid: "Hello".to_owned(),
                    ..ExtractedSingularMessage::default()
                }),
                ExtractedMessage::Plural(ExtractedPluralMessage {
                    msgid: "Hello".to_owned(),
                    source: PluralSource {
                        one: Some("One".to_owned()),
                        other: "Many".to_owned(),
                    },
                    ..ExtractedPluralMessage::default()
                }),
            ],
            ..UpdateCatalogOptions::default()
        })
        .expect_err("conflict");

        assert!(matches!(error, ApiError::Conflict(_)));
    }

    #[test]
    fn plural_icu_export_uses_structural_input() {
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("en".to_owned()),
            extracted: vec![ExtractedMessage::Plural(ExtractedPluralMessage {
                msgid: "{count, plural, one {# item} other {# items}}".to_owned(),
                source: PluralSource {
                    one: Some("# item".to_owned()),
                    other: "# items".to_owned(),
                },
                placeholders: BTreeMap::from([("count".to_owned(), vec!["count".to_owned()])]),
                ..ExtractedPluralMessage::default()
            })],
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        let parsed = parse_po(&result.content).expect("parse output");
        assert!(parsed.items[0].msgid.contains("{count, plural,"));
        assert!(parsed.items[0].msgid_plural.is_none());
    }

    #[test]
    fn parse_catalog_projects_gettext_plural_into_structured_shape() {
        let parsed = parse_catalog(ParseCatalogOptions {
            content: concat!(
                "msgid \"book\"\n",
                "msgid_plural \"books\"\n",
                "msgstr[0] \"Buch\"\n",
                "msgstr[1] \"Buecher\"\n",
            )
            .to_owned(),
            locale: Some("de".to_owned()),
            source_locale: "en".to_owned(),
            plural_encoding: PluralEncoding::Gettext,
            strict: false,
        })
        .expect("parse");

        match &parsed.messages[0].translation {
            TranslationShape::Plural {
                source,
                translation,
            } => {
                assert_eq!(source.one.as_deref(), Some("book"));
                assert_eq!(source.other, "books");
                assert_eq!(translation.get("one").map(String::as_str), Some("Buch"));
                assert_eq!(
                    translation.get("other").map(String::as_str),
                    Some("Buecher")
                );
            }
            other => panic!("expected plural translation, got {other:?}"),
        }
    }

    #[test]
    fn parse_catalog_detects_simple_icu_plural_when_requested() {
        let parsed = parse_catalog(ParseCatalogOptions {
            content: concat!(
                "msgid \"{count, plural, one {# item} other {# items}}\"\n",
                "msgstr \"{count, plural, one {# Artikel} other {# Artikel}}\"\n",
            )
            .to_owned(),
            locale: Some("de".to_owned()),
            source_locale: "en".to_owned(),
            plural_encoding: PluralEncoding::Icu,
            strict: false,
        })
        .expect("parse");

        match &parsed.messages[0].translation {
            TranslationShape::Plural { translation, .. } => {
                assert_eq!(
                    translation.get("one").map(String::as_str),
                    Some("# Artikel")
                );
                assert_eq!(
                    translation.get("other").map(String::as_str),
                    Some("# Artikel")
                );
            }
            other => panic!("expected plural translation, got {other:?}"),
        }
    }

    #[test]
    fn parse_catalog_strict_fails_on_malformed_icu_plural() {
        let error = parse_catalog(ParseCatalogOptions {
            content: concat!(
                "msgid \"{count, plural, one {# item} other {# items}\"\n",
                "msgstr \"{count, plural, one {# Artikel} other {# Artikel}}\"\n",
            )
            .to_owned(),
            locale: Some("de".to_owned()),
            source_locale: "en".to_owned(),
            plural_encoding: PluralEncoding::Icu,
            strict: true,
        })
        .expect_err("strict parse should fail");

        match error {
            ApiError::Unsupported(message) => assert!(message.contains("strict mode")),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn update_catalog_file_writes_only_when_changed() {
        let temp_dir = std::env::temp_dir().join("ferrox-po-update-file-test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("create temp dir");
        let path = temp_dir.join("messages.po");

        let first = update_catalog_file(UpdateCatalogFileOptions {
            target_path: path.clone(),
            source_locale: "en".to_owned(),
            locale: Some("en".to_owned()),
            extracted: vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            })],
            ..UpdateCatalogFileOptions::default()
        })
        .expect("first write");
        assert!(first.created);

        let second = update_catalog_file(UpdateCatalogFileOptions {
            target_path: path.clone(),
            source_locale: "en".to_owned(),
            locale: Some("en".to_owned()),
            extracted: vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            })],
            ..UpdateCatalogFileOptions::default()
        })
        .expect("second write");
        assert!(!second.created);
        assert!(!second.updated);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn update_catalog_gettext_export_emits_plural_slots() {
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("de".to_owned()),
            plural_encoding: PluralEncoding::Gettext,
            extracted: vec![ExtractedMessage::Plural(ExtractedPluralMessage {
                msgid: "books".to_owned(),
                source: PluralSource {
                    one: Some("book".to_owned()),
                    other: "books".to_owned(),
                },
                placeholders: BTreeMap::from([("count".to_owned(), vec!["count".to_owned()])]),
                ..ExtractedPluralMessage::default()
            })],
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        let parsed = parse_po(&result.content).expect("parse output");
        assert_eq!(parsed.items[0].msgid, "book");
        assert_eq!(parsed.items[0].msgid_plural.as_deref(), Some("books"));
        assert_eq!(parsed.items[0].msgstr.len(), 2);
    }

    #[test]
    fn parse_catalog_requires_source_locale() {
        let error = parse_catalog(ParseCatalogOptions {
            content: String::new(),
            source_locale: String::new(),
            ..ParseCatalogOptions::default()
        })
        .expect_err("missing source locale");

        match error {
            ApiError::InvalidArguments(message) => {
                assert!(message.contains("source_locale"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn warnings_use_expected_namespace() {
        let mut placeholders = BTreeMap::new();
        placeholders.insert("first".to_owned(), vec!["first".to_owned()]);
        placeholders.insert("second".to_owned(), vec!["second".to_owned()]);

        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("de".to_owned()),
            extracted: vec![ExtractedMessage::Plural(ExtractedPluralMessage {
                msgid: "Developers".to_owned(),
                source: PluralSource {
                    one: Some("Developer".to_owned()),
                    other: "Developers".to_owned(),
                },
                placeholders,
                ..ExtractedPluralMessage::default()
            })],
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.starts_with("plural."))
        );
        assert!(result.diagnostics.iter().all(|diagnostic| matches!(
            diagnostic.severity,
            DiagnosticSeverity::Warning | DiagnosticSeverity::Error | DiagnosticSeverity::Info
        )));
    }
}
