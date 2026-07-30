use std::borrow::Cow;

use crate::line_state::{PoLineContext, PoLineState};
use crate::scan::{
    CommentKind, Keyword, LineKind, LineScanner, classify_line, find_quoted_bounds, has_byte,
    parse_plural_index, split_once_byte, trim_ascii, trim_ascii_start, unrecognized_po_line,
};
use crate::text::{extract_quoted_bytes_cow, for_each_reference_token};
use crate::utf8::input_slice_as_str;
use crate::{Header, MsgStr, ParseError, ParsePosition, PoFile, PoItem, PoVec};

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

impl BorrowedPoFile<'_> {
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

impl BorrowedHeader<'_> {
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
    pub references: PoVec<Cow<'a, str>>,
    /// Optional plural source identifier.
    pub msgid_plural: Option<Cow<'a, str>>,
    /// Translation payload for the message.
    pub msgstr: BorrowedMsgStr<'a>,
    /// Translator comments attached to the item.
    pub comments: PoVec<Cow<'a, str>>,
    /// Extracted comments attached to the item.
    pub extracted_comments: PoVec<Cow<'a, str>>,
    /// Flags such as `fuzzy`.
    pub flags: PoVec<Cow<'a, str>>,
    /// Raw metadata lines that do not fit the dedicated fields.
    pub metadata: PoVec<(Cow<'a, str>, Cow<'a, str>)>,
    /// Whether the item is marked obsolete.
    pub obsolete: bool,
    /// Number of plural slots expected when the item is serialized.
    pub nplurals: usize,
}

impl BorrowedPoItem<'_> {
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
    pub(crate) const fn is_empty(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns the number of translation values present, mirroring
    /// [`MsgStr::len`].
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::None => 0,
            Self::Singular(_) => 1,
            Self::Plural(values) => values.len(),
        }
    }

    /// Consumes the payload and yields its translation values in slot order,
    /// mirroring [`MsgStr::iter`] without re-allocating the values.
    pub(crate) fn into_values(self) -> std::vec::IntoIter<Cow<'a, str>> {
        match self {
            Self::None => Vec::new().into_iter(),
            Self::Singular(value) => vec![value].into_iter(),
            Self::Plural(values) => values.into_iter(),
        }
    }

    pub(crate) fn set_slot(&mut self, plural_index: usize, value: Cow<'a, str>) {
        match (&mut *self, plural_index) {
            (Self::None, 0) => *self = Self::Singular(value),
            (Self::Singular(existing), 0) => *existing = value,
            (Self::Plural(values), 0) => {
                if values.is_empty() {
                    values.push(Cow::Borrowed(""));
                }
                values[0] = value;
            }
            _ => {
                let values = self.promote_plural(plural_index);
                values[plural_index] = value;
            }
        }
    }

    pub(crate) fn append_slot(&mut self, plural_index: usize, value: Cow<'a, str>) {
        match (&mut *self, plural_index) {
            (Self::None, 0) => *self = Self::Singular(value),
            (Self::Singular(existing), 0) => existing.to_mut().push_str(value.as_ref()),
            (Self::Plural(values), 0) => {
                if values.is_empty() {
                    values.push(Cow::Borrowed(""));
                }
                values[0].to_mut().push_str(value.as_ref());
            }
            _ => {
                let values = self.promote_plural(plural_index);
                values[plural_index].to_mut().push_str(value.as_ref());
            }
        }
    }

    pub(crate) fn ensure_singular(&mut self) {
        if self.is_empty() {
            *self = Self::Singular(Cow::Borrowed(""));
        }
    }

    pub(crate) fn expand_singular_to_plural_width(&mut self, width: usize) {
        match self {
            Self::Singular(value) => {
                let mut values = vec![std::mem::take(value)];
                values.resize(width.max(1), Cow::Borrowed(""));
                *self = Self::Plural(values);
            }
            Self::Plural(values) if values.len() == 1 => {
                values.resize(width.max(1), Cow::Borrowed(""));
            }
            Self::None | Self::Plural(_) => {}
        }
    }

    fn promote_plural(&mut self, plural_index: usize) -> &mut Vec<Cow<'a, str>> {
        match self {
            Self::None => {
                *self = Self::Plural(Vec::with_capacity(2));
                self.promote_plural(plural_index)
            }
            Self::Singular(value) => {
                let value = std::mem::take(value);
                *self = Self::Plural(vec![value]);
                self.promote_plural(plural_index)
            }
            Self::Plural(values) => {
                if values.len() <= plural_index {
                    values.resize(plural_index + 1, Cow::Borrowed(""));
                }
                values
            }
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

#[derive(Debug)]
struct ParserState<'a> {
    item: BorrowedPoItem<'a>,
    header_entries: Vec<BorrowedHeader<'a>>,
    /// Whether splitting the header block per physical fragment still yields the
    /// same headers as decoding the whole header `msgstr` first (see
    /// [`ParserState::push_header_fragment`]).
    header_entries_faithful: bool,
    /// Whether the previous header fragment left a header line unterminated, so
    /// the next fragment continues it instead of starting a new header.
    header_line_open: bool,
    msgstr: BorrowedMsgStr<'a>,
    line: PoLineState,
}

