use std::borrow::Cow;

use memchr::memchr_iter;

use crate::line_state::{PoLineContext, PoLineState};
use crate::scan::{
    CommentKind, Keyword, LineKind, LineScanner, checked_plural_forms, checked_plural_index,
    classify_line, find_byte, find_quoted_bounds, split_once_byte, trim_ascii,
    unrecognized_po_line,
};
use crate::text::{extract_quoted_bytes_cow, for_each_reference_token};
use crate::utf8::input_slice_as_str;
use crate::{Header, MsgStr, ParseError, ParsePosition, PoFile, PoItem};

#[derive(Debug)]
struct ParserState {
    item: PoItem,
    msgstr: MsgStr,
    line: PoLineState,
}

impl ParserState {
    fn new(nplurals: usize) -> Self {
        Self {
            item: PoItem::new(nplurals),
            msgstr: MsgStr::None,
            line: PoLineState::default(),
        }
    }

    fn reset(&mut self, nplurals: usize) {
        self.item.clear_for_reuse(nplurals);
        self.reset_after_take(nplurals);
    }

    fn reset_after_take(&mut self, nplurals: usize) {
        self.item.nplurals = nplurals;
        self.msgstr = MsgStr::None;
        self.line.reset();
    }

    fn set_msgstr(&mut self, plural_index: usize, value: String) {
        match (&mut self.msgstr, plural_index) {
            (MsgStr::None, 0) => self.msgstr = MsgStr::Singular(value),
            (MsgStr::Singular(existing), 0) => *existing = value,
            (MsgStr::Plural(values), 0) => {
                if values.is_empty() {
                    values.push(String::new());
                }
                values[0] = value;
            }
            _ => {
                let msgstr = self.promote_plural_msgstr(plural_index);
                msgstr[plural_index] = value;
            }
        }
    }

    fn append_msgstr(&mut self, plural_index: usize, value: &str) {
        match (&mut self.msgstr, plural_index) {
            (MsgStr::None, 0) => self.msgstr = MsgStr::Singular(value.to_owned()),
            (MsgStr::Singular(existing), 0) => existing.push_str(value),
            (MsgStr::Plural(values), 0) => {
                if values.is_empty() {
                    values.push(String::new());
                }
                values[0].push_str(value);
            }
            _ => {
                let msgstr = self.promote_plural_msgstr(plural_index);
                msgstr[plural_index].push_str(value);
            }
        }
    }

    fn header_msgstr(&self) -> &str {
        self.msgstr.first().unwrap_or_default()
    }

    fn materialize_msgstr(&mut self) {
        debug_assert!(self.item.msgstr.is_empty());
        self.item.msgstr = core::mem::take(&mut self.msgstr);
    }

    fn promote_plural_msgstr(&mut self, plural_index: usize) -> &mut Vec<String> {
        if !matches!(self.msgstr, MsgStr::Plural(_)) {
            self.msgstr = match core::mem::take(&mut self.msgstr) {
                MsgStr::None => MsgStr::Plural(Vec::with_capacity(2)),
                MsgStr::Singular(value) => {
                    let mut values = Vec::with_capacity(2);
                    values.push(value);
                    MsgStr::Plural(values)
                }
                MsgStr::Plural(values) => MsgStr::Plural(values),
            };
        }
        let MsgStr::Plural(msgstr) = &mut self.msgstr else {
            unreachable!("plural msgstr promotion must yield plural storage");
        };
        if msgstr.len() <= plural_index {
            msgstr.resize(plural_index + 1, String::new());
        }
        msgstr
    }
}

#[derive(Debug, Clone, Copy)]
struct BorrowedLine<'a> {
    trimmed: &'a [u8],
    obsolete: bool,
    position: ParsePosition,
}

