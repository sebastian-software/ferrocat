use std::borrow::Cow;
use std::str;

use crate::scan::{
    CommentKind, Keyword, LineKind, LineScanner, classify_line, find_quoted_bounds, has_byte,
    parse_plural_index, split_once_byte, trim_ascii,
};
use crate::text::{extract_quoted_bytes_cow, split_reference_comment};
use crate::{Header, MsgStr, ParseError, PoFile, PoItem};

/// Borrowed PO document that reuses slices from the original input whenever
/// possible.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BorrowedPoFile<'a> {
    /// File-level translator comments that appear before the header block.
    pub comments: Vec<Cow<'a, str>>,
    /// File-level extracted comments that appear before the header block.
    pub extracted_comments: Vec<Cow<'a, str>>,
    /// Parsed header entries from the leading empty `msgid` block.
    pub headers: Vec<BorrowedHeader<'a>>,
    /// Regular catalog items in source order.
    pub items: Vec<BorrowedPoItem<'a>>,
}

impl<'a> BorrowedPoFile<'a> {
    /// Converts the borrowed document into the owned [`PoFile`] representation.
    #[must_use]
    pub fn into_owned(self) -> PoFile {
        PoFile {
            comments: self.comments.into_iter().map(Cow::into_owned).collect(),
            extracted_comments: self
                .extracted_comments
                .into_iter()
                .map(Cow::into_owned)
                .collect(),
            headers: self
                .headers
                .into_iter()
                .map(BorrowedHeader::into_owned)
                .collect(),
            items: self
                .items
                .into_iter()
                .map(BorrowedPoItem::into_owned)
                .collect(),
        }
    }
}

/// Borrowed header entry from the PO header block.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BorrowedHeader<'a> {
    /// Header name such as `Language` or `Plural-Forms`.
    pub key: Cow<'a, str>,
    /// Header value without the trailing newline.
    pub value: Cow<'a, str>,
}

impl<'a> BorrowedHeader<'a> {
    /// Converts the borrowed header into an owned [`Header`].
    #[must_use]
    pub fn into_owned(self) -> Header {
        Header {
            key: self.key.into_owned(),
            value: self.value.into_owned(),
        }
    }
}

/// Borrowed gettext message entry.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BorrowedPoItem<'a> {
    /// Source message identifier.
    pub msgid: Cow<'a, str>,
    /// Optional gettext message context.
    pub msgctxt: Option<Cow<'a, str>>,
    /// Source references such as `src/app.rs:10`.
    pub references: Vec<Cow<'a, str>>,
    /// Optional plural source identifier.
    pub msgid_plural: Option<Cow<'a, str>>,
    /// Translation payload for the message.
    pub msgstr: BorrowedMsgStr<'a>,
    /// Translator comments attached to the item.
    pub comments: Vec<Cow<'a, str>>,
    /// Extracted comments attached to the item.
    pub extracted_comments: Vec<Cow<'a, str>>,
    /// Flags such as `fuzzy`.
    pub flags: Vec<Cow<'a, str>>,
    /// Raw metadata lines that do not fit the dedicated fields.
    pub metadata: Vec<(Cow<'a, str>, Cow<'a, str>)>,
    /// Whether the item is marked obsolete.
    pub obsolete: bool,
    /// Number of plural slots expected when the item is serialized.
    pub nplurals: usize,
}

impl<'a> BorrowedPoItem<'a> {
    fn new(nplurals: usize) -> Self {
        Self {
            nplurals,
            ..Self::default()
        }
    }

    /// Converts the borrowed item into an owned [`PoItem`].
    #[must_use]
    pub fn into_owned(self) -> PoItem {
        PoItem {
            msgid: self.msgid.into_owned(),
            msgctxt: self.msgctxt.map(Cow::into_owned),
            references: self.references.into_iter().map(Cow::into_owned).collect(),
            msgid_plural: self.msgid_plural.map(Cow::into_owned),
            msgstr: self.msgstr.into_owned(),
            comments: self.comments.into_iter().map(Cow::into_owned).collect(),
            extracted_comments: self
                .extracted_comments
                .into_iter()
                .map(Cow::into_owned)
                .collect(),
            flags: self.flags.into_iter().map(Cow::into_owned).collect(),
            metadata: self
                .metadata
                .into_iter()
                .map(|(key, value)| (key.into_owned(), value.into_owned()))
                .collect(),
            obsolete: self.obsolete,
            nplurals: self.nplurals,
        }
    }
}

