use core::fmt;

/// High-level classification of ICU parse failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcuErrorKind {
    /// The input violates the supported ICU syntax.
    SyntaxError,
}

/// Byte offset plus line/column location inside the original input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IcuPosition {
    /// Zero-based byte offset from the start of the input.
    pub offset: usize,
    /// One-based line number.
    pub line: usize,
    /// One-based column number.
    pub column: usize,
}

/// Error returned when parsing ICU messages fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcuParseError {
    /// High-level failure kind.
    pub kind: IcuErrorKind,
    /// Human-readable parser error message.
    pub message: String,
    /// Source location for the parser failure.
    pub position: IcuPosition,
}

impl IcuParseError {
    /// Creates a syntax error at `offset` within `input`.
    #[must_use]
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

fn position_for_offset(input: &str, offset: usize) -> IcuPosition {
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
