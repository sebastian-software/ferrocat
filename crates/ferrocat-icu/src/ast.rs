/// Parsed ICU message.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IcuMessage {
    /// Top-level AST nodes in source order.
    pub nodes: Vec<IcuNode>,
}

/// Distinguishes cardinal and ordinal plural forms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IcuPluralKind {
    /// Cardinal plural categories such as `one` and `other`.
    Cardinal,
    /// Ordinal plural categories such as `one`, `two`, `few`, and `other`.
    Ordinal,
}

/// A selector branch inside a plural or select expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcuOption {
    /// Raw selector label such as `one`, `other`, or `=0`.
    pub selector: String,
    /// Nested nodes rendered when the selector matches.
    pub value: Vec<IcuNode>,
}

/// AST node emitted by the ICU parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IcuNode {
    /// Literal text content.
    Literal(String),
    /// Simple argument substitution such as `{name}`.
    Argument {
        /// Argument identifier.
        name: String,
    },
    /// Number formatter such as `{count, number}`.
    Number {
        /// Argument identifier.
        name: String,
        /// Optional formatter style segment.
        style: Option<String>,
    },
    /// Date formatter such as `{createdAt, date, short}`.
    Date {
        /// Argument identifier.
        name: String,
        /// Optional formatter style segment.
        style: Option<String>,
    },
    /// Time formatter such as `{createdAt, time, short}`.
    Time {
        /// Argument identifier.
        name: String,
        /// Optional formatter style segment.
        style: Option<String>,
    },
    /// List formatter such as `{items, list}`.
    List {
        /// Argument identifier.
        name: String,
        /// Optional formatter style segment.
        style: Option<String>,
    },
    /// Duration formatter such as `{elapsed, duration}`.
    Duration {
        /// Argument identifier.
        name: String,
        /// Optional formatter style segment.
        style: Option<String>,
    },
    /// Relative-time "ago" formatter.
    Ago {
        /// Argument identifier.
        name: String,
        /// Optional formatter style segment.
        style: Option<String>,
    },
    /// Name formatter.
    Name {
        /// Argument identifier.
        name: String,
        /// Optional formatter style segment.
        style: Option<String>,
    },
    /// Select expression with labeled branches.
    Select {
        /// Argument identifier.
        name: String,
        /// Available selector branches.
        options: Vec<IcuOption>,
    },
    /// Cardinal or ordinal plural expression.
    Plural {
        /// Argument identifier.
        name: String,
        /// Whether the plural expression is cardinal or ordinal.
        kind: IcuPluralKind,
        /// Parsed plural offset value.
        offset: u32,
        /// Available plural branches.
        options: Vec<IcuOption>,
    },
    /// `#` placeholder inside plural branches.
    Pound,
    /// Rich-text style tag with nested children.
    Tag {
        /// Tag name without angle brackets.
        name: String,
        /// Nested child nodes inside the tag body.
        children: Vec<Self>,
    },
}
