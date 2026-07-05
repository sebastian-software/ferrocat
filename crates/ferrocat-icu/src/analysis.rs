use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use crate::ast::{IcuMessage, IcuNode, IcuOption, IcuPluralKind};
use crate::diagnostic_codes::{self, DiagnosticCode};

/// Severity level attached to ICU authoring diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcuDiagnosticSeverity {
    /// Informational message that does not indicate a problem.
    Info,
    /// Non-fatal condition that may require author or translator attention.
    Warning,
    /// Structural incompatibility that can break runtime formatting.
    Error,
}

/// Broad role of an ICU argument in a parsed message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum IcuArgumentKind {
    /// Simple substitution such as `{name}`.
    Argument,
    /// Number formatter such as `{count, number}`.
    Number,
    /// Date formatter such as `{created, date}`.
    Date,
    /// Time formatter such as `{created, time}`.
    Time,
    /// List formatter such as `{items, list}`.
    List,
    /// Duration formatter such as `{elapsed, duration}`.
    Duration,
    /// Relative-time "ago" formatter.
    Ago,
    /// Name formatter.
    Name,
    /// Select expression.
    Select,
    /// Cardinal plural expression.
    Plural,
    /// Ordinal plural expression.
    SelectOrdinal,
}

impl fmt::Display for IcuArgumentKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Argument => "argument",
            Self::Number => "number",
            Self::Date => "date",
            Self::Time => "time",
            Self::List => "list",
            Self::Duration => "duration",
            Self::Ago => "ago",
            Self::Name => "name",
            Self::Select => "select",
            Self::Plural => "plural",
            Self::SelectOrdinal => "selectordinal",
        })
    }
}

/// Classification of an optional formatter style segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IcuStyleKind {
    /// Formatter has no style segment.
    None,
    /// Formatter uses a known named style.
    Predefined,
    /// Formatter uses an ICU skeleton, identified by the `::` prefix.
    Skeleton,
    /// Formatter uses an opaque pattern-style segment.
    Pattern,
}

/// One data argument reference discovered in an ICU message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcuArgument {
    /// Argument name referenced by the message.
    pub name: String,
    /// Structural role of the argument.
    pub kind: IcuArgumentKind,
}

/// One formatter reference discovered in an ICU message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcuFormatter {
    /// Argument name passed to the formatter.
    pub name: String,
    /// Formatter kind.
    pub kind: IcuArgumentKind,
    /// Raw optional style segment.
    pub style: Option<String>,
    /// Classification of the raw style segment.
    pub style_kind: IcuStyleKind,
}

/// Consumer-defined support decision for an ICU formatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IcuFormatterSupport {
    /// The formatter kind and style are supported by the consumer.
    Supported,
    /// The formatter kind is not supported by the consumer.
    ///
    /// Use this when a runtime cannot handle the formatter category at all,
    /// such as rejecting every `list` formatter.
    UnsupportedKind {
        /// Severity to attach to the emitted diagnostic.
        severity: IcuDiagnosticSeverity,
    },
    /// The formatter style is not supported by the consumer.
    ///
    /// Use this when a runtime accepts the formatter kind but rejects the
    /// specific style. A missing style is treated as the formatter's default
    /// style; reject it with this variant when the default style is unsupported.
    UnsupportedStyle {
        /// Severity to attach to the emitted diagnostic.
        severity: IcuDiagnosticSeverity,
    },
}

/// One select expression discovered in an ICU message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcuSelectSummary {
    /// Argument name used for selection.
    pub name: String,
    /// Selector labels in source order.
    pub selectors: Vec<String>,
}

/// One cardinal or ordinal plural expression discovered in an ICU message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcuPluralSummary {
    /// Argument name used for plural selection.
    pub name: String,
    /// Whether the plural expression is cardinal or ordinal.
    pub kind: IcuPluralKind,
    /// Parsed plural offset.
    pub offset: u32,
    /// Selector labels in source order.
    pub selectors: Vec<String>,
}

/// One rich-text tag discovered in an ICU message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcuTagSummary {
    /// Tag name without angle brackets.
    pub name: String,
}

/// Structural summary of a parsed ICU message.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IcuAnalysis {
    /// Data arguments referenced by the message, excluding rich-text tags.
    pub arguments: Vec<IcuArgument>,
    /// Formatter arguments referenced by the message.
    pub formatters: Vec<IcuFormatter>,
    /// Cardinal and ordinal plural expressions.
    pub plurals: Vec<IcuPluralSummary>,
    /// Select expressions.
    pub selects: Vec<IcuSelectSummary>,
    /// Rich-text tags.
    pub tags: Vec<IcuTagSummary>,
}

