use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    IcuAnalysis, IcuArgumentKind, IcuDiagnosticSeverity, IcuParseError, IcuPluralKind,
    IcuStyleKind, analyze_icu, parse_icu,
};

/// Authoring input for semantic message metadata.
///
/// This is the progressive, JSON-friendly shape. Only `msgid` is required;
/// semantic fields may be omitted and derived from the ICU MessageFormat v1
/// source where possible.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct MessageMetadataInput {
    /// Exact catalog identity and authored source payload.
    pub msgid: String,
    /// Optional gettext-style context used to disambiguate identical `msgid`s.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub msgctxt: Option<String>,
    /// Optional translator-facing note.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub description: Option<String>,
    /// Optional extraction origins.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub origin: Vec<MessageOriginMetadata>,
    /// Optional argument metadata keyed by argument name.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub args: Option<BTreeMap<String, MessageArgumentMetadataInput>>,
    /// Optional rich-text tag names.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub tags: Option<Vec<String>>,
    /// Optional selector metadata keyed by selecting argument name.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub selectors: Option<BTreeMap<String, MessageSelectorMetadata>>,
}

impl MessageMetadataInput {
    /// Creates a minimal metadata input from a `msgid`.
    #[must_use]
    pub fn new(msgid: impl Into<String>) -> Self {
        Self {
            msgid: msgid.into(),
            msgctxt: None,
            description: None,
            origin: Vec::new(),
            args: None,
            tags: None,
            selectors: None,
        }
    }
}

/// Normalized semantic metadata for one source message.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct MessageMetadata {
    /// Exact catalog identity and authored source payload.
    pub msgid: String,
    /// Optional gettext-style context used to disambiguate identical `msgid`s.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub msgctxt: Option<String>,
    /// Optional translator-facing note.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub description: Option<String>,
    /// Optional extraction origins.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub origin: Vec<MessageOriginMetadata>,
    /// Normalized argument metadata keyed by argument name.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "BTreeMap::is_empty")
    )]
    pub args: BTreeMap<String, MessageArgumentMetadata>,
    /// Rich-text tag names in first-seen order.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub tags: Vec<String>,
    /// Normalized selector metadata keyed by selecting argument name.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "BTreeMap::is_empty")
    )]
    pub selectors: BTreeMap<String, MessageSelectorMetadata>,
}

/// Extraction origin attached to semantic message metadata.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct MessageOriginMetadata {
    /// Source file path, when known.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub file: Option<String>,
    /// Source line number, when known.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub line: Option<u32>,
    /// Host component name, when known.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub component: Option<String>,
    /// Host route or page identifier, when known.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub route: Option<String>,
}

/// Progressive authoring input for one argument.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum MessageArgumentMetadataInput {
    /// Shorthand kind form, for example `"string"`.
    Kind(MessageArgumentKind),
    /// Full object form.
    Details(MessageArgumentMetadata),
}

impl From<MessageArgumentKind> for MessageArgumentMetadataInput {
    fn from(kind: MessageArgumentKind) -> Self {
        Self::Kind(kind)
    }
}

impl From<MessageArgumentMetadata> for MessageArgumentMetadataInput {
    fn from(metadata: MessageArgumentMetadata) -> Self {
        Self::Details(metadata)
    }
}

impl MessageArgumentMetadataInput {
    fn into_metadata(self) -> MessageArgumentMetadata {
        match self {
            Self::Kind(kind) => MessageArgumentMetadata {
                kind,
                ..MessageArgumentMetadata::default()
            },
            Self::Details(metadata) => metadata,
        }
    }
}

/// Normalized semantic metadata for one message argument.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct MessageArgumentMetadata {
    /// Broad message-level data kind.
    #[cfg_attr(feature = "serde", serde(default))]
    pub kind: MessageArgumentKind,
    /// Optional semantic role such as `count`, `currency`, or `url`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub role: Option<String>,
    /// Allowed enum/select values when known from extraction.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub values: Vec<String>,
    /// Optional formatter metadata.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub format: Option<MessageArgumentFormatMetadata>,
}

