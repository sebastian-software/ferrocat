use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use ferrocat::runtime::{
    compile_catalog as compile_catalog_runtime, compile_icu as compile_icu_runtime,
    CompiledCatalog, CompiledMessage, MessageValue, MessageValues,
};
use ferrocat::{
    catalog_to_items, compare_variables, create_default_headers, create_reference,
    extract_variable_info, extract_variables, format_po_date, format_reference, format_references,
    generate_message_id, generate_message_ids, get_plural_categories, get_plural_count,
    get_plural_forms_header, get_plural_index, gettext_to_icu, has_icu_syntax, has_plural,
    has_select, has_select_ordinal, icu_to_gettext_source, is_plural_item, items_to_catalog,
    merge_catalogs, normalize_file_path, normalize_item_to_icu, normalize_to_icu, parse_icu,
    parse_plural_forms, parse_po, parse_reference, parse_references, serialize_compiled_catalog,
    stringify_po, validate_icu, Catalog, CatalogEntry, CatalogKeyStrategy, CatalogToItemsOptions,
    CatalogTranslation, CompileCatalogOptions, CompileIcuOptions, CreateHeadersOptions,
    FormatReferenceOptions, GettextToIcuOptions, IcuNode, IcuParseError, IcuParserOptions,
    IcuPluralOption, IcuPluralType, IcuSelectOption, IcuValidationResult, IcuVariable,
    IcuVariableComparison, ItemsToCatalogOptions, MessageIdInput, PoDateTime, PoFile, PoItem,
    SerializeOptions, SerializedCompiledCatalog, SerializedCompiledEntry,
    SerializedCompiledMessage, SerializedCompiledMessageKind, SourceReference,
};
use napi::bindgen_prelude::Result;
use napi::Error;
use napi_derive::napi;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
struct JsPoFile {
    comments: Vec<String>,
    #[serde(rename = "extractedComments")]
    extracted_comments: Vec<String>,
    headers: BTreeMap<String, String>,
    #[serde(rename = "headerOrder")]
    header_order: Vec<String>,
    items: Vec<JsPoItem>,
}

#[derive(Debug, Serialize)]
struct JsPoItem {
    msgid: String,
    msgctxt: Option<String>,
    references: Vec<String>,
    #[serde(rename = "msgid_plural")]
    msgid_plural: Option<String>,
    msgstr: Vec<String>,
    comments: Vec<String>,
    #[serde(rename = "extractedComments")]
    extracted_comments: Vec<String>,
    flags: BTreeMap<String, bool>,
    metadata: BTreeMap<String, String>,
    obsolete: bool,
    nplurals: usize,
}

#[derive(Debug, Default, Deserialize)]
struct InputPoFile {
    comments: Option<Vec<String>>,
    #[serde(rename = "extractedComments")]
    extracted_comments: Option<Vec<String>>,
    headers: Option<BTreeMap<String, String>>,
    #[serde(rename = "headerOrder")]
    header_order: Option<Vec<String>>,
    items: Option<Vec<InputPoItem>>,
}