/// Borrowed translation payload for a PO item.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BorrowedMsgStr<'a> {
    /// No translation values are present.
    #[default]
    None,
    /// Single translation string.
    Singular(Cow<'a, str>),
    /// Plural translation strings indexed by plural slot.
    Plural(Vec<Cow<'a, str>>),
}

impl<'a> BorrowedMsgStr<'a> {
    fn is_empty(&self) -> bool {
        matches!(self, Self::None)
    }

    fn len(&self) -> usize {
        match self {
            Self::None => 0,
            Self::Singular(_) => 1,
            Self::Plural(values) => values.len(),
        }
    }

    /// Converts the borrowed payload into an owned [`MsgStr`].
    #[must_use]
    pub fn into_owned(self) -> MsgStr {
        match self {
            Self::None => MsgStr::None,
            Self::Singular(value) => MsgStr::Singular(value.into_owned()),
            Self::Plural(values) => {
                MsgStr::Plural(values.into_iter().map(Cow::into_owned).collect())
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Context {
    Id,
    IdPlural,
    Str,
    Ctxt,
}

#[derive(Debug)]
struct ParserState<'a> {
    item: BorrowedPoItem<'a>,
    header_entries: Vec<BorrowedHeader<'a>>,
    msgstr: BorrowedMsgStr<'a>,
    context: Option<Context>,
    plural_index: usize,
    obsolete_line_count: usize,
    content_line_count: usize,
    has_keyword: bool,
}

impl<'a> ParserState<'a> {
    fn new(nplurals: usize) -> Self {
        Self {
            item: BorrowedPoItem::new(nplurals),
            header_entries: Vec::new(),
            msgstr: BorrowedMsgStr::None,
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

    fn set_msgstr(&mut self, plural_index: usize, value: Cow<'a, str>) {
        match (&mut self.msgstr, plural_index) {
            (BorrowedMsgStr::None, 0) => self.msgstr = BorrowedMsgStr::Singular(value),
            (BorrowedMsgStr::Singular(existing), 0) => *existing = value,
            (BorrowedMsgStr::Plural(values), 0) => {
                if values.is_empty() {
                    values.push(Cow::Borrowed(""));
                }
                values[0] = value;
            }
            _ => {
                let msgstr = self.promote_plural_msgstr(plural_index);
                msgstr[plural_index] = value;
            }
        }
    }

    fn append_msgstr(&mut self, plural_index: usize, value: Cow<'a, str>) {
        match (&mut self.msgstr, plural_index) {
            (BorrowedMsgStr::None, 0) => self.msgstr = BorrowedMsgStr::Singular(value),
            (BorrowedMsgStr::Singular(existing), 0) => existing.to_mut().push_str(value.as_ref()),
            (BorrowedMsgStr::Plural(values), 0) => {
                if values.is_empty() {
                    values.push(Cow::Borrowed(""));
                }
                values[0].to_mut().push_str(value.as_ref());
            }
            _ => {
                let msgstr = self.promote_plural_msgstr(plural_index);
                msgstr[plural_index].to_mut().push_str(value.as_ref());
            }
        }
    }

    fn materialize_msgstr(&mut self) {
        debug_assert!(self.item.msgstr.is_empty());
        self.item.msgstr = std::mem::take(&mut self.msgstr);
    }

    fn promote_plural_msgstr(&mut self, plural_index: usize) -> &mut Vec<Cow<'a, str>> {
        if !matches!(self.msgstr, BorrowedMsgStr::Plural(_)) {
            self.msgstr = match std::mem::take(&mut self.msgstr) {
                BorrowedMsgStr::None => BorrowedMsgStr::Plural(Vec::with_capacity(2)),
                BorrowedMsgStr::Singular(value) => {
                    let mut values = Vec::with_capacity(2);
                    values.push(value);
                    BorrowedMsgStr::Plural(values)
                }
                BorrowedMsgStr::Plural(values) => BorrowedMsgStr::Plural(values),
            };
        }
        let BorrowedMsgStr::Plural(values) = &mut self.msgstr else {
            unreachable!("plural msgstr promotion must yield plural storage");
        };
        if values.len() <= plural_index {
            values.resize(plural_index + 1, Cow::Borrowed(""));
        }
        values
    }
}

#[derive(Debug, Clone, Copy)]
struct BorrowedLine<'a> {
    trimmed: &'a [u8],
    obsolete: bool,
}

/// Parses PO content into a borrowed representation.
///
/// This parser keeps references into `input` for fields that do not need
/// unescaping, which reduces allocations compared with [`crate::parse_po`].
///
/// # Errors
///
/// Returns [`ParseError`] when the input is not valid PO syntax.
pub fn parse_po_borrowed<'a>(input: &'a str) -> Result<BorrowedPoFile<'a>, ParseError> {
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    if input.as_bytes().contains(&b'\r') {
        return Err(ParseError::new(
            "borrowed PO parsing currently requires LF-only input",
        ));
    }

    let mut file = BorrowedPoFile::default();
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

    finish_item(&mut state, &mut file, &mut current_nplurals)?;

    Ok(file)
}

fn parse_line<'a>(
    line: BorrowedLine<'a>,
    state: &mut ParserState<'a>,
    file: &mut BorrowedPoFile<'a>,
    current_nplurals: &mut usize,
) -> Result<(), ParseError> {
    match classify_line(line.trimmed) {
        LineKind::Continuation => {
            append_continuation(line.trimmed, line.obsolete, state)?;
            Ok(())
        }
        LineKind::Comment(kind) => {
            parse_comment_line(line.trimmed, kind, state, file, current_nplurals)
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

fn parse_comment_line<'a>(
    line_bytes: &'a [u8],
    kind: CommentKind,
    state: &mut ParserState<'a>,
    file: &mut BorrowedPoFile<'a>,
    current_nplurals: &mut usize,
) -> Result<(), ParseError> {
    finish_item(state, file, current_nplurals)?;

    match kind {
        CommentKind::Reference => {
            let reference_line = trimmed_str(&line_bytes[2..])?;
            state
                .item
                .references
                .extend(split_reference_comment(reference_line));
        }
        CommentKind::Flags => {
            for flag in trimmed_str(&line_bytes[2..])?.split(',') {
                state.item.flags.push(Cow::Borrowed(flag.trim()));
            }
        }
        CommentKind::Extracted => state
            .item
            .extracted_comments
            .push(trimmed_cow(&line_bytes[2..])?),
        CommentKind::Metadata => {
            let trimmed = trim_ascii(&line_bytes[2..]);
            if let Some((key_bytes, value_bytes)) = split_once_byte(trimmed, b':') {
                let key = trimmed_cow(key_bytes)?;
                if !key.is_empty() {
                    let value = trimmed_cow(value_bytes)?;
                    state.item.metadata.push((key, value));
                }
            }
        }
        CommentKind::Translator => state.item.comments.push(trimmed_cow(&line_bytes[1..])?),
        CommentKind::Other => {}
    }

    Ok(())
}

fn parse_keyword_line<'a>(
    line_bytes: &'a [u8],
    obsolete: bool,
    keyword: Keyword,
    state: &mut ParserState<'a>,
    file: &mut BorrowedPoFile<'a>,
    current_nplurals: &mut usize,
) -> Result<(), ParseError> {
    match keyword {
        Keyword::IdPlural => {
            state.obsolete_line_count += usize::from(obsolete);
            state.item.msgid_plural = Some(extract_quoted_bytes_cow(line_bytes)?);
            state.context = Some(Context::IdPlural);
            state.content_line_count += 1;
            state.has_keyword = true;
        }
        Keyword::Id => {
            finish_item(state, file, current_nplurals)?;
            state.obsolete_line_count += usize::from(obsolete);
            state.item.msgid = extract_quoted_bytes_cow(line_bytes)?;
            state.context = Some(Context::Id);
            state.content_line_count += 1;
            state.has_keyword = true;
        }
        Keyword::Str => {
            let plural_index = parse_plural_index(line_bytes).unwrap_or(0);
            state.plural_index = plural_index;
            state.obsolete_line_count += usize::from(obsolete);
            state.set_msgstr(plural_index, extract_quoted_bytes_cow(line_bytes)?);
            if is_header_candidate(state) {
                state
                    .header_entries
                    .extend(parse_header_fragment(line_bytes)?);
            }
            state.context = Some(Context::Str);
            state.content_line_count += 1;
            state.has_keyword = true;
        }
        Keyword::Ctxt => {
            finish_item(state, file, current_nplurals)?;
            state.obsolete_line_count += usize::from(obsolete);
            state.item.msgctxt = Some(extract_quoted_bytes_cow(line_bytes)?);
            state.context = Some(Context::Ctxt);
            state.content_line_count += 1;
            state.has_keyword = true;
        }
    }

    Ok(())
}

fn append_continuation<'a>(
    line_bytes: &'a [u8],
    obsolete: bool,
    state: &mut ParserState<'a>,
) -> Result<(), ParseError> {
    state.obsolete_line_count += usize::from(obsolete);
    state.content_line_count += 1;
    let value = extract_quoted_bytes_cow(line_bytes)?;

    match state.context {
        Some(Context::Str) => {
            state.append_msgstr(state.plural_index, value);
            if is_header_candidate(state) {
                state
                    .header_entries
                    .extend(parse_header_fragment(line_bytes)?);
            }
        }
        Some(Context::Id) => state.item.msgid.to_mut().push_str(value.as_ref()),
        Some(Context::IdPlural) => {
            let target = state.item.msgid_plural.get_or_insert(Cow::Borrowed(""));
            target.to_mut().push_str(value.as_ref());
        }
        Some(Context::Ctxt) => {
            let target = state.item.msgctxt.get_or_insert(Cow::Borrowed(""));
            target.to_mut().push_str(value.as_ref());
        }
        None => {}
    }

    Ok(())
}

fn finish_item<'a>(
    state: &mut ParserState<'a>,
    file: &mut BorrowedPoFile<'a>,
    current_nplurals: &mut usize,
) -> Result<(), ParseError> {
    if !state.has_keyword {
        return Ok(());
    }

    if state.item.msgid.is_empty() && !is_header_state(state) {
        return Ok(());
    }

    if state.obsolete_line_count >= state.content_line_count && state.content_line_count > 0 {
        state.item.obsolete = true;
    }

    if is_header_state(state) && file.headers.is_empty() && file.items.is_empty() {
        file.comments = std::mem::take(&mut state.item.comments);
        file.extracted_comments = std::mem::take(&mut state.item.extracted_comments);
        file.headers = std::mem::take(&mut state.header_entries);
        *current_nplurals = parse_nplurals(&file.headers).unwrap_or(2);
        state.reset(*current_nplurals);
        return Ok(());
    }

    state.materialize_msgstr();

    if state.item.msgstr.is_empty() {
        state.item.msgstr = BorrowedMsgStr::Singular(Cow::Borrowed(""));
    }
    if state.item.msgid_plural.is_some() && state.item.msgstr.len() == 1 {
        let mut values = match std::mem::take(&mut state.item.msgstr) {
            BorrowedMsgStr::None => Vec::new(),
            BorrowedMsgStr::Singular(value) => vec![value],
            BorrowedMsgStr::Plural(values) => values,
        };
        values.resize(state.item.nplurals.max(1), Cow::Borrowed(""));
        state.item.msgstr = BorrowedMsgStr::Plural(values);
    }

    state.item.nplurals = *current_nplurals;
    file.items.push(std::mem::take(&mut state.item));
    state.reset(*current_nplurals);
    Ok(())
}

