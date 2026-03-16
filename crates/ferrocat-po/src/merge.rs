use std::borrow::Cow;
use std::str;

use crate::scan::{
    CommentKind, Keyword, LineKind, LineScanner, classify_line, find_byte, find_quoted_bounds,
    has_byte, parse_plural_index, split_once_byte, trim_ascii,
};
use crate::serialize::{write_keyword, write_prefixed_line};
use crate::text::{escape_string_into, unescape_string, validate_quoted_content};
use crate::{BorrowedMsgStr, ParseError, SerializeOptions};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtractedMessage<'a> {
    pub msgctxt: Option<Cow<'a, str>>,
    pub msgid: Cow<'a, str>,
    pub msgid_plural: Option<Cow<'a, str>>,
    pub references: Vec<Cow<'a, str>>,
    pub extracted_comments: Vec<Cow<'a, str>>,
    pub flags: Vec<Cow<'a, str>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct MergeBorrowedFile<'a> {
    comments: Vec<&'a str>,
    extracted_comments: Vec<&'a str>,
    headers: Vec<MergeHeader<'a>>,
    items: Vec<MergeBorrowedItem<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct MergeHeader<'a> {
    key: Cow<'a, str>,
    value: Cow<'a, str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct MergeBorrowedItem<'a> {
    msgid: Cow<'a, str>,
    msgctxt: Option<Cow<'a, str>>,
    references: Vec<&'a str>,
    msgid_plural: Option<Cow<'a, str>>,
    msgstr: BorrowedMsgStr<'a>,
    comments: Vec<&'a str>,
    extracted_comments: Vec<&'a str>,
    flags: Vec<&'a str>,
    metadata: Vec<(&'a str, &'a str)>,
    obsolete: bool,
    nplurals: usize,
}

impl<'a> MergeBorrowedItem<'a> {
    fn new(nplurals: usize) -> Self {
        Self {
            nplurals,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Context {
    MsgId,
    MsgIdPlural,
    MsgStr,
    MsgCtxt,
}

#[derive(Debug)]
struct ParserState<'a> {
    item: MergeBorrowedItem<'a>,
    header_entries: Vec<MergeHeader<'a>>,
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
            item: MergeBorrowedItem::new(nplurals),
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

    #[inline]
    fn reset_after_take(&mut self, nplurals: usize) {
        self.item.nplurals = nplurals;
        self.header_entries.clear();
        self.msgstr = BorrowedMsgStr::None;
        self.context = None;
        self.plural_index = 0;
        self.obsolete_line_count = 0;
        self.content_line_count = 0;
        self.has_keyword = false;
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
        self.item.msgstr = std::mem::take(&mut self.msgstr);
    }

    fn promote_plural_msgstr(&mut self, plural_index: usize) -> &mut Vec<Cow<'a, str>> {
        if !matches!(self.msgstr, BorrowedMsgStr::Plural(_)) {
            self.msgstr = match std::mem::take(&mut self.msgstr) {
                BorrowedMsgStr::None => BorrowedMsgStr::Plural(Vec::with_capacity(2)),
                BorrowedMsgStr::Singular(value) => BorrowedMsgStr::Plural(vec![value]),
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
struct MergeLine<'a> {
    trimmed: &'a [u8],
    obsolete: bool,
}

pub fn merge_catalog<'a>(
    existing_po: &'a str,
    extracted_messages: &[ExtractedMessage<'a>],
) -> Result<String, ParseError> {
    let normalized;
    let input = if existing_po.as_bytes().contains(&b'\r') {
        normalized = existing_po.replace("\r\n", "\n").replace('\r', "\n");
        normalized.as_str()
    } else {
        existing_po
    };

    let existing = parse_merge_po(input)?;
    let nplurals = parse_nplurals(&existing.headers).unwrap_or(2);
    let options = SerializeOptions::default();
    let mut out = String::with_capacity(estimate_merge_capacity(input, extracted_messages));
    let mut scratch = String::new();

    write_file_preamble(&mut out, &existing);

    let mut existing_index =
        std::collections::HashMap::<&str, Vec<(Option<&str>, usize)>>::with_capacity(
            existing.items.len(),
        );
    for (index, item) in existing.items.iter().enumerate() {
        existing_index
            .entry(item.msgid.as_ref())
            .or_default()
            .push((item.msgctxt.as_deref(), index));
    }

    let mut matched = vec![false; existing.items.len()];
    let mut wrote_item = false;

    for extracted in extracted_messages {
        if wrote_item {
            out.push('\n');
        }
        let existing_index = find_existing_index(
            &existing_index,
            extracted.msgctxt.as_deref(),
            extracted.msgid.as_ref(),
        );

        match existing_index {
            Some(index) => {
                matched[index] = true;
                write_merged_existing_item(
                    &mut out,
                    &mut scratch,
                    &existing.items[index],
                    extracted,
                    nplurals,
                    &options,
                );
            }
            None => write_new_item(&mut out, &mut scratch, extracted, nplurals, &options),
        }
        out.push('\n');
        wrote_item = true;
    }

    for (index, item) in existing.items.iter().enumerate() {
        if matched[index] {
            continue;
        }
        if wrote_item {
            out.push('\n');
        }
        write_existing_item(&mut out, &mut scratch, item, true, &options);
        out.push('\n');
        wrote_item = true;
    }

    Ok(out)
}

fn parse_merge_po<'a>(input: &'a str) -> Result<MergeBorrowedFile<'a>, ParseError> {
    let mut file = MergeBorrowedFile::default();
    file.items.reserve((input.len() / 96).max(1));
    let mut current_nplurals = 2usize;
    let mut state = ParserState::new(current_nplurals);

    for line in LineScanner::new(input.as_bytes()) {
        parse_line(
            MergeLine {
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
    line: MergeLine<'a>,
    state: &mut ParserState<'a>,
    file: &mut MergeBorrowedFile<'a>,
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
    file: &mut MergeBorrowedFile<'a>,
    current_nplurals: &mut usize,
) -> Result<(), ParseError> {
    finish_item(state, file, current_nplurals)?;

    match kind {
        CommentKind::Reference => state.item.references.push(trimmed_str(&line_bytes[2..])?),
        CommentKind::Flags => {
            for flag in trimmed_str(&line_bytes[2..])?.split(',') {
                state.item.flags.push(flag.trim());
            }
        }
        CommentKind::Extracted => state
            .item
            .extracted_comments
            .push(trimmed_str(&line_bytes[2..])?),
        CommentKind::Metadata => {
            let trimmed = trim_ascii(&line_bytes[2..]);
            if let Some((key_bytes, value_bytes)) = split_once_byte(trimmed, b':') {
                let key = trimmed_str(key_bytes)?;
                if !key.is_empty() {
                    state.item.metadata.push((key, trimmed_str(value_bytes)?));
                }
            }
        }
        CommentKind::Translator => state.item.comments.push(trimmed_str(&line_bytes[1..])?),
        CommentKind::Other => {}
    }

    Ok(())
}

fn parse_keyword_line<'a>(
    line_bytes: &'a [u8],
    obsolete: bool,
    keyword: Keyword,
    state: &mut ParserState<'a>,
    file: &mut MergeBorrowedFile<'a>,
    current_nplurals: &mut usize,
) -> Result<(), ParseError> {
    match keyword {
        Keyword::MsgIdPlural => {
            state.obsolete_line_count += usize::from(obsolete);
            state.item.msgid_plural = Some(extract_merge_quoted_cow(line_bytes)?);
            state.context = Some(Context::MsgIdPlural);
            state.content_line_count += 1;
            state.has_keyword = true;
        }
        Keyword::MsgId => {
            finish_item(state, file, current_nplurals)?;
            state.obsolete_line_count += usize::from(obsolete);
            state.item.msgid = extract_merge_quoted_cow(line_bytes)?;
            state.context = Some(Context::MsgId);
            state.content_line_count += 1;
            state.has_keyword = true;
        }
        Keyword::MsgStr => {
            let plural_index = parse_plural_index(line_bytes).unwrap_or(0);
            state.plural_index = plural_index;
            state.obsolete_line_count += usize::from(obsolete);
            state.set_msgstr(plural_index, extract_merge_quoted_cow(line_bytes)?);
            if is_header_candidate(state) {
                state
                    .header_entries
                    .extend(parse_header_fragment(line_bytes)?);
            }
            state.context = Some(Context::MsgStr);
            state.content_line_count += 1;
            state.has_keyword = true;
        }
        Keyword::MsgCtxt => {
            finish_item(state, file, current_nplurals)?;
            state.obsolete_line_count += usize::from(obsolete);
            state.item.msgctxt = Some(extract_merge_quoted_cow(line_bytes)?);
            state.context = Some(Context::MsgCtxt);
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
    let value = extract_merge_quoted_cow(line_bytes)?;

    match state.context {
        Some(Context::MsgStr) => {
            state.append_msgstr(state.plural_index, value);
            if is_header_candidate(state) {
                state
                    .header_entries
                    .extend(parse_header_fragment(line_bytes)?);
            }
        }
        Some(Context::MsgId) => state.item.msgid.to_mut().push_str(value.as_ref()),
        Some(Context::MsgIdPlural) => {
            let target = state
                .item
                .msgid_plural
                .get_or_insert_with(|| Cow::Borrowed(""));
            target.to_mut().push_str(value.as_ref());
        }
        Some(Context::MsgCtxt) => {
            let target = state.item.msgctxt.get_or_insert_with(|| Cow::Borrowed(""));
            target.to_mut().push_str(value.as_ref());
        }
        None => {}
    }

    Ok(())
}

fn finish_item<'a>(
    state: &mut ParserState<'a>,
    file: &mut MergeBorrowedFile<'a>,
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

    if matches!(state.item.msgstr, BorrowedMsgStr::None) {
        state.item.msgstr = BorrowedMsgStr::Singular(Cow::Borrowed(""));
    }
    if state.item.msgid_plural.is_some() && msgstr_len(&state.item.msgstr) == 1 {
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
    state.reset_after_take(*current_nplurals);
    Ok(())
}

fn msgstr_len(msgstr: &BorrowedMsgStr<'_>) -> usize {
    match msgstr {
        BorrowedMsgStr::None => 0,
        BorrowedMsgStr::Singular(_) => 1,
        BorrowedMsgStr::Plural(values) => values.len(),
    }
}

fn is_header_state(state: &ParserState<'_>) -> bool {
    state.item.msgid.is_empty()
        && state.item.msgctxt.is_none()
        && state.item.msgid_plural.is_none()
        && !matches!(state.msgstr, BorrowedMsgStr::None)
}

fn is_header_candidate(state: &ParserState<'_>) -> bool {
    state.item.msgid.is_empty()
        && state.item.msgctxt.is_none()
        && state.item.msgid_plural.is_none()
        && state.plural_index == 0
}

fn parse_header_fragment<'a>(line_bytes: &'a [u8]) -> Result<Vec<MergeHeader<'a>>, ParseError> {
    let Some(raw) = merge_quoted_raw(line_bytes) else {
        return Ok(Vec::new());
    };

    if header_fragment_is_borrowable(raw) {
        return parse_header_fragment_borrowed(raw);
    }

    parse_header_fragment_owned(line_bytes)
}

fn parse_header_fragment_borrowed<'a>(raw: &'a [u8]) -> Result<Vec<MergeHeader<'a>>, ParseError> {
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
    out: &mut Vec<MergeHeader<'a>>,
) -> Result<(), ParseError> {
    if segment.is_empty() {
        return Ok(());
    }
    if let Some((key_bytes, value_bytes)) = split_once_byte(segment, b':') {
        out.push(MergeHeader {
            key: Cow::Borrowed(trimmed_str(key_bytes)?),
            value: Cow::Borrowed(trimmed_str(value_bytes)?),
        });
    }
    Ok(())
}

fn parse_header_fragment_owned<'a>(
    line_bytes: &'a [u8],
) -> Result<Vec<MergeHeader<'a>>, ParseError> {
    let decoded = extract_merge_quoted_cow(line_bytes)?;
    let mut headers = Vec::new();
    for segment in decoded.split('\n') {
        if segment.is_empty() {
            continue;
        }
        if let Some((key, value)) = segment.split_once(':') {
            headers.push(MergeHeader {
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

#[inline]
fn extract_merge_quoted_cow<'a>(line_bytes: &'a [u8]) -> Result<Cow<'a, str>, ParseError> {
    let Some(raw) = merge_quoted_raw(line_bytes) else {
        return Ok(Cow::Borrowed(""));
    };

    validate_quoted_content(raw)?;
    if !has_byte(b'\\', raw) {
        return Ok(Cow::Borrowed(bytes_to_str(raw)?));
    }

    Ok(Cow::Owned(unescape_string(bytes_to_str(raw)?)?))
}

#[inline]
fn merge_quoted_raw(line_bytes: &[u8]) -> Option<&[u8]> {
    let start = match line_bytes.first() {
        Some(b'"') => 1,
        _ => find_byte(b'"', line_bytes)? + 1,
    };

    if start > line_bytes.len() {
        return None;
    }

    if line_bytes.len() >= start + 1 && line_bytes.last() == Some(&b'"') {
        return Some(&line_bytes[start..line_bytes.len() - 1]);
    }

    let (quoted_start, quoted_end) = find_quoted_bounds(line_bytes)?;
    Some(&line_bytes[quoted_start..quoted_end])
}

fn find_existing_index(
    existing_index: &std::collections::HashMap<&str, Vec<(Option<&str>, usize)>>,
    msgctxt: Option<&str>,
    msgid: &str,
) -> Option<usize> {
    let candidates = existing_index.get(msgid)?;
    candidates
        .iter()
        .find_map(|(candidate_ctxt, index)| (*candidate_ctxt == msgctxt).then_some(*index))
}

fn estimate_merge_capacity(input: &str, extracted_messages: &[ExtractedMessage<'_>]) -> usize {
    let extracted_bytes: usize = extracted_messages
        .iter()
        .map(|message| {
            message.msgid.len()
                + message.msgctxt.as_ref().map_or(0, |value| value.len())
                + message.msgid_plural.as_ref().map_or(0, |value| value.len())
                + message
                    .references
                    .iter()
                    .map(|value| value.len())
                    .sum::<usize>()
                + message
                    .extracted_comments
                    .iter()
                    .map(|value| value.len())
                    .sum::<usize>()
                + message.flags.iter().map(|value| value.len()).sum::<usize>()
        })
        .sum();

    input.len() + extracted_bytes + 256
}

fn write_file_preamble(out: &mut String, file: &MergeBorrowedFile<'_>) {
    write_prefixed_lines(out, "", "#", &file.comments);
    write_prefixed_lines(out, "", "#.", &file.extracted_comments);

    out.push_str("msgid \"\"\n");
    out.push_str("msgstr \"\"\n");
    for header in &file.headers {
        out.push('"');
        escape_string_into(out, header.key.as_ref());
        out.push_str(": ");
        escape_string_into(out, header.value.as_ref());
        out.push_str("\\n\"\n");
    }
    out.push('\n');
}

fn write_merged_existing_item(
    out: &mut String,
    scratch: &mut String,
    existing: &MergeBorrowedItem<'_>,
    extracted: &ExtractedMessage<'_>,
    nplurals: usize,
    options: &SerializeOptions,
) {
    let obsolete_prefix = "";

    write_prefixed_lines(out, obsolete_prefix, "#", &existing.comments);
    write_prefixed_lines(out, obsolete_prefix, "#.", &extracted.extracted_comments);
    write_metadata_lines(out, obsolete_prefix, &existing.metadata);
    write_prefixed_lines(out, obsolete_prefix, "#:", &extracted.references);
    write_merged_flags_line(out, obsolete_prefix, &existing.flags, &extracted.flags);

    if let Some(context) = extracted.msgctxt.as_deref() {
        write_keyword(
            out,
            scratch,
            obsolete_prefix,
            "msgctxt",
            context,
            None,
            options,
        );
    }
    write_keyword(
        out,
        scratch,
        obsolete_prefix,
        "msgid",
        extracted.msgid.as_ref(),
        None,
        options,
    );
    if let Some(plural) = extracted.msgid_plural.as_deref() {
        write_keyword(
            out,
            scratch,
            obsolete_prefix,
            "msgid_plural",
            plural,
            None,
            options,
        );
    }

    write_normalized_msgstr(
        out,
        scratch,
        obsolete_prefix,
        &existing.msgstr,
        existing.msgid_plural.is_some(),
        extracted.msgid_plural.is_some(),
        nplurals,
        options,
    );
}

fn write_new_item(
    out: &mut String,
    scratch: &mut String,
    extracted: &ExtractedMessage<'_>,
    nplurals: usize,
    options: &SerializeOptions,
) {
    let obsolete_prefix = "";

    write_prefixed_lines(out, obsolete_prefix, "#.", &extracted.extracted_comments);
    write_prefixed_lines(out, obsolete_prefix, "#:", &extracted.references);
    write_flags_line(out, obsolete_prefix, &extracted.flags);

    if let Some(context) = extracted.msgctxt.as_deref() {
        write_keyword(
            out,
            scratch,
            obsolete_prefix,
            "msgctxt",
            context,
            None,
            options,
        );
    }
    write_keyword(
        out,
        scratch,
        obsolete_prefix,
        "msgid",
        extracted.msgid.as_ref(),
        None,
        options,
    );
    if let Some(plural) = extracted.msgid_plural.as_deref() {
        write_keyword(
            out,
            scratch,
            obsolete_prefix,
            "msgid_plural",
            plural,
            None,
            options,
        );
    }

    write_default_msgstr(
        out,
        scratch,
        obsolete_prefix,
        extracted.msgid_plural.is_some(),
        nplurals,
        options,
    );
}

fn write_existing_item(
    out: &mut String,
    scratch: &mut String,
    item: &MergeBorrowedItem<'_>,
    obsolete: bool,
    options: &SerializeOptions,
) {
    let obsolete_prefix = if obsolete { "#~ " } else { "" };

    write_prefixed_lines(out, obsolete_prefix, "#", &item.comments);
    write_prefixed_lines(out, obsolete_prefix, "#.", &item.extracted_comments);
    write_metadata_lines(out, obsolete_prefix, &item.metadata);
    write_prefixed_lines(out, obsolete_prefix, "#:", &item.references);
    write_flags_line(out, obsolete_prefix, &item.flags);

    if let Some(context) = item.msgctxt.as_deref() {
        write_keyword(
            out,
            scratch,
            obsolete_prefix,
            "msgctxt",
            context,
            None,
            options,
        );
    }
    write_keyword(
        out,
        scratch,
        obsolete_prefix,
        "msgid",
        item.msgid.as_ref(),
        None,
        options,
    );
    if let Some(plural) = item.msgid_plural.as_deref() {
        write_keyword(
            out,
            scratch,
            obsolete_prefix,
            "msgid_plural",
            plural,
            None,
            options,
        );
    }

    write_existing_msgstr(
        out,
        scratch,
        obsolete_prefix,
        &item.msgstr,
        item.msgid_plural.is_some(),
        item.nplurals,
        options,
    );
}

fn write_prefixed_lines<T: AsRef<str>>(
    out: &mut String,
    obsolete_prefix: &str,
    prefix: &str,
    values: &[T],
) {
    for value in values {
        write_prefixed_line(out, obsolete_prefix, prefix, value.as_ref());
    }
}

fn write_metadata_lines(out: &mut String, obsolete_prefix: &str, values: &[(&str, &str)]) {
    for (key, value) in values {
        out.push_str(obsolete_prefix);
        out.push_str("#@ ");
        out.push_str(key);
        out.push_str(": ");
        out.push_str(value);
        out.push('\n');
    }
}

fn write_flags_line<T: AsRef<str>>(out: &mut String, obsolete_prefix: &str, values: &[T]) {
    if values.is_empty() {
        return;
    }

    out.push_str(obsolete_prefix);
    out.push_str("#, ");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(value.as_ref());
    }
    out.push('\n');
}

fn write_merged_flags_line(
    out: &mut String,
    obsolete_prefix: &str,
    existing: &[&str],
    extracted: &[Cow<'_, str>],
) {
    if existing.is_empty() && extracted.is_empty() {
        return;
    }

    out.push_str(obsolete_prefix);
    out.push_str("#, ");

    let mut wrote_any = false;
    let mut seen = Vec::with_capacity(existing.len() + extracted.len());
    for flag in existing
        .iter()
        .copied()
        .chain(extracted.iter().map(|value| value.as_ref()))
    {
        if seen.iter().any(|existing| *existing == flag) {
            continue;
        }
        if wrote_any {
            out.push(',');
        }
        out.push_str(flag);
        wrote_any = true;
        seen.push(flag);
    }
    out.push('\n');
}

fn write_existing_msgstr(
    out: &mut String,
    scratch: &mut String,
    obsolete_prefix: &str,
    msgstr: &BorrowedMsgStr<'_>,
    is_plural: bool,
    nplurals: usize,
    options: &SerializeOptions,
) {
    if is_plural {
        for index in 0..nplurals.max(1) {
            let value = match msgstr {
                BorrowedMsgStr::None => "",
                BorrowedMsgStr::Singular(value) if index == 0 => value.as_ref(),
                BorrowedMsgStr::Singular(_) => "",
                BorrowedMsgStr::Plural(values) => {
                    values.get(index).map_or("", |value| value.as_ref())
                }
            };
            write_keyword(
                out,
                scratch,
                obsolete_prefix,
                "msgstr",
                value,
                Some(index),
                options,
            );
        }
        return;
    }

    let value = match msgstr {
        BorrowedMsgStr::None => "",
        BorrowedMsgStr::Singular(value) => value.as_ref(),
        BorrowedMsgStr::Plural(values) => values.first().map_or("", |value| value.as_ref()),
    };
    write_keyword(
        out,
        scratch,
        obsolete_prefix,
        "msgstr",
        value,
        None,
        options,
    );
}

fn write_normalized_msgstr(
    out: &mut String,
    scratch: &mut String,
    obsolete_prefix: &str,
    msgstr: &BorrowedMsgStr<'_>,
    was_plural: bool,
    is_plural: bool,
    nplurals: usize,
    options: &SerializeOptions,
) {
    if was_plural != is_plural {
        write_default_msgstr(out, scratch, obsolete_prefix, is_plural, nplurals, options);
        return;
    }

    write_existing_msgstr(
        out,
        scratch,
        obsolete_prefix,
        msgstr,
        is_plural,
        nplurals,
        options,
    );
}

fn write_default_msgstr(
    out: &mut String,
    scratch: &mut String,
    obsolete_prefix: &str,
    is_plural: bool,
    nplurals: usize,
    options: &SerializeOptions,
) {
    if is_plural {
        for index in 0..nplurals.max(1) {
            write_keyword(
                out,
                scratch,
                obsolete_prefix,
                "msgstr",
                "",
                Some(index),
                options,
            );
        }
        return;
    }

    write_keyword(out, scratch, obsolete_prefix, "msgstr", "", None, options);
}

fn parse_nplurals(headers: &[MergeHeader<'_>]) -> Option<usize> {
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

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{ExtractedMessage, merge_catalog};
    use crate::parse_po;

    #[test]
    fn preserves_existing_translations_and_updates_references() {
        let existing = concat!(
            "msgid \"hello\"\n",
            "msgstr \"world\"\n\n",
            "msgid \"old\"\n",
            "msgstr \"alt\"\n",
        );
        let extracted = vec![ExtractedMessage {
            msgid: Cow::Borrowed("hello"),
            references: vec![Cow::Borrowed("src/new.rs:10")],
            ..ExtractedMessage::default()
        }];

        let merged = merge_catalog(existing, &extracted).expect("merge");
        let reparsed = parse_po(&merged).expect("reparse");
        let old_items: Vec<_> = reparsed
            .items
            .iter()
            .filter(|item| item.msgid == "old")
            .map(|item| (item.obsolete, item.msgstr[0].clone()))
            .collect();
        assert_eq!(old_items, vec![(true, "alt".to_owned())]);

        let hello = reparsed
            .items
            .iter()
            .find(|item| item.msgid == "hello")
            .expect("merged hello item");
        assert_eq!(hello.msgstr[0], "world");
        assert_eq!(hello.references, vec!["src/new.rs:10".to_owned()]);
    }

    #[test]
    fn creates_new_items_for_new_extracted_messages() {
        let merged = merge_catalog(
            "",
            &[ExtractedMessage {
                msgid: Cow::Borrowed("fresh"),
                extracted_comments: vec![Cow::Borrowed("from extractor")],
                ..ExtractedMessage::default()
            }],
        )
        .expect("merge");
        let reparsed = parse_po(&merged).expect("reparse");

        assert_eq!(reparsed.items[0].msgid, "fresh");
        assert_eq!(reparsed.items[0].msgstr[0], "");
        assert_eq!(
            reparsed.items[0].extracted_comments,
            vec!["from extractor".to_owned()]
        );
    }

    #[test]
    fn resets_msgstr_when_switching_between_singular_and_plural() {
        let existing = concat!("msgid \"count\"\n", "msgstr \"Anzahl\"\n",);
        let extracted = vec![ExtractedMessage {
            msgid: Cow::Borrowed("count"),
            msgid_plural: Some(Cow::Borrowed("counts")),
            ..ExtractedMessage::default()
        }];

        let merged = merge_catalog(existing, &extracted).expect("merge");
        let reparsed = parse_po(&merged).expect("reparse");

        assert!(reparsed.items[0].msgid_plural.is_some());
        assert_eq!(reparsed.items[0].msgstr.len(), 2);
        assert_eq!(reparsed.items[0].msgstr[0], "");
        assert_eq!(reparsed.items[0].msgstr[1], "");
    }
}
