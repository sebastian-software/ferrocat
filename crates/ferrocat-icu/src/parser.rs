use std::borrow::Cow;

use crate::ast::{IcuMessage, IcuNode, IcuOption, IcuPluralKind};
use crate::error::IcuParseError;

// Bounds recursive option and tag bodies, including mixed nesting.
const MAX_NESTING_DEPTH: usize = 64;

/// Options controlling ICU parsing behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct IcuParserOptions {
    /// When `true`, rich-text style tags are treated as plain text.
    pub ignore_tag: bool,
    /// When `true`, select and plural arguments must include an `other` clause.
    pub requires_other_clause: bool,
}

impl Default for IcuParserOptions {
    fn default() -> Self {
        Self {
            ignore_tag: false,
            requires_other_clause: true,
        }
    }
}

impl IcuParserOptions {
    /// Returns options that enable or disable rich-text tag parsing.
    #[must_use]
    pub fn with_ignore_tag(mut self, ignore_tag: bool) -> Self {
        self.ignore_tag = ignore_tag;
        self
    }

    /// Returns options that enable or disable required `other` clauses.
    #[must_use]
    pub fn with_requires_other_clause(mut self, requires_other_clause: bool) -> Self {
        self.requires_other_clause = requires_other_clause;
        self
    }
}

/// Parses ICU `MessageFormat` input with the default parser options.
///
/// # Errors
///
/// Returns [`IcuParseError`] when the input is malformed or exceeds 64 nested
/// selector option bodies and paired tags.
pub fn parse_icu(input: &str) -> Result<IcuMessage, IcuParseError> {
    parse_icu_with_options(input, &IcuParserOptions::default())
}

/// Parses ICU `MessageFormat` input with explicit parser options.
///
/// # Errors
///
/// Returns [`IcuParseError`] when the input is malformed or exceeds 64 nested
/// selector option bodies and paired tags.
pub fn parse_icu_with_options(
    input: &str,
    options: &IcuParserOptions,
) -> Result<IcuMessage, IcuParseError> {
    if input.is_empty() {
        return Ok(IcuMessage { nodes: Vec::new() });
    }
    if is_plain_literal(input.as_bytes(), options) {
        return Ok(IcuMessage {
            nodes: vec![IcuNode::Literal(input.to_owned())],
        });
    }
    let mut parser = Parser::new(input, options);
    let nodes = parser.parse_nodes(None, 0)?;
    if !parser.is_eof() {
        return Err(parser.error("Unexpected trailing input"));
    }
    Ok(IcuMessage { nodes })
}

/// Validates ICU `MessageFormat` input without allocating an owned AST.
pub(crate) fn validate_icu_with_options(
    input: &str,
    options: &IcuParserOptions,
) -> Result<(), IcuParseError> {
    if input.is_empty() || is_plain_literal(input.as_bytes(), options) {
        return Ok(());
    }
    let mut parser = Parser::new(input, options);
    parser.validate_nodes(None, 0)?;
    if !parser.is_eof() {
        return Err(parser.error("Unexpected trailing input"));
    }
    Ok(())
}

