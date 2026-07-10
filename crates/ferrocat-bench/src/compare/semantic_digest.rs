//! Semantic normalization and digest generation for benchmark artifacts.

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct PoSemanticSummary {
    pub(super) headers: Vec<PoHeaderSummary>,
    pub(super) items: Vec<PoItemSummary>,
}

impl PoSemanticSummary {
    pub(super) fn from_po_file(file: &PoFile) -> Self {
        let headers = file
            .headers
            .iter()
            .map(|header| PoHeaderSummary {
                key: header.key.clone(),
                value: header.value.clone(),
            })
            .collect::<Vec<_>>();
        let items = file
            .items
            .iter()
            .map(|item| PoItemSummary {
                msgctxt: item.msgctxt.clone(),
                msgid: item.msgid.clone(),
                msgid_plural: item.msgid_plural.clone(),
                msgstr: match &item.msgstr {
                    MsgStr::None => Vec::new(),
                    MsgStr::Singular(value) => vec![value.clone()],
                    MsgStr::Plural(values) => values.clone(),
                },
                obsolete: item.obsolete,
            })
            .collect::<Vec<_>>();
        Self { headers, items }.normalized()
    }

    pub(super) fn from_borrowed_po_file(file: &BorrowedPoFile<'_>) -> Self {
        let headers = file
            .headers
            .iter()
            .map(|header| PoHeaderSummary {
                key: header.key.as_ref().to_owned(),
                value: header.value.as_ref().to_owned(),
            })
            .collect::<Vec<_>>();
        let items = file
            .items
            .iter()
            .map(|item| PoItemSummary {
                msgctxt: item.msgctxt.as_ref().map(|value| value.as_ref().to_owned()),
                msgid: item.msgid.as_ref().to_owned(),
                msgid_plural: item
                    .msgid_plural
                    .as_ref()
                    .map(|value| value.as_ref().to_owned()),
                msgstr: match &item.msgstr {
                    BorrowedMsgStr::None => Vec::new(),
                    BorrowedMsgStr::Singular(value) => vec![value.as_ref().to_owned()],
                    BorrowedMsgStr::Plural(values) => values
                        .iter()
                        .map(|value| value.as_ref().to_owned())
                        .collect(),
                },
                obsolete: item.obsolete,
            })
            .collect::<Vec<_>>();
        Self { headers, items }.normalized()
    }

    pub(super) fn normalized(mut self) -> Self {
        self.headers
            .retain(|header| should_keep_benchmark_header(&header.key, &header.value));
        self.headers.sort();
        self.items.iter_mut().for_each(PoItemSummary::normalize);
        self.items.sort();
        self
    }
}