impl<'a> ParserState<'a> {
    fn new(nplurals: usize) -> Self {
        Self {
            item: BorrowedPoItem::new(nplurals),
            header_entries: Vec::new(),
            header_entries_faithful: true,
            header_line_open: false,
            msgstr: BorrowedMsgStr::None,
            line: PoLineState::default(),
        }
    }

    fn reset(&mut self, nplurals: usize) {
        *self = Self::new(nplurals);
    }

    fn set_msgstr(&mut self, plural_index: usize, value: Cow<'a, str>) {
        self.msgstr.set_slot(plural_index, value);
    }

    fn append_msgstr(&mut self, plural_index: usize, value: Cow<'a, str>) {
        self.msgstr.append_slot(plural_index, value);
    }

    /// Collects the headers of one physical header line while they can be kept
    /// as slices of the input.
    ///
    /// Per-fragment splitting only matches the reference algorithm — decode the
    /// whole header `msgstr`, then split it into lines — when every fragment
    /// ends its header line, carries no escape other than `\n`, and starts no
    /// obsolete (`#~`) line. Anything else clears
    /// [`ParserState::header_entries_faithful`] so [`finish_item`] falls back to
    /// the reference algorithm on the decoded `msgstr`.
    ///
    /// `starts_new_msgstr` marks a `msgstr` keyword line, which replaces the
    /// header text collected so far instead of extending it.
    fn push_header_fragment(&mut self, line_bytes: &'a [u8], starts_new_msgstr: bool) {
        if starts_new_msgstr {
            self.header_entries.clear();
            self.header_line_open = false;
        }
        if !self.header_entries_faithful {
            return;
        }

        let Some((start, end)) = find_quoted_bounds(line_bytes) else {
            self.header_entries_faithful = false;
            return;
        };
        let raw = &line_bytes[start..end];

        if self.header_line_open
            || !header_fragment_is_borrowable(raw)
            || !push_borrowed_header_segments(raw, &mut self.header_entries)
        {
            self.header_entries_faithful = false;
            return;
        }

        self.header_line_open = !raw.is_empty() && !raw.ends_with(br"\n");
    }

    /// Returns the decoded header `msgstr`, mirroring the owned parser.
    fn header_msgstr(&self) -> &str {
        match &self.msgstr {
            BorrowedMsgStr::None => "",
            BorrowedMsgStr::Singular(value) => value.as_ref(),
            BorrowedMsgStr::Plural(values) => values.first().map_or("", Cow::as_ref),
        }
    }

    fn materialize_msgstr(&mut self) {
        debug_assert!(self.item.msgstr.is_empty());
        self.item.msgstr = std::mem::take(&mut self.msgstr);
    }
}