/// Parses PO content into the owned [`PoFile`] representation.
///
/// LF, CRLF, and bare CR line endings are accepted, and the UTF-8 BOM is
/// ignored when present.
///
/// # Errors
///
/// Returns [`ParseError`] when the input has invalid PO syntax, a declared
/// plural width above 128, or a zero-based plural index above 127.
pub fn parse_po(input: &str) -> Result<PoFile, ParseError> {
    let input = strip_utf8_bom(input);

    let mut file = PoFile::default();
    file.items.reserve((input.len() / 96).max(1));
    let mut current_nplurals = 2;
    let mut state = ParserState::new(current_nplurals);

    let mut last_position = ParsePosition::new(0, 1, 1);
    for line in LineScanner::new(input.as_bytes()) {
        last_position = line.position;
        parse_line(
            BorrowedLine {
                trimmed: line.trimmed,
                obsolete: line.obsolete,
                position: line.position,
            },
            &mut state,
            &mut file,
            &mut current_nplurals,
        )?;
    }

    finish_item(&mut state, &mut file, &mut current_nplurals, last_position)?;

    Ok(file)
}

/// Parses UTF-8 PO bytes into the owned [`PoFile`] representation.
///
/// This is the byte-oriented companion to [`parse_po`]. It rejects declared
/// non-UTF-8 PO charsets before decoding, validates the input bytes as UTF-8,
/// then delegates to [`parse_po`] for syntax parsing.
///
/// # Errors
///
/// Returns [`ParseError`] for unsupported declared charsets, invalid UTF-8,
/// invalid PO syntax, or input that exceeds the limit of 128 plural slots
/// per entry.
pub fn parse_po_bytes(input: &[u8]) -> Result<PoFile, ParseError> {
    reject_unsupported_declared_charset(input)?;

    let input = std::str::from_utf8(input).map_err(|error| {
        ParseError::new(format!(
            "PO input is not valid UTF-8 at byte {}",
            error.valid_up_to()
        ))
    })?;

    parse_po(input)
}

#[inline]
fn strip_utf8_bom(input: &str) -> &str {
    input.strip_prefix('\u{feff}').unwrap_or(input)
}

fn reject_unsupported_declared_charset(input: &[u8]) -> Result<(), ParseError> {
    let Some(charset) = declared_charset(input) else {
        return Ok(());
    };

    if charset.eq_ignore_ascii_case("utf-8") || charset.eq_ignore_ascii_case("utf8") {
        return Ok(());
    }

    Err(ParseError::new(format!(
        "unsupported PO charset `{charset}`; parse_po_bytes accepts UTF-8 input"
    )))
}

fn declared_charset(input: &[u8]) -> Option<Cow<'_, str>> {
    let input = input.strip_prefix(b"\xef\xbb\xbf").unwrap_or(input);
    let mut saw_empty_msgid = false;
    let mut in_header_msgstr = false;
    let mut current_line = Vec::new();

    for line in LineScanner::new(input) {
        if line.obsolete {
            break;
        }
        match classify_line(line.trimmed) {
            LineKind::Comment(_) if !saw_empty_msgid => {}
            LineKind::Keyword(Keyword::Id) if !saw_empty_msgid => {
                let (start, end) = find_quoted_bounds(line.trimmed)?;
                if start != end {
                    return None;
                }
                saw_empty_msgid = true;
            }
            LineKind::Continuation if saw_empty_msgid && !in_header_msgstr => {
                let (start, end) = find_quoted_bounds(line.trimmed)?;
                if start != end {
                    return None;
                }
            }
            LineKind::Keyword(Keyword::Str) if saw_empty_msgid && !in_header_msgstr => {
                in_header_msgstr = true;
                if let Some(charset) = scan_header_fragment(line.trimmed, &mut current_line) {
                    return Some(charset);
                }
            }
            LineKind::Continuation if in_header_msgstr => {
                if let Some(charset) = scan_header_fragment(line.trimmed, &mut current_line) {
                    return Some(charset);
                }
            }
            _ => break,
        }
    }

    if in_header_msgstr && !current_line.is_empty() {
        return charset_from_header_line(&current_line).map(|value| Cow::Owned(value.to_owned()));
    }

    None
}

