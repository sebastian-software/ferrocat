use memchr::{memchr, memrchr};

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
    memchr(byte, haystack)
}

pub fn find_last_byte(byte: u8, haystack: &[u8]) -> Option<usize> {
    memrchr(byte, haystack)
}

pub fn has_byte(byte: u8, haystack: &[u8]) -> bool {
    find_byte(byte, haystack).is_some()
}

pub fn split_once_byte(haystack: &[u8], needle: u8) -> Option<(&[u8], &[u8])> {
    let index = find_byte(needle, haystack)?;
    Some((&haystack[..index], &haystack[index + 1..]))
}

#[cfg(test)]
mod tests {
    use super::{LineScanner, find_byte, find_last_byte, split_once_byte, trim_ascii};

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
        assert_eq!(split_once_byte(b"a:b", b':'), Some((&b"a"[..], &b"b"[..])));
    }
}
