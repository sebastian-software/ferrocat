use std::borrow::Cow;

use memchr::memchr_iter;

use crate::scan::{
    CommentKind, Keyword, LineKind, LineScanner, classify_line, parse_plural_index,
    split_once_byte, trim_ascii,
};
use crate::text::{extract_quoted_bytes_cow, split_reference_comment};
use crate::utf8::input_slice_as_str;
use crate::{Header, MsgStr, ParseError, PoFile, PoItem};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Context {
    Id,
    IdPlural,
    Str,
    Ctxt,
}

#[derive(Debug)]
struct ParserState {
    item: PoItem,
    msgstr: MsgStr,
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
            msgstr: MsgStr::None,
            context: None,
            plural_index: 0,
            obsolete_line_count: 0,
            content_line_count: 0,
            has_keyword: false,
        }
    }

    fn reset(&mut self, nplurals: usize) {
        self.item.clear_for_reuse(nplurals);
        self.reset_after_take(nplurals);
    }

    fn reset_after_take(&mut self, nplurals: usize) {
        self.item.nplurals = nplurals;
        self.msgstr = MsgStr::None;
        self.context = None;
        self.plural_index = 0;
        self.obsolete_line_count = 0;
        self.content_line_count = 0;
        self.has_keyword = false;
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
        self.msgstr.first_str().unwrap_or_default()
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
}

/// Parses PO content into the owned [`PoFile`] representation.
///
/// Line endings are normalized before parsing, and the UTF-8 BOM is ignored
/// when present.
///
/// # Errors
///
/// Returns [`ParseError`] when the input is not valid PO syntax.
pub fn parse_po(input: &str) -> Result<PoFile, ParseError> {
    let input = strip_utf8_bom(input);
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
        parse_line(
            BorrowedLine {
                trimmed: line.trimmed,
                obsolete: line.obsolete,
            },
            &mut state,
            &mut file,
            &mut current_nplurals,
        )?;
    }

    finish_item(&mut state, &mut file, &mut current_nplurals);

    Ok(file)
}

#[inline]
fn strip_utf8_bom(input: &str) -> &str {
    input.strip_prefix('\u{feff}').unwrap_or(input)
}

fn parse_line(
    line: BorrowedLine<'_>,
    state: &mut ParserState,
    file: &mut PoFile,
    current_nplurals: &mut usize,
) -> Result<(), ParseError> {
    match classify_line(line.trimmed) {
        LineKind::Continuation => {
            append_continuation(line.trimmed, line.obsolete, state)?;
            Ok(())
        }
        LineKind::Comment(kind) => {
            parse_comment_line(line.trimmed, kind, state, file, current_nplurals);
            Ok(())
        }
        LineKind::Keyword(keyword) => parse_keyword_line(
            line.trimmed,
            line.obsolete,
            keyword,
            state,
            file,
            current_nplurals,
        ),
        LineKind::Other => Ok(()),
    }
}