fn scan_header_fragment<'a>(line: &'a [u8], current_line: &mut Vec<u8>) -> Option<Cow<'a, str>> {
    let (start, end) = find_quoted_bounds(line)?;
    let raw = &line[start..end];
    let mut index = 0usize;

    while index < raw.len() {
        let Some(relative) = find_byte(b'\\', &raw[index..]) else {
            current_line.extend_from_slice(&raw[index..]);
            break;
        };
        let pos = index + relative;
        let segment = &raw[index..pos];
        let Some(&escaped) = raw.get(pos + 1) else {
            current_line.extend_from_slice(segment);
            current_line.push(b'\\');
            break;
        };

        match escaped {
            b'n' | b'r' => {
                if current_line.is_empty() {
                    if let Some(charset) = charset_from_header_line(segment) {
                        return Some(Cow::Borrowed(charset));
                    }
                } else {
                    current_line.extend_from_slice(segment);
                    if let Some(charset) = charset_from_header_line(current_line) {
                        return Some(Cow::Owned(charset.to_owned()));
                    }
                    current_line.clear();
                }
            }
            b't' => {
                current_line.extend_from_slice(segment);
                current_line.push(b'\t');
            }
            other => {
                current_line.extend_from_slice(segment);
                current_line.push(other);
            }
        }

        index = pos + 2;
    }

    None
}

fn charset_from_header_line(line: &[u8]) -> Option<&str> {
    let (key, value) = split_once_byte(trim_ascii(line), b':')?;
    if !trim_ascii(key).eq_ignore_ascii_case(b"content-type") {
        return None;
    }

    for part in value.split(|&byte| byte == b';') {
        let Some((name, raw_value)) = split_once_byte(part, b'=') else {
            continue;
        };
        if !trim_ascii(name).eq_ignore_ascii_case(b"charset") {
            continue;
        }
        let raw_value = trim_ascii(raw_value);
        let unquoted = raw_value
            .strip_prefix(b"\"")
            .or_else(|| raw_value.strip_prefix(b"'"))
            .unwrap_or(raw_value);
        let value_end = unquoted
            .iter()
            .position(|byte| matches!(byte, b'"' | b'\'' | b'\\') || byte.is_ascii_whitespace())
            .unwrap_or(unquoted.len());
        if value_end > 0 {
            return std::str::from_utf8(&unquoted[..value_end]).ok();
        }
    }

    None
}

fn parse_line(
    line: BorrowedLine<'_>,
    state: &mut ParserState,
    file: &mut PoFile,
    current_nplurals: &mut usize,
) -> Result<(), ParseError> {
    match classify_line(line.trimmed) {
        LineKind::Continuation => {
            append_continuation(line.trimmed, line.obsolete, line.position, state)?;
            Ok(())
        }
        LineKind::Comment(kind) => parse_comment_line(
            line.trimmed,
            kind,
            state,
            file,
            current_nplurals,
            line.position,
        ),
        LineKind::Keyword(keyword) => parse_keyword_line(
            line.trimmed,
            line.obsolete,
            line.position,
            keyword,
            state,
            file,
            current_nplurals,
        ),
        LineKind::Other => Err(unrecognized_po_line(line.position)),
    }
}

fn parse_comment_line(
    line_bytes: &[u8],
    kind: CommentKind,
    state: &mut ParserState,
    file: &mut PoFile,
    current_nplurals: &mut usize,
    position: ParsePosition,
) -> Result<(), ParseError> {
    finish_item(state, file, current_nplurals, position)?;

    match kind {
        CommentKind::Reference => {
            let reference_line = trimmed_str(&line_bytes[2..]);
            for_each_reference_token(reference_line, |token| {
                state.item.references.push(token.into_owned());
            });
        }
        CommentKind::Flags => {
            for flag in trimmed_str(&line_bytes[2..]).split(',') {
                state.item.flags.push(flag.trim().to_owned());
            }
        }
        CommentKind::Extracted => state
            .item
            .extracted_comments
            .push(trimmed_string(&line_bytes[2..])),
        CommentKind::Metadata => {
            let trimmed = trim_ascii(&line_bytes[2..]);
            if let Some(value_bytes) = ferrocat_mt_metadata_value(trimmed) {
                state.item.metadata.push((
                    "ferrocat-mt".to_owned(),
                    trimmed_str(value_bytes).to_owned(),
                ));
            } else if let Some((key_bytes, value_bytes)) = split_once_byte(trimmed, b':') {
                let key = trimmed_str(key_bytes);
                if !key.is_empty() {
                    let value = trimmed_str(value_bytes);
                    state.item.metadata.push((key.to_owned(), value.to_owned()));
                }
            }
        }
        CommentKind::Translator => state.item.comments.push(trimmed_string(&line_bytes[1..])),
        CommentKind::Other => {}
    }
    Ok(())
}

