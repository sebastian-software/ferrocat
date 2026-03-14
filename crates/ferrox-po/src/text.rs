use std::borrow::Cow;

use crate::ParseError;
use crate::scan::{find_byte, find_last_byte, has_byte};

pub fn escape_string(input: &str) -> String {
    if !input.as_bytes().iter().any(|byte| {
        matches!(
            byte,
            b'\x07' | b'\x08' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' | b'"' | b'\\'
        )
    }) {
        return input.to_owned();
    }

    let mut out = String::with_capacity(input.len() + 8);
    for ch in input.chars() {
        match ch {
            '\u{0007}' => out.push_str("\\a"),
            '\u{0008}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{000b}' => out.push_str("\\v"),
            '\u{000c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(ch),
        }
    }

    out
}

pub fn unescape_string(input: &str) -> Result<String, ParseError> {
    if !has_byte(b'\\', input.as_bytes()) {
        return Ok(input.to_owned());
    }

    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut index = 0;

    while index < bytes.len() {
        let next_escape = match find_byte(b'\\', &bytes[index..]) {
            Some(relative) => index + relative,
            None => {
                out.push_str(&input[index..]);
                break;
            }
        };

        out.push_str(&input[index..next_escape]);
        index = next_escape + 1;
        if index >= bytes.len() {
            return Err(ParseError::new("unterminated escape sequence"));
        }

        let escaped = bytes[index];
        match escaped {
            b'a' => out.push('\u{0007}'),
            b'b' => out.push('\u{0008}'),
            b't' => out.push('\t'),
            b'n' => out.push('\n'),
            b'v' => out.push('\u{000b}'),
            b'f' => out.push('\u{000c}'),
            b'r' => out.push('\r'),
            b'\'' => out.push('\''),
            b'"' => out.push('"'),
            b'\\' => out.push('\\'),
            b'?' => out.push('?'),
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
                    Some(ch) => out.push(ch),
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
                    Some(ch) => out.push(ch),
                    None => return Err(ParseError::new("invalid hex escape value")),
                }
                index += 2;
            }
            other => out.push(char::from(other)),
        }

        index += 1;
    }

    Ok(out)
}

pub fn extract_quoted_cow<'a>(line: &'a str) -> Result<Cow<'a, str>, ParseError> {
    let bytes = line.as_bytes();
    let first_quote = match find_byte(b'"', bytes) {
        Some(index) => index,
        None => return Ok(Cow::Borrowed("")),
    };
    let last_quote = match find_last_byte(b'"', bytes) {
        Some(index) if index > first_quote => index,
        _ => return Ok(Cow::Borrowed("")),
    };

    let raw = &line[first_quote + 1..last_quote];
    if !has_byte(b'\\', raw.as_bytes()) {
        return Ok(Cow::Borrowed(raw));
    }

    Ok(Cow::Owned(unescape_string(raw)?))
}

pub fn extract_quoted(line: &str) -> Result<String, ParseError> {
    Ok(extract_quoted_cow(line)?.into_owned())
}

fn decode_hex(byte: u8) -> Result<u8, ParseError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(ParseError::new("invalid hex escape")),
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{escape_string, extract_quoted, extract_quoted_cow, unescape_string};

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
}
