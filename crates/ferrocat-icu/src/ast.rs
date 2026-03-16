#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IcuMessage {
    pub nodes: Vec<IcuNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IcuPluralKind {
    Cardinal,
    Ordinal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcuOption {
    pub selector: String,
    pub value: Vec<IcuNode>,
}

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
