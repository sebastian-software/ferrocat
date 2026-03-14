use std::borrow::Cow;
use std::str;

use crate::scan::{LineScanner, find_byte, split_once_byte, trim_ascii};
use crate::{Header, ParseError, PoFile, PoItem, extract_quoted_cow};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Context {
    MsgId,
    MsgIdPlural,
    MsgStr,
    MsgCtxt,
}

#[derive(Debug)]
struct ParserState {
    item: PoItem,
    context: Option<Context>,
    plural_index: usize,
    obsolete_line_count: usize,
    content_line_count: usize,
    has_keyword: bool,
}

impl ParserState {
    fn new(nplurals: usize) -> Self {
        Self {
            item: PoItem::new(nplurals),
            context: None,
            plural_index: 0,
            obsolete_line_count: 0,
            content_line_count: 0,
            has_keyword: false,
        }
    }

    fn reset(&mut self, nplurals: usize) {
        *self = Self::new(nplurals);
    }
}

#[derive(Debug, Clone, Copy)]
struct BorrowedLine<'a> {
    trimmed: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BorrowedItem<'a> {
    msgid: Cow<'a, str>,
    msgctxt: Option<Cow<'a, str>>,
    msgid_plural: Option<Cow<'a, str>>,
    msgstr: Vec<Cow<'a, str>>,
}

impl<'a> BorrowedItem<'a> {
    fn new() -> Self {
        Self {
            msgid: Cow::Borrowed(""),
            msgctxt: None,
            msgid_plural: None,
            msgstr: Vec::new(),
        }
    }
}

pub fn parse_po(input: &str) -> Result<PoFile, ParseError> {
    let normalized;
    let input = if input.as_bytes().contains(&b'\r') {
        normalized = input.replace("\r\n", "\n").replace('\r', "\n");
        normalized.as_str()
    } else {
        input
    };

    let mut file = PoFile::default();
    file.items.reserve((input.len() / 96).max(1));
    let mut current_nplurals = 2;
    let mut state = ParserState::new(current_nplurals);

    for line in LineScanner::new(input.as_bytes()) {
        if line.obsolete {
            state.obsolete_line_count += 1;
        }

        parse_line(
            BorrowedLine {
                trimmed: line.trimmed,
            },
            &mut state,
            &mut file,
            &mut current_nplurals,
        )?;
    }

    finish_item(&mut state, &mut file, &mut current_nplurals)?;

    Ok(file)
}

fn parse_line(
    line: BorrowedLine<'_>,
    state: &mut ParserState,
    file: &mut PoFile,
    current_nplurals: &mut usize,
) -> Result<(), ParseError> {
    match line.trimmed.first().copied() {
        Some(b'"') => {
            append_continuation(line.trimmed, state)?;
            Ok(())
        }
        Some(b'#') => parse_comment_line(line.trimmed, state, file, current_nplurals),
        Some(b'm') => parse_keyword_line(line.trimmed, state, file, current_nplurals),
        _ => Ok(()),
    }
}

fn parse_comment_line(
    line_bytes: &[u8],
    state: &mut ParserState,
    file: &mut PoFile,
    current_nplurals: &mut usize,
) -> Result<(), ParseError> {
    finish_item(state, file, current_nplurals)?;

    match line_bytes.get(1).copied() {
        Some(b':') => state
            .item
            .references
            .push(trimmed_string(&line_bytes[2..])?),
        Some(b',') => {
            for flag in trimmed_str(&line_bytes[2..])?.split(',') {
                state.item.flags.push(flag.trim().to_owned());
            }
        }
        Some(b'.') => state
            .item
            .extracted_comments
            .push(trimmed_string(&line_bytes[2..])?),
        Some(b'@') => {
            let trimmed = trim_ascii(&line_bytes[2..]);
            if let Some((key_bytes, value_bytes)) = split_once_byte(trimmed, b':') {
                let key = trimmed_str(key_bytes)?;
                if !key.is_empty() {
                    let value = trimmed_str(value_bytes)?;
                    state.item.metadata.push((key.to_owned(), value.to_owned()));
                }
            }
        }
        Some(b' ') | None => state.item.comments.push(trimmed_string(&line_bytes[1..])?),
        _ => {}
    }

    Ok(())
}