impl Default for MessageArgumentMetadata {
    fn default() -> Self {
        Self {
            kind: MessageArgumentKind::Unknown,
            role: None,
            values: Vec::new(),
            format: None,
        }
    }
}

/// Broad argument kind used by semantic message metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum MessageArgumentKind {
    /// Text or string-like value.
    String,
    /// Number-like value.
    Number,
    /// Date value.
    Date,
    /// Time value.
    Time,
    /// Date-time value.
    Datetime,
    /// Boolean value.
    Boolean,
    /// Enumerated value.
    Enum,
    /// List value.
    List,
    /// Duration value.
    Duration,
    /// Relative-time value.
    RelativeTime,
    /// Person, region, or display name value.
    Name,
    /// Unknown or intentionally unspecified value.
    #[default]
    Unknown,
}

/// Formatter metadata attached to an argument.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct MessageArgumentFormatMetadata {
    /// Raw formatter style, when present.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub style: Option<String>,
    /// Classification of the formatter style.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub style_kind: Option<MessageFormatStyleKind>,
}

/// Classification of a message formatter style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum MessageFormatStyleKind {
    /// Formatter has no style segment.
    None,
    /// Formatter uses a known named style.
    Predefined,
    /// Formatter uses an ICU skeleton.
    Skeleton,
    /// Formatter uses an opaque pattern-style segment.
    Pattern,
}

/// Selector metadata attached to a selecting argument.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct MessageSelectorMetadata {
    /// Selector expression kind.
    pub kind: MessageSelectorKind,
    /// Known selector cases in source order.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub cases: Vec<String>,
    /// Optional plural offset. Omitted when zero.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub offset: Option<u32>,
}

/// Selector kind used by semantic message metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum MessageSelectorKind {
    /// General select expression.
    Select,
    /// Cardinal plural expression.
    Plural,
    /// Ordinal plural expression.
    #[cfg_attr(feature = "serde", serde(rename = "selectordinal"))]
    SelectOrdinal,
}

/// One diagnostic emitted while validating semantic message metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageMetadataDiagnostic {
    /// Severity for the diagnostic.
    pub severity: IcuDiagnosticSeverity,
    /// Stable machine-readable diagnostic code.
    pub code: String,
    /// Human-readable explanation of the condition.
    pub message: String,
    /// Argument, selector, tag, or field name associated with the diagnostic.
    pub name: Option<String>,
}

impl MessageMetadataDiagnostic {
    fn new(
        severity: IcuDiagnosticSeverity,
        code: &'static str,
        message: impl Into<String>,
        name: impl Into<Option<String>>,
    ) -> Self {
        Self {
            severity,
            code: code.to_owned(),
            message: message.into(),
            name: name.into(),
        }
    }
}

/// Report returned by [`validate_message_metadata`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MessageMetadataValidationReport {
    /// Diagnostics found while validating the metadata.
    pub diagnostics: Vec<MessageMetadataDiagnostic>,
}

impl MessageMetadataValidationReport {
    /// Returns `true` when the report contains at least one error diagnostic.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == IcuDiagnosticSeverity::Error)
    }
}

/// Derives normalized semantic metadata from an ICU MessageFormat v1 `msgid`.
///
/// Plain text messages are valid and produce metadata without arguments, tags,
/// or selectors.
///
/// # Errors
///
/// Returns an ICU parse error when `msgid` is not valid ICU MessageFormat v1.
pub fn derive_message_metadata_from_icu(
    msgid: &str,
    msgctxt: Option<&str>,
) -> Result<MessageMetadata, IcuParseError> {
    let message = parse_icu(msgid)?;
    let analysis = analyze_icu(&message);
    Ok(MessageMetadata {
        msgid: msgid.to_owned(),
        msgctxt: msgctxt.map(str::to_owned),
        description: None,
        origin: Vec::new(),
        args: derive_args(&analysis),
        tags: unique_strings(analysis.tags.iter().map(|tag| tag.name.as_str())),
        selectors: derive_selectors(&analysis),
    })
}

