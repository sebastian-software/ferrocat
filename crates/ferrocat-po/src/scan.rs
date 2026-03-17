use memchr::Memchr;

mod backend {
    use memchr::{memchr, memchr3, memrchr};

    #[inline]
    pub fn find_byte(byte: u8, haystack: &[u8]) -> Option<usize> {
        memchr(byte, haystack)
    }

    #[inline]
    pub fn find_last_byte(byte: u8, haystack: &[u8]) -> Option<usize> {
        memrchr(byte, haystack)
    }

    #[cfg(test)]
    #[inline]
    pub fn find_either(first: u8, second: u8, haystack: &[u8]) -> Option<usize> {
        use memchr::memchr2;
        memchr2(first, second, haystack)
    }

    #[inline]
    pub fn find_escapable_byte(haystack: &[u8]) -> Option<usize> {
        find_escapable_byte_impl(haystack)
    }

    #[cfg(target_arch = "aarch64")]
    #[inline]
    fn find_escapable_byte_impl(haystack: &[u8]) -> Option<usize> {
        // Apple Silicon is our primary target, so this is the first place where
        // a dedicated NEON path is worth the maintenance cost.
        unsafe { neon_find_escapable_byte(haystack) }
    }

    #[cfg(not(target_arch = "aarch64"))]
    #[inline]
    fn find_escapable_byte_impl(haystack: &[u8]) -> Option<usize> {
        fallback_find_escapable_byte(haystack)
    }

    #[inline]
    fn fallback_find_escapable_byte(haystack: &[u8]) -> Option<usize> {
        min_option3(
            memchr3(b'"', b'\\', b'\n', haystack),
            memchr3(b'\t', b'\r', b'\x0b', haystack),
            memchr3(b'\x07', b'\x08', b'\x0c', haystack),
        )
    }

    #[inline]
    fn min_option3(
        first: Option<usize>,
        second: Option<usize>,
        third: Option<usize>,
    ) -> Option<usize> {
        min_option(min_option(first, second), third)
    }

    #[inline]
    fn min_option(first: Option<usize>, second: Option<usize>) -> Option<usize> {
        match (first, second) {
            (Some(left), Some(right)) => Some(left.min(right)),
            (Some(value), None) | (None, Some(value)) => Some(value),
            (None, None) => None,
        }
    }

    #[cfg(target_arch = "aarch64")]
    #[target_feature(enable = "neon")]
    unsafe fn neon_find_escapable_byte(haystack: &[u8]) -> Option<usize> {
        use core::arch::aarch64::{vceqq_u8, vdupq_n_u8, vld1q_u8, vmaxvq_u8, vorrq_u8};

        let mut offset = 0usize;
        while offset + 16 <= haystack.len() {
            let matched = unsafe {
                let chunk = vld1q_u8(haystack.as_ptr().add(offset));
                let mut matched = vceqq_u8(chunk, vdupq_n_u8(b'"'));
                matched = vorrq_u8(matched, vceqq_u8(chunk, vdupq_n_u8(b'\\')));
                matched = vorrq_u8(matched, vceqq_u8(chunk, vdupq_n_u8(b'\n')));
                matched = vorrq_u8(matched, vceqq_u8(chunk, vdupq_n_u8(b'\t')));
                matched = vorrq_u8(matched, vceqq_u8(chunk, vdupq_n_u8(b'\r')));
                matched = vorrq_u8(matched, vceqq_u8(chunk, vdupq_n_u8(b'\x0b')));
                matched = vorrq_u8(matched, vceqq_u8(chunk, vdupq_n_u8(b'\x07')));
                matched = vorrq_u8(matched, vceqq_u8(chunk, vdupq_n_u8(b'\x08')));
                vorrq_u8(matched, vceqq_u8(chunk, vdupq_n_u8(b'\x0c')))
            };
            if vmaxvq_u8(matched) != 0 {
                return haystack[offset..offset + 16]
                    .iter()
                    .position(|byte| {
                        matches!(
                            *byte,
                            b'"' | b'\\'
                                | b'\x07'
                                | b'\x08'
                                | b'\t'
                                | b'\n'
                                | b'\x0b'
                                | b'\x0c'
                                | b'\r'
                        )
                    })
                    .map(|index| offset + index);
            }
            offset += 16;
        }

        fallback_find_escapable_byte(&haystack[offset..]).map(|index| offset + index)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    Id,
    IdPlural,
    Str,
    Ctxt,
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
    pub trimmed: &'a [u8],
    pub obsolete: bool,
}

pub struct LineScanner<'a> {
    bytes: &'a [u8],
    newlines: Memchr<'a>,
    offset: usize,
    finished: bool,
}

impl<'a> LineScanner<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            newlines: Memchr::new(b'\n', bytes),
            offset: 0,
            finished: false,
        }
    }
}