fn parse_keyword_line(
    line_bytes: &[u8],
    state: &mut ParserState,
    file: &mut PoFile,
    current_nplurals: &mut usize,
) -> Result<(), ParseError> {
    let line = bytes_to_str(line_bytes)?;
    let mut borrowed = BorrowedItem::new();

    if line_bytes.starts_with(b"msgid_plural") {
        borrowed.msgid_plural = Some(extract_quoted_cow(line)?);
        assign_borrowed_keyword(state, borrowed, Context::MsgIdPlural);
        return Ok(());
    }

    if line_bytes.starts_with(b"msgid") {
        finish_item(state, file, current_nplurals)?;
        borrowed.msgid = extract_quoted_cow(line)?;
        assign_borrowed_keyword(state, borrowed, Context::MsgId);
        return Ok(());
    }

    if line_bytes.starts_with(b"msgstr") {
        let plural_index = parse_plural_index(line_bytes);
        borrowed.msgstr.push(extract_quoted_cow(line)?);
        assign_borrowed_plural(state, borrowed, plural_index);
        return Ok(());
    }

    if line_bytes.starts_with(b"msgctxt") {
        finish_item(state, file, current_nplurals)?;
        borrowed.msgctxt = Some(extract_quoted_cow(line)?);
        assign_borrowed_keyword(state, borrowed, Context::MsgCtxt);
    }

    Ok(())
}

fn assign_borrowed_keyword(state: &mut ParserState, borrowed: BorrowedItem<'_>, context: Context) {
    match context {
        Context::MsgId => state.item.msgid = borrowed.msgid.into_owned(),
        Context::MsgIdPlural => {
            state.item.msgid_plural = borrowed.msgid_plural.map(Cow::into_owned)
        }
        Context::MsgCtxt => state.item.msgctxt = borrowed.msgctxt.map(Cow::into_owned),
        Context::MsgStr => {}
    }
    state.context = Some(context);
    state.content_line_count += 1;
    state.has_keyword = true;
}

fn assign_borrowed_plural(
    state: &mut ParserState,
    borrowed: BorrowedItem<'_>,
    plural_index: usize,
) {
    state.plural_index = plural_index;
    if state.item.msgstr.len() <= plural_index {
        state.item.msgstr.resize(plural_index + 1, String::new());
    }
    if let Some(value) = borrowed.msgstr.into_iter().next() {
        state.item.msgstr[plural_index] = value.into_owned();
    }
    state.context = Some(Context::MsgStr);
    state.content_line_count += 1;
    state.has_keyword = true;
}

fn parse_plural_index(line_bytes: &[u8]) -> usize {
    if line_bytes.get(6) != Some(&b'[') {
        return 0;
    }
    let close = match find_byte(b']', &line_bytes[7..]) {
        Some(offset) => 7 + offset,
        None => return 0,
    };

    match str::from_utf8(&line_bytes[7..close]) {
        Ok(index) => index.parse::<usize>().unwrap_or(0),
        Err(_) => 0,
    }
}

fn append_continuation(line_bytes: &[u8], state: &mut ParserState) -> Result<(), ParseError> {
    state.content_line_count += 1;
    let value = extract_quoted_cow(bytes_to_str(line_bytes)?)?;

    match state.context {
        Some(Context::MsgStr) => {
            if state.item.msgstr.len() <= state.plural_index {
                state
                    .item
                    .msgstr
                    .resize(state.plural_index + 1, String::new());
            }
            state.item.msgstr[state.plural_index].push_str(value.as_ref());
        }
        Some(Context::MsgId) => state.item.msgid.push_str(value.as_ref()),
        Some(Context::MsgIdPlural) => {
            let target = state.item.msgid_plural.get_or_insert_with(String::new);
            target.push_str(value.as_ref());
        }
        Some(Context::MsgCtxt) => {
            let target = state.item.msgctxt.get_or_insert_with(String::new);
            target.push_str(value.as_ref());
        }
        None => {}
    }

    Ok(())
}