/// Normalizes progressive semantic message metadata into canonical object form.
///
/// Omitted semantic fields are derived from `msgid`. Explicit metadata is kept
/// and enriched with any derived facts that do not conflict.
///
/// # Errors
///
/// Returns an ICU parse error when `input.msgid` is not valid ICU MessageFormat
/// v1.
pub fn normalize_message_metadata(
    input: MessageMetadataInput,
) -> Result<MessageMetadata, IcuParseError> {
    let mut derived = derive_message_metadata_from_icu(&input.msgid, input.msgctxt.as_deref())?;
    derived.description = input.description;
    derived.origin = input.origin;

    if let Some(args) = input.args {
        let mut normalized_args = args
            .into_iter()
            .map(|(name, argument)| (name, argument.into_metadata()))
            .collect::<BTreeMap<_, _>>();
        for (name, argument) in derived.args {
            normalized_args.entry(name).or_insert(argument);
        }
        derived.args = normalized_args;
    }

    if let Some(tags) = input.tags {
        let mut normalized_tags = unique_strings(tags.iter().map(String::as_str));
        for tag in derived.tags {
            if !normalized_tags.contains(&tag) {
                normalized_tags.push(tag);
            }
        }
        derived.tags = normalized_tags;
    }

    if let Some(selectors) = input.selectors {
        let mut normalized_selectors = selectors;
        for (name, selector) in derived.selectors {
            normalized_selectors.entry(name).or_insert(selector);
        }
        derived.selectors = normalized_selectors;
    }

    Ok(derived)
}

/// Validates progressive semantic message metadata against its `msgid`.
#[must_use]
pub fn validate_message_metadata(input: &MessageMetadataInput) -> MessageMetadataValidationReport {
    let Ok(derived) = derive_message_metadata_from_icu(&input.msgid, input.msgctxt.as_deref())
    else {
        return MessageMetadataValidationReport {
            diagnostics: vec![MessageMetadataDiagnostic::new(
                IcuDiagnosticSeverity::Error,
                "metadata.invalid_msgid",
                "Message metadata `msgid` is not valid ICU MessageFormat v1.",
                Some("msgid".to_owned()),
            )],
        };
    };

    let mut report = MessageMetadataValidationReport::default();
    if let Some(args) = &input.args {
        validate_args(args, &derived.args, &mut report);
    }
    if let Some(tags) = &input.tags {
        validate_tags(tags, &derived.tags, &mut report);
    }
    if let Some(selectors) = &input.selectors {
        validate_selectors(selectors, &derived.selectors, &mut report);
    }
    report
}

fn derive_args(analysis: &IcuAnalysis) -> BTreeMap<String, MessageArgumentMetadata> {
    let mut args = BTreeMap::<String, MessageArgumentMetadata>::new();
    for argument in &analysis.arguments {
        args.entry(argument.name.clone())
            .and_modify(|metadata| merge_icu_argument(metadata, argument.kind))
            .or_insert_with(|| metadata_for_icu_argument(argument.kind));
    }
    for formatter in &analysis.formatters {
        args.entry(formatter.name.clone())
            .and_modify(|metadata| {
                merge_icu_argument(metadata, formatter.kind);
                metadata.format = Some(MessageArgumentFormatMetadata {
                    style: formatter.style.clone(),
                    style_kind: Some(style_kind(formatter.style_kind)),
                });
            })
            .or_insert_with(|| {
                let mut metadata = metadata_for_icu_argument(formatter.kind);
                metadata.format = Some(MessageArgumentFormatMetadata {
                    style: formatter.style.clone(),
                    style_kind: Some(style_kind(formatter.style_kind)),
                });
                metadata
            });
    }
    for select in &analysis.selects {
        let metadata = args.entry(select.name.clone()).or_default();
        metadata.kind = MessageArgumentKind::Enum;
        metadata.values = select.selectors.clone();
    }
    for plural in &analysis.plurals {
        let metadata = args.entry(plural.name.clone()).or_default();
        match plural.kind {
            IcuPluralKind::Cardinal => {
                metadata.kind = MessageArgumentKind::Number;
                metadata.role.get_or_insert_with(|| "count".to_owned());
            }
            IcuPluralKind::Ordinal => {
                metadata.kind = MessageArgumentKind::Number;
                metadata.role.get_or_insert_with(|| "ordinal".to_owned());
            }
        }
    }
    args
}

