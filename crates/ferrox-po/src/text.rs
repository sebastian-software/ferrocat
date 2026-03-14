use crate::ParseError;

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
    if !input.as_bytes().contains(&b'\\') {
        return Ok(input.to_owned());
    }

    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];
        if byte != b'\\' {
            let next = next_utf8_char_boundary(bytes, index + 1);
            match input.get(index..next) {
                Some(segment) => out.push_str(segment),
                None => {
                    return Err(ParseError::new(
                        "invalid UTF-8 boundary while unescaping string",
                    ));
                }
            }
            index = next;
            continue;
        }

        index += 1;
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

pub fn extract_quoted(line: &str) -> Result<String, ParseError> {
    let first_quote = match line.find('"') {
        Some(index) => index,
        None => return Ok(String::new()),
    };
    let last_quote = match line.rfind('"') {
        Some(index) if index > first_quote => index,
        _ => return Ok(String::new()),
    };
    match line.get(first_quote + 1..last_quote) {
        Some(raw) => unescape_string(raw),
        None => Err(ParseError::new("invalid quote boundaries")),
    }
}

fn decode_hex(byte: u8) -> Result<u8, ParseError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(ParseError::new("invalid hex escape")),
    }
}

fn next_utf8_char_boundary(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && (bytes[index] & 0b1100_0000) == 0b1000_0000 {
        index += 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::{escape_string, extract_quoted, unescape_string};

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
}