fn ferrocat_mt_metadata_value(trimmed: &[u8]) -> Option<&[u8]> {
    const KEY: &[u8] = b"ferrocat-mt";
    let rest = trimmed.strip_prefix(KEY)?;
    rest.first()
        .is_some_and(u8::is_ascii_whitespace)
        .then(|| trim_ascii(rest))
}

fn parse_keyword_line(
    line_bytes: &[u8],
    obsolete: bool,
    position: ParsePosition,
    keyword: Keyword,
    state: &mut ParserState,
    file: &mut PoFile,
    current_nplurals: &mut usize,
) -> Result<(), ParseError> {
    match keyword {
        Keyword::IdPlural => {
            state
                .line
                .mark_keyword(PoLineContext::IdPlural, 0, obsolete);
            state.item.msgid_plural = Some(
                at_line_position(extract_quoted_bytes_cow(line_bytes), position)?.into_owned(),
            );
        }
        Keyword::Id => {
            finish_item(state, file, current_nplurals, position)?;
            state.line.mark_keyword(PoLineContext::Id, 0, obsolete);
            state.item.msgid =
                at_line_position(extract_quoted_bytes_cow(line_bytes), position)?.into_owned();
        }
        Keyword::Str => {
            let plural_index = checked_plural_index(line_bytes, position)?;
            state
                .line
                .mark_keyword(PoLineContext::Str, plural_index, obsolete);
            state.set_msgstr(
                plural_index,
                at_line_position(extract_quoted_bytes_cow(line_bytes), position)?.into_owned(),
            );
        }
        Keyword::Ctxt => {
            finish_item(state, file, current_nplurals, position)?;
            state.line.mark_keyword(PoLineContext::Ctxt, 0, obsolete);
            state.item.msgctxt = Some(
                at_line_position(extract_quoted_bytes_cow(line_bytes), position)?.into_owned(),
            );
        }
    }

    Ok(())
}

fn append_continuation(
    line_bytes: &[u8],
    obsolete: bool,
    position: ParsePosition,
    state: &mut ParserState,
) -> Result<(), ParseError> {
    state.line.mark_continuation(obsolete);
    let value = at_line_position(extract_quoted_bytes_cow(line_bytes), position)?;

    match state.line.context() {
        Some(PoLineContext::Str) => {
            state.append_msgstr(state.line.plural_index(), value.as_ref());
        }
        Some(PoLineContext::Id) => state.item.msgid.push_str(value.as_ref()),
        Some(PoLineContext::IdPlural) => {
            let target = state.item.msgid_plural.get_or_insert_with(String::new);
            target.push_str(value.as_ref());
        }
        Some(PoLineContext::Ctxt) => {
            let target = state.item.msgctxt.get_or_insert_with(String::new);
            target.push_str(value.as_ref());
        }
        None => {}
    }

    Ok(())
}

#[inline]
fn at_line_position<T>(
    result: Result<T, ParseError>,
    position: ParsePosition,
) -> Result<T, ParseError> {
    result.map_err(|error| error.with_position_if_missing(position))
}

