/// Parsed ICU message.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IcuMessage {
    pub nodes: Vec<IcuNode>,
}

/// Distinguishes cardinal and ordinal plural forms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IcuPluralKind {
    Cardinal,
    Ordinal,
}

/// A selector branch inside a plural or select expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcuOption {
    pub selector: String,
    pub value: Vec<IcuNode>,
}

/// AST node emitted by the ICU parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IcuNode {
    Literal(String),
    Argument {
        name: String,
    },
    Number {
        name: String,
        style: Option<String>,
    },
    Date {
        name: String,
        style: Option<String>,
    },
    Time {
        name: String,
        style: Option<String>,
    },
    List {
        name: String,
        style: Option<String>,
    },
    Duration {
        name: String,
        style: Option<String>,
    },
    Ago {
        name: String,
        style: Option<String>,
    },
    Name {
        name: String,
        style: Option<String>,
    },
    Select {
        name: String,
        options: Vec<IcuOption>,
    },
    Plural {
        name: String,
        kind: IcuPluralKind,
        offset: u32,
        options: Vec<IcuOption>,
    },
    Pound,
    Tag {
        name: String,
        children: Vec<IcuNode>,
    },
}