fn is_header_state(state: &ParserState<'_>) -> bool {
    state.item.msgid.is_empty()
        && state.item.msgctxt.is_none()
        && state.item.msgid_plural.is_none()
        && !state.msgstr.is_empty()
}

fn is_header_candidate(state: &ParserState<'_>) -> bool {
    state.item.msgid.is_empty()
        && state.item.msgctxt.is_none()
        && state.item.msgid_plural.is_none()
        && state.plural_index == 0
}

fn parse_header_fragment<'a>(line_bytes: &'a [u8]) -> Result<Vec<BorrowedHeader<'a>>, ParseError> {
    let Some((start, end)) = find_quoted_bounds(line_bytes) else {
        return Ok(Vec::new());
    };
    let raw = &line_bytes[start..end];

    if header_fragment_is_borrowable(raw) {
        return parse_header_fragment_borrowed(raw);
    }

    parse_header_fragment_owned(line_bytes)
}

fn parse_header_fragment_borrowed<'a>(
    raw: &'a [u8],
) -> Result<Vec<BorrowedHeader<'a>>, ParseError> {
    let mut headers = Vec::new();
    let mut start = 0usize;
    let mut index = 0usize;

    while index < raw.len() {
        if raw[index] == b'\\' && raw.get(index + 1) == Some(&b'n') {
            push_borrowed_header_segment(&raw[start..index], &mut headers)?;
            index += 2;
            start = index;
            continue;
        }
        index += 1;
    }

    push_borrowed_header_segment(&raw[start..], &mut headers)?;
    Ok(headers)
}