fn finish_item(
    state: &mut ParserState,
    file: &mut PoFile,
    current_nplurals: &mut usize,
) -> Result<(), ParseError> {
    if !state.has_keyword {
        return Ok(());
    }

    if state.item.msgid.is_empty() && !is_header_item(&state.item) {
        return Ok(());
    }

    if state.obsolete_line_count >= state.content_line_count && state.content_line_count > 0 {
        state.item.obsolete = true;
    }

    if is_header_item(&state.item) && file.headers.is_empty() && file.items.is_empty() {
        file.comments = core::mem::take(&mut state.item.comments);
        file.extracted_comments = core::mem::take(&mut state.item.extracted_comments);
        parse_headers(
            state
                .item
                .msgstr
                .first()
                .map(String::as_str)
                .unwrap_or_default(),
            &mut file.headers,
        )?;
        *current_nplurals = parse_nplurals(&file.headers).unwrap_or(2);
        state.reset(*current_nplurals);
        return Ok(());
    }

    if state.item.msgstr.is_empty() {
        state.item.msgstr.push(String::new());
    }
    if state.item.msgid_plural.is_some() && state.item.msgstr.is_empty() {
        state
            .item
            .msgstr
            .resize(state.item.nplurals.max(1), String::new());
    }

    state.item.nplurals = *current_nplurals;
    file.items.push(core::mem::take(&mut state.item));
    state.reset(*current_nplurals);
    Ok(())
}

fn is_header_item(item: &PoItem) -> bool {
    item.msgid.is_empty()
        && item.msgctxt.is_none()
        && item.msgid_plural.is_none()
        && !item.msgstr.is_empty()
}

fn parse_headers(raw: &str, out: &mut Vec<Header>) -> Result<(), ParseError> {
    let bytes = raw.as_bytes();
    out.reserve(bytes.iter().filter(|byte| **byte == b'\n').count() + 1);

    for line in LineScanner::new(bytes) {
        if let Some((key_bytes, value_bytes)) = split_once_byte(line.trimmed, b':') {
            out.push(Header {
                key: trimmed_string(key_bytes)?,
                value: trimmed_string(value_bytes)?,
            });
        }
    }

    Ok(())
}

fn parse_nplurals(headers: &[Header]) -> Option<usize> {
    let plural_forms = headers.iter().find_map(|header| {
        if header.key == "Plural-Forms" {
            Some(header.value.as_str())
        } else {
            None
        }
    })?;

    for part in plural_forms.split(';') {
        let trimmed = part.trim();
        if let Some((key, value)) = trimmed.split_once('=')
            && key.trim() == "nplurals"
            && let Ok(parsed) = value.trim().parse::<usize>()
        {
            return Some(parsed);
        }
    }

    None
}

fn bytes_to_str(bytes: &[u8]) -> Result<&str, ParseError> {
    str::from_utf8(bytes).map_err(|_| ParseError::new("invalid UTF-8 in PO input"))
}

fn trimmed_str(bytes: &[u8]) -> Result<&str, ParseError> {
    bytes_to_str(trim_ascii(bytes))
}

fn trimmed_string(bytes: &[u8]) -> Result<String, ParseError> {
    Ok(trimmed_str(bytes)?.to_owned())
}

#[cfg(test)]
mod tests {
    use super::parse_po;

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
            po.items[3].comments,
            vec!["commented obsolete item".to_owned()]
        );
        assert_eq!(po.items[3].flags, vec!["fuzzy".to_owned()]);
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
}
