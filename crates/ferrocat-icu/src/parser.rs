use crate::ast::{IcuMessage, IcuNode, IcuOption, IcuPluralKind};
use crate::error::IcuParseError;

/// Options controlling ICU parsing behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// Parses ICU `MessageFormat` input with the default parser options.
///
/// # Errors
///
/// Returns [`IcuParseError`] when the input is malformed.
pub fn parse_icu(input: &str) -> Result<IcuMessage, IcuParseError> {
    parse_icu_with_options(input, &IcuParserOptions::default())
}

/// Parses ICU `MessageFormat` input with explicit parser options.
///
/// # Errors
///
/// Returns [`IcuParseError`] when the input is malformed.
pub fn parse_icu_with_options(
    input: &str,
    options: &IcuParserOptions,
) -> Result<IcuMessage, IcuParseError> {
    let mut parser = Parser::new(input, options);
    let nodes = parser.parse_nodes(None, 0)?;
    if !parser.is_eof() {
        return Err(parser.error("Unexpected trailing input"));
    }
    Ok(IcuMessage { nodes })
}

struct Parser<'a> {
    input: &'a str,
    input_bytes: &'a [u8],
    pos: usize,
    options: &'a IcuParserOptions,
}

impl<'a> Parser<'a> {
    const OFFSET_PREFIX: &'static [u8] = b"offset:";
    const CLOSE_TAG_PREFIX: &'static [u8] = b"</";

    const fn new(input: &'a str, options: &'a IcuParserOptions) -> Self {
        Self {
            input,
            input_bytes: input.as_bytes(),
            pos: 0,
            options,
        }
    }

    fn parse_nodes(
        &mut self,
        until_tag: Option<&str>,
        plural_depth: usize,
    ) -> Result<Vec<IcuNode>, IcuParseError> {
        let mut nodes = Vec::with_capacity(4);
        let mut literal = String::with_capacity(16);

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
                b'\'' => literal.push_str(&self.parse_apostrophe_literal()?),
                _ => literal.push(self.advance_char().expect("byte implies char")),
            }
        }

        Self::flush_literal(&mut literal, &mut nodes);
        Ok(nodes)
    }

    fn parse_argument(&mut self, plural_depth: usize) -> Result<IcuNode, IcuParseError> {
        self.expect_char('{')?;
        self.skip_whitespace();
        let name = self.parse_identifier()?;
        self.skip_whitespace();

        if self.consume_char('}') {
            return Ok(IcuNode::Argument { name });
        }

        self.expect_char(',')?;
        self.skip_whitespace();
        let kind = self.parse_identifier()?;
        self.skip_whitespace();

        match kind.as_str() {
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

    fn parse_simple_formatter(
        &mut self,
        name: String,
        kind: FormatterKind,
    ) -> Result<IcuNode, IcuParseError> {
        let style = if self.consume_char(',') {
            let style = self.read_until_closing_brace()?.trim().to_owned();
            Some(style).filter(|style| !style.is_empty())
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

        loop {
            self.skip_whitespace();
            if self.starts_with_bytes(Self::OFFSET_PREFIX) {
                self.pos += Self::OFFSET_PREFIX.len();
                self.skip_whitespace();
                offset = self.parse_unsigned_int()?;
            } else {
                break;
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
            self.skip_whitespace();
            self.expect_char('{')?;
            let value = self.parse_nodes(None, plural_depth)?;
            self.expect_char('}')?;
            options.push(IcuOption { selector, value });
        }

        if options.is_empty() {
            return Err(self.error("Expected at least one ICU option"));
        }

        Ok(options)
    }

    fn parse_tag(&mut self, plural_depth: usize) -> Result<IcuNode, IcuParseError> {
        self.expect_char('<')?;
        let name = self.parse_tag_name()?;
        self.expect_char('>')?;
        let children = self.parse_nodes(Some(&name), plural_depth)?;
        self.expect_bytes(Self::CLOSE_TAG_PREFIX)?;
        let close_name = self.parse_tag_name()?;
        if close_name != name {
            return Err(self.error("Mismatched closing tag"));
        }
        self.expect_char('>')?;
        Ok(IcuNode::Tag { name, children })
    }

    fn parse_apostrophe_literal(&mut self) -> Result<String, IcuParseError> {
        let start = self.pos;
        self.expect_char('\'')?;

        if self.consume_char('\'') {
            return Ok("'".to_owned());
        }

        let mut out = String::with_capacity(8);
        while let Some(byte) = self.byte_at() {
            if byte == b'\'' {
                self.pos += 1;
                if self.consume_char('\'') {
                    out.push('\'');
                } else {
                    return Ok(out);
                }
            } else {
                out.push(self.advance_char().expect("byte implies char"));
            }
        }

        Err(IcuParseError::syntax(
            "Unterminated apostrophe escape",
            self.input,
            start,
        ))
    }

    fn read_until_closing_brace(&mut self) -> Result<String, IcuParseError> {
        let mut out = String::with_capacity(8);
        while let Some(byte) = self.byte_at() {
            if byte == b'}' {
                return Ok(out);
            }
            if byte == b'\'' {
                out.push_str(&self.parse_apostrophe_literal()?);
            } else {
                out.push(self.advance_char().expect("byte implies char"));
            }
        }
        Err(self.error("Unterminated ICU argument"))
    }

    fn parse_selector(&mut self) -> Result<String, IcuParseError> {
        let start = self.pos;
        if self.consume_char('=') {
            let number = self.parse_unsigned_int()?;
            return Ok(format!("={number}"));
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

        Ok(self.input[start..self.pos].to_owned())
    }

    fn parse_identifier(&mut self) -> Result<String, IcuParseError> {
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

        Ok(self.input[start..self.pos].to_owned())
    }

    fn parse_tag_name(&mut self) -> Result<String, IcuParseError> {
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

        Ok(self.input[start..self.pos].to_owned())
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

#[cfg(test)]
mod tests {
    use crate::{
        IcuNode, IcuParseError, IcuParserOptions, IcuPluralKind, parse_icu, parse_icu_with_options,
        validate_icu,
    };

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
    fn parses_tags_and_nested_content() {
        let message =
            parse_icu("<0>{count, plural, one {<b>#</b>} other {items}}</0>").expect("parse");
        assert!(matches!(
            &message.nodes[0],
            IcuNode::Tag { name, children } if name == "0" && !children.is_empty()
        ));
    }

    #[test]
    fn ignore_tag_treats_tags_as_literal_text() {
        let message = parse_icu_with_options(
            "<b>Hello</b>",
            &IcuParserOptions {
                ignore_tag: true,
                ..IcuParserOptions::default()
            },
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
            &IcuParserOptions {
                requires_other_clause: false,
                ..IcuParserOptions::default()
            },
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
}
