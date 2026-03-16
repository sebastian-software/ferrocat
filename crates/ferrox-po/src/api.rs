use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use ferrox_icu::{IcuMessage, IcuNode, IcuPluralKind, parse_icu};
use icu_locale::Locale;
use icu_plurals::{PluralCategory, PluralRules};
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
    diagnostics: Vec<Diagnostic>,
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

enum IcuPluralProjection {
    NotPlural,
    Projected(ParsedIcuPlural),
    Unsupported(&'static str),
    Malformed,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct ParsedPluralFormsHeader {
    raw: Option<String>,
    nplurals: Option<usize>,
    plural: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PluralProfile {
    categories: Vec<String>,
}

impl PluralProfile {
    fn new(locale: Option<&str>, nplurals: Option<usize>) -> Self {
        let categories = if let Some(locale_categories) = locale.and_then(icu_plural_categories_for) {
            if nplurals.is_none() || nplurals == Some(locale_categories.len()) {
                locale_categories
            } else {
                fallback_plural_categories(nplurals)
            }
        } else {
            fallback_plural_categories(nplurals)
        };

        Self { categories }
    }

    fn for_locale(locale: Option<&str>) -> Self {
        Self::new(locale, None)
    }

    fn for_gettext_slots(locale: Option<&str>, nplurals: Option<usize>) -> Self {
        Self::new(locale, nplurals)
    }

    fn for_translation(locale: Option<&str>, translation_by_category: &BTreeMap<String, String>) -> Self {
        Self::new(locale, Some(translation_by_category.len()))
    }

    fn categories(&self) -> &[String] {
        &self.categories
    }

    fn nplurals(&self) -> usize {
        self.categories.len().max(1)
    }

    fn materialize_translation(
        &self,
        translation: &BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        self.categories
            .iter()
            .map(|category| {
                (
                    category.clone(),
                    translation.get(category).cloned().unwrap_or_default(),
                )
            })
            .collect()
    }

    fn source_locale_translation(&self, source: &PluralSource) -> BTreeMap<String, String> {
        let mut translation = BTreeMap::new();
        for category in &self.categories {
            let value = match category.as_str() {
                "one" => source.one.clone().unwrap_or_else(|| source.other.clone()),
                "other" => source.other.clone(),
                _ => source.other.clone(),
            };
            translation.insert(category.clone(), value);
        }
        translation
    }

    fn empty_translation(&self) -> BTreeMap<String, String> {
        self.categories
            .iter()
            .map(|category| (category.clone(), String::new()))
            .collect()
    }

    fn gettext_values(&self, translation: &BTreeMap<String, String>) -> Vec<String> {
        self.categories
            .iter()
            .map(|category| translation.get(category).cloned().unwrap_or_default())
            .collect()
    }

    fn gettext_header(&self) -> Option<String> {
        match self.nplurals() {
            1 => Some("nplurals=1; plural=0;".to_owned()),
            2 => Some("nplurals=2; plural=(n != 1);".to_owned()),
            _ => None,
        }
    }
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
            diagnostics: Vec::new(),
        },
    };

    let locale = options
        .locale
        .clone()
        .or_else(|| existing.locale.clone())
        .or_else(|| existing.headers.get("Language").cloned());
    let normalized = normalize_extracted(&options.extracted)?;
    let mut diagnostics = existing.diagnostics.clone();
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
        options.plural_encoding,
        &mut diagnostics,
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
        diagnostics: catalog.diagnostics,
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
            diagnostics: existing.diagnostics,
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
    let plural_profile = match &next.kind {
        NormalizedKind::Singular => None,
        NormalizedKind::Plural(_) => Some(PluralProfile::for_locale(locale)),
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
                translation_by_category: plural_profile
                    .as_ref()
                    .expect("plural messages require plural profile")
                    .materialize_translation(&previous_map),
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
                    plural_profile
                        .as_ref()
                        .expect("plural messages require plural profile")
                        .source_locale_translation(source)
                } else {
                    plural_profile
                        .as_ref()
                        .expect("plural messages require plural profile")
                        .empty_translation()
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
    plural_encoding: PluralEncoding,
    diagnostics: &mut Vec<Diagnostic>,
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
    if plural_encoding == PluralEncoding::Gettext && !custom.contains_key("Plural-Forms") {
        let profile = PluralProfile::for_locale(locale);
        let parsed_header = parse_plural_forms_from_headers(headers);
        match (parsed_header.raw.as_deref(), profile.gettext_header()) {
            (None, Some(header)) => {
                headers.insert("Plural-Forms".to_owned(), header);
            }
            (None, None) => diagnostics.push(Diagnostic::new(
                DiagnosticSeverity::Info,
                "plural.missing_plural_forms_header",
                "No safe default Plural-Forms header is known for this locale; keeping the header unset.",
            )),
            (Some(_), Some(header))
                if parsed_header.nplurals == Some(profile.nplurals())
                    && parsed_header.plural.is_none() =>
            {
                headers.insert("Plural-Forms".to_owned(), header);
                diagnostics.push(Diagnostic::new(
                    DiagnosticSeverity::Info,
                    "plural.completed_plural_forms_header",
                    "Plural-Forms header was missing the plural expression and has been completed using a safe locale default.",
                ));
            }
            _ => {}
        }
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
    let plural_profile = match &message.translation {
        CanonicalTranslation::Plural {
            translation_by_category,
            ..
        } => Some(PluralProfile::for_translation(locale, translation_by_category)),
        CanonicalTranslation::Singular { .. } => None,
    };
    let nplurals = match &message.translation {
        CanonicalTranslation::Plural {
            translation_by_category,
            ..
        } => plural_profile
            .as_ref()
            .expect("plural messages require plural profile")
            .nplurals()
            .max(translation_by_category.len().max(1)),
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
                plural_profile
                    .as_ref()
                    .expect("plural messages require plural profile")
                    .gettext_values(translation_by_category),
            );
            item.nplurals = plural_profile
                .as_ref()
                .expect("plural messages require plural profile")
                .nplurals();
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
    let plural_forms = parse_plural_forms_from_headers(&headers);
    let nplurals = plural_forms.nplurals;
    let mut diagnostics = Vec::new();
    validate_plural_forms_header(
        locale.as_deref(),
        &plural_forms,
        plural_encoding,
        &mut diagnostics,
    );
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
        diagnostics.extend(conversion_diagnostics);
        messages.push(message);
    }

    Ok(Catalog {
        locale,
        headers,
        file_comments: file.comments,
        file_extracted_comments: file.extracted_comments,
        messages,
        diagnostics,
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
        let plural_profile =
            PluralProfile::for_gettext_slots(locale, nplurals.or(Some(item.msgstr.len())));
        CanonicalTranslation::Plural {
            source: PluralSource {
                one: Some(item.msgid.clone()),
                other: msgid_plural.clone(),
            },
            translation_by_category: plural_profile
                .categories()
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
            match project_icu_plural(&item.msgid) {
                IcuPluralProjection::Projected(source_plural) => {
                    let translated_projection = project_icu_plural(&msgstr);
                    match translated_projection {
                    IcuPluralProjection::Projected(translated_plural)
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
                    IcuPluralProjection::Projected(_) => {
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
                    IcuPluralProjection::Unsupported(_) | IcuPluralProjection::Malformed => {
                        if strict && matches!(translated_projection, IcuPluralProjection::Malformed)
                        {
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
                    IcuPluralProjection::NotPlural => {
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
                }
                }
                IcuPluralProjection::NotPlural => CanonicalTranslation::Singular { value: msgstr },
                IcuPluralProjection::Malformed if strict => {
                    return Err(ApiError::Unsupported(
                        "ICU plural parsing failed in strict mode".to_owned(),
                    ));
                }
                IcuPluralProjection::Unsupported(message) => {
                    diagnostics.push(
                        Diagnostic::new(
                            DiagnosticSeverity::Warning,
                            "plural.unsupported_icu_projection",
                            message,
                        )
                        .with_identity(&item.msgid, item.msgctxt.as_deref()),
                    );
                    CanonicalTranslation::Singular { value: msgstr }
                }
                IcuPluralProjection::Malformed => CanonicalTranslation::Singular { value: msgstr },
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

fn parse_plural_forms_from_headers(headers: &BTreeMap<String, String>) -> ParsedPluralFormsHeader {
    let Some(plural_forms) = headers.get("Plural-Forms") else {
        return ParsedPluralFormsHeader::default();
    };

    let mut parsed = ParsedPluralFormsHeader {
        raw: Some(plural_forms.clone()),
        ..ParsedPluralFormsHeader::default()
    };
    for part in plural_forms.split(';') {
        let trimmed = part.trim();
        if let Some(value) = trimmed.strip_prefix("nplurals=") {
            parsed.nplurals = value.trim().parse().ok();
        } else if let Some(value) = trimmed.strip_prefix("plural=") {
            let value = value.trim();
            if !value.is_empty() {
                parsed.plural = Some(value.to_owned());
            }
        }
    }

    parsed
}

fn validate_plural_forms_header(
    locale: Option<&str>,
    plural_forms: &ParsedPluralFormsHeader,
    plural_encoding: PluralEncoding,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if plural_encoding != PluralEncoding::Gettext {
        return;
    }

    if let Some(nplurals) = plural_forms.nplurals {
        let profile = PluralProfile::for_locale(locale);
        let expected = profile.nplurals();
        if locale.is_some() && nplurals != expected {
            diagnostics.push(Diagnostic::new(
                DiagnosticSeverity::Warning,
                "plural.nplurals_locale_mismatch",
                format!(
                    "Plural-Forms declares nplurals={nplurals}, but locale-derived categories expect {expected}."
                ),
            ));
        }
    } else if plural_forms.plural.is_some() {
        diagnostics.push(Diagnostic::new(
            DiagnosticSeverity::Warning,
            "parse.invalid_plural_forms_header",
            "Plural-Forms header contains a plural expression but no parseable nplurals value.",
        ));
    }

    if plural_forms.nplurals.is_some() && plural_forms.plural.is_none() {
        diagnostics.push(Diagnostic::new(
            DiagnosticSeverity::Info,
            "plural.missing_plural_expression",
            "Plural-Forms header declares nplurals but omits the plural expression.",
        ));
    }
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

fn icu_plural_categories_for(locale: &str) -> Option<Vec<String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<Vec<String>>>>> = OnceLock::new();

    let normalized = normalize_plural_locale(locale);
    if normalized.is_empty() {
        return None;
    }

    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(cached) = cache
        .lock()
        .expect("plural category cache mutex poisoned")
        .get(&normalized)
        .cloned()
    {
        return cached;
    }

    let resolved = normalized
        .parse::<Locale>()
        .ok()
        .and_then(|locale| PluralRules::try_new_cardinal(locale.into()).ok())
        .map(|rules| {
            rules
                .categories()
                .map(plural_category_name)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        });

    cache
        .lock()
        .expect("plural category cache mutex poisoned")
        .insert(normalized, resolved.clone());

    resolved
}

fn normalize_plural_locale(locale: &str) -> String {
    locale.trim().replace('_', "-")
}

fn plural_category_name(category: PluralCategory) -> &'static str {
    match category {
        PluralCategory::Zero => "zero",
        PluralCategory::One => "one",
        PluralCategory::Two => "two",
        PluralCategory::Few => "few",
        PluralCategory::Many => "many",
        PluralCategory::Other => "other",
    }
}

fn fallback_plural_categories(nplurals: Option<usize>) -> Vec<String> {
    let categories = match nplurals.unwrap_or(2) {
        0 | 1 => vec!["other"],
        2 => vec!["one", "other"],
        3 => vec!["one", "few", "other"],
        4 => vec!["one", "few", "many", "other"],
        5 => vec!["zero", "one", "few", "many", "other"],
        _ => vec!["zero", "one", "two", "few", "many", "other"],
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

fn project_icu_plural(input: &str) -> IcuPluralProjection {
    if !looks_like_icu_message(input.as_bytes()) {
        return IcuPluralProjection::NotPlural;
    }

    let message = match parse_icu(input) {
        Ok(message) => message,
        Err(_) => return IcuPluralProjection::Malformed,
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
fn looks_like_icu_message(input: &[u8]) -> bool {
    input
        .iter()
        .any(|byte| matches!(byte, b'{' | b'}' | b'<'))
}

fn only_node(message: &IcuMessage) -> Option<&IcuNode> {
    match message.nodes.as_slice() {
        [node] => Some(node),
        _ => None,
    }
}

fn render_projectable_icu_nodes(nodes: &[IcuNode]) -> Result<String, &'static str> {
    let mut out = String::new();
    for node in nodes {
        render_projectable_icu_node(node, &mut out)?;
    }
    Ok(out)
}

fn render_projectable_icu_node(node: &IcuNode, out: &mut String) -> Result<(), &'static str> {
    match node {
        IcuNode::Literal(value) => out.push_str(&escape_icu_literal(value)),
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
            render_formatter("duration", name, style.as_deref(), out)
        }
        IcuNode::Ago { name, style } => render_formatter("ago", name, style.as_deref(), out),
        IcuNode::Name { name, style } => render_formatter("name", name, style.as_deref(), out),
        IcuNode::Pound => out.push('#'),
        IcuNode::Tag { name, children } => {
            out.push('<');
            out.push_str(name);
            out.push('>');
            for child in children {
                render_projectable_icu_node(child, out)?;
            }
            out.push_str("</");
            out.push_str(name);
            out.push('>');
        }
        IcuNode::Select { .. } | IcuNode::Plural { .. } => {
            return Err(
                "Nested ICU select/plural structures are not projected into the current catalog plural model.",
            )
        }
    }

    Ok(())
}

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

fn escape_icu_literal(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
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
    out
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
    fn parse_catalog_uses_icu_plural_categories_for_french_gettext() {
        let parsed = parse_catalog(ParseCatalogOptions {
            content: concat!(
                "msgid \"fichier\"\n",
                "msgid_plural \"fichiers\"\n",
                "msgstr[0] \"fichier\"\n",
                "msgstr[1] \"millions de fichiers\"\n",
                "msgstr[2] \"fichiers\"\n",
            )
            .to_owned(),
            locale: Some("fr".to_owned()),
            source_locale: "en".to_owned(),
            plural_encoding: PluralEncoding::Gettext,
            strict: false,
        })
        .expect("parse");

        match &parsed.messages[0].translation {
            TranslationShape::Plural { translation, .. } => {
                assert_eq!(translation.get("one").map(String::as_str), Some("fichier"));
                assert_eq!(
                    translation.get("many").map(String::as_str),
                    Some("millions de fichiers")
                );
                assert_eq!(translation.get("other").map(String::as_str), Some("fichiers"));
            }
            other => panic!("expected plural translation, got {other:?}"),
        }
    }

    #[test]
    fn parse_catalog_prefers_gettext_slot_count_when_it_disagrees_with_locale_categories() {
        let parsed = parse_catalog(ParseCatalogOptions {
            content: concat!(
                "msgid \"\"\n",
                "msgstr \"\"\n",
                "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n",
                "\n",
                "msgid \"livre\"\n",
                "msgid_plural \"livres\"\n",
                "msgstr[0] \"livre\"\n",
                "msgstr[1] \"livres\"\n",
            )
            .to_owned(),
            locale: Some("fr".to_owned()),
            source_locale: "en".to_owned(),
            plural_encoding: PluralEncoding::Gettext,
            strict: false,
        })
        .expect("parse");

        match &parsed.messages[0].translation {
            TranslationShape::Plural { translation, .. } => {
                assert_eq!(translation.len(), 2);
                assert_eq!(translation.get("one").map(String::as_str), Some("livre"));
                assert_eq!(translation.get("other").map(String::as_str), Some("livres"));
                assert!(translation.get("many").is_none());
            }
            other => panic!("expected plural translation, got {other:?}"),
        }
    }

    #[test]
    fn parse_catalog_reports_plural_forms_locale_mismatch() {
        let parsed = parse_catalog(ParseCatalogOptions {
            content: concat!(
                "msgid \"\"\n",
                "msgstr \"\"\n",
                "\"Language: fr\\n\"\n",
                "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n",
            )
            .to_owned(),
            locale: Some("fr".to_owned()),
            source_locale: "en".to_owned(),
            plural_encoding: PluralEncoding::Gettext,
            strict: false,
        })
        .expect("parse");

        assert!(parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "plural.nplurals_locale_mismatch"));
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
    fn parse_catalog_warns_and_falls_back_for_unsupported_nested_icu_plural() {
        let parsed = parse_catalog(ParseCatalogOptions {
            content: concat!(
                "msgid \"{count, plural, one {{gender, select, male {He has one item} other {They have one item}}} other {{gender, select, male {He has # items} other {They have # items}}}}\"\n",
                "msgstr \"{count, plural, one {{gender, select, male {Er hat einen Artikel} other {Sie haben einen Artikel}}} other {{gender, select, male {Er hat # Artikel} other {Sie haben # Artikel}}}}\"\n",
            )
            .to_owned(),
            locale: Some("de".to_owned()),
            source_locale: "en".to_owned(),
            plural_encoding: PluralEncoding::Icu,
            strict: false,
        })
        .expect("parse");

        assert!(parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "plural.unsupported_icu_projection"));
        assert!(matches!(
            parsed.messages[0].translation,
            TranslationShape::Singular { .. }
        ));
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
    fn update_catalog_gettext_export_uses_icu_plural_categories_for_french() {
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("fr".to_owned()),
            plural_encoding: PluralEncoding::Gettext,
            extracted: vec![ExtractedMessage::Plural(ExtractedPluralMessage {
                msgid: "files".to_owned(),
                source: PluralSource {
                    one: Some("file".to_owned()),
                    other: "files".to_owned(),
                },
                placeholders: BTreeMap::from([("count".to_owned(), vec!["count".to_owned()])]),
                ..ExtractedPluralMessage::default()
            })],
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        let parsed = parse_po(&result.content).expect("parse output");
        assert_eq!(parsed.items[0].msgstr.len(), 3);
    }

    #[test]
    fn update_catalog_gettext_sets_safe_plural_forms_header_for_two_form_locale() {
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("de".to_owned()),
            plural_encoding: PluralEncoding::Gettext,
            extracted: vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            })],
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        let parsed = parse_po(&result.content).expect("parse output");
        let plural_forms = parsed
            .headers
            .iter()
            .find(|header| header.key == "Plural-Forms")
            .map(|header| header.value.as_str());
        assert_eq!(plural_forms, Some("nplurals=2; plural=(n != 1);"));
    }

    #[test]
    fn update_catalog_gettext_reports_when_no_safe_plural_forms_header_is_known() {
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("fr".to_owned()),
            plural_encoding: PluralEncoding::Gettext,
            extracted: vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Bonjour".to_owned(),
                ..ExtractedSingularMessage::default()
            })],
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "plural.missing_plural_forms_header"));
    }

    #[test]
    fn update_catalog_gettext_completes_partial_plural_forms_header_when_safe() {
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("de".to_owned()),
            plural_encoding: PluralEncoding::Gettext,
            existing: Some(
                concat!(
                    "msgid \"\"\n",
                    "msgstr \"\"\n",
                    "\"Language: de\\n\"\n",
                    "\"Plural-Forms: nplurals=2;\\n\"\n",
                )
                .to_owned(),
            ),
            extracted: vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            })],
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "plural.completed_plural_forms_header"));

        let parsed = parse_po(&result.content).expect("parse output");
        let plural_forms = parsed
            .headers
            .iter()
            .find(|header| header.key == "Plural-Forms")
            .map(|header| header.value.as_str());
        assert_eq!(plural_forms, Some("nplurals=2; plural=(n != 1);"));
    }

    #[test]
    fn update_catalog_gettext_preserves_existing_complete_plural_forms_header() {
        let result = update_catalog(UpdateCatalogOptions {
            source_locale: "en".to_owned(),
            locale: Some("de".to_owned()),
            plural_encoding: PluralEncoding::Gettext,
            existing: Some(
                concat!(
                    "msgid \"\"\n",
                    "msgstr \"\"\n",
                    "\"Language: de\\n\"\n",
                    "\"Plural-Forms: nplurals=2; plural=(n > 1);\\n\"\n",
                )
                .to_owned(),
            ),
            extracted: vec![ExtractedMessage::Singular(ExtractedSingularMessage {
                msgid: "Hello".to_owned(),
                ..ExtractedSingularMessage::default()
            })],
            ..UpdateCatalogOptions::default()
        })
        .expect("update");

        assert!(!result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "plural.completed_plural_forms_header"));

        let parsed = parse_po(&result.content).expect("parse output");
        let plural_forms = parsed
            .headers
            .iter()
            .find(|header| header.key == "Plural-Forms")
            .map(|header| header.value.as_str());
        assert_eq!(plural_forms, Some("nplurals=2; plural=(n > 1);"));
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
