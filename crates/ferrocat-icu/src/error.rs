use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcuErrorKind {
    SyntaxError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IcuPosition {
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcuParseError {
    pub kind: IcuErrorKind,
    pub message: String,
    pub position: IcuPosition,
}

impl IcuParseError {
    pub fn syntax(message: impl Into<String>, input: &str, offset: usize) -> Self {
        Self {
            kind: IcuErrorKind::SyntaxError,
            message: message.into(),
            position: position_for_offset(input, offset),
        }
    }
}

impl fmt::Display for IcuParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at line {}, column {}",
            self.message, self.position.line, self.position.column
        )
    }
}

impl std::error::Error for IcuParseError {}

pub(crate) fn position_for_offset(input: &str, offset: usize) -> IcuPosition {
    let clamped = offset.min(input.len());
    let mut line = 1usize;
    let mut column = 1usize;

    for ch in input[..clamped].chars() {
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    IcuPosition {
        offset: clamped,
        line,
        column,
    }
}