fn push_borrowed_header_segment<'a>(
    segment: &'a [u8],
    out: &mut Vec<BorrowedHeader<'a>>,
) -> Result<(), ParseError> {
    if segment.is_empty() {
        return Ok(());
    }
    if let Some((key_bytes, value_bytes)) = split_once_byte(segment, b':') {
        out.push(BorrowedHeader {
            key: trimmed_cow(key_bytes)?,
            value: trimmed_cow(value_bytes)?,
        });
    }
    Ok(())
}

fn parse_header_fragment_owned<'a>(
    line_bytes: &'a [u8],
) -> Result<Vec<BorrowedHeader<'a>>, ParseError> {
    let decoded = extract_quoted_bytes_cow(line_bytes)?;
    let mut headers = Vec::new();
    for segment in decoded.split('\n') {
        if segment.is_empty() {
            continue;
        }
        if let Some((key, value)) = segment.split_once(':') {
            headers.push(BorrowedHeader {
                key: Cow::Owned(key.trim().to_owned()),
                value: Cow::Owned(value.trim().to_owned()),
            });
        }
    }
    Ok(headers)
}

fn header_fragment_is_borrowable(raw: &[u8]) -> bool {
    let mut index = 0usize;
    while index < raw.len() {
        if raw[index] == b'\\' {
            if raw.get(index + 1) != Some(&b'n') {
                return false;
            }
            index += 2;
            continue;
        }
        index += 1;
    }
    !has_byte(b'"', raw)
}