#[derive(Debug, Default, Deserialize)]
struct InputPoItem {
    msgid: Option<String>,
    msgctxt: Option<String>,
    references: Option<Vec<String>>,
    #[serde(rename = "msgid_plural")]
    msgid_plural: Option<String>,
    msgstr: Option<Vec<String>>,
    comments: Option<Vec<String>>,
    #[serde(rename = "extractedComments")]
    extracted_comments: Option<Vec<String>>,
    flags: Option<BTreeMap<String, bool>>,
    metadata: Option<BTreeMap<String, String>>,
    obsolete: Option<bool>,
    nplurals: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
struct InputSerializeOptions {
    #[serde(rename = "foldLength")]
    fold_length: Option<usize>,
    #[serde(rename = "compactMultiline")]
    compact_multiline: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct InputCompileIcuOptions {
    locale: String,
    strict: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct InputCompileCatalogOptions {
    locale: String,
    #[serde(rename = "useMessageId")]
    use_message_id: Option<bool>,
    strict: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct InputMessageIdInput {
    message: String,
    context: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum InputPluralForms {
    Value(String),
    Omit(bool),
}

#[derive(Debug, Deserialize)]
struct InputPoDateTime {
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    #[serde(rename = "offsetMinutes")]
    offset_minutes: i16,
}

#[derive(Debug, Default, Deserialize)]
struct InputCreateHeadersOptions {
    language: Option<String>,
    generator: Option<String>,
    #[serde(rename = "projectIdVersion")]
    project_id_version: Option<String>,
    #[serde(rename = "reportBugsTo")]
    report_bugs_to: Option<String>,
    #[serde(rename = "lastTranslator")]
    last_translator: Option<String>,
    #[serde(rename = "languageTeam")]
    language_team: Option<String>,
    #[serde(rename = "pluralForms")]
    plural_forms: Option<InputPluralForms>,
    now: Option<InputPoDateTime>,
    custom: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct InputSourceReference {
    file: String,
    line: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
struct InputFormatReferenceOptions {
    #[serde(rename = "includeLineNumbers")]
    include_line_numbers: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
struct InputCatalogToItemsOptions {
    #[serde(rename = "includeOrigins")]
    include_origins: Option<bool>,
    #[serde(rename = "includeLineNumbers")]
    include_line_numbers: Option<bool>,
    nplurals: Option<usize>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
enum InputCatalogKeyStrategy {
    Msgid,
    #[default]
    ContextMsgid,
}

#[derive(Debug, Default, Deserialize)]
struct InputItemsToCatalogOptions {
    #[serde(rename = "keyStrategy")]
    key_strategy: Option<InputCatalogKeyStrategy>,
    #[serde(rename = "includeOrigins")]
    include_origins: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
struct InputGettextToIcuOptions {
    locale: String,
    #[serde(rename = "pluralVariable")]
    plural_variable: Option<String>,
    #[serde(rename = "expandOctothorpe")]
    expand_octothorpe: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
struct InputIcuParserOptions {
    #[serde(rename = "ignoreTag")]
    ignore_tag: Option<bool>,
    #[serde(rename = "requiresOtherClause")]
    requires_other_clause: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
struct InputCatalogEntry {
    message: Option<String>,
    translation: Option<InputCatalogTranslation>,
    #[serde(rename = "pluralSource")]
    plural_source: Option<String>,
    context: Option<String>,
    comments: Option<Vec<String>>,
    #[serde(rename = "extractedComments")]
    extracted_comments: Option<Vec<String>>,
    origins: Option<Vec<InputSourceReference>>,
    obsolete: Option<bool>,
    flags: Option<BTreeMap<String, bool>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum InputCatalogTranslation {
    Singular(String),
    Plural(Vec<String>),
}

type InputCatalog = BTreeMap<String, InputCatalogEntry>;

#[derive(Debug, Serialize, Clone)]
struct JsIcuPosition {
    offset: usize,
    line: usize,
    column: usize,
}

#[derive(Debug, Serialize)]
struct JsIcuLocation {
    start: JsIcuPosition,
    end: JsIcuPosition,
}

#[derive(Debug, Serialize)]
struct JsIcuParseError {
    kind: &'static str,
    message: String,
    location: JsIcuLocation,
}

#[derive(Debug, Serialize)]
struct JsParsedPluralForms {
    nplurals: Option<String>,
    plural: Option<String>,
}

#[derive(Debug, Serialize)]
struct JsSourceReference {
    file: String,
    line: Option<usize>,
}

#[derive(Debug, Serialize)]
struct JsCatalogEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    translation: Option<JsCatalogTranslation>,
    #[serde(rename = "pluralSource", skip_serializing_if = "Option::is_none")]
    plural_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    comments: Option<Vec<String>>,
    #[serde(rename = "extractedComments", skip_serializing_if = "Option::is_none")]
    extracted_comments: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    origins: Option<Vec<JsSourceReference>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    obsolete: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    flags: Option<BTreeMap<String, bool>>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum JsCatalogTranslation {
    Singular(String),
    Plural(Vec<String>),
}

#[derive(Debug, Serialize)]
struct JsIcuVariable {
    name: String,
    #[serde(rename = "type")]
    variable_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    style: Option<String>,
}

#[derive(Debug, Serialize)]
struct JsIcuValidationResult {
    valid: bool,
    errors: Vec<JsIcuParseError>,
}

#[derive(Debug, Serialize)]
struct JsIcuVariableComparison {
    missing: Vec<String>,
    extra: Vec<String>,
    #[serde(rename = "isMatch")]
    is_match: bool,
}

#[derive(Debug, Serialize)]
struct JsIcuToGettextSource {
    msgid: String,
    #[serde(rename = "msgid_plural")]
    msgid_plural: String,
    #[serde(rename = "pluralVariable")]
    plural_variable: String,
}

#[derive(Debug, Serialize)]
struct JsNormalizeItemToIcuResult {
    changed: bool,
    item: JsPoItem,
}

#[derive(Debug, Serialize)]
struct JsIcuPluralOption {
    value: Vec<JsIcuNode>,
}

#[derive(Debug, Serialize)]
struct JsIcuSelectOption {
    value: Vec<JsIcuNode>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum JsIcuNode {
    Literal {
        value: String,
    },
    Argument {
        value: String,
    },
    Number {
        value: String,
        style: Option<String>,
    },
    Date {
        value: String,
        style: Option<String>,
    },
    Time {
        value: String,
        style: Option<String>,
    },
    List {
        value: String,
        style: Option<String>,
    },
    Duration {
        value: String,
        style: Option<String>,
    },
    Ago {
        value: String,
        style: Option<String>,
    },
    Name {
        value: String,
        style: Option<String>,
    },
    Select {
        value: String,
        options: BTreeMap<String, JsIcuSelectOption>,
    },
    Plural {
        value: String,
        options: BTreeMap<String, JsIcuPluralOption>,
        offset: i32,
        #[serde(rename = "pluralType")]
        plural_type: &'static str,
    },
    Pound,
    Tag {
        value: String,
        children: Vec<JsIcuNode>,
    },
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum JsIcuParseResult {
    Success {
        success: bool,
        ast: Vec<JsIcuNode>,
        errors: Vec<JsIcuParseError>,
    },
    Failure {
        success: bool,
        ast: Option<Vec<JsIcuNode>>,
        errors: Vec<JsIcuParseError>,
    },
}

#[derive(Debug, Serialize)]
struct JsSerializedCompiledCatalog {
    locale: String,
    entries: Vec<JsSerializedCompiledEntry>,
}

#[derive(Debug, Serialize)]
struct JsSerializedCompiledEntry {
    key: String,
    message: JsSerializedCompiledMessage,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind")]
enum JsSerializedCompiledMessage {
    #[serde(rename = "icu")]
    Icu { ast: Vec<JsIcuNode> },
    #[serde(rename = "gettextPlural")]
    GettextPlural {
        variable: String,
        forms: Vec<JsSerializedCompiledMessage>,
    },
    #[serde(rename = "fallback")]
    Fallback { text: String },
}

struct Registry<T> {
    next_id: AtomicU32,
    values: Mutex<BTreeMap<u32, Arc<T>>>,
}

impl<T> Registry<T> {
    fn new() -> Self {
        Self {
            next_id: AtomicU32::new(1),
            values: Mutex::new(BTreeMap::new()),
        }
    }

    fn insert(&self, value: T) -> u32 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let mut values = self.values.lock().expect("registry mutex poisoned");
        values.insert(id, Arc::new(value));
        id
    }

    fn get(&self, id: u32) -> Option<Arc<T>> {
        let values = self.values.lock().expect("registry mutex poisoned");
        values.get(&id).cloned()
    }

    fn remove(&self, id: u32) {
        let mut values = self.values.lock().expect("registry mutex poisoned");
        values.remove(&id);
    }
}

fn compiled_message_registry() -> &'static Registry<CompiledMessage> {
    static REGISTRY: OnceLock<Registry<CompiledMessage>> = OnceLock::new();
    REGISTRY.get_or_init(Registry::new)
}

fn compiled_catalog_registry() -> &'static Registry<CompiledCatalog> {
    static REGISTRY: OnceLock<Registry<CompiledCatalog>> = OnceLock::new();
    REGISTRY.get_or_init(Registry::new)
}

impl From<PoFile> for JsPoFile {
    fn from(value: PoFile) -> Self {
        Self {
            comments: value.comments,
            extracted_comments: value.extracted_comments,
            headers: value.headers,
            header_order: value.header_order,
            items: value.items.into_iter().map(JsPoItem::from).collect(),
        }
    }
}

impl From<PoItem> for JsPoItem {
    fn from(value: PoItem) -> Self {
        Self {
            msgid: value.msgid,
            msgctxt: value.msgctxt,
            references: value.references,
            msgid_plural: value.msgid_plural,
            msgstr: value.msgstr,
            comments: value.comments,
            extracted_comments: value.extracted_comments,
            flags: value.flags,
            metadata: value.metadata,
            obsolete: value.obsolete,
            nplurals: value.nplurals,
        }
    }
}

impl From<InputPoFile> for PoFile {
    fn from(value: InputPoFile) -> Self {
        let mut po = PoFile::new();
        po.comments = value.comments.unwrap_or_default();
        po.extracted_comments = value.extracted_comments.unwrap_or_default();
        po.headers = value.headers.unwrap_or_else(|| po.headers.clone());
        po.header_order = value.header_order.unwrap_or_default();
        po.items = value
            .items
            .unwrap_or_default()
            .into_iter()
            .map(PoItem::from)
            .collect();
        po
    }
}

impl From<InputPoItem> for PoItem {
    fn from(value: InputPoItem) -> Self {
        let mut item = PoItem::new(value.nplurals.unwrap_or(2));
        item.msgid = value.msgid.unwrap_or_default();
        item.msgctxt = value.msgctxt;
        item.references = value.references.unwrap_or_default();
        item.msgid_plural = value.msgid_plural;
        item.msgstr = value.msgstr.unwrap_or_default();
        item.comments = value.comments.unwrap_or_default();
        item.extracted_comments = value.extracted_comments.unwrap_or_default();
        item.flags = value.flags.unwrap_or_default();
        item.metadata = value.metadata.unwrap_or_default();
        item.obsolete = value.obsolete.unwrap_or(false);
        item
    }
}

impl From<InputCatalogTranslation> for CatalogTranslation {
    fn from(value: InputCatalogTranslation) -> Self {
        match value {
            InputCatalogTranslation::Singular(text) => Self::Singular(text),
            InputCatalogTranslation::Plural(texts) => Self::Plural(texts),
        }
    }
}

impl From<IcuPluralOption> for JsIcuPluralOption {
    fn from(value: IcuPluralOption) -> Self {
        Self {
            value: value.value.into_iter().map(JsIcuNode::from).collect(),
        }
    }
}

impl From<IcuSelectOption> for JsIcuSelectOption {
    fn from(value: IcuSelectOption) -> Self {
        Self {
            value: value.value.into_iter().map(JsIcuNode::from).collect(),
        }
    }
}

impl From<IcuNode> for JsIcuNode {
    fn from(value: IcuNode) -> Self {
        match value {
            IcuNode::Literal { value } => Self::Literal { value },
            IcuNode::Argument { value } => Self::Argument { value },
            IcuNode::Number { value, style } => Self::Number { value, style },
            IcuNode::Date { value, style } => Self::Date { value, style },
            IcuNode::Time { value, style } => Self::Time { value, style },
            IcuNode::List { value, style } => Self::List { value, style },
            IcuNode::Duration { value, style } => Self::Duration { value, style },
            IcuNode::Ago { value, style } => Self::Ago { value, style },
            IcuNode::Name { value, style } => Self::Name { value, style },
            IcuNode::Select { value, options } => Self::Select {
                value,
                options: options
                    .into_iter()
                    .map(|(key, option)| (key, JsIcuSelectOption::from(option)))
                    .collect(),
            },
            IcuNode::Plural {
                value,
                options,
                offset,
                plural_type,
            } => Self::Plural {
                value,
                options: options
                    .into_iter()
                    .map(|(key, option)| (key, JsIcuPluralOption::from(option)))
                    .collect(),
                offset,
                plural_type: match plural_type {
                    IcuPluralType::Cardinal => "cardinal",
                    IcuPluralType::Ordinal => "ordinal",
                },
            },
            IcuNode::Pound => Self::Pound,
            IcuNode::Tag { value, children } => Self::Tag {
                value,
                children: children.into_iter().map(JsIcuNode::from).collect(),
            },
        }
    }
}

impl From<SerializedCompiledCatalog> for JsSerializedCompiledCatalog {
    fn from(value: SerializedCompiledCatalog) -> Self {
        Self {
            locale: value.locale,
            entries: value
                .entries
                .into_iter()
                .map(JsSerializedCompiledEntry::from)
                .collect(),
        }
    }
}

impl From<SerializedCompiledEntry> for JsSerializedCompiledEntry {
    fn from(value: SerializedCompiledEntry) -> Self {
        Self {
            key: value.key,
            message: JsSerializedCompiledMessage::from(value.message),
        }
    }
}

impl From<SerializedCompiledMessage> for JsSerializedCompiledMessage {
    fn from(value: SerializedCompiledMessage) -> Self {
        match value.kind {
            SerializedCompiledMessageKind::Icu { ast } => Self::Icu {
                ast: ast.into_iter().map(JsIcuNode::from).collect(),
            },
            SerializedCompiledMessageKind::GettextPlural { variable, forms } => {
                Self::GettextPlural {
                    variable,
                    forms: forms
                        .into_iter()
                        .map(JsSerializedCompiledMessage::from)
                        .collect(),
                }
            }
            SerializedCompiledMessageKind::Fallback { text } => Self::Fallback { text },
        }
    }
}

impl From<SourceReference> for JsSourceReference {
    fn from(value: SourceReference) -> Self {
        Self {
            file: value.file,
            line: value.line,
        }
    }
}

impl From<InputSourceReference> for SourceReference {
    fn from(value: InputSourceReference) -> Self {
        Self {
            file: value.file,
            line: value.line,
        }
    }
}

impl From<CatalogTranslation> for JsCatalogTranslation {
    fn from(value: CatalogTranslation) -> Self {
        match value {
            CatalogTranslation::Singular(value) => Self::Singular(value),
            CatalogTranslation::Plural(values) => Self::Plural(values),
        }
    }
}

impl From<InputCatalogKeyStrategy> for CatalogKeyStrategy {
    fn from(value: InputCatalogKeyStrategy) -> Self {
        match value {
            InputCatalogKeyStrategy::Msgid => Self::Msgid,
            InputCatalogKeyStrategy::ContextMsgid => Self::ContextMsgid,
        }
    }
}

impl From<CatalogEntry> for JsCatalogEntry {
    fn from(value: CatalogEntry) -> Self {
        Self {
            message: value.message,
            translation: value.translation.map(JsCatalogTranslation::from),
            plural_source: value.plural_source,
            context: value.context,
            comments: value.comments,
            extracted_comments: value.extracted_comments,
            origins: value
                .origins
                .map(|origins| origins.into_iter().map(JsSourceReference::from).collect()),
            obsolete: value.obsolete,
            flags: value.flags,
        }
    }
}

impl From<IcuVariable> for JsIcuVariable {
    fn from(value: IcuVariable) -> Self {
        let variable_type = match value.kind.as_str() {
            "number" => "number",
            "date" => "date",
            "time" => "time",
            "plural" => "plural",
            "select" => "select",
            _ => "argument",
        };

        Self {
            name: value.name,
            variable_type,
            style: value.style,
        }
    }
}

#[napi]
pub fn parse_po_json(input: String) -> Result<String> {
    let po = parse_po(&input);
    serde_json::to_string(&JsPoFile::from(po)).map_err(to_napi_error)
}

#[napi]
pub fn parse_icu_json(message: String, options_json: Option<String>) -> Result<String> {
    let options = options_json
        .as_deref()
        .map(serde_json::from_str::<InputIcuParserOptions>)
        .transpose()
        .map_err(to_napi_error)?
        .unwrap_or_default();

    let parser_options = IcuParserOptions {
        ignore_tag: options.ignore_tag.unwrap_or(false),
        requires_other_clause: options.requires_other_clause.unwrap_or(true),
    };

    let result = match parse_icu(&message, parser_options) {
        Ok(ast) => JsIcuParseResult::Success {
            success: true,
            ast: ast.into_iter().map(JsIcuNode::from).collect(),
            errors: Vec::new(),
        },
        Err(error) => JsIcuParseResult::Failure {
            success: false,
            ast: None,
            errors: vec![js_icu_parse_error(&message, error)],
        },
    };

    serde_json::to_string(&result).map_err(to_napi_error)
}

#[napi]
pub fn stringify_po_json(input: String, options_json: Option<String>) -> Result<String> {
    let po = serde_json::from_str::<InputPoFile>(&input).map_err(to_napi_error)?;
    let options = options_json
        .as_deref()
        .map(serde_json::from_str::<InputSerializeOptions>)
        .transpose()
        .map_err(to_napi_error)?
        .unwrap_or_default();

    let mut serialize_options = SerializeOptions::default();
    if let Some(fold_length) = options.fold_length {
        serialize_options.fold_length = fold_length;
    }
    if let Some(compact_multiline) = options.compact_multiline {
        serialize_options.compact_multiline = compact_multiline;
    }

    Ok(stringify_po(&PoFile::from(po), serialize_options))
}

#[napi]
pub fn format_po_date_json(input_json: String) -> Result<String> {
    let input = serde_json::from_str::<InputPoDateTime>(&input_json).map_err(to_napi_error)?;
    Ok(format_po_date(PoDateTime {
        year: input.year,
        month: input.month,
        day: input.day,
        hour: input.hour,
        minute: input.minute,
        offset_minutes: input.offset_minutes,
    }))
}

#[napi]
pub fn create_default_headers_json(options_json: Option<String>) -> Result<String> {
    let options = options_json
        .map(|json| serde_json::from_str::<InputCreateHeadersOptions>(&json))
        .transpose()
        .map_err(to_napi_error)?
        .unwrap_or_default();

    let plural_forms = match options.plural_forms {
        Some(InputPluralForms::Value(value)) => Some(Some(value)),
        Some(InputPluralForms::Omit(false)) => Some(None),
        Some(InputPluralForms::Omit(true)) => None,
        None => None,
    };

    let headers = create_default_headers(&CreateHeadersOptions {
        language: options.language,
        generator: options.generator,
        project_id_version: options.project_id_version,
        report_bugs_to: options.report_bugs_to,
        last_translator: options.last_translator,
        language_team: options.language_team,
        plural_forms,
        now: options.now.map(|now| PoDateTime {
            year: now.year,
            month: now.month,
            day: now.day,
            hour: now.hour,
            minute: now.minute,
            offset_minutes: now.offset_minutes,
        }),
        custom: options.custom.unwrap_or_default(),
    });

    serde_json::to_string(&headers).map_err(to_napi_error)
}

#[napi]
pub fn generate_message_id_json(message: String, context: Option<String>) -> String {
    generate_message_id(&message, context.as_deref())
}

#[napi]
pub fn generate_message_ids_json(inputs_json: String) -> Result<String> {
    let inputs =
        serde_json::from_str::<Vec<InputMessageIdInput>>(&inputs_json).map_err(to_napi_error)?;
    let inputs = inputs
        .into_iter()
        .map(|input| MessageIdInput {
            message: input.message,
            context: input.context,
        })
        .collect::<Vec<_>>();

    serde_json::to_string(&generate_message_ids(&inputs)).map_err(to_napi_error)
}

#[napi]
pub fn parse_plural_forms_json(input: Option<String>) -> Result<String> {
    let parsed = parse_plural_forms(input.as_deref());
    serde_json::to_string(&JsParsedPluralForms {
        nplurals: parsed.nplurals,
        plural: parsed.plural,
    })
    .map_err(to_napi_error)
}

#[napi]
pub fn get_plural_categories_json(locale: String) -> Result<String> {
    serde_json::to_string(get_plural_categories(&locale)).map_err(to_napi_error)
}

#[napi]
pub fn get_plural_count_json(locale: String) -> Result<u32> {
    u32::try_from(get_plural_count(&locale)).map_err(to_napi_error)
}

#[napi]
pub fn get_plural_index_json(locale: String, value: f64) -> Result<u32> {
    u32::try_from(get_plural_index(&locale, value)).map_err(to_napi_error)
}

#[napi]
pub fn get_plural_forms_header_json(locale: String) -> String {
    get_plural_forms_header(&locale)
}

#[napi]
pub fn normalize_file_path_json(path: String) -> String {
    normalize_file_path(&path)
}

#[napi]
pub fn parse_reference_json(reference: String) -> Result<String> {
    serde_json::to_string(&JsSourceReference::from(
        parse_reference(&reference).map_err(to_napi_error)?,
    ))
    .map_err(to_napi_error)
}

#[napi]
pub fn format_reference_json(
    reference_json: String,
    options_json: Option<String>,
) -> Result<String> {
    let reference =
        serde_json::from_str::<InputSourceReference>(&reference_json).map_err(to_napi_error)?;
    let options = options_json
        .map(|json| serde_json::from_str::<InputFormatReferenceOptions>(&json))
        .transpose()
        .map_err(to_napi_error)?
        .unwrap_or_default();

    Ok(format_reference(
        &SourceReference {
            file: reference.file,
            line: reference.line,
        },
        FormatReferenceOptions {
            include_line_numbers: options.include_line_numbers.unwrap_or(true),
        },
    ))
}

#[napi]
pub fn parse_references_json(references: String) -> Result<String> {
    let parsed = parse_references(&references).map_err(to_napi_error)?;
    let parsed = parsed
        .into_iter()
        .map(JsSourceReference::from)
        .collect::<Vec<_>>();
    serde_json::to_string(&parsed).map_err(to_napi_error)
}

#[napi]
pub fn format_references_json(
    references_json: String,
    options_json: Option<String>,
) -> Result<String> {
    let references = serde_json::from_str::<Vec<InputSourceReference>>(&references_json)
        .map_err(to_napi_error)?;
    let options = options_json
        .map(|json| serde_json::from_str::<InputFormatReferenceOptions>(&json))
        .transpose()
        .map_err(to_napi_error)?
        .unwrap_or_default();

    Ok(format_references(
        &references
            .into_iter()
            .map(|reference| SourceReference {
                file: reference.file,
                line: reference.line,
            })
            .collect::<Vec<_>>(),
        FormatReferenceOptions {
            include_line_numbers: options.include_line_numbers.unwrap_or(true),
        },
    ))
}

#[napi]
pub fn create_reference_json(file: String, line: Option<f64>) -> Result<String> {
    let line = line
        .map(|value| {
            if value <= 0.0 || value.fract() != 0.0 {
                Err(Error::from_reason(format!(
                    "line number must be a positive integer, got: {value}"
                )))
            } else {
                usize::try_from(value as u64).map_err(to_napi_error)
            }
        })
        .transpose()?;
    let reference = create_reference(&file, line).map_err(to_napi_error)?;
    serde_json::to_string(&JsSourceReference::from(reference)).map_err(to_napi_error)
}

#[napi]
pub fn catalog_to_items_json(catalog_json: String, options_json: Option<String>) -> Result<String> {
    let catalog = serde_json::from_str::<InputCatalog>(&catalog_json).map_err(to_napi_error)?;
    let options = options_json
        .map(|json| serde_json::from_str::<InputCatalogToItemsOptions>(&json))
        .transpose()
        .map_err(to_napi_error)?
        .unwrap_or_default();

    let items = catalog_to_items(
        &input_catalog_to_catalog(catalog),
        CatalogToItemsOptions {
            include_origins: options.include_origins.unwrap_or(true),
            include_line_numbers: options.include_line_numbers.unwrap_or(true),
            nplurals: options.nplurals.unwrap_or(2),
        },
    );

    serde_json::to_string(
        &items
            .into_iter()
            .map(JsPoItem::from)
            .collect::<Vec<JsPoItem>>(),
    )
    .map_err(to_napi_error)
}

#[napi]
pub fn items_to_catalog_json(items_json: String, options_json: Option<String>) -> Result<String> {
    let items = serde_json::from_str::<Vec<InputPoItem>>(&items_json).map_err(to_napi_error)?;
    let options = options_json
        .map(|json| serde_json::from_str::<InputItemsToCatalogOptions>(&json))
        .transpose()
        .map_err(to_napi_error)?
        .unwrap_or_default();

    let catalog = items_to_catalog(
        &items.into_iter().map(PoItem::from).collect::<Vec<_>>(),
        ItemsToCatalogOptions {
            key_strategy: options.key_strategy.unwrap_or_default().into(),
            include_origins: options.include_origins.unwrap_or(true),
        },
    )
    .map_err(to_napi_error)?;

    let catalog = catalog
        .into_iter()
        .map(|(key, entry)| (key, JsCatalogEntry::from(entry)))
        .collect::<BTreeMap<_, _>>();

    serde_json::to_string(&catalog).map_err(to_napi_error)
}

#[napi]
pub fn merge_catalogs_json(base_json: String, updates_json: String) -> Result<String> {
    let base = serde_json::from_str::<InputCatalog>(&base_json).map_err(to_napi_error)?;
    let updates = serde_json::from_str::<InputCatalog>(&updates_json).map_err(to_napi_error)?;
    let merged = merge_catalogs(
        &input_catalog_to_catalog(base),
        &input_catalog_to_catalog(updates),
    );

    let merged = merged
        .into_iter()
        .map(|(key, entry)| (key, JsCatalogEntry::from(entry)))
        .collect::<BTreeMap<_, _>>();

    serde_json::to_string(&merged).map_err(to_napi_error)
}

#[napi]
pub fn extract_variables_json(message: String) -> Result<String> {
    serde_json::to_string(&extract_variables(&message)).map_err(to_napi_error)
}

#[napi]
pub fn extract_variable_info_json(message: String) -> Result<String> {
    let variables = extract_variable_info(&message)
        .into_iter()
        .map(JsIcuVariable::from)
        .collect::<Vec<_>>();
    serde_json::to_string(&variables).map_err(to_napi_error)
}

#[napi]
pub fn validate_icu_json(message: String, options_json: Option<String>) -> Result<String> {
    let options = options_json
        .map(|json| serde_json::from_str::<InputIcuParserOptions>(&json))
        .transpose()
        .map_err(to_napi_error)?
        .unwrap_or_default();
    let parser_options = IcuParserOptions {
        ignore_tag: options.ignore_tag.unwrap_or(false),
        requires_other_clause: options.requires_other_clause.unwrap_or(true),
    };

    let result: IcuValidationResult = validate_icu(&message, parser_options);
    let errors = result
        .errors
        .into_iter()
        .map(|error| js_icu_parse_error(&message, error))
        .collect::<Vec<_>>();
    serde_json::to_string(&JsIcuValidationResult {
        valid: result.valid,
        errors,
    })
    .map_err(to_napi_error)
}

#[napi]
pub fn compare_variables_json(source: String, translation: String) -> Result<String> {
    let result: IcuVariableComparison = compare_variables(&source, &translation);
    serde_json::to_string(&JsIcuVariableComparison {
        missing: result.missing,
        extra: result.extra,
        is_match: result.is_match,
    })
    .map_err(to_napi_error)
}

#[napi]
pub fn has_plural_json(message: String) -> bool {
    has_plural(&message)
}

#[napi]
pub fn has_select_json(message: String) -> bool {
    has_select(&message)
}

#[napi]
pub fn has_select_ordinal_json(message: String) -> bool {
    has_select_ordinal(&message)
}

#[napi]
pub fn has_icu_syntax_json(message: String) -> bool {
    has_icu_syntax(&message)
}

#[napi]
pub fn gettext_to_icu_json(item_json: String, options_json: String) -> Result<String> {
    let item = serde_json::from_str::<InputPoItem>(&item_json).map_err(to_napi_error)?;
    let options =
        serde_json::from_str::<InputGettextToIcuOptions>(&options_json).map_err(to_napi_error)?;
    let result = gettext_to_icu(
        &PoItem::from(item),
        &GettextToIcuOptions {
            locale: options.locale,
            plural_variable: options
                .plural_variable
                .unwrap_or_else(|| String::from("count")),
            expand_octothorpe: options.expand_octothorpe.unwrap_or(true),
        },
    );

    serde_json::to_string(&result).map_err(to_napi_error)
}

#[napi]
pub fn is_plural_item_json(item_json: String) -> Result<bool> {
    let item = serde_json::from_str::<InputPoItem>(&item_json).map_err(to_napi_error)?;
    Ok(is_plural_item(&PoItem::from(item)))
}

#[napi]
pub fn normalize_item_to_icu_json(item_json: String, options_json: String) -> Result<String> {
    let item = serde_json::from_str::<InputPoItem>(&item_json).map_err(to_napi_error)?;
    let options =
        serde_json::from_str::<InputGettextToIcuOptions>(&options_json).map_err(to_napi_error)?;
    let mut item = PoItem::from(item);
    let changed = normalize_item_to_icu(
        &mut item,
        &GettextToIcuOptions {
            locale: options.locale,
            plural_variable: options
                .plural_variable
                .unwrap_or_else(|| String::from("count")),
            expand_octothorpe: options.expand_octothorpe.unwrap_or(true),
        },
    );

    serde_json::to_string(&JsNormalizeItemToIcuResult {
        changed,
        item: JsPoItem::from(item),
    })
    .map_err(to_napi_error)
}

#[napi]
pub fn normalize_to_icu_json(po_json: String, options_json: String) -> Result<String> {
    let po = serde_json::from_str::<InputPoFile>(&po_json).map_err(to_napi_error)?;
    let options =
        serde_json::from_str::<InputGettextToIcuOptions>(&options_json).map_err(to_napi_error)?;
    let result = normalize_to_icu(
        &PoFile::from(po),
        &GettextToIcuOptions {
            locale: options.locale,
            plural_variable: options
                .plural_variable
                .unwrap_or_else(|| String::from("count")),
            expand_octothorpe: options.expand_octothorpe.unwrap_or(true),
        },
    );

    serde_json::to_string(&JsPoFile::from(result)).map_err(to_napi_error)
}

#[napi]
pub fn icu_to_gettext_source_json(icu: String, expand_octothorpe: Option<bool>) -> Result<String> {
    let result = icu_to_gettext_source(&icu, expand_octothorpe.unwrap_or(true)).map(
        |(msgid, msgid_plural, plural_variable)| JsIcuToGettextSource {
            msgid,
            msgid_plural,
            plural_variable,
        },
    );

    serde_json::to_string(&result).map_err(to_napi_error)
}

#[napi]
pub fn compile_icu_json(message: String, options_json: String) -> Result<u32> {
    let options =
        serde_json::from_str::<InputCompileIcuOptions>(&options_json).map_err(to_napi_error)?;
    let compiled = compile_icu_runtime(
        &message,
        &CompileIcuOptions {
            locale: options.locale,
            strict: options.strict.unwrap_or(true),
        },
    )
    .map_err(to_napi_error)?;

    Ok(compiled_message_registry().insert(compiled))
}

#[napi]
pub fn compile_icu_payload_json(message: String, options_json: String) -> Result<String> {
    let options =
        serde_json::from_str::<InputCompileIcuOptions>(&options_json).map_err(to_napi_error)?;
    let compiled = ferrocat::compile_icu(
        &message,
        &CompileIcuOptions {
            locale: options.locale,
            strict: options.strict.unwrap_or(true),
        },
    )
    .map_err(to_napi_error)?;

    serde_json::to_string(&JsSerializedCompiledMessage::from(compiled)).map_err(to_napi_error)
}

#[napi]
pub fn format_compiled_message_json(handle: u32, values_json: Option<String>) -> Result<String> {
    let values = parse_message_values(values_json)?;
    let compiled = compiled_message_registry()
        .get(handle)
        .ok_or_else(|| Error::from_reason(format!("Unknown compiled message handle: {handle}")))?;

    Ok(compiled.format(&values))
}

#[napi]
pub fn free_compiled_message(handle: u32) {
    compiled_message_registry().remove(handle);
}

#[napi]
pub fn compile_catalog_json(catalog_json: String, options_json: String) -> Result<u32> {
    let catalog = serde_json::from_str::<InputCatalog>(&catalog_json).map_err(to_napi_error)?;
    let options =
        serde_json::from_str::<InputCompileCatalogOptions>(&options_json).map_err(to_napi_error)?;
    let compiled = compile_catalog_runtime(
        &input_catalog_to_catalog(catalog),
        &CompileCatalogOptions {
            locale: options.locale,
            use_message_id: options.use_message_id.unwrap_or(true),
            strict: options.strict.unwrap_or(false),
        },
    )
    .map_err(to_napi_error)?;

    Ok(compiled_catalog_registry().insert(compiled))
}

#[napi]
pub fn serialize_compiled_catalog_json(
    catalog_json: String,
    options_json: String,
) -> Result<String> {
    let catalog = serde_json::from_str::<InputCatalog>(&catalog_json).map_err(to_napi_error)?;
    let options =
        serde_json::from_str::<InputCompileCatalogOptions>(&options_json).map_err(to_napi_error)?;
    let serialized = serialize_compiled_catalog(
        &input_catalog_to_catalog(catalog),
        &CompileCatalogOptions {
            locale: options.locale,
            use_message_id: options.use_message_id.unwrap_or(true),
            strict: options.strict.unwrap_or(false),
        },
    )
    .map_err(to_napi_error)?;

    serde_json::to_string(&JsSerializedCompiledCatalog::from(serialized)).map_err(to_napi_error)
}

#[napi]
pub fn format_compiled_catalog_json(
    handle: u32,
    key: String,
    values_json: Option<String>,
) -> Result<String> {
    let values = parse_message_values(values_json)?;
    let compiled = compiled_catalog_registry()
        .get(handle)
        .ok_or_else(|| Error::from_reason(format!("Unknown compiled catalog handle: {handle}")))?;

    Ok(compiled.format(&key, &values))
}

#[napi]
pub fn compiled_catalog_has(handle: u32, key: String) -> Result<bool> {
    let compiled = compiled_catalog_registry()
        .get(handle)
        .ok_or_else(|| Error::from_reason(format!("Unknown compiled catalog handle: {handle}")))?;

    Ok(compiled.has(&key))
}

#[napi]
pub fn compiled_catalog_keys_json(handle: u32) -> Result<String> {
    let compiled = compiled_catalog_registry()
        .get(handle)
        .ok_or_else(|| Error::from_reason(format!("Unknown compiled catalog handle: {handle}")))?;

    serde_json::to_string(&compiled.keys()).map_err(to_napi_error)
}

#[napi]
pub fn compiled_catalog_size(handle: u32) -> Result<u32> {
    let compiled = compiled_catalog_registry()
        .get(handle)
        .ok_or_else(|| Error::from_reason(format!("Unknown compiled catalog handle: {handle}")))?;

    u32::try_from(compiled.size()).map_err(to_napi_error)
}

#[napi]
pub fn compiled_catalog_locale(handle: u32) -> Result<String> {
    let compiled = compiled_catalog_registry()
        .get(handle)
        .ok_or_else(|| Error::from_reason(format!("Unknown compiled catalog handle: {handle}")))?;

    Ok(compiled.locale.clone())
}

#[napi]
pub fn free_compiled_catalog(handle: u32) {
    compiled_catalog_registry().remove(handle);
}

#[napi]
pub fn binding_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn parse_message_values(values_json: Option<String>) -> Result<MessageValues> {
    let Some(values_json) = values_json else {
        return Ok(MessageValues::new());
    };

    let raw = serde_json::from_str::<Value>(&values_json).map_err(to_napi_error)?;
    let Value::Object(map) = raw else {
        return Err(Error::from_reason(String::from(
            "Expected JSON object for message values",
        )));
    };

    map.into_iter()
        .map(|(key, value)| match json_to_message_value(value) {
            Some(value) => Ok((key, value)),
            None => Err(Error::from_reason(format!(
                "Unsupported message value for key `{key}`"
            ))),
        })
        .collect()
}

fn input_catalog_to_catalog(value: InputCatalog) -> Catalog {
    value
        .into_iter()
        .map(|(key, entry)| {
            (
                key,
                CatalogEntry {
                    message: entry.message,
                    translation: entry.translation.map(CatalogTranslation::from),
                    plural_source: entry.plural_source,
                    context: entry.context,
                    comments: entry.comments,
                    extracted_comments: entry.extracted_comments,
                    origins: entry
                        .origins
                        .map(|origins| origins.into_iter().map(SourceReference::from).collect()),
                    obsolete: entry.obsolete,
                    flags: entry.flags,
                },
            )
        })
        .collect()
}

fn json_to_message_value(value: Value) -> Option<MessageValue> {
    match value {
        Value::String(value) => Some(MessageValue::String(value)),
        Value::Number(value) => value.as_f64().map(MessageValue::Number),
        Value::Bool(value) => Some(MessageValue::Bool(value)),
        Value::Array(values) => values
            .into_iter()
            .map(json_to_message_value)
            .collect::<Option<Vec<_>>>()
            .map(MessageValue::List),
        Value::Null | Value::Object(_) => None,
    }
}

fn to_napi_error(error: impl std::fmt::Display) -> Error {
    Error::from_reason(error.to_string())
}

fn js_icu_parse_error(message: &str, error: IcuParseError) -> JsIcuParseError {
    let position = offset_to_position(message, error.offset);
    JsIcuParseError {
        kind: "SYNTAX_ERROR",
        message: error.message,
        location: JsIcuLocation {
            start: position.clone(),
            end: position,
        },
    }
}

fn offset_to_position(message: &str, offset: usize) -> JsIcuPosition {
    let mut line = 1usize;
    let mut column = 1usize;

    for ch in message.chars().take(offset) {
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    JsIcuPosition {
        offset,
        line,
        column,
    }
}