#[derive(Debug, Clone, Copy)]
struct BorrowedLine<'a> {
    trimmed: &'a [u8],
    obsolete: bool,
    position: ParsePosition,
}

/// Parses PO content into a borrowed representation.
///
/// This parser keeps references into `input` for fields that do not need
/// unescaping, which reduces allocations compared with [`crate::parse_po`].
/// LF, CRLF, and bare CR line endings are accepted.
///
/// # Errors
///
/// Returns [`ParseError`] when the input is not valid PO syntax.
pub fn parse_po_borrowed(input: &str) -> Result<BorrowedPoFile<'_>, ParseError> {
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);

    let mut file = BorrowedPoFile::default();
    file.items.reserve((input.len() / 96).max(1));
    let mut current_nplurals = 2;
    let mut state = ParserState::new(current_nplurals);

    for line in LineScanner::new(input.as_bytes()) {
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

    finish_item(&mut state, &mut file, &mut current_nplurals);

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
            append_continuation(line.trimmed, line.obsolete, line.position, state)?;
            Ok(())
        }
        LineKind::Comment(kind) => {
            parse_comment_line(line.trimmed, kind, state, file, current_nplurals);
            Ok(())
        }
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

fn parse_comment_line<'a>(
    line_bytes: &'a [u8],
    kind: CommentKind,
    state: &mut ParserState<'a>,
    file: &mut BorrowedPoFile<'a>,
    current_nplurals: &mut usize,
) {
    finish_item(state, file, current_nplurals);

    match kind {
        CommentKind::Reference => {
            let reference_line = trimmed_str(&line_bytes[2..]);
            for_each_reference_token(reference_line, |token| {
                state.item.references.push(token);
            });
        }
        CommentKind::Flags => {
            for flag in trimmed_str(&line_bytes[2..]).split(',') {
                state.item.flags.push(Cow::Borrowed(flag.trim()));
            }
        }
        CommentKind::Extracted => state
            .item
            .extracted_comments
            .push(trimmed_cow(&line_bytes[2..])),
        CommentKind::Metadata => {
            let trimmed = trim_ascii(&line_bytes[2..]);
            if let Some(value_bytes) = ferrocat_mt_metadata_value(trimmed) {
                state
                    .item
                    .metadata
                    .push((Cow::Borrowed("ferrocat-mt"), trimmed_cow(value_bytes)));
            } else if let Some((key_bytes, value_bytes)) = split_once_byte(trimmed, b':') {
                let key = trimmed_cow(key_bytes);
                if !key.is_empty() {
                    let value = trimmed_cow(value_bytes);
                    state.item.metadata.push((key, value));
                }
            }
        }
        CommentKind::Translator => state.item.comments.push(trimmed_cow(&line_bytes[1..])),
        CommentKind::Other => {}
    }
}

fn ferrocat_mt_metadata_value(trimmed: &[u8]) -> Option<&[u8]> {
    const KEY: &[u8] = b"ferrocat-mt";
    let rest = trimmed.strip_prefix(KEY)?;
    rest.first()
        .is_some_and(u8::is_ascii_whitespace)
        .then(|| trim_ascii(rest))
}