/// Options controlling ICU source/translation compatibility checks.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct IcuCompatibilityOptions {
    /// Whether translation-only data arguments should be reported.
    pub report_extra_arguments: bool,
    /// Whether translation-only rich-text tags should be reported.
    pub report_extra_tags: bool,
    /// Whether translation-only select selectors should be reported.
    pub report_extra_selectors: bool,
    /// Whether opaque number/date/time pattern styles should be reported.
    pub report_pattern_styles: bool,
}

impl Default for IcuCompatibilityOptions {
    fn default() -> Self {
        Self {
            report_extra_arguments: true,
            report_extra_tags: true,
            report_extra_selectors: true,
            report_pattern_styles: true,
        }
    }
}

impl IcuCompatibilityOptions {
    /// Returns options that enable or disable translation-only argument diagnostics.
    #[must_use]
    pub fn with_report_extra_arguments(mut self, report_extra_arguments: bool) -> Self {
        self.report_extra_arguments = report_extra_arguments;
        self
    }

    /// Returns options that enable or disable translation-only rich-text tag diagnostics.
    #[must_use]
    pub fn with_report_extra_tags(mut self, report_extra_tags: bool) -> Self {
        self.report_extra_tags = report_extra_tags;
        self
    }

    /// Returns options that enable or disable translation-only select selector diagnostics.
    #[must_use]
    pub fn with_report_extra_selectors(mut self, report_extra_selectors: bool) -> Self {
        self.report_extra_selectors = report_extra_selectors;
        self
    }

    /// Returns options that enable or disable opaque number/date/time pattern diagnostics.
    #[must_use]
    pub fn with_report_pattern_styles(mut self, report_pattern_styles: bool) -> Self {
        self.report_pattern_styles = report_pattern_styles;
        self
    }
}

/// One diagnostic emitted by an ICU compatibility check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcuDiagnostic {
    /// Severity for the diagnostic.
    pub severity: IcuDiagnosticSeverity,
    /// Stable machine-readable diagnostic code.
    pub code: DiagnosticCode,
    /// Human-readable explanation of the condition.
    pub message: String,
    /// Argument, selector, or tag name associated with the diagnostic.
    pub name: Option<String>,
}

impl IcuDiagnostic {
    fn new(
        severity: IcuDiagnosticSeverity,
        code: impl Into<DiagnosticCode>,
        message: impl Into<String>,
        name: impl Into<Option<String>>,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
            name: name.into(),
        }
    }
}

/// Report returned by [`compare_icu_messages`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IcuCompatibilityReport {
    /// Diagnostics found while comparing source and translation.
    pub diagnostics: Vec<IcuDiagnostic>,
}

impl IcuCompatibilityReport {
    /// Returns `true` when the report contains at least one error diagnostic.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == IcuDiagnosticSeverity::Error)
    }
}

/// Produces a structural summary of a parsed ICU message.
///
/// The summary preserves first-seen occurrences for data arguments, formatter
/// arguments, plural/select expressions, and rich-text tags. Use it when a tool
/// needs to inspect authoring structure without rendering or normalizing the
/// original message text.
///
/// # Examples
///
/// ```rust
/// use ferrocat_icu::{IcuArgumentKind, IcuStyleKind, analyze_icu, parse_icu};
///
/// let message = parse_icu(
///     "Hi <bold>{name}</bold>, {count, plural, one {# file} other {# files}} \
///      due {due, date, ::yMMMd}",
/// )?;
/// let analysis = analyze_icu(&message);
///
/// assert!(analysis.arguments.iter().any(|arg| arg.name == "name"));
/// assert!(analysis.plurals.iter().any(|plural| plural.name == "count"));
/// assert!(analysis.tags.iter().any(|tag| tag.name == "bold"));
/// assert!(analysis.formatters.iter().any(|formatter| {
///     formatter.name == "due"
///         && formatter.kind == IcuArgumentKind::Date
///         && formatter.style_kind == IcuStyleKind::Skeleton
/// }));
/// # Ok::<(), ferrocat_icu::IcuParseError>(())
/// ```
#[must_use]
pub fn analyze_icu(message: &IcuMessage) -> IcuAnalysis {
    let mut analysis = IcuAnalysis::default();
    visit_nodes(&message.nodes, &mut analysis);
    analysis
}

/// Extracts data argument names in first-seen order, excluding rich-text tags.
#[must_use]
pub fn extract_argument_names(message: &IcuMessage) -> Vec<String> {
    unique_names(analyze_icu(message).arguments.iter().map(|arg| &arg.name))
}

