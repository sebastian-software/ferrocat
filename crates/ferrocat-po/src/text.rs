use std::borrow::Cow;

use crate::ParseError;
use crate::scan::{find_byte, find_escapable_byte, find_quoted_bounds, has_byte};
use crate::utf8::{input_slice_as_str, string_from_utf8};

/// Escapes a PO string literal payload.
#[must_use]
pub fn escape_string(input: &str) -> String {
    let bytes = input.as_bytes();
    let Some(first_escape) = find_escapable_byte(bytes) else {
        return input.to_owned();
    };

    let mut out = String::with_capacity(input.len() + 8);
    out.push_str(&input[..first_escape]);
    escape_string_from(&mut out, input, bytes, first_escape);

    out
}

pub fn escape_string_into(out: &mut String, input: &str) {
    let bytes = input.as_bytes();
    let Some(first_escape) = find_escapable_byte(bytes) else {
        out.push_str(input);
        return;
    };

    escape_string_into_known(out, input, first_escape);
}

pub fn escape_string_into_with_first_escape(
    out: &mut String,
    input: &str,
    first_escape: Option<usize>,
) {
    let Some(first_escape) = first_escape else {
        out.push_str(input);
        return;
    };

    escape_string_into_known(out, input, first_escape);
}

/// Unescapes a PO string literal payload.
///
/// # Errors
///
/// Returns [`ParseError`] when the escape sequence is malformed.
pub fn unescape_string(input: &str) -> Result<String, ParseError> {
    let bytes = input.as_bytes();
    if !has_byte(b'\\', bytes) {
        return Ok(input.to_owned());
    }

    let mut out = Vec::with_capacity(input.len());
    let mut index = 0;

    while index < bytes.len() {
        let next_escape = if let Some(relative) = find_byte(b'\\', &bytes[index..]) {
            index + relative
        } else {
            out.extend_from_slice(&bytes[index..]);
            break;
        };

        out.extend_from_slice(&bytes[index..next_escape]);
        index = next_escape + 1;
        if index >= bytes.len() {
            return Err(ParseError::new("unterminated escape sequence"));
        }

        let escaped = bytes[index];
        match escaped {
            b'a' => out.push(b'\x07'),
            b'b' => out.push(b'\x08'),
            b't' => out.push(b'\t'),
            b'n' => out.push(b'\n'),
            b'v' => out.push(b'\x0b'),
            b'f' => out.push(b'\x0c'),
            b'r' => out.push(b'\r'),
            b'\'' => out.push(b'\''),
            b'"' => out.push(b'"'),
            b'\\' => out.push(b'\\'),
            b'?' => out.push(b'?'),
            b'0'..=b'7' => {
                let mut value = u32::from(escaped - b'0');
                let mut consumed = 1;
                while consumed < 3 && index + consumed < bytes.len() {
                    let next = bytes[index + consumed];
                    if !(b'0'..=b'7').contains(&next) {
                        break;
                    }
                    value = (value * 8) + u32::from(next - b'0');
                    consumed += 1;
                }
                match char::from_u32(value) {
                    Some(ch) => push_char_bytes(&mut out, ch),
                    None => return Err(ParseError::new("invalid octal escape value")),
                }
                index += consumed - 1;
            }
            b'x' => {
                if index + 2 >= bytes.len() {
                    return Err(ParseError::new("incomplete hex escape"));
                }
                let hi = decode_hex(bytes[index + 1])?;
                let lo = decode_hex(bytes[index + 2])?;
                let value = u32::from((hi << 4) | lo);
                match char::from_u32(value) {
                    Some(ch) => push_char_bytes(&mut out, ch),
                    None => return Err(ParseError::new("invalid hex escape value")),
                }
                index += 2;
            }
            other => out.push(other),
        }

        index += 1;
    }

    Ok(string_from_utf8(out))
}

/// Extracts and unescapes the first quoted PO string from `line`, borrowing
/// from the input when no escapes are present.
///
/// # Errors
///
/// Returns [`ParseError`] when the quoted content is malformed.
pub fn extract_quoted_cow(line: &str) -> Result<Cow<'_, str>, ParseError> {
    extract_quoted_bytes_cow(line.as_bytes())
}

pub fn extract_quoted_bytes_cow(line: &[u8]) -> Result<Cow<'_, str>, ParseError> {
    let Some((start, end)) = find_quoted_bounds(line) else {
        return Ok(Cow::Borrowed(""));
    };

    let raw = &line[start..end];
    validate_quoted_content(raw)?;
    if !has_byte(b'\\', raw) {
        return Ok(Cow::Borrowed(bytes_to_str(raw)));
    }

    Ok(Cow::Owned(unescape_string(bytes_to_str(raw))?))
}