fn parse_keyword_line<'a>(
    line_bytes: &'a [u8],
    obsolete: bool,
    position: ParsePosition,
    keyword: Keyword,
    state: &mut ParserState<'a>,
    file: &mut BorrowedPoFile<'a>,
    current_nplurals: &mut usize,
) -> Result<(), ParseError> {
    match keyword {
        Keyword::IdPlural => {
            state
                .line
                .mark_keyword(PoLineContext::IdPlural, 0, obsolete);
            state.item.msgid_plural = Some(at_line_position(
                extract_quoted_bytes_cow(line_bytes),
                position,
            )?);
        }
        Keyword::Id => {
            finish_item(state, file, current_nplurals);
            state.line.mark_keyword(PoLineContext::Id, 0, obsolete);
            state.item.msgid = at_line_position(extract_quoted_bytes_cow(line_bytes), position)?;
        }
        Keyword::Str => {
            let plural_index = parse_plural_index(line_bytes).unwrap_or(0);
            state
                .line
                .mark_keyword(PoLineContext::Str, plural_index, obsolete);
            state.set_msgstr(
                plural_index,
                at_line_position(extract_quoted_bytes_cow(line_bytes), position)?,
            );
            if is_header_candidate(state) {
                state.push_header_fragment(line_bytes, true);
            }
        }
        Keyword::Ctxt => {
            finish_item(state, file, current_nplurals);
            state.line.mark_keyword(PoLineContext::Ctxt, 0, obsolete);
            state.item.msgctxt = Some(at_line_position(
                extract_quoted_bytes_cow(line_bytes),
                position,
            )?);
        }
    }

    Ok(())
}

fn append_continuation<'a>(
    line_bytes: &'a [u8],
    obsolete: bool,
    position: ParsePosition,
    state: &mut ParserState<'a>,
) -> Result<(), ParseError> {
    state.line.mark_continuation(obsolete);
    let value = at_line_position(extract_quoted_bytes_cow(line_bytes), position)?;

    match state.line.context() {
        Some(PoLineContext::Str) => {
            state.append_msgstr(state.line.plural_index(), value);
            if is_header_candidate(state) {
                state.push_header_fragment(line_bytes, false);
            }
        }
        Some(PoLineContext::Id) => state.item.msgid.to_mut().push_str(value.as_ref()),
        Some(PoLineContext::IdPlural) => {
            let target = state.item.msgid_plural.get_or_insert(Cow::Borrowed(""));
            target.to_mut().push_str(value.as_ref());
        }
        Some(PoLineContext::Ctxt) => {
            let target = state.item.msgctxt.get_or_insert(Cow::Borrowed(""));
            target.to_mut().push_str(value.as_ref());
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

fn finish_item<'a>(
    state: &mut ParserState<'a>,
    file: &mut BorrowedPoFile<'a>,
    current_nplurals: &mut usize,
) {
    if !state.line.has_keyword() {
        return;
    }

    if state.item.msgid.is_empty() && !is_header_state(state) {
        return;
    }

    if state.line.is_obsolete_item() {
        state.item.obsolete = true;
    }

    if is_header_state(state) && file.headers.is_empty() && file.items.is_empty() {
        file.comments = std::mem::take(&mut state.item.comments).into_vec();
        file.extracted_comments = std::mem::take(&mut state.item.extracted_comments).into_vec();
        if state.header_entries_faithful {
            file.headers = std::mem::take(&mut state.header_entries);
        } else {
            parse_owned_headers(state.header_msgstr(), &mut file.headers);
        }
        *current_nplurals = parse_nplurals(&file.headers).unwrap_or(2);
        state.reset(*current_nplurals);
        return;
    }

    state.materialize_msgstr();

    state.item.msgstr.ensure_singular();
    if state.item.msgid_plural.is_some() {
        state
            .item
            .msgstr
            .expand_singular_to_plural_width(state.item.nplurals);
    }

    state.item.nplurals = *current_nplurals;
    file.items.push(std::mem::take(&mut state.item));
    state.reset(*current_nplurals);
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
        && state.line.plural_index() == 0
}

/// Splits one borrowable header fragment into `\n`-terminated header lines.
///
/// Returns `false` when a segment starts an obsolete (`#~`) line, which the
/// reference line splitter strips but this fast path cannot.
fn push_borrowed_header_segments<'a>(raw: &'a [u8], out: &mut Vec<BorrowedHeader<'a>>) -> bool {
    let mut start = 0usize;
    let mut index = 0usize;

    while index < raw.len() {
        if raw[index] == b'\\' && raw.get(index + 1) == Some(&b'n') {
            if !push_borrowed_header_segment(&raw[start..index], out) {
                return false;
            }
            index += 2;
            start = index;
            continue;
        }
        index += 1;
    }

    push_borrowed_header_segment(&raw[start..], out)
}

fn push_borrowed_header_segment<'a>(segment: &'a [u8], out: &mut Vec<BorrowedHeader<'a>>) -> bool {
    let segment = trim_ascii_start(segment);
    if segment.is_empty() {
        return true;
    }
    if segment.starts_with(b"#~") {
        return false;
    }
    if let Some((key_bytes, value_bytes)) = split_once_byte(segment, b':') {
        out.push(BorrowedHeader {
            key: trimmed_cow(key_bytes),
            value: trimmed_cow(value_bytes),
        });
    }
    true
}