#[inline]
fn is_plain_literal(bytes: &[u8], options: &IcuParserOptions) -> bool {
    if options.ignore_tag {
        memchr::memchr3(b'{', b'}', b'\'', bytes).is_none()
    } else {
        find_literal_stop(bytes, false).is_none()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BorrowedSelector<'a> {
    Exact(u32),
    Keyword(&'a str),
}

struct Parser<'a> {
    input: &'a str,
    input_bytes: &'a [u8],
    pos: usize,
    options: &'a IcuParserOptions,
    nesting_depth: usize,
}

impl<'a> Parser<'a> {
    const OFFSET_PREFIX: &'static [u8] = b"offset:";
    const CLOSE_TAG_PREFIX: &'static [u8] = b"</";
    const SELF_CLOSING_TAG_SUFFIX: &'static [u8] = b"/>";

    const fn new(input: &'a str, options: &'a IcuParserOptions) -> Self {
        Self {
            input,
            input_bytes: input.as_bytes(),
            pos: 0,
            options,
            nesting_depth: 0,
        }
    }

    fn parse_nodes(
        &mut self,
        until_tag: Option<&str>,
        plural_depth: usize,
    ) -> Result<Vec<IcuNode>, IcuParseError> {
        let mut nodes = Vec::new();
        let mut literal = String::new();

        while let Some(byte) = self.byte_at() {
            if byte == b'}' {
                break;
            }

            if let Some(tag_name) = until_tag {
                if self.starts_with_close_tag(tag_name) {
                    break;
                }
                if !self.options.ignore_tag && self.peek_close_tag() {
                    return Err(self.error("Mismatched closing tag"));
                }
            } else if !self.options.ignore_tag && self.peek_close_tag() {
                return Err(self.error("Unexpected closing tag"));
            }

            match byte {
                b'{' => {
                    Self::flush_literal(&mut literal, &mut nodes);
                    nodes.push(self.parse_argument(plural_depth)?);
                }
                b'<' if !self.options.ignore_tag && self.peek_open_tag() => {
                    Self::flush_literal(&mut literal, &mut nodes);
                    nodes.push(self.parse_tag(plural_depth)?);
                }
                b'#' if plural_depth > 0 => {
                    Self::flush_literal(&mut literal, &mut nodes);
                    self.pos += 1;
                    nodes.push(IcuNode::Pound);
                }
                b'\'' => self.parse_apostrophe_literal_into(&mut literal)?,
                b'<' => {
                    // A `<` that is not an open tag (close tags are handled above):
                    // copy it as a literal byte and continue scanning.
                    literal.push('<');
                    self.pos += 1;
                }
                _ => {
                    // Bulk-copy the literal run up to the next syntactically significant
                    // byte (memchr/SIMD jump) instead of decoding+copying char by char.
                    let rest = &self.input_bytes[self.pos..];
                    let stop = find_literal_stop(rest, plural_depth > 0)
                        .unwrap_or(rest.len())
                        .max(1);
                    literal.push_str(&self.input[self.pos..self.pos + stop]);
                    self.pos += stop;
                }
            }
        }

        Self::flush_literal(&mut literal, &mut nodes);
        Ok(nodes)
    }

    fn validate_nodes(
        &mut self,
        until_tag: Option<&'a str>,
        plural_depth: usize,
    ) -> Result<(), IcuParseError> {
        while let Some(byte) = self.byte_at() {
            if byte == b'}' {
                break;
            }

            if let Some(tag_name) = until_tag {
                if self.starts_with_close_tag(tag_name) {
                    break;
                }
                if !self.options.ignore_tag && self.peek_close_tag() {
                    return Err(self.error("Mismatched closing tag"));
                }
            } else if !self.options.ignore_tag && self.peek_close_tag() {
                return Err(self.error("Unexpected closing tag"));
            }

            match byte {
                b'{' => self.validate_argument(plural_depth)?,
                b'<' if !self.options.ignore_tag && self.peek_open_tag() => {
                    self.validate_tag(plural_depth)?;
                }
                b'#' if plural_depth > 0 => {
                    self.pos += 1;
                }
                b'\'' => self.skip_apostrophe_literal()?,
                b'<' => {
                    self.pos += 1;
                }
                _ => {
                    let rest = &self.input_bytes[self.pos..];
                    let stop = find_literal_stop(rest, plural_depth > 0)
                        .unwrap_or(rest.len())
                        .max(1);
                    self.pos += stop;
                }
            }
        }

        Ok(())
    }

    fn parse_nested_nodes(
        &mut self,
        until_tag: Option<&str>,
        plural_depth: usize,
    ) -> Result<Vec<IcuNode>, IcuParseError> {
        if self.nesting_depth >= MAX_NESTING_DEPTH {
            return Err(self.error(format!(
                "ICU message exceeds maximum nesting depth of {MAX_NESTING_DEPTH}"
            )));
        }
        self.nesting_depth += 1;
        let result = self.parse_nodes(until_tag, plural_depth);
        self.nesting_depth -= 1;
        result
    }

    fn validate_nested_nodes(
        &mut self,
        until_tag: Option<&'a str>,
        plural_depth: usize,
    ) -> Result<(), IcuParseError> {
        if self.nesting_depth >= MAX_NESTING_DEPTH {
            return Err(self.error(format!(
                "ICU message exceeds maximum nesting depth of {MAX_NESTING_DEPTH}"
            )));
        }
        self.nesting_depth += 1;
        let result = self.validate_nodes(until_tag, plural_depth);
        self.nesting_depth -= 1;
        result
    }

    fn parse_argument(&mut self, plural_depth: usize) -> Result<IcuNode, IcuParseError> {
        self.expect_char('{')?;
        self.skip_whitespace();
        let name = self.parse_identifier_str()?.to_owned();
        self.skip_whitespace();

        if self.consume_char('}') {
            return Ok(IcuNode::Argument { name });
        }

        self.expect_char(',')?;
        self.skip_whitespace();
        let kind = self.parse_identifier_str()?;
        self.skip_whitespace();

        match kind {
            "number" => self.parse_simple_formatter(name, FormatterKind::Number),
            "date" => self.parse_simple_formatter(name, FormatterKind::Date),
            "time" => self.parse_simple_formatter(name, FormatterKind::Time),
            "list" => self.parse_simple_formatter(name, FormatterKind::List),
            "duration" => self.parse_simple_formatter(name, FormatterKind::Duration),
            "ago" => self.parse_simple_formatter(name, FormatterKind::Ago),
            "name" => self.parse_simple_formatter(name, FormatterKind::Name),
            "select" => self.parse_select(name, plural_depth),
            "plural" => self.parse_plural(name, plural_depth, IcuPluralKind::Cardinal),
            "selectordinal" => self.parse_plural(name, plural_depth, IcuPluralKind::Ordinal),
            _ => Err(self.error("Unsupported ICU argument type")),
        }
    }

    fn validate_argument(&mut self, plural_depth: usize) -> Result<(), IcuParseError> {
        self.expect_char('{')?;
        self.skip_whitespace();
        let _name = self.parse_identifier_str()?;
        self.skip_whitespace();

        if self.consume_char('}') {
            return Ok(());
        }

        self.expect_char(',')?;
        self.skip_whitespace();
        let kind = self.parse_identifier_str()?;
        self.skip_whitespace();

        match kind {
            "number" | "date" | "time" | "list" | "duration" | "ago" | "name" => {
                if self.consume_char(',') {
                    self.skip_until_closing_brace()?;
                }
                self.expect_char('}')?;
                Ok(())
            }
            "select" => {
                if self.consume_char(',') {
                    self.skip_whitespace();
                }
                let has_other = self.validate_options(plural_depth)?;
                if self.options.requires_other_clause && !has_other {
                    return Err(self.error("Select argument requires an \"other\" clause"));
                }
                self.expect_char('}')?;
                Ok(())
            }
            "plural" | "selectordinal" => {
                if self.consume_char(',') {
                    self.skip_whitespace();
                }
                self.skip_whitespace();
                if self.starts_with_bytes(Self::OFFSET_PREFIX) {
                    self.pos += Self::OFFSET_PREFIX.len();
                    self.skip_whitespace();
                    let _offset = self.parse_unsigned_int()?;
                    self.skip_whitespace();
                    if self.starts_with_bytes(Self::OFFSET_PREFIX) {
                        return Err(self.error("Duplicate plural offset"));
                    }
                }
                let has_other = self.validate_options(plural_depth + 1)?;
                if self.options.requires_other_clause && !has_other {
                    return Err(self.error("Plural argument requires an \"other\" clause"));
                }
                self.expect_char('}')?;
                Ok(())
            }
            _ => Err(self.error("Unsupported ICU argument type")),
        }
    }

    fn parse_simple_formatter(
        &mut self,
        name: String,
        kind: FormatterKind,
    ) -> Result<IcuNode, IcuParseError> {
        let style = if self.consume_char(',') {
            let raw_style = self.read_until_closing_brace()?;
            let trimmed = raw_style.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_owned())
        } else {
            None
        };
        self.expect_char('}')?;

        Ok(match kind {
            FormatterKind::Number => IcuNode::Number { name, style },
            FormatterKind::Date => IcuNode::Date { name, style },
            FormatterKind::Time => IcuNode::Time { name, style },
            FormatterKind::List => IcuNode::List { name, style },
            FormatterKind::Duration => IcuNode::Duration { name, style },
            FormatterKind::Ago => IcuNode::Ago { name, style },
            FormatterKind::Name => IcuNode::Name { name, style },
        })
    }

    fn parse_select(
        &mut self,
        name: String,
        plural_depth: usize,
    ) -> Result<IcuNode, IcuParseError> {
        if self.consume_char(',') {
            self.skip_whitespace();
        }
        let options = self.parse_options(plural_depth)?;
        if self.options.requires_other_clause && !has_other_clause(&options) {
            return Err(self.error("Select argument requires an \"other\" clause"));
        }
        self.expect_char('}')?;
        Ok(IcuNode::Select { name, options })
    }

    fn parse_plural(
        &mut self,
        name: String,
        plural_depth: usize,
        kind: IcuPluralKind,
    ) -> Result<IcuNode, IcuParseError> {
        let mut offset = 0u32;

        if self.consume_char(',') {
            self.skip_whitespace();
        }

        self.skip_whitespace();
        if self.starts_with_bytes(Self::OFFSET_PREFIX) {
            self.pos += Self::OFFSET_PREFIX.len();
            self.skip_whitespace();
            offset = self.parse_unsigned_int()?;
            self.skip_whitespace();
            if self.starts_with_bytes(Self::OFFSET_PREFIX) {
                return Err(self.error("Duplicate plural offset"));
            }
        }

        let options = self.parse_options(plural_depth + 1)?;
        if self.options.requires_other_clause && !has_other_clause(&options) {
            return Err(self.error("Plural argument requires an \"other\" clause"));
        }
        self.expect_char('}')?;

        Ok(IcuNode::Plural {
            name,
            kind,
            offset,
            options,
        })
    }

    fn parse_options(&mut self, plural_depth: usize) -> Result<Vec<IcuOption>, IcuParseError> {
        let mut options = Vec::with_capacity(4);

        loop {
            self.skip_whitespace();
            if self.byte_at() == Some(b'}') {
                break;
            }
            let selector = self.parse_selector()?;
            if options
                .iter()
                .any(|option: &IcuOption| option.selector == selector)
            {
                return Err(self.error("Duplicate ICU selector"));
            }
            self.skip_whitespace();
            self.expect_char('{')?;
            let value = self.parse_nested_nodes(None, plural_depth)?;
            self.expect_char('}')?;
            options.push(IcuOption { selector, value });
        }

        if options.is_empty() {
            return Err(self.error("Expected at least one ICU option"));
        }

        Ok(options)
    }

    fn validate_options(&mut self, plural_depth: usize) -> Result<bool, IcuParseError> {
        let mut stack_selectors = [BorrowedSelector::Keyword(""); 16];
        let mut stack_len = 0usize;
        let mut overflow_selectors: Vec<BorrowedSelector<'a>> = Vec::new();
        let mut has_other = false;

        loop {
            self.skip_whitespace();
            if self.byte_at() == Some(b'}') {
                break;
            }
            let selector = self.parse_selector_borrowed()?;
            let is_duplicate = stack_selectors[..stack_len].contains(&selector)
                || overflow_selectors.contains(&selector);
            if is_duplicate {
                return Err(self.error("Duplicate ICU selector"));
            }
            if selector == BorrowedSelector::Keyword("other") {
                has_other = true;
            }
            if stack_len < stack_selectors.len() {
                stack_selectors[stack_len] = selector;
                stack_len += 1;
            } else {
                overflow_selectors.push(selector);
            }
            self.skip_whitespace();
            self.expect_char('{')?;
            self.validate_nested_nodes(None, plural_depth)?;
            self.expect_char('}')?;
        }

        if stack_len == 0 {
            return Err(self.error("Expected at least one ICU option"));
        }

        Ok(has_other)
    }

    fn parse_tag(&mut self, plural_depth: usize) -> Result<IcuNode, IcuParseError> {
        self.expect_char('<')?;
        let name = self.parse_tag_name_str()?;
        if self.starts_with_bytes(Self::SELF_CLOSING_TAG_SUFFIX) {
            self.pos += Self::SELF_CLOSING_TAG_SUFFIX.len();
            return Ok(IcuNode::Tag {
                name: name.to_owned(),
                children: Vec::new(),
            });
        }
        self.expect_char('>')?;
        let children = self.parse_nested_nodes(Some(name), plural_depth)?;
        self.expect_bytes(Self::CLOSE_TAG_PREFIX)?;
        let close_name = self.parse_tag_name_str()?;
        if close_name != name {
            return Err(self.error("Mismatched closing tag"));
        }
        self.expect_char('>')?;
        Ok(IcuNode::Tag {
            name: name.to_owned(),
            children,
        })
    }

    fn validate_tag(&mut self, plural_depth: usize) -> Result<(), IcuParseError> {
        self.expect_char('<')?;
        let name = self.parse_tag_name_str()?;
        if self.starts_with_bytes(Self::SELF_CLOSING_TAG_SUFFIX) {
            self.pos += Self::SELF_CLOSING_TAG_SUFFIX.len();
            return Ok(());
        }
        self.expect_char('>')?;
        self.validate_nested_nodes(Some(name), plural_depth)?;
        self.expect_bytes(Self::CLOSE_TAG_PREFIX)?;
        let close_name = self.parse_tag_name_str()?;
        if close_name != name {
            return Err(self.error("Mismatched closing tag"));
        }
        self.expect_char('>')?;
        Ok(())
    }

    fn parse_apostrophe_literal_into(&mut self, out: &mut String) -> Result<(), IcuParseError> {
        let start = self.pos;
        self.expect_char('\'')?;

        if self.consume_char('\'') {
            out.push('\'');
            return Ok(());
        }

        while let Some(byte) = self.byte_at() {
            if byte == b'\'' {
                self.pos += 1;
                if self.consume_char('\'') {
                    out.push('\'');
                } else {
                    return Ok(());
                }
            } else {
                let rest = &self.input_bytes[self.pos..];
                let stop = memchr::memchr(b'\'', rest).unwrap_or(rest.len()).max(1);
                out.push_str(&self.input[self.pos..self.pos + stop]);
                self.pos += stop;
            }
        }

        Err(IcuParseError::syntax(
            "Unterminated apostrophe escape",
            self.input,
            start,
        ))
    }

    fn skip_apostrophe_literal(&mut self) -> Result<(), IcuParseError> {
        let start = self.pos;
        self.expect_char('\'')?;

        if self.consume_char('\'') {
            return Ok(());
        }

        while let Some(byte) = self.byte_at() {
            if byte == b'\'' {
                self.pos += 1;
                if !self.consume_char('\'') {
                    return Ok(());
                }
            } else {
                let rest = &self.input_bytes[self.pos..];
                let stop = memchr::memchr(b'\'', rest).unwrap_or(rest.len()).max(1);
                self.pos += stop;
            }
        }

        Err(IcuParseError::syntax(
            "Unterminated apostrophe escape",
            self.input,
            start,
        ))
    }

    fn read_until_closing_brace(&mut self) -> Result<Cow<'a, str>, IcuParseError> {
        let start = self.pos;
        let rest = &self.input_bytes[start..];
        match memchr::memchr2(b'}', b'\'', rest) {
            Some(stop) if rest[stop] == b'}' => {
                self.pos = start + stop;
                return Ok(Cow::Borrowed(&self.input[start..self.pos]));
            }
            None => {
                self.pos = self.input.len();
                return Err(self.error("Unterminated ICU argument"));
            }
            Some(_) => {}
        }

        let mut out = String::new();
        while let Some(byte) = self.byte_at() {
            if byte == b'}' {
                return Ok(Cow::Owned(out));
            }
            if byte == b'\'' {
                self.parse_apostrophe_literal_into(&mut out)?;
            } else {
                let rest = &self.input_bytes[self.pos..];
                let stop = memchr::memchr2(b'}', b'\'', rest)
                    .unwrap_or(rest.len())
                    .max(1);
                out.push_str(&self.input[self.pos..self.pos + stop]);
                self.pos += stop;
            }
        }
        Err(self.error("Unterminated ICU argument"))
    }

    fn skip_until_closing_brace(&mut self) -> Result<(), IcuParseError> {
        while let Some(byte) = self.byte_at() {
            if byte == b'}' {
                return Ok(());
            }
            if byte == b'\'' {
                self.skip_apostrophe_literal()?;
            } else {
                let rest = &self.input_bytes[self.pos..];
                let stop = memchr::memchr2(b'}', b'\'', rest)
                    .unwrap_or(rest.len())
                    .max(1);
                self.pos += stop;
            }
        }
        Err(self.error("Unterminated ICU argument"))
    }

    fn parse_selector(&mut self) -> Result<String, IcuParseError> {
        match self.parse_selector_borrowed()? {
            BorrowedSelector::Exact(number) => Ok(format!("={number}")),
            BorrowedSelector::Keyword(keyword) => Ok(keyword.to_owned()),
        }
    }

    fn parse_selector_borrowed(&mut self) -> Result<BorrowedSelector<'a>, IcuParseError> {
        let start = self.pos;
        if self.consume_char('=') {
            let number = self.parse_unsigned_int()?;
            return Ok(BorrowedSelector::Exact(number));
        }

        while let Some(byte) = self.byte_at() {
            if byte.is_ascii_whitespace() || byte == b'{' {
                break;
            }
            if byte.is_ascii() {
                self.pos += 1;
            } else {
                self.advance_char();
            }
        }

        if self.pos == start {
            return Err(self.error("Expected ICU selector"));
        }

        Ok(BorrowedSelector::Keyword(&self.input[start..self.pos]))
    }

    fn parse_identifier_str(&mut self) -> Result<&'a str, IcuParseError> {
        let start = self.pos;
        while let Some(byte) = self.byte_at() {
            if byte.is_ascii_whitespace() || matches!(byte, b'{' | b'}' | b',' | b'<' | b'>') {
                break;
            }
            if byte.is_ascii() {
                self.pos += 1;
            } else {
                self.advance_char();
            }
        }

        if self.pos == start {
            return Err(self.error("Expected ICU identifier"));
        }

        Ok(&self.input[start..self.pos])
    }

    fn parse_tag_name_str(&mut self) -> Result<&'a str, IcuParseError> {
        let start = self.pos;
        while let Some(byte) = self.byte_at() {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.') {
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.pos == start {
            return Err(self.error("Expected tag name"));
        }

        Ok(&self.input[start..self.pos])
    }

    fn parse_unsigned_int(&mut self) -> Result<u32, IcuParseError> {
        let start = self.pos;
        while let Some(byte) = self.byte_at() {
            if byte.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.pos == start {
            return Err(self.error("Expected integer"));
        }

        self.input[start..self.pos]
            .parse::<u32>()
            .map_err(|_| self.error("Invalid integer"))
    }

    fn skip_whitespace(&mut self) {
        while let Some(byte) = self.byte_at() {
            if byte.is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn flush_literal(literal: &mut String, nodes: &mut Vec<IcuNode>) {
        if !literal.is_empty() {
            nodes.push(IcuNode::Literal(core::mem::take(literal)));
        }
    }

    fn expect_char(&mut self, ch: char) -> Result<(), IcuParseError> {
        if ch.is_ascii() {
            if self.byte_at() == Some(ch as u8) {
                self.pos += 1;
                return Ok(());
            }
            return Err(self.error(format!("Expected '{ch}'")));
        }

        match self.peek_char() {
            Some(current) if current == ch => {
                self.pos += ch.len_utf8();
                Ok(())
            }
            _ => Err(self.error(format!("Expected '{ch}'"))),
        }
    }

    fn expect_bytes(&mut self, expected: &[u8]) -> Result<(), IcuParseError> {
        if self.starts_with_bytes(expected) {
            self.pos += expected.len();
            Ok(())
        } else {
            let expected = core::str::from_utf8(expected).unwrap_or("<bytes>");
            Err(self.error(format!("Expected \"{expected}\"")))
        }
    }

    fn consume_char(&mut self, ch: char) -> bool {
        if ch.is_ascii() {
            if self.byte_at() == Some(ch as u8) {
                self.pos += 1;
                return true;
            }
            return false;
        }

        if self.peek_char() == Some(ch) {
            self.pos += ch.len_utf8();
            true
        } else {
            false
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn byte_at(&self) -> Option<u8> {
        self.input_bytes.get(self.pos).copied()
    }

    fn advance_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn peek_open_tag(&self) -> bool {
        let Some(rest) = self.input_bytes.get(self.pos..) else {
            return false;
        };
        if !rest.starts_with(b"<") || rest.starts_with(b"</") {
            return false;
        }
        rest.get(1).is_some_and(u8::is_ascii_alphanumeric)
    }

    fn peek_close_tag(&self) -> bool {
        self.input_bytes[self.pos..].starts_with(b"</")
    }

    fn starts_with_close_tag(&self, name: &str) -> bool {
        let Some(rest) = self.input_bytes.get(self.pos..) else {
            return false;
        };
        rest.starts_with(Self::CLOSE_TAG_PREFIX)
            && rest[2..].starts_with(name.as_bytes())
            && rest.get(2 + name.len()) == Some(&b'>')
    }

    fn starts_with_bytes(&self, expected: &[u8]) -> bool {
        self.input_bytes[self.pos..].starts_with(expected)
    }

    const fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn error(&self, message: impl Into<String>) -> IcuParseError {
        IcuParseError::syntax(message, self.input, self.pos)
    }
}

#[derive(Clone, Copy)]
enum FormatterKind {
    Number,
    Date,
    Time,
    List,
    Duration,
    Ago,
    Name,
}

fn has_other_clause(options: &[IcuOption]) -> bool {
    options.iter().any(|option| option.selector == "other")
}

/// Finds the offset of the next byte that ends a plain-literal run: an ICU
/// structural byte (`{`, `}`, `<`, `'`) or, inside a plural/selectordinal body,
/// `#`. Uses memchr (SIMD) so long literal segments are skipped in bulk.
#[inline]
fn find_literal_stop(haystack: &[u8], in_plural: bool) -> Option<usize> {
    let structural = memchr::memchr3(b'{', b'}', b'<', haystack);
    let search_end = structural.unwrap_or(haystack.len());
    let literal_prefix = &haystack[..search_end];
    let quote = memchr::memchr(b'\'', literal_prefix);
    let stop = min_option(structural, quote);
    if in_plural {
        min_option(stop, memchr::memchr(b'#', literal_prefix))
    } else {
        stop
    }
}

#[inline]
fn min_option(first: Option<usize>, second: Option<usize>) -> Option<usize> {
    match (first, second) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        IcuNode, IcuParseError, IcuParserOptions, IcuPluralKind, parse_icu, parse_icu_with_options,
        validate_icu,
    };

    use super::find_literal_stop;

    #[test]
    fn parses_simple_argument_message() {
        let message = parse_icu("Hello {name}!").expect("parse");
        assert_eq!(
            message.nodes,
            vec![
                IcuNode::Literal("Hello ".to_owned()),
                IcuNode::Argument {
                    name: "name".to_owned()
                },
                IcuNode::Literal("!".to_owned())
            ]
        );
    }

    #[test]
    fn rejects_duplicate_selectors_and_offsets() {
        for input in [
            "{n, plural, one {a} one {b} other {c}}",
            "{n, plural, =01 {a} =1 {b} other {c}}",
            "{x, select, other {a} other {b}}",
            "{n, plural, offset:0 offset:1 other {a}}",
        ] {
            assert!(parse_icu(input).is_err(), "accepted {input}");
        }
    }

    #[test]
    fn parses_formatter_styles_as_opaque_strings() {
        let message = parse_icu(
            "{n, number, currency} {d, date, short} {t, time, ::HHmm} {items, list, disjunction}",
        )
        .expect("parse");
        assert!(matches!(
            &message.nodes[0],
            IcuNode::Number {
                style: Some(style),
                ..
            } if style == "currency"
        ));
        assert!(matches!(
            &message.nodes[2],
            IcuNode::Date {
                style: Some(style),
                ..
            } if style == "short"
        ));
        assert!(matches!(
            &message.nodes[4],
            IcuNode::Time {
                style: Some(style),
                ..
            } if style == "::HHmm"
        ));
        assert!(matches!(
            &message.nodes[6],
            IcuNode::List {
                style: Some(style),
                ..
            } if style == "disjunction"
        ));
    }

    #[test]
    fn parses_plural_select_and_selectordinal() {
        let message = parse_icu(
            "{count, plural, offset:1 =0 {none} one {# item} other {{gender, select, male {his} other {their}} items}} {rank, selectordinal, one {#st} other {#th}}",
        )
        .expect("parse");

        assert!(matches!(
            &message.nodes[0],
            IcuNode::Plural {
                kind: IcuPluralKind::Cardinal,
                offset: 1,
                options,
                ..
            } if options.len() == 3
        ));
        assert!(matches!(
            &message.nodes[2],
            IcuNode::Plural {
                kind: IcuPluralKind::Ordinal,
                options,
                ..
            } if options.len() == 2
        ));
    }

    #[test]
    fn bounds_nested_option_and_tag_bodies() {
        for kind in ["select", "plural", "selectordinal", "tag", "mixed"] {
            let nested = |depth: usize| {
                (0..depth).fold("# {name} '<literal>'".to_owned(), |body, index| {
                    match if kind == "mixed" {
                        ["select", "plural", "selectordinal", "tag"][index % 4]
                    } else {
                        kind
                    } {
                        "tag" => format!("<b>{body}</b>"),
                        argument_kind => format!("{{x, {argument_kind}, other {{{body}}}}}"),
                    }
                })
            };
            parse_icu(&nested(super::MAX_NESTING_DEPTH)).expect("limit is accepted");
            let error = parse_icu(&nested(super::MAX_NESTING_DEPTH + 1))
                .expect_err("one level beyond limit is rejected");
            assert!(error.message.contains("maximum nesting depth"), "{error}");
        }
    }

    #[test]
    fn nesting_limit_counts_depth_not_siblings_or_pounds() {
        let siblings = "<b>#</b>{x, plural, other {###}}".repeat(100);
        parse_icu(&siblings).expect("siblings do not accumulate depth");
        let depth = super::MAX_NESTING_DEPTH + 1;
        let ignored = format!("{}text{}", "<b>".repeat(depth), "</b>".repeat(depth));
        parse_icu_with_options(&ignored, &IcuParserOptions::default().with_ignore_tag(true))
            .expect("ignored tags are literals");
    }

    #[test]
    fn parses_tags_and_nested_content() {
        let message =
            parse_icu("<0>{count, plural, one {<b>#</b>} other {items}}</0>").expect("parse");
        assert!(matches!(
            &message.nodes[0],
            IcuNode::Tag { name, children }
                if name == "0" && !children.is_empty()
        ));
    }

    #[test]
    fn parses_self_closing_tag() {
        let message = parse_icu("Foo <0/> bar").expect("parse");
        assert!(matches!(
            &message.nodes[1],
            IcuNode::Tag { name, children }
                if name == "0" && children.is_empty()
        ));
    }

    #[test]
    fn self_closing_tag_round_trips() {
        let message = parse_icu("line one<0/>line two").expect("parse");
        assert_eq!(crate::stringify_icu(&message), "line one<0/>line two");
    }

    #[test]
    fn parses_self_closing_tag_inside_plural_branch() {
        let message = parse_icu("{count, plural, one {a<br/>b} other {items}}").expect("parse");
        assert_eq!(
            crate::stringify_icu(&message),
            "{count, plural, one {a<br/>b} other {items}}"
        );
    }

    #[test]
    fn empty_tags_serialize_self_closing() {
        // `<n/>` and the empty paired form `<n></n>` both parse to a tag with
        // empty children and are serialized in the compact self-closing shape.
        let paired = parse_icu("<0></0>").expect("parse paired");
        let self_closing = parse_icu("<0/>").expect("parse self-closing");
        assert_eq!(crate::stringify_icu(&paired), "<0/>");
        assert_eq!(crate::stringify_icu(&self_closing), "<0/>");
    }

    #[test]
    fn ignore_tag_treats_tags_as_literal_text() {
        let message = parse_icu_with_options(
            "<b>Hello</b>",
            &IcuParserOptions::default().with_ignore_tag(true),
        )
        .expect("parse");
        assert_eq!(
            message.nodes,
            vec![IcuNode::Literal("<b>Hello</b>".to_owned())]
        );
    }

    #[test]
    fn apostrophe_escaping_works() {
        let message = parse_icu("'{'{name}'}' ''").expect("parse");
        assert_eq!(
            message.nodes,
            vec![
                IcuNode::Literal("{".to_owned()),
                IcuNode::Argument {
                    name: "name".to_owned()
                },
                IcuNode::Literal("} '".to_owned()),
            ]
        );
    }

    #[test]
    fn missing_other_clause_fails_by_default() {
        let error = parse_icu("{count, plural, one {item}}").expect_err("missing other");
        assert!(error.message.contains("other"));
    }

    #[test]
    fn missing_other_clause_can_be_disabled() {
        parse_icu_with_options(
            "{count, plural, one {item}}",
            &IcuParserOptions::default().with_requires_other_clause(false),
        )
        .expect("parse");
    }

    #[test]
    fn mismatched_closing_tag_fails() {
        let error = parse_icu("<a>hello</b>").expect_err("mismatch");
        assert!(error.message.contains("Mismatched"));
    }

    #[test]
    fn invalid_offset_fails() {
        let error = parse_icu("{count, plural, offset:x other {#}}").expect_err("invalid offset");
        assert!(error.message.contains("integer"));
    }

    #[test]
    fn validate_icu_uses_same_error_surface() {
        let parse_error = parse_icu("{unclosed").expect_err("parse");
        let validate_error = validate_icu("{unclosed").expect_err("validate");
        assert_eq!(parse_error, validate_error);
    }

    #[test]
    fn error_positions_are_reported() {
        let error = parse_icu("Hello\n{unclosed").expect_err("parse");
        assert_eq!(error.position.line, 2);
        assert!(error.position.column >= 2);
    }

    #[test]
    fn pound_outside_plural_is_literal() {
        let message = parse_icu("Total # items").expect("parse");
        assert_eq!(
            message.nodes,
            vec![IcuNode::Literal("Total # items".to_owned())]
        );
    }

    #[test]
    fn literal_stop_search_ignores_markers_after_structural_stop() {
        assert_eq!(find_literal_stop(b"abc{later#'", true), Some(3));
        assert_eq!(find_literal_stop(b"abc#later{", true), Some(3));
        assert_eq!(find_literal_stop(b"abc#later{", false), Some(9));
        assert_eq!(find_literal_stop(b"abc'later{", true), Some(3));
    }

    #[test]
    fn parse_error_type_is_result_based() {
        let result: Result<_, IcuParseError> = parse_icu("{broken");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_unsupported_types_and_unexpected_trailing_input() {
        let unsupported = parse_icu("{name, foo}").expect_err("unsupported type");
        assert!(
            unsupported
                .message
                .contains("Unsupported ICU argument type")
        );

        let trailing = parse_icu("hello}").expect_err("trailing input");
        assert!(trailing.message.contains("Unexpected trailing input"));
    }

    #[test]
    fn rejects_unterminated_apostrophe_and_unexpected_closing_tag() {
        let apostrophe = parse_icu("'unterminated").expect_err("unterminated apostrophe");
        assert!(
            apostrophe
                .message
                .contains("Unterminated apostrophe escape")
        );

        let closing = parse_icu("</b>").expect_err("unexpected closing tag");
        assert!(closing.message.contains("Unexpected closing tag"));
    }

    #[test]
    fn parses_formatters_without_style_and_invalid_tag_names_fail() {
        let message = parse_icu("{value, number}").expect("parse formatter without style");
        assert!(matches!(
            &message.nodes[0],
            IcuNode::Number { style: None, .. }
        ));

        let error = parse_icu("<a>broken</>").expect_err("invalid closing tag");
        assert!(error.message.contains("Mismatched closing tag"));
    }

    #[test]
    fn validate_icu_matches_parse_icu_for_valid_and_invalid_inputs() {
        let cases = [
            "",
            "Plain literal without special chars",
            "Literal with # outside plural and < not a tag",
            "Hello {name}!",
            "Escaped ''apostrophe'' and '{not_arg}' and '<not_tag>'",
            "{n, number} {n, number, currency} {n, number, 'prefix '0.00} {d, date, short} {t, time, ::HHmm} {l, list, disjunction} {u, duration} {a, ago} {p, name}",
            "{count, plural, offset:1 =0 {none} one {# item} other {{gender, select, male {his} other {their}} items}}",
            "{rank, selectordinal, one {#st} two {#nd} few {#rd} other {#th}}",
            "<0>hello <br/> <b>{name}</b></0>",
            // >16 selectors to exercise overflow path in validate_options (valid and duplicate with Unicode prefix)
            "{k, select, s0 {a} s1 {a} s2 {a} s3 {a} s4 {a} s5 {a} s6 {a} s7 {a} s8 {a} s9 {a} s10 {a} s11 {a} s12 {a} s13 {a} s14 {a} s15 {a} s16 {a} other {z}}",
            "é\n{k, select, s0 {a} s1 {a} s2 {a} s3 {a} s4 {a} s5 {a} s6 {a} s7 {a} s8 {a} s9 {a} s10 {a} s11 {a} s12 {a} s13 {a} s14 {a} s15 {a} s16 {a} s16 {b} other {z}}",
            // Invalid cases
            "hello}",
            "{broken",
            "{}",
            "{name, }",
            "{name, unknown}",
            "{name, number, 'unterminated}",
            "{name, number, unterminated",
            "{n, plural, one {a}}",
            "{x, select, a {1}}",
            "{n, plural, }",
            "{n, plural, one {a} one {b} other {c}}",
            "{n, plural, =01 {a} =1 {b} other {c}}",
            "{n, plural, =abc {a} other {b}}",
            "{n, plural, offset:0 offset:1 other {a}}",
            "{n, plural, offset:abc other {a}}",
            "'unterminated",
            "</b>",
            "<a>broken</b>",
            "<a>broken</>",
            "<a>unclosed",
            "<a",
            // Exceeding max nesting depth
            &"<b>".repeat(super::MAX_NESTING_DEPTH + 1),
        ];

        for input in cases {
            let parsed = parse_icu(input).map(|_| ());
            let validated = validate_icu(input);
            assert_eq!(
                validated, parsed,
                "validate_icu disagreed with parse_icu for input {input:?}"
            );
        }

        // Also verify with ignore_tag=true and requires_other_clause=false
        let relaxed = IcuParserOptions::default()
            .with_ignore_tag(true)
            .with_requires_other_clause(false);
        for input in [
            "<b>Hello</b>",
            "<b>Hello</b> {name}",
            "{n, plural, one {#}}",
            "{x, select, a {1}}",
            "{x, select, a {1} a {2}}",
        ] {
            let parsed = parse_icu_with_options(input, &relaxed).map(|_| ());
            let validated = super::validate_icu_with_options(input, &relaxed);
            assert_eq!(
                validated, parsed,
                "validate_icu_with_options disagreed for {input:?}"
            );
        }
    }
}