fn finish_item(
    state: &mut ParserState,
    file: &mut PoFile,
    current_nplurals: &mut usize,
    position: ParsePosition,
) -> Result<(), ParseError> {
    if !state.line.has_keyword() {
        return Ok(());
    }

    if state.item.msgid.is_empty() && !is_header_state(state) {
        return Ok(());
    }

    if state.line.is_obsolete_item() {
        state.item.obsolete = true;
    }

    if is_header_state(state) && file.headers.is_empty() && file.items.is_empty() {
        file.comments = core::mem::take(&mut state.item.comments).into_vec();
        file.extracted_comments = core::mem::take(&mut state.item.extracted_comments).into_vec();
        parse_headers(state.header_msgstr(), &mut file.headers);
        *current_nplurals = match file
            .headers
            .iter()
            .find(|header| header.key == "Plural-Forms")
        {
            Some(header) => checked_plural_forms(&header.value, position)?.unwrap_or(2),
            None => 2,
        };
        state.reset(*current_nplurals);
        return Ok(());
    }

    state.materialize_msgstr();

    if state.item.msgstr.is_empty() {
        state.item.msgstr = MsgStr::Singular(String::new());
    }
    if state.item.msgid_plural.is_some() && state.item.msgstr.len() == 1 {
        let mut values = core::mem::take(&mut state.item.msgstr).into_vec();
        values.resize(state.item.nplurals.max(1), String::new());
        state.item.msgstr = MsgStr::Plural(values);
    }

    state.item.nplurals = *current_nplurals;
    file.items.push(core::mem::take(&mut state.item));
    state.reset_after_take(*current_nplurals);
    Ok(())
}

fn is_header_state(state: &ParserState) -> bool {
    state.item.msgid.is_empty()
        && state.item.msgctxt.is_none()
        && state.item.msgid_plural.is_none()
        && !state.msgstr.is_empty()
}

fn parse_headers(raw: &str, out: &mut Vec<Header>) {
    let bytes = raw.as_bytes();
    out.reserve(memchr_iter(b'\n', bytes).count() + 1);

    for line in LineScanner::new(bytes) {
        if let Some((key_bytes, value_bytes)) = split_once_byte(line.trimmed, b':') {
            out.push(Header {
                key: trimmed_string(key_bytes),
                value: trimmed_string(value_bytes),
            });
        }
    }
}

fn bytes_to_str(bytes: &[u8]) -> &str {
    input_slice_as_str(bytes)
}

fn trimmed_str(bytes: &[u8]) -> &str {
    bytes_to_str(trim_ascii(bytes))
}

fn trimmed_string(bytes: &[u8]) -> String {
    trimmed_str(bytes).to_owned()
}

#[cfg(test)]
mod tests {
    use super::{declared_charset, parse_po, parse_po_bytes};

    const MULTI_LINE: &str = r#"# French translation of Link (6.x-2.9)
# Copyright (c) 2011 by the French translation team
#
## Plural-Forms by polish translation team to demonstrate multi-line ##
#
msgid ""
msgstr ""
"Project-Id-Version: Link (6.x-2.9)\n"
"POT-Creation-Date: 2011-12-31 23:39+0000\n"
"PO-Revision-Date: 2013-12-17 14:21+0100\n"
"Language-Team: French\n"
"MIME-Version: 1.0\n"
"Content-Type: text/plain; charset=UTF-8\n"
"Content-Transfer-Encoding: 8bit\n"
"Plural-Forms: nplurals=3; plural=n==1 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 "
"|| n%100>=20) ? 1 : 2;\n"
"Last-Translator: Ruben Vermeersch <ruben@rocketeer.be>\n"
"Language: fr\n"
"X-Generator: Poedit 1.6.2\n"

msgid ""
"The following placeholder tokens can be used in both paths and titles. When "
"used in a path or title, they will be replaced with the appropriate values."
msgstr ""
"Les ébauches de jetons suivantes peuvent être utilisées à la fois dans les "
"chemins et in the titles. Lorsqu'elles sont utilisées dans un chemin ou un "
"titre, elles seront remplacées par les valeurs appropriées."
"#;