fn parse_nplurals(headers: &[BorrowedHeader<'_>]) -> Option<usize> {
    let plural_forms = headers
        .iter()
        .find(|header| header.key.as_ref() == "Plural-Forms")?
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
            && let Ok(value) = bytes_to_str(trim_ascii(value))
            && let Ok(parsed) = value.parse::<usize>()
        {
            return Some(parsed);
        }
        rest = next;
    }

    None
}

fn bytes_to_str(bytes: &[u8]) -> Result<&str, ParseError> {
    Ok(unsafe { str::from_utf8_unchecked(bytes) })
}

fn trimmed_str(bytes: &[u8]) -> Result<&str, ParseError> {
    bytes_to_str(trim_ascii(bytes))
}

fn trimmed_cow<'a>(bytes: &'a [u8]) -> Result<Cow<'a, str>, ParseError> {
    Ok(Cow::Borrowed(trimmed_str(bytes)?))
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::parse_po_borrowed;

    #[test]
    fn borrows_simple_fields() {
        let input = r#"
# translator
msgid "hello"
msgstr "world"
"#;

        let file = parse_po_borrowed(input).expect("borrowed parse");
        assert_eq!(file.items[0].comments[0], Cow::Borrowed("translator"));
        assert_eq!(file.items[0].msgid, Cow::Borrowed("hello"));
        assert_eq!(
            file.items[0].msgstr,
            super::BorrowedMsgStr::Singular(Cow::Borrowed("world"))
        );
    }

    #[test]
    fn owns_unescaped_sequences_only_when_needed() {
        let input = "msgid \"a\\n\"\nmsgstr \"b\\t\"\n";
        let file = parse_po_borrowed(input).expect("borrowed parse with escapes");
        assert_eq!(file.items[0].msgid, Cow::<str>::Owned("a\n".to_owned()));
        assert_eq!(
            file.items[0].msgstr,
            super::BorrowedMsgStr::Singular(Cow::<str>::Owned("b\t".to_owned()))
        );
    }

    #[test]
    fn converts_borrowed_parse_to_owned() {
        let input = "msgid \"hello\"\nmsgstr \"world\"\n";
        let owned = parse_po_borrowed(input)
            .expect("borrowed parse")
            .into_owned();
        assert_eq!(owned.items[0].msgid, "hello");
        assert_eq!(owned.items[0].msgstr[0], "world");
    }

    #[test]
    fn borrows_header_key_values_without_escapes() {
        let input = concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Language: de\\n\"\n",
            "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n",
        );
        let file = parse_po_borrowed(input).expect("borrowed parse with headers");
        assert_eq!(file.headers[0].key, Cow::Borrowed("Language"));
        assert_eq!(file.headers[0].value, Cow::Borrowed("de"));
    }

    #[test]
    fn strips_utf8_bom_prefix() {
        let input = "\u{feff}msgid \"foo\"\nmsgstr \"bar\"\n";
        let file = parse_po_borrowed(input).expect("borrowed parse");

        assert_eq!(file.items.len(), 1);
        assert_eq!(file.items[0].msgid, Cow::Borrowed("foo"));
        assert_eq!(
            file.items[0].msgstr,
            super::BorrowedMsgStr::Singular(Cow::Borrowed("bar"))
        );
    }
}