fn merge_icu_argument(metadata: &mut MessageArgumentMetadata, kind: IcuArgumentKind) {
    let next = argument_kind(kind);
    if metadata.kind == MessageArgumentKind::Unknown || next != MessageArgumentKind::Unknown {
        metadata.kind = next;
    }
}

fn metadata_for_icu_argument(kind: IcuArgumentKind) -> MessageArgumentMetadata {
    MessageArgumentMetadata {
        kind: argument_kind(kind),
        role: role_for_icu_argument(kind),
        values: Vec::new(),
        format: None,
    }
}

fn argument_kind(kind: IcuArgumentKind) -> MessageArgumentKind {
    match kind {
        IcuArgumentKind::Argument => MessageArgumentKind::Unknown,
        IcuArgumentKind::Number | IcuArgumentKind::Plural | IcuArgumentKind::SelectOrdinal => {
            MessageArgumentKind::Number
        }
        IcuArgumentKind::Date => MessageArgumentKind::Date,
        IcuArgumentKind::Time => MessageArgumentKind::Time,
        IcuArgumentKind::List => MessageArgumentKind::List,
        IcuArgumentKind::Duration => MessageArgumentKind::Duration,
        IcuArgumentKind::Ago => MessageArgumentKind::RelativeTime,
        IcuArgumentKind::Name => MessageArgumentKind::Name,
        IcuArgumentKind::Select => MessageArgumentKind::Enum,
    }
}

fn role_for_icu_argument(kind: IcuArgumentKind) -> Option<String> {
    match kind {
        IcuArgumentKind::Plural => Some("count".to_owned()),
        IcuArgumentKind::SelectOrdinal => Some("ordinal".to_owned()),
        _ => None,
    }
}

fn style_kind(kind: IcuStyleKind) -> MessageFormatStyleKind {
    match kind {
        IcuStyleKind::None => MessageFormatStyleKind::None,
        IcuStyleKind::Predefined => MessageFormatStyleKind::Predefined,
        IcuStyleKind::Skeleton => MessageFormatStyleKind::Skeleton,
        IcuStyleKind::Pattern => MessageFormatStyleKind::Pattern,
    }
}

fn derive_selectors(analysis: &IcuAnalysis) -> BTreeMap<String, MessageSelectorMetadata> {
    let mut selectors = BTreeMap::new();
    for select in &analysis.selects {
        selectors.insert(
            select.name.clone(),
            MessageSelectorMetadata {
                kind: MessageSelectorKind::Select,
                cases: select.selectors.clone(),
                offset: None,
            },
        );
    }
    for plural in &analysis.plurals {
        selectors.insert(
            plural.name.clone(),
            MessageSelectorMetadata {
                kind: match plural.kind {
                    IcuPluralKind::Cardinal => MessageSelectorKind::Plural,
                    IcuPluralKind::Ordinal => MessageSelectorKind::SelectOrdinal,
                },
                cases: plural.selectors.clone(),
                offset: (plural.offset != 0).then_some(plural.offset),
            },
        );
    }
    selectors
}

fn validate_args(
    input: &BTreeMap<String, MessageArgumentMetadataInput>,
    derived: &BTreeMap<String, MessageArgumentMetadata>,
    report: &mut MessageMetadataValidationReport,
) {
    let normalized = input
        .iter()
        .map(|(name, argument)| (name, argument.clone().into_metadata()))
        .collect::<BTreeMap<_, _>>();

    for name in derived.keys() {
        if !normalized.contains_key(name) {
            report.diagnostics.push(MessageMetadataDiagnostic::new(
                IcuDiagnosticSeverity::Warning,
                "metadata.missing_argument",
                format!("Message metadata is missing parsed ICU argument `{name}`."),
                Some(name.clone()),
            ));
        }
    }
    for (name, argument) in &normalized {
        let Some(derived_argument) = derived.get(*name) else {
            report.diagnostics.push(MessageMetadataDiagnostic::new(
                IcuDiagnosticSeverity::Error,
                "metadata.extra_argument",
                format!("Message metadata declares argument `{name}` that is not used by `msgid`."),
                Some((*name).clone()),
            ));
            continue;
        };
        if argument.kind != MessageArgumentKind::Unknown
            && derived_argument.kind != MessageArgumentKind::Unknown
            && argument.kind != derived_argument.kind
        {
            report.diagnostics.push(MessageMetadataDiagnostic::new(
                IcuDiagnosticSeverity::Error,
                "metadata.argument_kind_mismatch",
                format!(
                    "Message metadata declares argument `{name}` as {:?}, but `msgid` uses {:?}.",
                    argument.kind, derived_argument.kind
                ),
                Some((*name).clone()),
            ));
        }
    }
}