/// Extracts and unescapes the first quoted PO string from `line`.
///
/// # Errors
///
/// Returns [`ParseError`] when the quoted content is malformed.
pub fn extract_quoted(line: &str) -> Result<String, ParseError> {
    Ok(extract_quoted_bytes_cow(line.as_bytes())?.into_owned())
}

pub fn split_reference_comment(input: &str) -> Vec<Cow<'_, str>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return vec![Cow::Borrowed("")];
    }

    let mut parts = Vec::new();
    let mut start = None;
    let mut isolate_depth = 0usize;

    for (index, ch) in trimmed.char_indices() {
        match ch {
            '\u{2068}' => {
                if start.is_none() {
                    start = Some(index);
                }
                isolate_depth += 1;
            }
            '\u{2069}' => {
                if start.is_none() {
                    start = Some(index);
                }
                isolate_depth = isolate_depth.saturating_sub(1);
            }
            _ if ch.is_whitespace() && isolate_depth == 0 => {
                if let Some(segment_start) = start.take()
                    && segment_start < index
                {
                    parts.push(normalize_reference_token(&trimmed[segment_start..index]));
                }
            }
            _ => {
                if start.is_none() {
                    start = Some(index);
                }
            }
        }
    }

    if let Some(segment_start) = start
        && segment_start < trimmed.len()
    {
        parts.push(normalize_reference_token(&trimmed[segment_start..]));
    }

    if parts.len() == 1 {
        return vec![normalize_reference_token(trimmed)];
    }

    if parts.iter().all(|part| part.contains(':')) {
        return parts;
    }

    vec![Cow::Borrowed(trimmed)]
}

pub fn validate_quoted_content(raw: &[u8]) -> Result<(), ParseError> {
    let mut trailing_backslashes = 0usize;

    for &byte in raw {
        match byte {
            b'\\' => trailing_backslashes += 1,
            b'"' if trailing_backslashes % 2 == 0 => {
                return Err(ParseError::new("unescaped quote in string literal"));
            }
            _ => trailing_backslashes = 0,
        }
    }

    Ok(())
}

fn escape_string_from(out: &mut String, input: &str, bytes: &[u8], first_escape: usize) {
    let mut start = first_escape;

    loop {
        push_escape(out, bytes[start]);
        let next_index = start + 1;
        let Some(relative) = find_escapable_byte(&bytes[next_index..]) else {
            out.push_str(&input[next_index..]);
            break;
        };

        let absolute = next_index + relative;
        out.push_str(&input[next_index..absolute]);
        start = absolute;
    }
}

#[inline]
fn escape_string_into_known(out: &mut String, input: &str, first_escape: usize) {
    let bytes = input.as_bytes();
    out.push_str(&input[..first_escape]);
    escape_string_from(out, input, bytes, first_escape);
}

fn push_escape(out: &mut String, byte: u8) {
    out.push('\\');
    out.push(match byte {
        b'\x07' => 'a',
        b'\x08' => 'b',
        b'\t' => 't',
        b'\n' => 'n',
        b'\x0b' => 'v',
        b'\x0c' => 'f',
        b'\r' => 'r',
        b'"' => '"',
        b'\\' => '\\',
        _ => unreachable!("unexpected escape byte"),
    });
}

fn decode_hex(byte: u8) -> Result<u8, ParseError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(ParseError::new("invalid hex escape")),
    }
}

fn push_char_bytes(out: &mut Vec<u8>, ch: char) {
    if ch.is_ascii() {
        out.push(ch as u8);
        return;
    }

    let mut buf = [0u8; 4];
    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
}

fn bytes_to_str(bytes: &[u8]) -> &str {
    input_slice_as_str(bytes)
}