pub(super) fn should_keep_benchmark_header(key: &str, value: &str) -> bool {
    !value.is_empty()
        && !matches!(
            key,
            "MIME-Version" | "X-Generator" | "Content-Type" | "Content-Transfer-Encoding"
        )
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct PoHeaderSummary {
    pub(super) key: String,
    pub(super) value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct PoItemSummary {
    pub(super) msgctxt: Option<String>,
    pub(super) msgid: String,
    pub(super) msgid_plural: Option<String>,
    pub(super) msgstr: Vec<String>,
    pub(super) obsolete: bool,
}

impl PoItemSummary {
    pub(super) fn normalize(&mut self) {
        if self.msgctxt.as_deref() == Some("") {
            self.msgctxt = None;
        }
        if self.msgid_plural.as_deref() == Some("") {
            self.msgid_plural = None;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct CatalogSemanticSummary {
    pub(super) locale: Option<String>,
    pub(super) diagnostics: Vec<String>,
    pub(super) messages: Vec<CatalogMessageSummary>,
}

impl CatalogSemanticSummary {
    pub(super) fn from_parsed_catalog(parsed: ParsedCatalog) -> Result<Self, String> {
        let normalized = parsed
            .into_normalized_view()
            .map_err(|error| format!("failed to normalize parsed catalog: {error}"))?;
        let catalog = normalized.parsed_catalog();
        let messages = normalized
            .iter()
            .map(|(_, message)| CatalogMessageSummary::from_message(message))
            .collect::<Vec<_>>();
        let diagnostics = catalog
            .diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    "{}:{}:{}",
                    diagnostic_severity_label(diagnostic.severity),
                    diagnostic.code,
                    diagnostic.message
                )
            })
            .collect::<Vec<_>>();

        Ok(Self {
            locale: catalog.locale.clone(),
            diagnostics,
            messages,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct CatalogMessageSummary {
    pub(super) msgctxt: Option<String>,
    pub(super) msgid: String,
    pub(super) translation: CatalogTranslationSummary,
    pub(super) comments: Vec<String>,
    pub(super) origins: Vec<CatalogOriginSummary>,
    pub(super) obsolete: bool,
}

impl CatalogMessageSummary {
    pub(super) fn from_message(message: &CatalogMessage) -> Self {
        Self {
            msgctxt: message.msgctxt.clone(),
            msgid: message.msgid.clone(),
            translation: CatalogTranslationSummary::from_translation(&message.translation),
            comments: message.comments.clone(),
            origins: message
                .origin
                .iter()
                .map(CatalogOriginSummary::from_origin)
                .collect(),
            obsolete: message.obsolete.is_some(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum CatalogTranslationSummary {
    Singular(String),
    Plural {
        source_one: Option<String>,
        source_other: String,
        variable: String,
        forms: Vec<(String, String)>,
    },
}

impl CatalogTranslationSummary {
    pub(super) fn from_translation(translation: &TranslationShape) -> Self {
        match translation {
            TranslationShape::Singular { value } => Self::Singular(value.clone()),
            TranslationShape::Plural {
                source,
                translation,
                variable,
            } => Self::Plural {
                source_one: source.one.clone(),
                source_other: source.other.clone(),
                variable: variable.clone(),
                forms: translation
                    .iter()
                    .map(|(category, value)| (category.clone(), value.clone()))
                    .collect(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct CatalogOriginSummary {
    pub(super) file: String,
    pub(super) scope: Option<String>,
}

impl CatalogOriginSummary {
    pub(super) fn from_origin(origin: &CatalogOrigin) -> Self {
        Self {
            file: origin.file.clone(),
            scope: origin.scope.clone(),
        }
    }
}

pub(super) fn diagnostic_severity_label(severity: ferrocat_po::DiagnosticSeverity) -> &'static str {
    match severity {
        ferrocat_po::DiagnosticSeverity::Info => "info",
        ferrocat_po::DiagnosticSeverity::Warning => "warning",
        ferrocat_po::DiagnosticSeverity::Error => "error",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct IcuFixtureSummary {
    pub(super) messages: Vec<IcuMessageSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct IcuMessageSummary {
    pub(super) variable_names: Vec<String>,
    pub(super) selector_kinds: Vec<String>,
    pub(super) selectors: Vec<String>,
    pub(super) plural_categories: Vec<String>,
    pub(super) tag_names: Vec<String>,
    pub(super) formatter_kinds: Vec<String>,
    pub(super) literal_segments: usize,
    pub(super) argument_count: usize,
    pub(super) pound_count: usize,
    pub(super) max_depth: usize,
}

impl IcuMessageSummary {
    pub(super) fn from_message(message: &IcuMessage) -> Self {
        let mut collector = IcuCollector::default();
        collector.visit_nodes(&message.nodes, 1);
        Self {
            variable_names: collector.variable_names.into_iter().collect(),
            selector_kinds: collector.selector_kinds.into_iter().collect(),
            selectors: collector.selectors.into_iter().collect(),
            plural_categories: collector.plural_categories.into_iter().collect(),
            tag_names: collector.tag_names.into_iter().collect(),
            formatter_kinds: collector.formatter_kinds.into_iter().collect(),
            literal_segments: collector.literal_segments,
            argument_count: collector.argument_count,
            pound_count: collector.pound_count,
            max_depth: collector.max_depth,
        }
    }
}

#[derive(Default)]
pub(super) struct IcuCollector {
    pub(super) variable_names: BTreeSet<String>,
    pub(super) selector_kinds: BTreeSet<String>,
    pub(super) selectors: BTreeSet<String>,
    pub(super) plural_categories: BTreeSet<String>,
    pub(super) tag_names: BTreeSet<String>,
    pub(super) formatter_kinds: BTreeSet<String>,
    pub(super) literal_segments: usize,
    pub(super) argument_count: usize,
    pub(super) pound_count: usize,
    pub(super) max_depth: usize,
}

impl IcuCollector {
    pub(super) fn visit_nodes(&mut self, nodes: &[IcuNode], depth: usize) {
        self.max_depth = self.max_depth.max(depth);
        for node in nodes {
            self.visit_node(node, depth);
        }
    }

    pub(super) fn visit_node(&mut self, node: &IcuNode, depth: usize) {
        self.max_depth = self.max_depth.max(depth);
        match node {
            IcuNode::Literal(_) => self.literal_segments += 1,
            IcuNode::Argument { name } => {
                self.argument_count += 1;
                self.variable_names.insert(name.clone());
            }
            IcuNode::Number { name, .. } => self.push_formatter("number", name),
            IcuNode::Date { name, .. } => self.push_formatter("date", name),
            IcuNode::Time { name, .. } => self.push_formatter("time", name),
            IcuNode::List { name, .. } => self.push_formatter("list", name),
            IcuNode::Duration { name, .. } => self.push_formatter("duration", name),
            IcuNode::Ago { name, .. } => self.push_formatter("ago", name),
            IcuNode::Name { name, .. } => self.push_formatter("name", name),
            IcuNode::Select { name, options } => {
                self.argument_count += 1;
                self.variable_names.insert(name.clone());
                self.selector_kinds.insert("select".to_owned());
                self.visit_options(options, depth + 1, false);
            }
            IcuNode::Plural {
                name,
                kind,
                options,
                ..
            } => {
                self.argument_count += 1;
                self.variable_names.insert(name.clone());
                self.selector_kinds.insert(match kind {
                    IcuPluralKind::Cardinal => "plural".to_owned(),
                    IcuPluralKind::Ordinal => "selectordinal".to_owned(),
                });
                self.visit_options(options, depth + 1, true);
            }
            IcuNode::Pound => self.pound_count += 1,
            IcuNode::Tag { name, children, .. } => {
                self.tag_names.insert(name.clone());
                self.visit_nodes(children, depth + 1);
            }
            _ => {}
        }
    }

    pub(super) fn visit_options(&mut self, options: &[IcuOption], depth: usize, plural: bool) {
        for option in options {
            self.selectors.insert(option.selector.clone());
            if plural {
                self.plural_categories.insert(option.selector.clone());
            }
            self.visit_nodes(&option.value, depth);
        }
    }

    pub(super) fn push_formatter(&mut self, kind: &str, name: &str) {
        self.argument_count += 1;
        self.variable_names.insert(name.to_owned());
        self.formatter_kinds.insert(kind.to_owned());
    }
}

pub(super) fn digest_summary<T: Serialize>(value: &T) -> Result<String, String> {
    let canonical = canonical_json_string(value)?;
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let digest = hasher.finalize();
    Ok(bytes_to_lower_hex(digest.as_ref()))
}

pub(super) fn bytes_to_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[usize::from(byte >> 4)] as char);
        out.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    out
}

pub(super) fn canonical_json_string<T: Serialize>(value: &T) -> Result<String, String> {
    let value = serde_json::to_value(value)
        .map_err(|error| format!("failed to build canonical JSON value: {error}"))?;
    let sorted = sort_json_value(value);
    serde_json::to_string(&sorted)
        .map_err(|error| format!("failed to render canonical JSON: {error}"))
}

pub(super) fn sort_json_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(sort_json_value).collect::<Vec<_>>())
        }
        serde_json::Value::Object(values) => {
            let mut sorted = serde_json::Map::new();
            let mut keys = values.into_iter().collect::<Vec<_>>();
            keys.sort_by(|left, right| left.0.cmp(&right.0));
            for (key, value) in keys {
                sorted.insert(key, sort_json_value(value));
            }
            serde_json::Value::Object(sorted)
        }
        other => other,
    }
}