fn validate_tags(
    input: &[String],
    derived: &[String],
    report: &mut MessageMetadataValidationReport,
) {
    let input = input.iter().cloned().collect::<BTreeSet<_>>();
    let derived = derived.iter().cloned().collect::<BTreeSet<_>>();
    for tag in &derived {
        if !input.contains(tag) {
            report.diagnostics.push(MessageMetadataDiagnostic::new(
                IcuDiagnosticSeverity::Warning,
                "metadata.missing_tag",
                format!("Message metadata is missing parsed ICU tag `{tag}`."),
                Some(tag.clone()),
            ));
        }
    }
    for tag in &input {
        if !derived.contains(tag) {
            report.diagnostics.push(MessageMetadataDiagnostic::new(
                IcuDiagnosticSeverity::Error,
                "metadata.extra_tag",
                format!("Message metadata declares tag `{tag}` that is not used by `msgid`."),
                Some(tag.clone()),
            ));
        }
    }
}

fn validate_selectors(
    input: &BTreeMap<String, MessageSelectorMetadata>,
    derived: &BTreeMap<String, MessageSelectorMetadata>,
    report: &mut MessageMetadataValidationReport,
) {
    for name in derived.keys() {
        if !input.contains_key(name) {
            report.diagnostics.push(MessageMetadataDiagnostic::new(
                IcuDiagnosticSeverity::Warning,
                "metadata.missing_selector",
                format!("Message metadata is missing parsed ICU selector `{name}`."),
                Some(name.clone()),
            ));
        }
    }
    for (name, selector) in input {
        let Some(derived_selector) = derived.get(name) else {
            report.diagnostics.push(MessageMetadataDiagnostic::new(
                IcuDiagnosticSeverity::Error,
                "metadata.extra_selector",
                format!("Message metadata declares selector `{name}` that is not used by `msgid`."),
                Some(name.clone()),
            ));
            continue;
        };
        if selector.kind != derived_selector.kind {
            report.diagnostics.push(MessageMetadataDiagnostic::new(
                IcuDiagnosticSeverity::Error,
                "metadata.selector_kind_mismatch",
                format!(
                    "Message metadata declares selector `{name}` as {:?}, but `msgid` uses {:?}.",
                    selector.kind, derived_selector.kind
                ),
                Some(name.clone()),
            ));
        }
        let derived_cases = derived_selector
            .cases
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let input_cases = selector.cases.iter().cloned().collect::<BTreeSet<_>>();
        for case in &derived_selector.cases {
            if !input_cases.contains(case) {
                report.diagnostics.push(MessageMetadataDiagnostic::new(
                    IcuDiagnosticSeverity::Warning,
                    "metadata.missing_selector_case",
                    format!(
                        "Message metadata is missing parsed ICU selector case `{case}` for `{name}`."
                    ),
                    Some(format!("{name}:{case}")),
                ));
            }
        }
        for case in &selector.cases {
            if !derived_cases.contains(case) {
                report.diagnostics.push(MessageMetadataDiagnostic::new(
                    IcuDiagnosticSeverity::Error,
                    "metadata.extra_selector_case",
                    format!(
                        "Message metadata declares selector case `{case}` for `{name}` that is not used by `msgid`."
                    ),
                    Some(format!("{name}:{case}")),
                ));
            }
        }
        if selector.offset != derived_selector.offset {
            report.diagnostics.push(MessageMetadataDiagnostic::new(
                IcuDiagnosticSeverity::Error,
                "metadata.selector_offset_mismatch",
                format!(
                    "Message metadata declares selector `{name}` offset {:?}, but `msgid` uses {:?}.",
                    selector.offset, derived_selector.offset
                ),
                Some(name.clone()),
            ));
        }
    }
}