    const COMMENTED: &str = r#"msgid ""
msgstr ""
"Project-Id-Version: Test\n"
"Plural-Forms: nplurals=2; plural=(n != 1);\n"

#: .tmp/ui/settings/views/console-modal.html
msgid "{{dataLoader.data.length}} results"
msgstr "{{dataLoader.data.length}} resultaten"

#~ msgid "Add order"
#~ msgstr "Order toevoegen"

#~ # commented obsolete item
#~ #, fuzzy
#~ msgid "Commented item"
#~ msgstr "not sure"

# commented obsolete item
#, fuzzy
#~ msgid "Second commented item"
#~ msgstr "also not sure"
"#;

    const C_STRINGS: &str = r#"msgid ""
msgstr ""
"Plural-Forms: nplurals=2; plural=(n > 1);\n"

msgid "The name field must not contain characters like \" or \\"
msgstr ""

msgid ""
"%1$s\n"
"%2$s %3$s\n"
"%4$s\n"
"%5$s"
msgstr ""

msgid ""
"define('some/test/module', function () {\n"
"\t'use strict';\n"
"\treturn {};\n"
"});\n"
""
msgstr ""
"#;

    #[test]
    fn parses_multiline_headers_and_items() {
        let po = match parse_po(MULTI_LINE) {
            Ok(value) => value,
            Err(error) => panic!("parse failed: {error}"),
        };

        assert_eq!(po.headers[6].key, "Content-Transfer-Encoding");
        assert_eq!(
            po.headers
                .iter()
                .find(|header| header.key == "Plural-Forms")
                .map(|header| header.value.as_str()),
            Some(
                "nplurals=3; plural=n==1 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2;"
            )
        );
        assert_eq!(po.items.len(), 1);
        assert_eq!(
            po.items[0].msgid,
            "The following placeholder tokens can be used in both paths and titles. When used in a path or title, they will be replaced with the appropriate values."
        );
    }

    #[test]
    fn parses_c_string_escapes_and_multiline_values() {
        let po = match parse_po(C_STRINGS) {
            Ok(value) => value,
            Err(error) => panic!("parse failed: {error}"),
        };

        assert_eq!(
            po.items[0].msgid,
            "The name field must not contain characters like \" or \\"
        );
        assert_eq!(po.items[1].msgid, "%1$s\n%2$s %3$s\n%4$s\n%5$s");
        assert_eq!(
            po.items[2].msgid,
            "define('some/test/module', function () {\n\t'use strict';\n\treturn {};\n});\n"
        );
    }

    #[test]
    fn parse_errors_include_line_position() {
        let error = parse_po("msgid \"ok\"\n  msgstr \"bad\"quote\"\n")
            .expect_err("unescaped quote should fail");
        let position = error.position().expect("position metadata");

        assert_eq!(error.message(), "unescaped quote in string literal");
        assert_eq!(position.offset(), 13);
        assert_eq!(position.line(), 2);
        assert_eq!(position.column(), 3);
    }

    #[test]
    fn rejects_unrecognized_lines() {
        let error =
            parse_po("msgid \"ok\"\nmsgstr_ \"typo\"\n").expect_err("unknown PO line should fail");
        let position = error.position().expect("position metadata");

        assert_eq!(error.message(), "unrecognized PO syntax");
        assert_eq!(position.line(), 2);
        assert_eq!(position.column(), 1);
    }

    #[test]
    fn parses_obsolete_items() {
        let po = match parse_po(COMMENTED) {
            Ok(value) => value,
            Err(error) => panic!("parse failed: {error}"),
        };

        assert_eq!(po.items.len(), 4);
        assert!(!po.items[0].obsolete);
        assert!(po.items[1].obsolete);
        assert!(po.items[2].obsolete);
        assert!(po.items[3].obsolete);
        assert_eq!(
            po.items[3].comments.as_slice(),
            vec!["commented obsolete item".to_owned()].as_slice()
        );
        assert_eq!(
            po.items[3].flags.as_slice(),
            vec!["fuzzy".to_owned()].as_slice()
        );
    }