/// Reference header parsing: split the already decoded header `msgstr` into
/// lines the same way the file itself is scanned. Used whenever the borrowed
/// fast path cannot reproduce that result exactly.
fn parse_owned_headers<'a>(raw: &str, out: &mut Vec<BorrowedHeader<'a>>) {
    for line in LineScanner::new(raw.as_bytes()) {
        if let Some((key_bytes, value_bytes)) = split_once_byte(line.trimmed, b':') {
            out.push(BorrowedHeader {
                key: Cow::Owned(trimmed_str(key_bytes).to_owned()),
                value: Cow::Owned(trimmed_str(value_bytes).to_owned()),
            });
        }
    }
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

fn trimmed_cow(bytes: &[u8]) -> Cow<'_, str> {
    Cow::Borrowed(trimmed_str(bytes))
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::MsgStr;

    use super::{BorrowedMsgStr, parse_po_borrowed};

    #[test]
    fn borrowed_msgstr_helpers_cover_slot_promotion_and_plural_width() {
        assert_eq!(BorrowedMsgStr::None.into_owned(), MsgStr::None);

        let mut appended_none = BorrowedMsgStr::None;
        appended_none.append_slot(0, Cow::Borrowed("one"));
        assert_eq!(
            appended_none,
            BorrowedMsgStr::Singular(Cow::Borrowed("one"))
        );

        let mut singular = BorrowedMsgStr::None;
        singular.set_slot(0, Cow::Borrowed("one"));
        singular.set_slot(0, Cow::Borrowed("uno"));
        singular.append_slot(0, Cow::Borrowed(" plus"));
        assert_eq!(
            singular,
            BorrowedMsgStr::Singular(Cow::Borrowed("uno plus"))
        );

        let mut append_empty_plural = BorrowedMsgStr::Plural(Vec::new());
        append_empty_plural.append_slot(0, Cow::Borrowed("zero"));
        assert_eq!(
            append_empty_plural,
            BorrowedMsgStr::Plural(vec![Cow::Borrowed("zero")])
        );

        let mut empty_plural = BorrowedMsgStr::Plural(Vec::new());
        empty_plural.set_slot(0, Cow::Borrowed("zero"));
        empty_plural.append_slot(0, Cow::Borrowed(" plus"));
        assert_eq!(
            empty_plural,
            BorrowedMsgStr::Plural(vec![Cow::Borrowed("zero plus")])
        );

        let mut msgstr = BorrowedMsgStr::None;

        msgstr.set_slot(1, Cow::Borrowed("two"));
        assert_eq!(
            msgstr,
            BorrowedMsgStr::Plural(vec![Cow::Borrowed(""), Cow::Borrowed("two")])
        );

        msgstr.append_slot(1, Cow::Borrowed(" plus"));
        msgstr.append_slot(0, Cow::Borrowed("one"));
        assert_eq!(
            msgstr,
            BorrowedMsgStr::Plural(vec![Cow::Borrowed("one"), Cow::Borrowed("two plus")])
        );

        let mut defaulted = BorrowedMsgStr::None;
        defaulted.ensure_singular();
        defaulted.expand_singular_to_plural_width(3);
        assert_eq!(
            defaulted,
            BorrowedMsgStr::Plural(vec![
                Cow::Borrowed(""),
                Cow::Borrowed(""),
                Cow::Borrowed(""),
            ])
        );

        let mut existing_plural = BorrowedMsgStr::Plural(vec![Cow::Borrowed("one")]);
        existing_plural.expand_singular_to_plural_width(2);
        assert_eq!(
            existing_plural,
            BorrowedMsgStr::Plural(vec![Cow::Borrowed("one"), Cow::Borrowed("")])
        );
    }

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

    #[test]
    fn accepts_crlf_input_for_borrowed_parsing() {
        let file = parse_po_borrowed("msgid \"foo\"\r\nmsgstr \"bar\"\r\n")
            .expect("borrowed parse with crlf");

        assert_eq!(file.items[0].msgid, Cow::Borrowed("foo"));
        assert_eq!(
            file.items[0].msgstr,
            super::BorrowedMsgStr::Singular(Cow::Borrowed("bar"))
        );
    }

    #[test]
    fn parse_errors_include_line_position() {
        let error = parse_po_borrowed("msgid \"ok\"\nmsgstr \"bad\"quote\"\n")
            .expect_err("unescaped quote should fail");
        let position = error.position().expect("position metadata");

        assert_eq!(error.message(), "unescaped quote in string literal");
        assert_eq!(position.offset(), 11);
        assert_eq!(position.line(), 2);
        assert_eq!(position.column(), 1);
    }

    #[test]
    fn rejects_unrecognized_lines() {
        let error = parse_po_borrowed("msgid \"ok\"\nmsgstr_ \"typo\"\n")
            .expect_err("unknown PO line should fail");
        let position = error.position().expect("position metadata");

        assert_eq!(error.message(), "unrecognized PO syntax");
        assert_eq!(position.line(), 2);
        assert_eq!(position.column(), 1);
    }

    #[test]
    fn parse_errors_include_line_position_for_plural_and_context_keywords() {
        let plural_error =
            parse_po_borrowed("msgid \"file\"\nmsgid_plural \"bad\"quote\"\nmsgstr[0] \"\"\n")
                .expect_err("unescaped plural quote should fail");
        let plural_position = plural_error.position().expect("plural position metadata");
        assert_eq!(plural_error.message(), "unescaped quote in string literal");
        assert_eq!(plural_position.line(), 2);
        assert_eq!(plural_position.column(), 1);

        let context_error = parse_po_borrowed("msgctxt \"bad\"quote\"\nmsgid \"x\"\nmsgstr \"\"\n")
            .expect_err("unescaped context quote should fail");
        let context_position = context_error.position().expect("context position metadata");
        assert_eq!(context_error.message(), "unescaped quote in string literal");
        assert_eq!(context_position.line(), 1);
        assert_eq!(context_position.column(), 1);
    }

    #[test]
    fn parses_owned_header_fragments_in_keyword_and_continuation_lines() {
        let input = concat!(
            "msgid \"\"\n",
            "msgstr \"Project-Id-Version: ferrocat\\t1\\n\"\n",
            "\"Language: de\\t\\n\"\n",
        );
        let file = parse_po_borrowed(input).expect("borrowed parse with owned headers");

        assert_eq!(file.headers.len(), 2);
        assert_eq!(
            file.headers[0].key,
            Cow::<str>::Owned("Project-Id-Version".to_owned())
        );
        assert_eq!(
            file.headers[0].value,
            Cow::<str>::Owned("ferrocat\t1".to_owned())
        );
        assert_eq!(
            file.headers[1].key,
            Cow::<str>::Owned("Language".to_owned())
        );
        assert_eq!(file.headers[1].value, Cow::<str>::Owned("de".to_owned()));
    }

    #[test]
    fn parses_plural_metadata_flags_and_obsolete_items() {
        let input = concat!(
            "# translator\n",
            "#. extracted\n",
            "#: src/app.rs:1 src/lib.rs:2\n",
            "#, fuzzy, c-format\n",
            "#@ domain: admin\n",
            "msgctxt \"menu\"\n",
            "msgid \"file\"\n",
            "msgid_plural \"files\"\n",
            "msgstr[0] \"Datei\"\n",
            "msgstr[1] \"Dateien\"\n",
            "\n",
            "#~ msgid \"old\"\n",
            "#~ msgstr \"alt\"\n",
        );

        let file = parse_po_borrowed(input).expect("borrowed plural parse");
        assert_eq!(file.items.len(), 2);

        let item = &file.items[0];
        assert_eq!(item.msgctxt.as_deref(), Some("menu"));
        assert_eq!(item.msgid_plural.as_deref(), Some("files"));
        assert_eq!(
            item.msgstr,
            BorrowedMsgStr::Plural(vec![Cow::Borrowed("Datei"), Cow::Borrowed("Dateien"),])
        );
        assert_eq!(
            item.references.as_slice(),
            vec![Cow::Borrowed("src/app.rs:1"), Cow::Borrowed("src/lib.rs:2")].as_slice()
        );
        assert_eq!(
            item.flags.as_slice(),
            vec![Cow::Borrowed("fuzzy"), Cow::Borrowed("c-format")].as_slice()
        );
        assert_eq!(
            item.metadata.as_slice(),
            vec![(Cow::Borrowed("domain"), Cow::Borrowed("admin"))].as_slice()
        );

        assert!(file.items[1].obsolete);
        assert_eq!(file.items[1].msgid, Cow::Borrowed("old"));
    }

    #[test]
    fn parses_owned_headers_and_multiline_fields_when_escapes_are_present() {
        let input = concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Project-Id-Version: Demo \\\"Suite\\\"\\n\"\n",
            "\"Plural-Forms: nplurals=3; plural=(n > 1);\\n\"\n",
            "\n",
            "msgctxt \"cta\"\n",
            "msgid \"hel\"\n",
            "\"lo\"\n",
            "msgstr \"wor\"\n",
            "\"ld\"\n",
        );

        let file = parse_po_borrowed(input).expect("borrowed parse with owned headers");
        assert_eq!(
            file.headers[0],
            super::BorrowedHeader {
                key: Cow::Owned("Project-Id-Version".to_owned()),
                value: Cow::Owned("Demo \"Suite\"".to_owned()),
            }
        );
        assert_eq!(file.items[0].msgid, Cow::Borrowed("hello"));
        assert_eq!(file.items[0].msgctxt.as_deref(), Some("cta"));
        assert_eq!(
            file.items[0].msgstr,
            BorrowedMsgStr::Singular(Cow::Borrowed("world"))
        );
        assert_eq!(file.items[0].nplurals, 3);
    }

    #[test]
    fn parses_sparse_plural_slots_and_multiline_context_plural_and_msgstr() {
        let input = concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Plural-Forms: nplurals=3; plural=(n > 1);\\n\"\n",
            "\n",
            "msgctxt \"ct\"\n",
            "\"a\"\n",
            "msgid \"item\"\n",
            "msgid_plural \"items\"\n",
            "\" total\"\n",
            "msgstr[1] \"two\"\n",
            "\" plus\"\n",
            "msgstr[0] \"one\"\n",
            "\" plus\"\n",
            "\n",
            "msgid \"missing translation\"\n",
        );

        let file = parse_po_borrowed(input).expect("parse sparse plural");

        assert_eq!(file.items.len(), 2);
        let plural = &file.items[0];
        assert_eq!(plural.msgctxt.as_deref(), Some("cta"));
        assert_eq!(plural.msgid_plural.as_deref(), Some("items total"));
        assert_eq!(
            plural.msgstr,
            BorrowedMsgStr::Plural(vec![Cow::Borrowed("one plus"), Cow::Borrowed("two plus"),])
        );
        assert_eq!(file.items[1].msgid, Cow::Borrowed("missing translation"));
        assert_eq!(
            file.items[1].msgstr,
            BorrowedMsgStr::Singular(Cow::Borrowed(""))
        );
    }
}