/// Extracts rich-text tag names in first-seen order.
#[must_use]
pub fn extract_tag_names(message: &IcuMessage) -> Vec<String> {
    unique_names(analyze_icu(message).tags.iter().map(|tag| &tag.name))
}

/// Validates ICU formatter usage against a consumer-provided support policy.
///
/// The support callback receives each formatter discovered by [`analyze_icu`]
/// and decides whether the formatter is supported, has an unsupported kind, or
/// has an unsupported style. This keeps runtime-specific support policy outside
/// of `ferrocat-icu` while standardizing emitted diagnostics.
#[must_use]
pub fn validate_icu_formatter_support(
    message: &IcuMessage,
    support: impl FnMut(&IcuFormatter) -> IcuFormatterSupport,
) -> IcuCompatibilityReport {
    let analysis = analyze_icu(message);
    validate_icu_formatter_support_from_analysis(&analysis, support)
}

/// Validates analyzed ICU formatter usage against a consumer support policy.
///
/// Use this when the caller already has an [`IcuAnalysis`] from [`analyze_icu`]
/// and wants to avoid walking the parsed message a second time. The callback is
/// invoked only for formatter arguments such as `number`, `date`, `time`, or
/// `list`; plural and select selector arguments are not formatter callbacks.
#[must_use]
pub fn validate_icu_formatter_support_from_analysis(
    analysis: &IcuAnalysis,
    mut support: impl FnMut(&IcuFormatter) -> IcuFormatterSupport,
) -> IcuCompatibilityReport {
    let mut report = IcuCompatibilityReport::default();

    for formatter in &analysis.formatters {
        match support(formatter) {
            IcuFormatterSupport::Supported => {}
            IcuFormatterSupport::UnsupportedKind { severity } => {
                report.diagnostics.push(IcuDiagnostic::new(
                    severity,
                    diagnostic_codes::icu::UNSUPPORTED_FORMATTER_KIND,
                    format!(
                        "ICU formatter `{}` uses unsupported formatter kind `{}`.",
                        formatter.name, formatter.kind
                    ),
                    Some(formatter.name.clone()),
                ));
            }
            IcuFormatterSupport::UnsupportedStyle { severity } => {
                report.diagnostics.push(IcuDiagnostic::new(
                    severity,
                    diagnostic_codes::icu::UNSUPPORTED_FORMATTER_STYLE,
                    format!(
                        "ICU formatter `{}` uses unsupported `{}` formatter style `{}`.",
                        formatter.name,
                        formatter.kind,
                        formatter_style_label(formatter.style.as_deref())
                    ),
                    Some(formatter.name.clone()),
                ));
            }
        }
    }

    report
}

/// Compares source and translation ICU messages for authoring compatibility.
///
/// The comparison checks whether the translation keeps source-required data
/// arguments, rich-text tags, formatter roles and styles, select selectors, and
/// plural structure. Optional settings control whether translation-only
/// arguments, tags, selectors, and opaque pattern styles are reported.
///
/// # Examples
///
/// ```rust
/// use ferrocat_icu::{IcuCompatibilityOptions, compare_icu_messages, parse_icu};
///
/// let source = parse_icu("Hello {name}, you have <strong>{count}</strong> files")?;
/// let translation = parse_icu("Hallo {name}, du hast {count} Dateien")?;
///
/// let report = compare_icu_messages(&source, &translation, &IcuCompatibilityOptions::default());
///
/// assert!(report.has_errors());
/// assert!(report
///     .diagnostics
///     .iter()
///     .any(|diagnostic| diagnostic.name.as_deref() == Some("strong")));
/// # Ok::<(), ferrocat_icu::IcuParseError>(())
/// ```
#[must_use]
pub fn compare_icu_messages(
    source: &IcuMessage,
    translation: &IcuMessage,
    options: &IcuCompatibilityOptions,
) -> IcuCompatibilityReport {
    let source = analyze_icu(source);
    let translation = analyze_icu(translation);
    let mut report = IcuCompatibilityReport::default();

    compare_arguments(&source, &translation, options, &mut report);
    compare_formatter_styles(&source, &translation, &mut report);
    compare_tags(&source, &translation, options, &mut report);
    compare_selects(&source, &translation, options, &mut report);
    compare_plurals(&source, &translation, &mut report);
    if options.report_pattern_styles {
        report_pattern_styles(&source, "source", &mut report);
        report_pattern_styles(&translation, "translation", &mut report);
    }

    report
}