fn unique_strings<'a>(values: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for value in values {
        if seen.insert(value.to_owned()) {
            out.push(value.to_owned());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{
        IcuDiagnosticSeverity, MessageArgumentFormatMetadata, MessageArgumentKind,
        MessageArgumentMetadata, MessageArgumentMetadataInput, MessageFormatStyleKind,
        MessageMetadataInput, MessageSelectorKind, MessageSelectorMetadata,
        derive_message_metadata_from_icu, normalize_message_metadata, validate_message_metadata,
    };

    #[test]
    fn minimal_metadata_normalizes_without_semantic_fields() {
        let input = MessageMetadataInput::new("Cart");

        let metadata = normalize_message_metadata(input).expect("normalize metadata");

        assert_eq!(metadata.msgid, "Cart");
        assert!(metadata.args.is_empty());
        assert!(metadata.tags.is_empty());
        assert!(metadata.selectors.is_empty());
    }

    #[test]
    fn placeholder_msgid_derives_argument_when_omitted() {
        let metadata =
            derive_message_metadata_from_icu("Hello {name}", None).expect("derive metadata");

        assert_eq!(
            metadata.args.get("name").map(|argument| argument.kind),
            Some(MessageArgumentKind::Unknown)
        );
    }

    #[test]
    fn shorthand_argument_input_normalizes_to_object_metadata() {
        let mut args = BTreeMap::new();
        args.insert(
            "name".to_owned(),
            MessageArgumentMetadataInput::Kind(MessageArgumentKind::String),
        );
        let mut input = MessageMetadataInput::new("Hello {name}");
        input.args = Some(args);

        let metadata = normalize_message_metadata(input).expect("normalize metadata");

        assert_eq!(
            metadata.args.get("name").map(|argument| argument.kind),
            Some(MessageArgumentKind::String)
        );
    }

    #[test]
    fn msgctxt_and_msgid_remain_exact_source_identity() {
        let mut input = MessageMetadataInput::new("Home");
        input.msgctxt = Some("navigation".to_owned());

        let metadata = normalize_message_metadata(input).expect("normalize metadata");

        assert_eq!(metadata.msgid, "Home");
        assert_eq!(metadata.msgctxt.as_deref(), Some("navigation"));
    }

    #[test]
    fn plural_msgid_derives_count_argument_and_selector_cases() {
        let metadata = derive_message_metadata_from_icu(
            "{count, plural, one {One item} other {# items}}",
            None,
        )
        .expect("derive metadata");

        let count = metadata.args.get("count").expect("count metadata");
        assert_eq!(count.kind, MessageArgumentKind::Number);
        assert_eq!(count.role.as_deref(), Some("count"));
        let selector = metadata.selectors.get("count").expect("count selector");
        assert_eq!(selector.kind, MessageSelectorKind::Plural);
        assert_eq!(selector.cases, vec!["one", "other"]);
        assert_eq!(selector.offset, None);
    }

    #[test]
    fn select_msgid_derives_enum_argument_and_selector_cases() {
        let metadata = derive_message_metadata_from_icu(
            "{status, select, shipped {Shipped} cancelled {Cancelled} other {Updated}}",
            None,
        )
        .expect("derive metadata");

        let status = metadata.args.get("status").expect("status argument");
        assert_eq!(status.kind, MessageArgumentKind::Enum);
        assert_eq!(status.values, vec!["shipped", "cancelled", "other"]);
        assert_eq!(
            metadata
                .selectors
                .get("status")
                .map(|selector| selector.kind),
            Some(MessageSelectorKind::Select)
        );
    }

    #[test]
    fn rich_text_tags_derive_into_metadata() {
        let metadata =
            derive_message_metadata_from_icu("Read <link>terms</link>", None).expect("derive");

        assert_eq!(metadata.tags, vec!["link"]);
    }

    #[test]
    fn conflicting_explicit_metadata_emits_diagnostics() {
        let mut args = BTreeMap::new();
        args.insert(
            "count".to_owned(),
            MessageArgumentMetadataInput::Kind(MessageArgumentKind::String),
        );
        args.insert(
            "unused".to_owned(),
            MessageArgumentMetadataInput::Kind(MessageArgumentKind::String),
        );
        let mut input =
            MessageMetadataInput::new("{count, plural, one {One item} other {# items}}");
        input.args = Some(args);

        let report = validate_message_metadata(&input);
        let codes = report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>();

        assert!(report.has_errors());
        assert!(codes.contains(&"metadata.argument_kind_mismatch"));
        assert!(codes.contains(&"metadata.extra_argument"));
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == IcuDiagnosticSeverity::Error)
        );
    }

    #[test]
    fn selector_metadata_reports_missing_source_cases() {
        let mut selectors = BTreeMap::new();
        selectors.insert(
            "count".to_owned(),
            MessageSelectorMetadata {
                kind: MessageSelectorKind::Plural,
                cases: vec!["one".to_owned()],
                offset: None,
            },
        );
        let mut input =
            MessageMetadataInput::new("{count, plural, one {One item} other {# items}}");
        input.selectors = Some(selectors);

        let report = validate_message_metadata(&input);

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "metadata.missing_selector_case"
                && diagnostic.name.as_deref() == Some("count:other")
        }));
    }

    #[test]
    fn id_style_msgid_is_accepted_without_special_behavior() {
        let input = MessageMetadataInput::new("cart.item_count");

        let report = validate_message_metadata(&input);

        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn json_shorthand_argument_deserializes() {
        let input: MessageMetadataInput = serde_json::from_str(
            r#"{
                "msgid": "Hello {name}",
                "args": {
                    "name": "string"
                }
            }"#,
        )
        .expect("deserialize metadata input");

        let metadata = normalize_message_metadata(input).expect("normalize metadata");

        assert_eq!(
            metadata.args.get("name").map(|argument| argument.kind),
            Some(MessageArgumentKind::String)
        );
    }

    #[test]
    fn explicit_details_origin_tags_and_selectors_are_preserved_and_enriched() {
        let mut args = BTreeMap::new();
        args.insert(
            "name".to_owned(),
            MessageArgumentMetadata {
                kind: MessageArgumentKind::String,
                ..MessageArgumentMetadata::default()
            }
            .into(),
        );
        let mut selectors = BTreeMap::new();
        selectors.insert(
            "status".to_owned(),
            MessageSelectorMetadata {
                kind: MessageSelectorKind::Select,
                cases: vec!["open".to_owned(), "other".to_owned()],
                offset: None,
            },
        );
        let mut input = MessageMetadataInput::new(
            "<link>{status, select, open {Hello {name}} other {Done}}</link> <strong>!</strong>",
        );
        input.description = Some("Shown in the activity feed.".to_owned());
        input.origin.push(crate::MessageOriginMetadata {
            file: Some("src/app.rs".to_owned()),
            line: Some(12),
            component: Some("ActivityFeed".to_owned()),
            route: Some("/activity".to_owned()),
        });
        input.args = Some(args);
        input.tags = Some(vec!["link".to_owned(), "link".to_owned()]);
        input.selectors = Some(selectors);

        let metadata = normalize_message_metadata(input).expect("normalize metadata");

        assert_eq!(
            metadata.args.get("name").map(|argument| argument.kind),
            Some(MessageArgumentKind::String)
        );
        assert_eq!(
            metadata.args.get("status").map(|argument| argument.kind),
            Some(MessageArgumentKind::Enum)
        );
        assert_eq!(metadata.tags, vec!["link", "strong"]);
        assert_eq!(
            metadata.description.as_deref(),
            Some("Shown in the activity feed.")
        );
        assert_eq!(metadata.origin[0].file.as_deref(), Some("src/app.rs"));
        assert_eq!(
            metadata
                .selectors
                .get("status")
                .map(|selector| selector.cases.as_slice()),
            Some(&["open".to_owned(), "other".to_owned()][..])
        );
    }

    #[test]
    fn formatter_metadata_derives_kinds_styles_roles_and_selector_offsets() {
        let metadata = derive_message_metadata_from_icu(
            "{price, number, ::currency/USD} {created, date, short} {time, time, HH:mm} \
             {items, list, conjunction} {elapsed, duration} {since, ago} {person, name} \
             {rank, selectordinal, offset:1 one {#st} other {#th}}",
            None,
        )
        .expect("derive metadata");

        assert_eq!(
            metadata
                .args
                .get("price")
                .and_then(|argument| argument.format.as_ref()),
            Some(&MessageArgumentFormatMetadata {
                style: Some("::currency/USD".to_owned()),
                style_kind: Some(MessageFormatStyleKind::Skeleton),
            })
        );
        assert_eq!(
            metadata
                .args
                .get("created")
                .and_then(|argument| argument.format.as_ref()),
            Some(&MessageArgumentFormatMetadata {
                style: Some("short".to_owned()),
                style_kind: Some(MessageFormatStyleKind::Predefined),
            })
        );
        assert_eq!(
            metadata
                .args
                .get("time")
                .and_then(|argument| argument.format.as_ref()),
            Some(&MessageArgumentFormatMetadata {
                style: Some("HH:mm".to_owned()),
                style_kind: Some(MessageFormatStyleKind::Pattern),
            })
        );
        assert_eq!(
            metadata.args.get("items").map(|argument| argument.kind),
            Some(MessageArgumentKind::List)
        );
        assert_eq!(
            metadata.args.get("elapsed").map(|argument| argument.kind),
            Some(MessageArgumentKind::Duration)
        );
        assert_eq!(
            metadata.args.get("since").map(|argument| argument.kind),
            Some(MessageArgumentKind::RelativeTime)
        );
        assert_eq!(
            metadata.args.get("person").map(|argument| argument.kind),
            Some(MessageArgumentKind::Name)
        );
        let rank = metadata.args.get("rank").expect("rank argument");
        assert_eq!(rank.kind, MessageArgumentKind::Number);
        assert_eq!(rank.role.as_deref(), Some("ordinal"));
        let selector = metadata.selectors.get("rank").expect("rank selector");
        assert_eq!(selector.kind, MessageSelectorKind::SelectOrdinal);
        assert_eq!(selector.offset, Some(1));
    }

    #[test]
    fn invalid_msgid_reports_metadata_diagnostic() {
        let input = MessageMetadataInput::new("{count, plural, one {One item}}");

        let report = validate_message_metadata(&input);

        assert_eq!(
            report
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>(),
            vec!["metadata.invalid_msgid"]
        );
        assert!(report.has_errors());
    }

    #[test]
    fn validation_reports_missing_and_extra_tags_and_selectors() {
        let mut selectors = BTreeMap::new();
        selectors.insert(
            "status".to_owned(),
            MessageSelectorMetadata {
                kind: MessageSelectorKind::Plural,
                cases: vec!["open".to_owned(), "closed".to_owned(), "other".to_owned()],
                offset: Some(1),
            },
        );
        selectors.insert(
            "unused".to_owned(),
            MessageSelectorMetadata {
                kind: MessageSelectorKind::Select,
                cases: vec!["other".to_owned()],
                offset: None,
            },
        );
        let mut input =
            MessageMetadataInput::new("<link>{status, select, open {Open} other {Other}}</link>");
        input.tags = Some(vec!["button".to_owned()]);
        input.selectors = Some(selectors);

        let report = validate_message_metadata(&input);
        let codes = report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>();

        assert!(report.has_errors());
        assert!(codes.contains(&"metadata.missing_tag"));
        assert!(codes.contains(&"metadata.extra_tag"));
        assert!(codes.contains(&"metadata.selector_kind_mismatch"));
        assert!(codes.contains(&"metadata.extra_selector"));
        assert!(codes.contains(&"metadata.extra_selector_case"));
        assert!(codes.contains(&"metadata.selector_offset_mismatch"));
    }

    #[test]
    fn validation_reports_missing_argument_tag_and_selector_metadata() {
        let mut input = MessageMetadataInput::new(
            "<link>{count, plural, one {{name} has one item} other {{name} has # items}}</link>",
        );
        input.args = Some(BTreeMap::new());
        input.tags = Some(Vec::new());
        input.selectors = Some(BTreeMap::new());

        let report = validate_message_metadata(&input);
        let codes = report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>();

        assert!(!report.has_errors());
        assert!(codes.contains(&"metadata.missing_argument"));
        assert!(codes.contains(&"metadata.missing_tag"));
        assert!(codes.contains(&"metadata.missing_selector"));
    }
}