impl<'a> Iterator for LineScanner<'a> {
    type Item = Line<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while !self.finished {
            let next_newline = if let Some(index) = self.newlines.next() {
                index
            } else {
                self.finished = true;
                self.bytes.len()
            };
            if self.finished && self.offset == self.bytes.len() {
                return None;
            }
            let raw = &self.bytes[self.offset..next_newline];

            if next_newline < self.bytes.len() {
                self.offset = next_newline + 1;
            } else {
                self.offset = self.bytes.len();
            }

            let mut trimmed = trim_ascii_start(raw);
            if trimmed.is_empty() {
                if next_newline == self.bytes.len() {
                    return None;
                }
                continue;
            }

            let mut obsolete = false;
            if trimmed.starts_with(b"#~") {
                trimmed = trim_ascii_start(&trimmed[2..]);
                obsolete = true;
                if trimmed.is_empty() {
                    if next_newline == self.bytes.len() {
                        return None;
                    }
                    continue;
                }
            }

            return Some(Line { trimmed, obsolete });
        }

        None
    }
}

#[inline]
pub fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let trimmed = trim_ascii_start(bytes);
    let mut end = trimmed.len();

    while end > 0 && trimmed[end - 1].is_ascii_whitespace() {
        end -= 1;
    }

    &trimmed[..end]
}

#[inline]
pub fn trim_ascii_start(bytes: &[u8]) -> &[u8] {
    let mut start = 0;
    while start < bytes.len() && bytes[start].is_ascii_whitespace() {
        start += 1;
    }

    &bytes[start..]
}

#[inline]
pub fn find_byte(byte: u8, haystack: &[u8]) -> Option<usize> {
    backend::find_byte(byte, haystack)
}

#[inline]
pub fn find_last_byte(byte: u8, haystack: &[u8]) -> Option<usize> {
    backend::find_last_byte(byte, haystack)
}

#[cfg(test)]
pub fn find_either(first: u8, second: u8, haystack: &[u8]) -> Option<usize> {
    backend::find_either(first, second, haystack)
}

#[inline]
pub fn has_byte(byte: u8, haystack: &[u8]) -> bool {
    find_byte(byte, haystack).is_some()
}

#[cfg(test)]
pub fn find_quote_or_backslash(haystack: &[u8]) -> Option<usize> {
    find_either(b'"', b'\\', haystack)
}

#[inline]
pub fn find_escapable_byte(haystack: &[u8]) -> Option<usize> {
    backend::find_escapable_byte(haystack)
}

#[inline]
pub fn split_once_byte(haystack: &[u8], needle: u8) -> Option<(&[u8], &[u8])> {
    let index = find_byte(needle, haystack)?;
    Some((&haystack[..index], &haystack[index + 1..]))
}

#[inline]
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
        Some(b'm')
            if line.len() >= 5
                && line[1] == b's'
                && line[2] == b'g'
                && line[3] == b'i'
                && line[4] == b'd' =>
        {
            if line.len() >= 12
                && line[5] == b'_'
                && line[6] == b'p'
                && line[7] == b'l'
                && line[8] == b'u'
                && line[9] == b'r'
                && line[10] == b'a'
                && line[11] == b'l'
            {
                LineKind::Keyword(Keyword::IdPlural)
            } else {
                LineKind::Keyword(Keyword::Id)
            }
        }
        Some(b'm')
            if line.len() >= 6
                && line[1] == b's'
                && line[2] == b'g'
                && line[3] == b's'
                && line[4] == b't'
                && line[5] == b'r' =>
        {
            LineKind::Keyword(Keyword::Str)
        }
        Some(b'm')
            if line.len() >= 7
                && line[1] == b's'
                && line[2] == b'g'
                && line[3] == b'c'
                && line[4] == b't'
                && line[5] == b'x'
                && line[6] == b't' =>
        {
            LineKind::Keyword(Keyword::Ctxt)
        }
        _ => LineKind::Other,
    }
}

#[inline]
pub fn find_quoted_bounds(bytes: &[u8]) -> Option<(usize, usize)> {
    let first_quote = find_byte(b'"', bytes)?;
    let last_quote = find_last_byte(b'"', bytes)?;
    if last_quote > first_quote {
        Some((first_quote + 1, last_quote))
    } else {
        None
    }
}

#[inline]
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
        split_once_byte, trim_ascii, trim_ascii_start,
    };

    #[test]
    fn scans_trimmed_lines_and_obsolete_marker() {
        let input = b"  msgid \"x\"  \n#~ msgstr \"y\"\n\n";
        let mut scanner = LineScanner::new(input);

        let first = scanner.next().expect("first line");
        assert_eq!(first.trimmed, b"msgid \"x\"  ");
        assert!(!first.obsolete);

        let second = scanner.next().expect("second line");
        assert_eq!(second.trimmed, b"msgstr \"y\"");
        assert!(second.obsolete);

        assert!(scanner.next().is_none());
    }

    #[test]
    fn byte_helpers_work() {
        assert_eq!(trim_ascii(b"  abc \t"), b"abc");
        assert_eq!(trim_ascii_start(b"  abc \t"), b"abc \t");
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
            LineKind::Keyword(Keyword::Id)
        );
        assert_eq!(classify_line(br#""continued""#), LineKind::Continuation);
    }
}