fn visit_nodes(nodes: &[IcuNode], analysis: &mut IcuAnalysis) {
    for node in nodes {
        match node {
            IcuNode::Literal(_) | IcuNode::Pound => {}
            IcuNode::Argument { name } => {
                push_argument(analysis, name, IcuArgumentKind::Argument);
            }
            IcuNode::Number { name, style } => {
                push_formatter(analysis, name, IcuArgumentKind::Number, style);
            }
            IcuNode::Date { name, style } => {
                push_formatter(analysis, name, IcuArgumentKind::Date, style);
            }
            IcuNode::Time { name, style } => {
                push_formatter(analysis, name, IcuArgumentKind::Time, style);
            }
            IcuNode::List { name, style } => {
                push_formatter(analysis, name, IcuArgumentKind::List, style);
            }
            IcuNode::Duration { name, style } => {
                push_formatter(analysis, name, IcuArgumentKind::Duration, style);
            }
            IcuNode::Ago { name, style } => {
                push_formatter(analysis, name, IcuArgumentKind::Ago, style);
            }
            IcuNode::Name { name, style } => {
                push_formatter(analysis, name, IcuArgumentKind::Name, style);
            }
            IcuNode::Select { name, options } => {
                push_argument(analysis, name, IcuArgumentKind::Select);
                analysis.selects.push(IcuSelectSummary {
                    name: name.clone(),
                    selectors: selectors(options),
                });
                visit_options(options, analysis);
            }
            IcuNode::Plural {
                name,
                kind,
                offset,
                options,
            } => {
                let argument_kind = match kind {
                    IcuPluralKind::Cardinal => IcuArgumentKind::Plural,
                    IcuPluralKind::Ordinal => IcuArgumentKind::SelectOrdinal,
                };
                push_argument(analysis, name, argument_kind);
                analysis.plurals.push(IcuPluralSummary {
                    name: name.clone(),
                    kind: kind.clone(),
                    offset: *offset,
                    selectors: selectors(options),
                });
                visit_options(options, analysis);
            }
            IcuNode::Tag { name, children, .. } => {
                analysis.tags.push(IcuTagSummary { name: name.clone() });
                visit_nodes(children, analysis);
            }
        }
    }
}

fn visit_options(options: &[IcuOption], analysis: &mut IcuAnalysis) {
    for option in options {
        visit_nodes(&option.value, analysis);
    }
}

fn push_argument(analysis: &mut IcuAnalysis, name: &str, kind: IcuArgumentKind) {
    analysis.arguments.push(IcuArgument {
        name: name.to_owned(),
        kind,
    });
}

fn push_formatter(
    analysis: &mut IcuAnalysis,
    name: &str,
    kind: IcuArgumentKind,
    style: &Option<String>,
) {
    push_argument(analysis, name, kind);
    analysis.formatters.push(IcuFormatter {
        name: name.to_owned(),
        kind,
        style: style.clone(),
        style_kind: classify_style(kind, style.as_deref()),
    });
}

fn selectors(options: &[IcuOption]) -> Vec<String> {
    options
        .iter()
        .map(|option| option.selector.clone())
        .collect()
}

fn classify_style(kind: IcuArgumentKind, style: Option<&str>) -> IcuStyleKind {
    let Some(style) = style.map(str::trim).filter(|style| !style.is_empty()) else {
        return IcuStyleKind::None;
    };
    if style.starts_with("::") {
        return IcuStyleKind::Skeleton;
    }
    if is_predefined_style(kind, style) {
        return IcuStyleKind::Predefined;
    }
    IcuStyleKind::Pattern
}

fn is_predefined_style(kind: IcuArgumentKind, style: &str) -> bool {
    match kind {
        IcuArgumentKind::Number => matches!(style, "integer" | "currency" | "percent"),
        IcuArgumentKind::Date | IcuArgumentKind::Time => {
            matches!(style, "short" | "medium" | "long" | "full")
        }
        IcuArgumentKind::List => matches!(style, "conjunction" | "disjunction" | "unit"),
        _ => false,
    }
}

fn unique_names<'a>(names: impl IntoIterator<Item = &'a String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for name in names {
        if seen.insert(name.as_str()) {
            out.push(name.clone());
        }
    }
    out
}

fn argument_map(analysis: &IcuAnalysis) -> BTreeMap<&str, IcuArgumentKind> {
    analysis
        .arguments
        .iter()
        .map(|argument| (argument.name.as_str(), argument.kind))
        .collect()
}

fn formatter_map(analysis: &IcuAnalysis) -> BTreeMap<(&str, IcuArgumentKind), Option<&str>> {
    analysis
        .formatters
        .iter()
        .map(|formatter| {
            (
                (formatter.name.as_str(), formatter.kind),
                formatter.style.as_deref().map(str::trim),
            )
        })
        .collect()
}

fn selector_set(selectors: &[String]) -> BTreeSet<&str> {
    selectors.iter().map(String::as_str).collect()
}