fn parse_comment_line(
    line_bytes: &[u8],
    kind: CommentKind,
    state: &mut ParserState,
    file: &mut PoFile,
    current_nplurals: &mut usize,
) {
    finish_item(state, file, current_nplurals);

    match kind {
        CommentKind::Reference => {
            let reference_line = trimmed_str(&line_bytes[2..]);
            state.item.references.extend(
                split_reference_comment(reference_line)
                    .into_iter()
                    .map(Cow::into_owned),
            );
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
            if let Some((key_bytes, value_bytes)) = split_once_byte(trimmed, b':') {
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
}

fn parse_keyword_line(
    line_bytes: &[u8],
    obsolete: bool,
    keyword: Keyword,
    state: &mut ParserState,
    file: &mut PoFile,
    current_nplurals: &mut usize,
) -> Result<(), ParseError> {
    match keyword {
        Keyword::IdPlural => {
            state.obsolete_line_count += usize::from(obsolete);
            state.item.msgid_plural = Some(extract_quoted_bytes_cow(line_bytes)?.into_owned());
            state.context = Some(Context::IdPlural);
            state.content_line_count += 1;
            state.has_keyword = true;
        }
        Keyword::Id => {
            finish_item(state, file, current_nplurals);
            state.obsolete_line_count += usize::from(obsolete);
            state.item.msgid = extract_quoted_bytes_cow(line_bytes)?.into_owned();
            state.context = Some(Context::Id);
            state.content_line_count += 1;
            state.has_keyword = true;
        }
        Keyword::Str => {
            let plural_index = parse_plural_index(line_bytes).unwrap_or(0);
            state.plural_index = plural_index;
            state.obsolete_line_count += usize::from(obsolete);
            state.set_msgstr(
                plural_index,
                extract_quoted_bytes_cow(line_bytes)?.into_owned(),
            );
            state.context = Some(Context::Str);
            state.content_line_count += 1;
            state.has_keyword = true;
        }
        Keyword::Ctxt => {
            finish_item(state, file, current_nplurals);
            state.obsolete_line_count += usize::from(obsolete);
            state.item.msgctxt = Some(extract_quoted_bytes_cow(line_bytes)?.into_owned());
            state.context = Some(Context::Ctxt);
            state.content_line_count += 1;
            state.has_keyword = true;
        }
    }

    Ok(())
}

fn append_continuation(
    line_bytes: &[u8],
    obsolete: bool,
    state: &mut ParserState,
) -> Result<(), ParseError> {
    state.obsolete_line_count += usize::from(obsolete);
    state.content_line_count += 1;
    let value = extract_quoted_bytes_cow(line_bytes)?;

    match state.context {
        Some(Context::Str) => {
            state.append_msgstr(state.plural_index, value.as_ref());
        }
        Some(Context::Id) => state.item.msgid.push_str(value.as_ref()),
        Some(Context::IdPlural) => {
            let target = state.item.msgid_plural.get_or_insert_with(String::new);
            target.push_str(value.as_ref());
        }
        Some(Context::Ctxt) => {
            let target = state.item.msgctxt.get_or_insert_with(String::new);
            target.push_str(value.as_ref());
        }
        None => {}
    }

    Ok(())
}

fn finish_item(state: &mut ParserState, file: &mut PoFile, current_nplurals: &mut usize) {
    if !state.has_keyword {
        return;
    }

    if state.item.msgid.is_empty() && !is_header_state(state) {
        return;
    }

    if state.obsolete_line_count >= state.content_line_count && state.content_line_count > 0 {
        state.item.obsolete = true;
    }

    if is_header_state(state) && file.headers.is_empty() && file.items.is_empty() {
        file.comments = core::mem::take(&mut state.item.comments);
        file.extracted_comments = core::mem::take(&mut state.item.extracted_comments);
        parse_headers(state.header_msgstr(), &mut file.headers);
        *current_nplurals = parse_nplurals(&file.headers).unwrap_or(2);
        state.reset(*current_nplurals);
        return;
    }

    state.materialize_msgstr();

    if state.item.msgstr.is_empty() {
        state.item.msgstr = MsgStr::Singular(String::new());
    }
    if state.item.msgid_plural.is_some() && state.item.msgstr.len() == 1 {
        let mut values = state.item.msgstr.clone().into_vec();
        values.resize(state.item.nplurals.max(1), String::new());
        state.item.msgstr = MsgStr::Plural(values);
    }

    state.item.nplurals = *current_nplurals;
    file.items.push(core::mem::take(&mut state.item));
    state.reset_after_take(*current_nplurals);
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

fn parse_nplurals(headers: &[Header]) -> Option<usize> {
    let plural_forms = headers
        .iter()
        .find(|header| header.key == "Plural-Forms")?
        .value
        .as_bytes();
    let mut rest = plural_forms;

    while !rest.is_empty() {
        let (part, next) = match split_once_byte(rest, b';') {
            Some((part, tail)) => (part, tail),
            None => (rest, &b""[..]),
        };
        let trimmed = trim_ascii(part);
        if let Some((key, value)) = split_once_byte(trimmed, b'=')
            && trim_ascii(key) == b"nplurals"
            && let value = bytes_to_str(trim_ascii(value))
            && let Ok(parsed) = value.parse::<usize>()
        {
            return Some(parsed);
        }
        rest = next;
    }

    None
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

    #[test]
    fn strips_utf8_bom_prefix() {
        let input = "\u{feff}msgid \"foo\"\nmsgstr \"bar\"\n";
        let po = parse_po(input).expect("parse");

        assert_eq!(po.items.len(), 1);
        assert_eq!(po.items[0].msgid, "foo");
        assert_eq!(po.items[0].msgstr[0], "bar");
    }

    #[test]
    fn rejects_unescaped_quote_sequences() {
        let input = "msgid \"Some msgid with \\\"double\\\" quotes\"\nmsgstr \"\"\n\"Some msgstr with \"double\\\" quotes\"\n";
        let error = parse_po(input).expect_err("invalid quote pattern should fail");

        assert!(error.to_string().contains("unescaped"));
    }
}