    #[test]
    fn parses_context_without_creating_phantom_items() {
        let input = r#"msgid ""
msgstr ""
"Language: de\n"

msgctxt "menu"
msgid "File"
msgstr "Datei"
"#;

        let po = match parse_po(input) {
            Ok(value) => value,
            Err(error) => panic!("parse failed: {error}"),
        };

        assert_eq!(po.items.len(), 1);
        assert_eq!(po.items[0].msgctxt.as_deref(), Some("menu"));
        assert_eq!(po.items[0].msgid, "File");
    }

    #[test]
    fn strips_utf8_bom_prefix() {
        let input = "\u{feff}msgid \"foo\"\nmsgstr \"bar\"\n";
        let po = parse_po(input).expect("parse");

        assert_eq!(po.items.len(), 1);
        assert_eq!(po.items[0].msgid, "foo");
        assert_eq!(po.items[0].msgstr[0], "bar");
    }

    #[test]
    fn parse_po_bytes_accepts_utf8_po_content() {
        let input = b"msgid \"foo\"\nmsgstr \"bar\"\n";
        let po = parse_po_bytes(input).expect("parse bytes");

        assert_eq!(po.items.len(), 1);
        assert_eq!(po.items[0].msgid, "foo");
        assert_eq!(po.items[0].msgstr[0], "bar");
    }

    #[test]
    fn parse_po_bytes_accepts_declared_utf8_charset() {
        let input = b"msgid \"\"\nmsgstr \"\"\n\"Content-Type: text/plain; charset=UTF-8\\n\"\n\n";
        let po = parse_po_bytes(input).expect("parse bytes");

        assert_eq!(po.headers[0].key, "Content-Type");
        assert_eq!(po.headers[0].value, "text/plain; charset=UTF-8");
    }

    #[test]
    fn parse_po_bytes_rejects_declared_non_utf8_charset() {
        let input =
            b"msgid \"\"\nmsgstr \"\"\n\"Content-Type: text/plain; charset=ISO-8859-1\\n\"\n\n";
        let error = parse_po_bytes(input).expect_err("non-utf8 charset should fail");

        assert!(error.message().contains("unsupported PO charset"));
        assert!(error.message().contains("ISO-8859-1"));
    }

    #[test]
    fn parse_po_bytes_reports_invalid_utf8_input() {
        let input = b"msgid \"caf\xe9\"\nmsgstr \"\"\n";
        let error = parse_po_bytes(input).expect_err("invalid utf8 should fail");

        assert!(error.message().contains("not valid UTF-8"));
        assert!(error.message().contains("byte 10"));
    }

    #[test]
    fn parse_po_bytes_reports_declared_charset_before_utf8_error() {
        let input = b"msgid \"\"\nmsgstr \"\"\n\"Content-Type: text/plain; charset=ISO-8859-1\\n\"\n\nmsgid \"caf\xe9\"\nmsgstr \"\"\n";
        let error = parse_po_bytes(input).expect_err("declared charset should fail first");

        assert!(error.message().contains("unsupported PO charset"));
        assert!(error.message().contains("ISO-8859-1"));
    }

    #[test]
    fn declared_charset_reads_header_value_case_insensitively() {
        let input = b"msgid \"\"\nmsgstr \"\"\n\"Content-Type: text/plain; CHARSET=utf8\\n\"\n\n";

        assert_eq!(declared_charset(input).as_deref(), Some("utf8"));
    }

    #[test]
    fn declared_charset_ignores_non_header_text() {
        let input = b"msgid \"charset=ISO-8859-1\"\nmsgstr \"\"\n";

        assert_eq!(declared_charset(input), None);
    }