fn formatter_style_label(style: Option<&str>) -> &str {
    style
        .map(str::trim)
        .filter(|style| !style.is_empty())
        .unwrap_or("<none>")
}

fn compare_arguments(
    source: &IcuAnalysis,
    translation: &IcuAnalysis,
    options: &IcuCompatibilityOptions,
    report: &mut IcuCompatibilityReport,
) {
    let source_arguments = argument_map(source);
    let translation_arguments = argument_map(translation);

    for (name, source_kind) in &source_arguments {
        let Some(translation_kind) = translation_arguments.get(name) else {
            report.diagnostics.push(IcuDiagnostic::new(
                IcuDiagnosticSeverity::Error,
                diagnostic_codes::icu::MISSING_ARGUMENT,
                format!("Translation is missing ICU argument `{name}`."),
                Some((*name).to_owned()),
            ));
            continue;
        };
        if source_kind != translation_kind {
            report.diagnostics.push(IcuDiagnostic::new(
                IcuDiagnosticSeverity::Error,
                diagnostic_codes::icu::ARGUMENT_KIND_CHANGED,
                format!(
                    "Translation changes ICU argument `{name}` from {source_kind:?} to {translation_kind:?}."
                ),
                Some((*name).to_owned()),
            ));
        }
    }

    if options.report_extra_arguments {
        for name in translation_arguments.keys() {
            if !source_arguments.contains_key(name) {
                report.diagnostics.push(IcuDiagnostic::new(
                    IcuDiagnosticSeverity::Error,
                    diagnostic_codes::icu::EXTRA_ARGUMENT,
                    format!(
                        "Translation adds ICU argument `{name}` that is not present in source."
                    ),
                    Some((*name).to_owned()),
                ));
            }
        }
    }
}

fn compare_formatter_styles(
    source: &IcuAnalysis,
    translation: &IcuAnalysis,
    report: &mut IcuCompatibilityReport,
) {
    let source_formatters = formatter_map(source);
    let translation_formatters = formatter_map(translation);

    for ((name, kind), source_style) in source_formatters {
        let Some(translation_style) = translation_formatters.get(&(name, kind)) else {
            continue;
        };
        if source_style != *translation_style {
            report.diagnostics.push(IcuDiagnostic::new(
                IcuDiagnosticSeverity::Error,
                diagnostic_codes::icu::FORMATTER_STYLE_CHANGED,
                format!(
                    "Translation changes ICU formatter style for `{name}` from {source_style:?} to {translation_style:?}."
                ),
                Some(name.to_owned()),
            ));
        }
    }
}

fn tag_counts(analysis: &IcuAnalysis) -> BTreeMap<&str, usize> {
    let mut counts = BTreeMap::new();
    for tag in &analysis.tags {
        *counts.entry(tag.name.as_str()).or_insert(0) += 1;
    }
    counts
}

fn compare_tags(
    source: &IcuAnalysis,
    translation: &IcuAnalysis,
    options: &IcuCompatibilityOptions,
    report: &mut IcuCompatibilityReport,
) {
    let source_tags = tag_counts(source);
    let translation_tags = tag_counts(translation);
    for (name, source_count) in &source_tags {
        let translation_count = translation_tags.get(name).copied().unwrap_or_default();
        if translation_count < *source_count {
            report.diagnostics.push(IcuDiagnostic::new(
                IcuDiagnosticSeverity::Error,
                diagnostic_codes::icu::MISSING_TAG,
                format!("Translation is missing ICU tag `{name}`."),
                Some((*name).to_owned()),
            ));
        }
    }
    if options.report_extra_tags {
        for (name, translation_count) in &translation_tags {
            let source_count = source_tags.get(name).copied().unwrap_or_default();
            if *translation_count > source_count {
                report.diagnostics.push(IcuDiagnostic::new(
                    IcuDiagnosticSeverity::Error,
                    diagnostic_codes::icu::EXTRA_TAG,
                    format!("Translation adds ICU tag `{name}` that is not present in source."),
                    Some((*name).to_owned()),
                ));
            }
        }
    }
}

fn select_map(analysis: &IcuAnalysis) -> BTreeMap<&str, BTreeSet<&str>> {
    let mut out = BTreeMap::<&str, BTreeSet<&str>>::new();
    for select in &analysis.selects {
        out.entry(select.name.as_str())
            .or_default()
            .extend(selector_set(&select.selectors));
    }
    out
}

