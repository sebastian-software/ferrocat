//! Compact, performance-oriented ICU parsing primitives.

use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IcuMessage {
    pub nodes: Vec<IcuNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IcuNode {
    Literal(String),
    Argument { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcuParseError {
    message: String,
}

impl IcuParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for IcuParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for IcuParseError {}

pub fn parse_icu(_input: &str) -> Result<IcuMessage, IcuParseError> {
    Err(IcuParseError::new(
        "parse_icu is not implemented yet; the ICU AST and parser land in a later milestone",
    ))
}
