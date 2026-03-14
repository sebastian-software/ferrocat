mod backend {
    use memchr::{memchr, memchr2, memrchr};

    #[inline]
    pub fn find_byte(byte: u8, haystack: &[u8]) -> Option<usize> {
        memchr(byte, haystack)
    }

    #[inline]
    pub fn find_last_byte(byte: u8, haystack: &[u8]) -> Option<usize> {
        memrchr(byte, haystack)
    }

    #[inline]
    pub fn find_either(first: u8, second: u8, haystack: &[u8]) -> Option<usize> {
        memchr2(first, second, haystack)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    MsgId,
    MsgIdPlural,
    MsgStr,
    MsgCtxt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentKind {
    Translator,
    Reference,
    Flags,
    Extracted,
    Metadata,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Continuation,
    Comment(CommentKind),
    Keyword(Keyword),
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Line<'a> {
    pub raw: &'a [u8],
    pub trimmed: &'a [u8],
    pub obsolete: bool,
}

pub struct LineScanner<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> LineScanner<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }
}

impl<'a> Iterator for LineScanner<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.offset <= self.bytes.len() {
            let next_newline = find_byte(b'\n', &self.bytes[self.offset..])
                .map(|relative| self.offset + relative)
                .unwrap_or(self.bytes.len());
            let raw = &self.bytes[self.offset..next_newline];

            if next_newline == self.bytes.len() {
                self.offset = self.bytes.len() + 1;
            } else {
                self.offset = next_newline + 1;
            }

            let mut trimmed = trim_ascii(raw);
            if trimmed.is_empty() {
                if next_newline == self.bytes.len() {
                    return None;
                }
                continue;
            }

            let mut obsolete = false;
            if trimmed.starts_with(b"#~") {
                trimmed = trim_ascii(&trimmed[2..]);
                obsolete = true;
                if trimmed.is_empty() {
                    if next_newline == self.bytes.len() {
                        return None;
                    }
                    continue;
                }
            }

            return Some(Line {
                raw,
                trimmed,
                obsolete,
            });
        }

        None
    }
}

pub fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = bytes.len();

    while start < end && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    while end > start && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }

    &bytes[start..end]
}

pub fn find_byte(byte: u8, haystack: &[u8]) -> Option<usize> {
    backend::find_byte(byte, haystack)
}

pub fn find_last_byte(byte: u8, haystack: &[u8]) -> Option<usize> {
    backend::find_last_byte(byte, haystack)
}

pub fn find_either(first: u8, second: u8, haystack: &[u8]) -> Option<usize> {
    backend::find_either(first, second, haystack)
}

pub fn has_byte(byte: u8, haystack: &[u8]) -> bool {
    find_byte(byte, haystack).is_some()
}

pub fn find_quote_or_backslash(haystack: &[u8]) -> Option<usize> {
    find_either(b'"', b'\\', haystack)
}

pub fn find_escapable_byte(haystack: &[u8]) -> Option<usize> {
    let mut offset = 0usize;

    while let Some(relative) = find_quote_or_backslash(&haystack[offset..]) {
        return Some(offset + relative);
    }

    while offset < haystack.len() {
        let byte = haystack[offset];
        if matches!(
            byte,
            b'\x07' | b'\x08' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r'
        ) {
            return Some(offset);
        }
        offset += 1;
    }

    None
}

pub fn split_once_byte(haystack: &[u8], needle: u8) -> Option<(&[u8], &[u8])> {
    let index = find_byte(needle, haystack)?;
    Some((&haystack[..index], &haystack[index + 1..]))
}

pub fn classify_line(line: &[u8]) -> LineKind {
    match line.first().copied() {
        Some(b'"') => LineKind::Continuation,
        Some(b'#') => match line.get(1).copied() {
            Some(b':') => LineKind::Comment(CommentKind::Reference),
            Some(b',') => LineKind::Comment(CommentKind::Flags),
            Some(b'.') => LineKind::Comment(CommentKind::Extracted),
            Some(b'@') => LineKind::Comment(CommentKind::Metadata),
            Some(b' ') | None => LineKind::Comment(CommentKind::Translator),
            _ => LineKind::Comment(CommentKind::Other),
        },
        Some(b'm') => {
            if line.starts_with(b"msgid_plural") {
                LineKind::Keyword(Keyword::MsgIdPlural)
            } else if line.starts_with(b"msgid") {
                LineKind::Keyword(Keyword::MsgId)
            } else if line.starts_with(b"msgstr") {
                LineKind::Keyword(Keyword::MsgStr)
            } else if line.starts_with(b"msgctxt") {
                LineKind::Keyword(Keyword::MsgCtxt)
            } else {
                LineKind::Other
            }
        }
        _ => LineKind::Other,
    }
}

pub fn find_quoted_bounds(bytes: &[u8]) -> Option<(usize, usize)> {
    let first_quote = find_byte(b'"', bytes)?;
    let last_quote = find_last_byte(b'"', bytes)?;
    if last_quote > first_quote {
        Some((first_quote + 1, last_quote))
    } else {
        None
    }
}

pub fn parse_plural_index(line: &[u8]) -> Option<usize> {
    if line.get(6) != Some(&b'[') {
        return Some(0);
    }

    let close = find_byte(b']', &line[7..]).map(|offset| 7 + offset)?;
    let value = std::str::from_utf8(&line[7..close]).ok()?;
    value.parse::<usize>().ok()
}

#[cfg(test)]
mod tests {
    use super::{
        CommentKind, Keyword, LineKind, LineScanner, classify_line, find_byte, find_escapable_byte,
        find_last_byte, find_quote_or_backslash, find_quoted_bounds, parse_plural_index,
        split_once_byte, trim_ascii,
    };

    #[test]
    fn scans_trimmed_lines_and_obsolete_marker() {
        let input = b"  msgid \"x\"  \n#~ msgstr \"y\"\n\n";
        let mut scanner = LineScanner::new(input);

        let first = scanner.next().expect("first line");
        assert_eq!(first.trimmed, b"msgid \"x\"");
        assert!(!first.obsolete);

        let second = scanner.next().expect("second line");
        assert_eq!(second.trimmed, b"msgstr \"y\"");
        assert!(second.obsolete);

        assert!(scanner.next().is_none());
    }

    #[test]
    fn byte_helpers_work() {
        assert_eq!(trim_ascii(b"  abc \t"), b"abc");
        assert_eq!(find_byte(b':', b"a:b"), Some(1));
        assert_eq!(find_last_byte(b'"', br#""a" "b""#), Some(6));
        assert_eq!(find_quote_or_backslash(br#"abc\""#), Some(3));
        assert_eq!(split_once_byte(b"a:b", b':'), Some((&b"a"[..], &b"b"[..])));
        assert_eq!(find_quoted_bounds(br#"msgid "abc""#), Some((7, 10)));
        assert_eq!(find_escapable_byte(b"plain\ttext"), Some(5));
        assert_eq!(find_escapable_byte(b"plain\\text"), Some(5));
        assert_eq!(find_escapable_byte(b"plain text"), None);
        assert_eq!(parse_plural_index(b"msgstr[12] \"x\""), Some(12));
        assert_eq!(parse_plural_index(b"msgstr \"x\""), Some(0));
        assert_eq!(
            classify_line(b"#: src/main.rs:1"),
            LineKind::Comment(CommentKind::Reference)
        );
        assert_eq!(
            classify_line(b"msgid \"x\""),
            LineKind::Keyword(Keyword::MsgId)
        );
        assert_eq!(classify_line(br#""continued""#), LineKind::Continuation);
    }
}