fn compare_selects(
    source: &IcuAnalysis,
    translation: &IcuAnalysis,
    options: &IcuCompatibilityOptions,
    report: &mut IcuCompatibilityReport,
) {
    let source_selects = select_map(source);
    let translation_selects = select_map(translation);
    for (name, source_selectors) in &source_selects {
        let Some(translation_selectors) = translation_selects.get(name) else {
            continue;
        };
        for &selector in source_selectors {
            if !translation_selectors.contains(selector) {
                report.diagnostics.push(IcuDiagnostic::new(
                    IcuDiagnosticSeverity::Error,
                    diagnostic_codes::icu::MISSING_SELECT_SELECTOR,
                    format!(
                        "Translation is missing ICU select selector `{selector}` for `{name}`."
                    ),
                    Some(format!("{name}:{selector}")),
                ));
            }
        }
        if options.report_extra_selectors {
            for &selector in translation_selectors {
                if !source_selectors.contains(selector) {
                    report.diagnostics.push(IcuDiagnostic::new(
                        IcuDiagnosticSeverity::Warning,
                        diagnostic_codes::icu::EXTRA_SELECT_SELECTOR,
                        format!("Translation adds ICU select selector `{selector}` for `{name}`."),
                        Some(format!("{name}:{selector}")),
                    ));
                }
            }
        }
    }
}

fn plural_key(plural: &IcuPluralSummary) -> (&str, IcuPluralKind) {
    (plural.name.as_str(), plural.kind.clone())
}

fn plural_map(analysis: &IcuAnalysis) -> BTreeMap<(&str, IcuPluralKind), &IcuPluralSummary> {
    analysis
        .plurals
        .iter()
        .map(|plural| (plural_key(plural), plural))
        .collect()
}

fn compare_plurals(
    source: &IcuAnalysis,
    translation: &IcuAnalysis,
    report: &mut IcuCompatibilityReport,
) {
    let source_plurals = plural_map(source);
    let translation_plurals = plural_map(translation);

    for (key, source_plural) in source_plurals {
        let Some(translation_plural) = translation_plurals.get(&key) else {
            continue;
        };
        if source_plural.offset != translation_plural.offset {
            report.diagnostics.push(IcuDiagnostic::new(
                IcuDiagnosticSeverity::Error,
                diagnostic_codes::icu::PLURAL_OFFSET_CHANGED,
                format!(
                    "Translation changes ICU plural offset for `{}` from {} to {}.",
                    source_plural.name, source_plural.offset, translation_plural.offset
                ),
                Some(source_plural.name.clone()),
            ));
        }

        let translation_selectors = selector_set(&translation_plural.selectors);
        for selector in &source_plural.selectors {
            let selector = selector.as_str();
            if !translation_selectors.contains(selector) {
                report.diagnostics.push(IcuDiagnostic::new(
                    IcuDiagnosticSeverity::Error,
                    diagnostic_codes::icu::MISSING_PLURAL_SELECTOR,
                    format!(
                        "Translation is missing ICU plural selector `{selector}` for `{}`.",
                        source_plural.name
                    ),
                    Some(format!("{}:{selector}", source_plural.name)),
                ));
            }
        }
    }
}