fn normalize_reference_token(input: &str) -> Cow<'_, str> {
    if !input.contains('\u{2068}') && !input.contains('\u{2069}') {
        return Cow::Borrowed(input);
    }

    Cow::Owned(
        input
            .chars()
            .filter(|ch| *ch != '\u{2068}' && *ch != '\u{2069}')
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{
        escape_string, escape_string_into, escape_string_into_with_first_escape, extract_quoted,
        extract_quoted_bytes_cow, extract_quoted_cow, split_reference_comment, unescape_string,
        validate_quoted_content,
    };

    #[test]
    fn escapes_special_characters() {
        assert_eq!(escape_string("Say \"Hi\""), "Say \\\"Hi\\\"");
        assert_eq!(escape_string("a\tb"), "a\\tb");
    }

    #[test]
    fn unescapes_c_sequences() {
        assert_eq!(
            unescape_string("\\a\\b\\t\\n\\v\\f\\r\\'\\\"\\\\\\?").as_deref(),
            Ok("\u{0007}\u{0008}\t\n\u{000b}\u{000c}\r'\"\\?")
        );
    }

    #[test]
    fn extracts_and_unescapes_quoted_text() {
        assert_eq!(
            extract_quoted(
                "msgid \"The name field must not contain characters like \\\" or \\\\\""
            )
            .as_deref(),
            Ok("The name field must not contain characters like \" or \\")
        );
    }

    #[test]
    fn borrows_simple_quoted_text_without_escape() {
        assert_eq!(
            extract_quoted_cow("msgid \"plain text\""),
            Ok(Cow::Borrowed("plain text"))
        );
    }

    #[test]
    fn appends_escaped_text_into_existing_buffer() {
        let mut out = String::from("prefix:");
        escape_string_into(&mut out, "Say \"Hi\"\n");
        assert_eq!(out, "prefix:Say \\\"Hi\\\"\\n");
    }

    #[test]
    fn appends_escaped_text_into_existing_buffer_with_known_escape() {
        let mut out = String::from("prefix:");
        escape_string_into_with_first_escape(&mut out, "Say \"Hi\"\n", Some(4));
        assert_eq!(out, "prefix:Say \\\"Hi\\\"\\n");
    }

    #[test]
    fn appends_plain_text_when_no_escape_index_is_known() {
        let mut out = String::from("prefix:");
        escape_string_into_with_first_escape(&mut out, "plain", None);
        assert_eq!(out, "prefix:plain");
    }

    #[test]
    fn extracts_quoted_text_from_bytes() {
        assert_eq!(
            extract_quoted_bytes_cow(br#"msgid "byte path""#),
            Ok(Cow::Borrowed("byte path"))
        );
    }

    #[test]
    fn extracts_owned_quoted_text_when_unescaping_is_required() {
        assert_eq!(
            extract_quoted_bytes_cow(br#"msgid "line\nbreak""#),
            Ok(Cow::Owned("line\nbreak".to_owned()))
        );
        assert_eq!(extract_quoted("msgid bare"), Ok(String::new()));
    }

    #[test]
    fn splits_multiple_reference_tokens() {
        assert_eq!(
            split_reference_comment("src/app.js:1 src/lib.js:2"),
            vec![Cow::Borrowed("src/app.js:1"), Cow::Borrowed("src/lib.js:2")]
        );
    }

    #[test]
    fn preserves_standard_input_reference_lines() {
        assert_eq!(
            split_reference_comment("standard input:12 standard input:17"),
            vec![Cow::Borrowed("standard input:12 standard input:17")]
        );
    }

    #[test]
    fn strips_isolates_when_splitting_reference_tokens() {
        assert_eq!(
            split_reference_comment("\u{2068}main 1.py\u{2069}:1 other.py:2"),
            vec![
                Cow::Owned("main 1.py:1".to_owned()),
                Cow::Borrowed("other.py:2"),
            ]
        );
    }

    #[test]
    fn keeps_non_reference_whitespace_groups_and_empty_input_stable() {
        assert_eq!(
            split_reference_comment("foo bar"),
            vec![Cow::Borrowed("foo bar")]
        );
        assert_eq!(split_reference_comment("   "), vec![Cow::Borrowed("")]);
    }

    #[test]
    fn rejects_unescaped_quote_in_string_literal() {
        assert_eq!(
            validate_quoted_content(br#"Some msgstr with "double\" quotes"#)
                .expect_err("expected unescaped quote error")
                .to_string(),
            "unescaped quote in string literal"
        );
    }

    #[test]
    fn unescape_string_covers_octal_hex_and_error_paths() {
        assert_eq!(unescape_string("\\101\\x42").as_deref(), Ok("AB"));
        assert_eq!(
            unescape_string("\\x4")
                .expect_err("incomplete hex escape")
                .to_string(),
            "incomplete hex escape"
        );
        assert_eq!(
            unescape_string("\\xZZ")
                .expect_err("invalid hex escape")
                .to_string(),
            "invalid hex escape"
        );
        assert!(validate_quoted_content(br#"still safe\""#).is_ok());
    }
}
