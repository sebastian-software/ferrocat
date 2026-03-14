//! Performance-first PO parsing and serialization.

mod parse;
mod serialize;
mod text;

pub use parse::parse_po;
pub use serialize::stringify_po;
pub use text::{escape_string, extract_quoted, unescape_string};

use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PoFile {
    pub comments: Vec<String>,
    pub extracted_comments: Vec<String>,
    pub headers: Vec<Header>,
    pub items: Vec<PoItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Header {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PoItem {
    pub msgid: String,
    pub msgctxt: Option<String>,
    pub references: Vec<String>,
    pub msgid_plural: Option<String>,
    pub msgstr: Vec<String>,
    pub comments: Vec<String>,
    pub extracted_comments: Vec<String>,
    pub flags: Vec<String>,
    pub metadata: Vec<(String, String)>,
    pub obsolete: bool,
    pub nplurals: usize,
}

impl PoItem {
    pub fn new(nplurals: usize) -> Self {
        Self {
            nplurals,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializeOptions {
    pub fold_length: usize,
    pub compact_multiline: bool,
}

impl Default for SerializeOptions {
    fn default() -> Self {
        Self {
            fold_length: 80,
            compact_multiline: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    message: String,
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}