fn report_pattern_styles(analysis: &IcuAnalysis, label: &str, report: &mut IcuCompatibilityReport) {
    for formatter in &analysis.formatters {
        if !matches!(
            formatter.kind,
            IcuArgumentKind::Number | IcuArgumentKind::Date | IcuArgumentKind::Time
        ) || formatter.style_kind != IcuStyleKind::Pattern
        {
            continue;
        }
        report.diagnostics.push(IcuDiagnostic::new(
            IcuDiagnosticSeverity::Warning,
            diagnostic_codes::icu::PATTERN_STYLE_DISCOURAGED,
            format!(
                "{label} ICU formatter `{}` uses an opaque pattern style; prefer a predefined style or `::` skeleton.",
                formatter.name
            ),
            Some(formatter.name.clone()),
        ));
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        IcuArgumentKind, IcuCompatibilityOptions, IcuDiagnosticSeverity, IcuFormatterSupport,
        IcuStyleKind, analyze_icu, compare_icu_messages, extract_argument_names, extract_tag_names,
        parse_icu, validate_icu_formatter_support, validate_icu_formatter_support_from_analysis,
    };

    #[test]
    fn analysis_separates_arguments_formatters_plurals_selects_and_tags() {
        let message = parse_icu(
            "<link>{gender, select, other {{count, plural, one {{when, date, short}} other {{count, number, ::compact-short}}}}}</link>",
        )
        .expect("parse");
        let analysis = analyze_icu(&message);

        assert_eq!(
            extract_argument_names(&message),
            vec!["gender", "count", "when"]
        );
        assert_eq!(extract_tag_names(&message), vec!["link"]);
        assert_eq!(analysis.tags.len(), 1);
        assert_eq!(analysis.selects.len(), 1);
        assert_eq!(analysis.plurals.len(), 1);
        assert_eq!(analysis.formatters.len(), 2);
        assert!(
            analysis
                .arguments
                .iter()
                .any(|argument| argument.kind == IcuArgumentKind::Select)
        );
    }

    #[test]
    fn analysis_surfaces_self_closing_tag_names() {
        let message = parse_icu("line one<0/>line two").expect("parse");
        let analysis = analyze_icu(&message);

        assert_eq!(extract_tag_names(&message), vec!["0"]);
        assert_eq!(analysis.tags.len(), 1);
    }

    #[test]
    fn analysis_classifies_formatter_styles() {
        let message = parse_icu(
            "{n, number, integer} {total, number, ::currency/USD} {created, date, yyyy-MM-dd} {items, list, disjunction}",
        )
        .expect("parse");
        let analysis = analyze_icu(&message);
        let kinds = analysis
            .formatters
            .iter()
            .map(|formatter| formatter.style_kind)
            .collect::<Vec<_>>();

        assert_eq!(
            kinds,
            vec![
                IcuStyleKind::Predefined,
                IcuStyleKind::Skeleton,
                IcuStyleKind::Pattern,
                IcuStyleKind::Predefined
            ]
        );
    }

    #[test]
    fn compatibility_reports_argument_formatter_tag_and_selector_mismatches() {
        let source = parse_icu(
            "<link>{gender, select, male {{count, number, integer} for {user}} female {{count, number, integer}} other {{count, number, integer}}}</link>",
        )
        .expect("parse source");
        let translation = parse_icu(
            "<b>{gender, select, male {{count}} other {{total, number, integer}} extra {{count}}}</b>",
        )
        .expect("parse translation");

        let report =
            compare_icu_messages(&source, &translation, &IcuCompatibilityOptions::default());
        let codes = report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>();

        assert!(report.has_errors());
        assert!(codes.contains(&"icu.missing_argument"));
        assert!(codes.contains(&"icu.extra_argument"));
        assert!(codes.contains(&"icu.argument_kind_changed"));
        assert!(codes.contains(&"icu.missing_tag"));
        assert!(codes.contains(&"icu.extra_tag"));
        assert!(codes.contains(&"icu.missing_select_selector"));
        assert!(codes.contains(&"icu.extra_select_selector"));
    }

    #[test]
    fn compatibility_allows_extra_plural_categories_but_reports_offset_changes() {
        let source =
            parse_icu("{count, plural, offset:1 one {# file} other {# files}}").expect("source");
        let translation =
            parse_icu("{count, plural, offset:2 one {# Datei} few {# Dateien} other {# Dateien}}")
                .expect("translation");

        let report =
            compare_icu_messages(&source, &translation, &IcuCompatibilityOptions::default());

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "icu.plural_offset_changed")
        );
        assert!(
            !report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "icu.extra_plural_selector")
        );
    }

    #[test]
    fn compatibility_reports_discouraged_pattern_styles_as_warnings() {
        let source = parse_icu("{created, date, yyyy-MM-dd}").expect("source");
        let translation = parse_icu("{created, date, yyyy-MM-dd}").expect("translation");

        let report =
            compare_icu_messages(&source, &translation, &IcuCompatibilityOptions::default());

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "icu.pattern_style_discouraged"
                && diagnostic.severity == IcuDiagnosticSeverity::Warning
        }));
    }

    #[test]
    fn formatter_support_validation_allows_supported_formatters() {
        let message = parse_icu("{total, number, percent} {created, date, short}").expect("parse");

        let report = validate_icu_formatter_support(&message, |_| IcuFormatterSupport::Supported);

        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn formatter_support_validation_reports_unsupported_kinds() {
        let message = parse_icu("{items, list, conjunction}").expect("parse");

        let report = validate_icu_formatter_support(&message, |formatter| {
            if formatter.kind == IcuArgumentKind::List {
                IcuFormatterSupport::UnsupportedKind {
                    severity: IcuDiagnosticSeverity::Error,
                }
            } else {
                IcuFormatterSupport::Supported
            }
        });

        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].code, "icu.unsupported_formatter_kind");
        assert_eq!(
            report.diagnostics[0].message,
            "ICU formatter `items` uses unsupported formatter kind `list`."
        );
        assert_eq!(report.diagnostics[0].severity, IcuDiagnosticSeverity::Error);
        assert_eq!(report.diagnostics[0].name.as_deref(), Some("items"));
    }

    #[test]
    fn formatter_support_validation_reports_unsupported_styles() {
        let message = parse_icu("{total, number, ::compact-short}").expect("parse");

        let report = validate_icu_formatter_support(&message, |formatter| {
            if formatter.style.as_deref() == Some("::compact-short") {
                IcuFormatterSupport::UnsupportedStyle {
                    severity: IcuDiagnosticSeverity::Warning,
                }
            } else {
                IcuFormatterSupport::Supported
            }
        });

        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(
            report.diagnostics[0].code,
            "icu.unsupported_formatter_style"
        );
        assert_eq!(
            report.diagnostics[0].message,
            "ICU formatter `total` uses unsupported `number` formatter style `::compact-short`."
        );
        assert_eq!(
            report.diagnostics[0].severity,
            IcuDiagnosticSeverity::Warning
        );
        assert_eq!(report.diagnostics[0].name.as_deref(), Some("total"));
    }

    #[test]
    fn icu_argument_kind_display_uses_icu_formatter_names() {
        let labels = [
            IcuArgumentKind::Argument,
            IcuArgumentKind::Number,
            IcuArgumentKind::Date,
            IcuArgumentKind::Time,
            IcuArgumentKind::List,
            IcuArgumentKind::Duration,
            IcuArgumentKind::Ago,
            IcuArgumentKind::Name,
            IcuArgumentKind::Select,
            IcuArgumentKind::Plural,
            IcuArgumentKind::SelectOrdinal,
        ]
        .into_iter()
        .map(|kind| kind.to_string())
        .collect::<Vec<_>>();

        assert_eq!(
            labels,
            vec![
                "argument",
                "number",
                "date",
                "time",
                "list",
                "duration",
                "ago",
                "name",
                "select",
                "plural",
                "selectordinal",
            ]
        );
    }

    #[test]
    fn formatter_support_validation_reports_mixed_kind_and_style_decisions() {
        let message = parse_icu(
            "{total, number, ::compact-short} {created, date} {started, time, short} {items, list, conjunction}",
        )
        .expect("parse");

        let report = validate_icu_formatter_support(&message, |formatter| match formatter.kind {
            IcuArgumentKind::Number if formatter.style_kind == IcuStyleKind::Skeleton => {
                IcuFormatterSupport::UnsupportedStyle {
                    severity: IcuDiagnosticSeverity::Warning,
                }
            }
            IcuArgumentKind::Date if formatter.style.is_none() => {
                IcuFormatterSupport::UnsupportedStyle {
                    severity: IcuDiagnosticSeverity::Error,
                }
            }
            IcuArgumentKind::List => IcuFormatterSupport::UnsupportedKind {
                severity: IcuDiagnosticSeverity::Error,
            },
            _ => IcuFormatterSupport::Supported,
        });

        assert_eq!(
            report
                .diagnostics
                .iter()
                .map(|diagnostic| {
                    (
                        diagnostic.code.as_str(),
                        diagnostic.severity,
                        diagnostic.name.as_deref(),
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                (
                    "icu.unsupported_formatter_style",
                    IcuDiagnosticSeverity::Warning,
                    Some("total")
                ),
                (
                    "icu.unsupported_formatter_style",
                    IcuDiagnosticSeverity::Error,
                    Some("created")
                ),
                (
                    "icu.unsupported_formatter_kind",
                    IcuDiagnosticSeverity::Error,
                    Some("items")
                ),
            ]
        );
    }

    #[test]
    fn formatter_support_validation_accepts_precomputed_analysis() {
        let message = parse_icu("{total, number, integer} {created, date, short}").expect("parse");
        let analysis = analyze_icu(&message);

        let report = validate_icu_formatter_support_from_analysis(&analysis, |_| {
            IcuFormatterSupport::Supported
        });

        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn formatter_support_validation_does_not_visit_plural_or_select_arguments() {
        let message = parse_icu(
            "{gender, select, other {{count, plural, one {# file} other {# files}}}} {created, date, short}",
        )
        .expect("parse");
        let analysis = analyze_icu(&message);
        let mut visited = Vec::new();

        let report = validate_icu_formatter_support_from_analysis(&analysis, |formatter| {
            visited.push((formatter.name.clone(), formatter.kind));
            IcuFormatterSupport::Supported
        });

        assert!(report.diagnostics.is_empty());
        assert_eq!(visited, vec![("created".to_owned(), IcuArgumentKind::Date)]);
    }

    #[test]
    fn compatibility_option_builders_set_fields() {
        let options = IcuCompatibilityOptions::default()
            .with_report_extra_arguments(false)
            .with_report_extra_tags(false)
            .with_report_extra_selectors(false)
            .with_report_pattern_styles(false);

        assert!(!options.report_extra_arguments);
        assert!(!options.report_extra_tags);
        assert!(!options.report_extra_selectors);
        assert!(!options.report_pattern_styles);
    }
}