    #[test]
    fn parse_po_bytes_assembles_split_charset_and_matches_content_type_by_name() {
        let split_utf8 = b"msgid \"\"\nmsgstr \"\"\n\"Content-Type: text/plain; charset=UT\"\n\"F-8\\n\"\n\nmsgid \"Hello\"\nmsgstr \"Hallo\"\n";
        let parsed = parse_po_bytes(split_utf8).expect("split UTF-8 charset must parse");
        assert_eq!(parsed.headers[0].key, "Content-Type");
        assert_eq!(parsed.headers[0].value, "text/plain; charset=UTF-8");
        assert_eq!(parsed.items[0].msgstr[0], "Hallo");

        let x_notes = b"msgid \"\"\nmsgstr \"\"\n\"X-Notes: Content-Type: text/plain; \"\n\"charset=ISO-8859-1\\n\"\n\"Content-Type: text/plain; charset=UTF-8\\n\"\n\nmsgid \"Hello\"\nmsgstr \"Hallo\"\n";
        let parsed = parse_po_bytes(x_notes)
            .expect("Content-Type text inside X-Notes must not trigger charset rejection");
        assert_eq!(parsed.headers.len(), 2);
        assert_eq!(parsed.headers[0].key, "X-Notes");
        assert_eq!(parsed.headers[1].key, "Content-Type");

        let split_header_name =
            b"msgid \"\"\nmsgstr \"\"\n\"Content-\"\n\"Type: text/plain;\\tcharset=\\\"ISO-8859-1\\\"\"\n";
        let error = parse_po_bytes(split_header_name)
            .expect_err("split non-UTF-8 Content-Type without trailing newline should fail");
        assert!(error.message().contains("ISO-8859-1"));

        let other_param_charset = b"msgid \"\"\nmsgstr \"\"\n\"Content-Type: text/plain; x-charset=ISO-8859-1; charset=UTF-8\\n\"\n";
        assert_eq!(
            declared_charset(other_param_charset).as_deref(),
            Some("UTF-8")
        );
    }

    #[test]
    fn parse_po_bytes_scopes_declared_charset_to_leading_header() {
        let headerless = b"msgid \"Content-Type: text/plain; charset=ISO-8859-1\"\nmsgstr \"\"\n";
        let parsed = parse_po_bytes(headerless).expect("headerless message content must parse");
        assert_eq!(
            parsed.items[0].msgid,
            "Content-Type: text/plain; charset=ISO-8859-1"
        );

        let header_without_ct = b"msgid \"\"\nmsgstr \"\"\n\"Language: de\\n\"\n\nmsgid \"Content-Type: text/plain; charset=ISO-8859-1\"\nmsgstr \"\"\n";
        let parsed = parse_po_bytes(header_without_ct)
            .expect("non-header Content-Type must not trigger charset rejection");
        assert_eq!(parsed.items.len(), 1);

        let bom_non_utf8 =
            b"\xef\xbb\xbf# comment \xa9\nmsgid \"\"\n\"\"\nmsgstr \"\"\n\"Content-Type: text/plain; \"\n\"charset=ISO-8859-1\\n\"\n\nmsgid \"caf\xe9\"\nmsgstr \"\"\n";
        let error =
            parse_po_bytes(bom_non_utf8).expect_err("multiline header charset should be rejected");
        assert!(error.message().contains("ISO-8859-1"));

        assert_eq!(
            declared_charset(b"#~ msgid \"\"\n#~ msgstr \"Content-Type: charset=ISO-8859-1\\n\"\n"),
            None
        );
        assert_eq!(
            declared_charset(
                b"msgid \"\"\n\"non-empty\"\nmsgstr \"Content-Type: charset=ISO-8859-1\\n\"\n"
            ),
            None
        );
        assert_eq!(
            declared_charset(b"msgid \"\"\nmsgstr \"Content-Type: text/plain; charset=\\n\"\n"),
            None
        );
        assert_eq!(
            declared_charset(
                b"msgid \"\"\nmsgstr \"X-Test: a\\\\nb\\nContent-Type: text/plain; charset=ISO-8859-1\\n\"\n"
            )
            .as_deref(),
            Some("ISO-8859-1")
        );
    }

    #[test]
    fn rejects_unescaped_quote_sequences() {
        let input = "msgid \"Some msgid with \\\"double\\\" quotes\"\nmsgstr \"\"\n\"Some msgstr with \"double\\\" quotes\"\n";
        let error = parse_po(input).expect_err("invalid quote pattern should fail");

        assert!(error.to_string().contains("unescaped"));
    }
}
