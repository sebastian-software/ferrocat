use std::borrow::Cow;

use rustc_hash::FxHashMap;

use crate::line_state::{PoLineContext, PoLineState};
use crate::scan::{
    CommentKind, Keyword, LineKind, LineScanner, checked_plural_forms, checked_plural_index,
    classify_line, find_quoted_raw_from, has_byte, split_once_byte, trim_ascii, trim_ascii_start,
    unrecognized_po_line,
};
use crate::serialize::{write_keyword, write_prefixed_line};
use crate::text::{escape_string_into, extract_quoted_bytes_cow_from};
use crate::utf8::input_slice_as_str;
use crate::{BorrowedMsgStr, ParseError, ParsePosition, SerializeOptions};

/// Borrowed message input for the lightweight PO merge helper.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MergeMessageInput<'a> {
    /// Optional gettext message context.
    pub msgctxt: Option<Cow<'a, str>>,
    /// Source message identifier.
    pub msgid: Cow<'a, str>,
    /// Optional plural source identifier.
    pub msgid_plural: Option<Cow<'a, str>>,
    /// Source references such as `src/app.rs:10`.
    pub references: Vec<Cow<'a, str>>,
    /// Extracted translator guidance comments.
    pub extracted_comments: Vec<Cow<'a, str>>,
    /// Flags such as `fuzzy`.
    pub flags: Vec<Cow<'a, str>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct MergeBorrowedFile<'a> {
    comments: Vec<&'a str>,
    extracted_comments: Vec<&'a str>,
    headers: Vec<MergeHeader<'a>>,
    items: Vec<MergeBorrowedItem<'a>>,
    nplurals: usize,
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

impl MergeBorrowedItem<'_> {
    fn new(nplurals: usize) -> Self {
        Self {
            nplurals,
            ..Self::default()
        }
    }
}

#[derive(Debug)]
struct ParserState<'a> {
    item: MergeBorrowedItem<'a>,
    header_entries: Vec<MergeHeader<'a>>,
    header_entries_faithful: bool,
    header_line_open: bool,
    msgstr: BorrowedMsgStr<'a>,
    line: PoLineState,
}

impl<'a> ParserState<'a> {
    fn new(nplurals: usize) -> Self {
        Self {
            item: MergeBorrowedItem::new(nplurals),
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

    #[inline]
    fn reset_after_take(&mut self, nplurals: usize) {
        self.item.nplurals = nplurals;
        self.header_entries.clear();
        self.header_entries_faithful = true;
        self.header_line_open = false;
        self.msgstr = BorrowedMsgStr::None;
        self.line.reset();
    }

    fn set_msgstr(&mut self, plural_index: usize, value: Cow<'a, str>) {
        self.msgstr.set_slot(plural_index, value);
    }

    fn append_msgstr(&mut self, plural_index: usize, value: Cow<'a, str>) {
        self.msgstr.append_slot(plural_index, value);
    }

    fn push_header_fragment(&mut self, line_bytes: &'a [u8], starts_new_msgstr: bool) {
        if starts_new_msgstr {
            self.header_entries.clear();
            self.header_line_open = false;
        }
        if !self.header_entries_faithful {
            return;
        }

        let search_from = if starts_new_msgstr { 6 } else { 0 };
        let Some(raw) = find_quoted_raw_from(line_bytes, search_from) else {
            self.header_entries_faithful = false;
            return;
        };

        if self.header_line_open
            || !header_fragment_is_borrowable(raw)
            || !push_borrowed_header_segments(raw, &mut self.header_entries)
        {
            self.header_entries_faithful = false;
            return;
        }

        self.header_line_open = !raw.is_empty() && !raw.ends_with(br"\n");
    }

    fn header_msgstr(&self) -> &str {
        match &self.msgstr {
            BorrowedMsgStr::None => "",
            BorrowedMsgStr::Singular(value) => value.as_ref(),
            BorrowedMsgStr::Plural(values) => values.first().map_or("", Cow::as_ref),
        }
    }

    fn materialize_msgstr(&mut self) {
        self.item.msgstr = std::mem::take(&mut self.msgstr);
    }
}

#[derive(Debug, Clone, Copy)]
struct MergeLine<'a> {
    trimmed: &'a [u8],
    obsolete: bool,
    position: ParsePosition,
}

/// Merges extracted messages into an existing PO catalog while preserving the
/// existing translation payload.
///
/// # Errors
///
/// Returns [`ParseError`] when the existing PO file cannot be parsed or
/// exceeds the limit of 128 plural slots per entry.
///
/// # Examples
///
/// ```rust
/// use std::borrow::Cow;
///
/// use ferrocat_po::{MergeMessageInput, merge_catalog};
///
/// let existing = "msgid \"Hello\"\nmsgstr \"Hallo\"\n";
/// let extracted = [MergeMessageInput {
///     msgid: Cow::Borrowed("Hello"),
///     references: vec![Cow::Borrowed("src/app.rs:10")],
///     ..MergeMessageInput::default()
/// }];
///
/// let merged = merge_catalog(existing, &extracted)?;
/// assert!(merged.contains("msgstr \"Hallo\""));
/// assert!(merged.contains("#: src/app.rs:10"));
/// # Ok::<(), ferrocat_po::ParseError>(())
/// ```
pub fn merge_catalog<'a>(
    existing_po: &'a str,
    extracted_messages: &[MergeMessageInput<'a>],
) -> Result<String, ParseError> {
    let existing = parse_merge_po(existing_po)?;
    let nplurals = existing.nplurals;
    let options = SerializeOptions::default();
    let mut out = String::with_capacity(estimate_merge_capacity(existing_po, extracted_messages));
    let mut scratch = String::new();

    write_file_preamble(&mut out, &existing);

    let mut existing_index = FxHashMap::<(&str, Option<&str>), usize>::with_capacity_and_hasher(
        existing.items.len(),
        Default::default(),
    );
    for (index, item) in existing.items.iter().enumerate() {
        existing_index
            .entry((item.msgid.as_ref(), item.msgctxt.as_deref()))
            .or_insert(index);
    }

    let mut matched = vec![false; existing.items.len()];
    let mut wrote_item = false;

    for extracted in extracted_messages {
        if wrote_item {
            out.push('\n');
        }
        let existing_index = existing_index
            .get(&(extracted.msgid.as_ref(), extracted.msgctxt.as_deref()))
            .copied();

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

fn parse_merge_po(input: &str) -> Result<MergeBorrowedFile<'_>, ParseError> {
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    let mut file = MergeBorrowedFile {
        nplurals: 2,
        ..MergeBorrowedFile::default()
    };
    file.items.reserve((input.len() / 96).max(1));
    let mut current_nplurals = 2usize;
    let mut state = ParserState::new(current_nplurals);

    let mut last_position = ParsePosition::new(0, 1, 1);
    for line in LineScanner::new(input.as_bytes()) {
        last_position = line.position;
        parse_line(
            MergeLine {
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

fn parse_line<'a>(
    line: MergeLine<'a>,
    state: &mut ParserState<'a>,
    file: &mut MergeBorrowedFile<'a>,
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

fn parse_comment_line<'a>(
    line_bytes: &'a [u8],
    kind: CommentKind,
    state: &mut ParserState<'a>,
    file: &mut MergeBorrowedFile<'a>,
    current_nplurals: &mut usize,
    position: ParsePosition,
) -> Result<(), ParseError> {
    finish_item(state, file, current_nplurals, position)?;

    match kind {
        CommentKind::Reference => state.item.references.push(trimmed_str(&line_bytes[2..])),
        CommentKind::Flags => {
            for flag in trimmed_str(&line_bytes[2..]).split(',') {
                state.item.flags.push(flag.trim());
            }
        }
        CommentKind::Extracted => state
            .item
            .extracted_comments
            .push(trimmed_str(&line_bytes[2..])),
        CommentKind::Metadata => {
            let trimmed = trim_ascii(&line_bytes[2..]);
            if let Some(value_bytes) = ferrocat_mt_metadata_value(trimmed) {
                state
                    .item
                    .metadata
                    .push(("ferrocat-mt", trimmed_str(value_bytes)));
            } else if let Some((key_bytes, value_bytes)) = split_once_byte(trimmed, b':') {
                let key = trimmed_str(key_bytes);
                if !key.is_empty() {
                    state.item.metadata.push((key, trimmed_str(value_bytes)));
                }
            }
        }
        CommentKind::Translator => state.item.comments.push(trimmed_str(&line_bytes[1..])),
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

fn parse_keyword_line<'a>(
    line_bytes: &'a [u8],
    obsolete: bool,
    position: ParsePosition,
    keyword: Keyword,
    state: &mut ParserState<'a>,
    file: &mut MergeBorrowedFile<'a>,
    current_nplurals: &mut usize,
) -> Result<(), ParseError> {
    match keyword {
        Keyword::IdPlural => {
            state
                .line
                .mark_keyword(PoLineContext::IdPlural, 0, obsolete);
            state.item.msgid_plural = Some(at_line_position(
                extract_quoted_bytes_cow_from(line_bytes, 12),
                position,
            )?);
        }
        Keyword::Id => {
            finish_item(state, file, current_nplurals, position)?;
            state.line.mark_keyword(PoLineContext::Id, 0, obsolete);
            state.item.msgid =
                at_line_position(extract_quoted_bytes_cow_from(line_bytes, 5), position)?;
        }
        Keyword::Str => {
            let plural_index = checked_plural_index(line_bytes, position)?;
            state
                .line
                .mark_keyword(PoLineContext::Str, plural_index, obsolete);
            state.set_msgstr(
                plural_index,
                at_line_position(extract_quoted_bytes_cow_from(line_bytes, 6), position)?,
            );
            if is_header_candidate(state) {
                state.push_header_fragment(line_bytes, true);
            }
        }
        Keyword::Ctxt => {
            finish_item(state, file, current_nplurals, position)?;
            state.line.mark_keyword(PoLineContext::Ctxt, 0, obsolete);
            state.item.msgctxt = Some(at_line_position(
                extract_quoted_bytes_cow_from(line_bytes, 7),
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
    let value = at_line_position(extract_quoted_bytes_cow_from(line_bytes, 0), position)?;

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
    file: &mut MergeBorrowedFile<'a>,
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
        file.comments = std::mem::take(&mut state.item.comments);
        file.extracted_comments = std::mem::take(&mut state.item.extracted_comments);
        if state.header_entries_faithful {
            file.headers = std::mem::take(&mut state.header_entries);
        } else {
            parse_owned_headers(state.header_msgstr(), &mut file.headers);
        }
        *current_nplurals = match file
            .headers
            .iter()
            .find(|header| header.key == "Plural-Forms")
        {
            Some(header) => checked_plural_forms(&header.value, position)?.unwrap_or(2),
            None => 2,
        };
        file.nplurals = *current_nplurals;
        state.reset(*current_nplurals);
        return Ok(());
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
    state.reset_after_take(*current_nplurals);
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
        && state.line.plural_index() == 0
}

fn push_borrowed_header_segments<'a>(raw: &'a [u8], out: &mut Vec<MergeHeader<'a>>) -> bool {
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

fn push_borrowed_header_segment<'a>(segment: &'a [u8], out: &mut Vec<MergeHeader<'a>>) -> bool {
    let segment = trim_ascii_start(segment);
    if segment.is_empty() {
        return true;
    }
    if segment.starts_with(b"#~") {
        return false;
    }
    if let Some((key_bytes, value_bytes)) = split_once_byte(segment, b':') {
        out.push(MergeHeader {
            key: Cow::Borrowed(trimmed_str(key_bytes)),
            value: Cow::Borrowed(trimmed_str(value_bytes)),
        });
    }
    true
}

fn parse_owned_headers<'a>(raw: &str, out: &mut Vec<MergeHeader<'a>>) {
    for line in LineScanner::new(raw.as_bytes()) {
        if let Some((key_bytes, value_bytes)) = split_once_byte(line.trimmed, b':') {
            out.push(MergeHeader {
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

fn estimate_merge_capacity(input: &str, extracted_messages: &[MergeMessageInput<'_>]) -> usize {
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
    extracted: &MergeMessageInput<'_>,
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
        MsgstrShape {
            preserve_existing: existing.msgid_plural.is_some() == extracted.msgid_plural.is_some(),
            plural: extracted.msgid_plural.is_some(),
        },
        nplurals,
        options,
    );
}

fn write_new_item(
    out: &mut String,
    scratch: &mut String,
    extracted: &MergeMessageInput<'_>,
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
        write_metadata_line(out, key, value);
        out.push('\n');
    }
}

fn write_metadata_line(out: &mut String, key: &str, value: &str) {
    out.push_str("#@ ");
    out.push_str(key);
    if key == "ferrocat-mt" {
        out.push(' ');
    } else {
        out.push_str(": ");
    }
    out.push_str(value);
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
        .chain(extracted.iter().map(AsRef::as_ref))
    {
        if seen.contains(&flag) {
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
                BorrowedMsgStr::Singular(value) if index == 0 => value.as_ref(),
                BorrowedMsgStr::None | BorrowedMsgStr::Singular(_) => "",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MsgstrShape {
    preserve_existing: bool,
    plural: bool,
}

fn write_normalized_msgstr(
    out: &mut String,
    scratch: &mut String,
    obsolete_prefix: &str,
    msgstr: &BorrowedMsgStr<'_>,
    shape: MsgstrShape,
    nplurals: usize,
    options: &SerializeOptions,
) {
    if !shape.preserve_existing {
        write_default_msgstr(
            out,
            scratch,
            obsolete_prefix,
            shape.plural,
            nplurals,
            options,
        );
        return;
    }

    write_existing_msgstr(
        out,
        scratch,
        obsolete_prefix,
        msgstr,
        shape.plural,
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

fn bytes_to_str(bytes: &[u8]) -> &str {
    input_slice_as_str(bytes)
}

fn trimmed_str(bytes: &[u8]) -> &str {
    bytes_to_str(trim_ascii(bytes))
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{
        MergeHeader, MergeMessageInput, estimate_merge_capacity, header_fragment_is_borrowable,
        merge_catalog, parse_merge_po,
    };
    use crate::text::extract_quoted_bytes_cow_from;
    use crate::{BorrowedMsgStr, parse_po};

    #[test]
    fn preserves_existing_translations_and_updates_references() {
        let existing = concat!(
            "msgid \"hello\"\n",
            "msgstr \"world\"\n\n",
            "msgid \"old\"\n",
            "msgstr \"alt\"\n",
        );
        let extracted = vec![MergeMessageInput {
            msgid: Cow::Borrowed("hello"),
            references: vec![Cow::Borrowed("src/new.rs:10")],
            ..MergeMessageInput::default()
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
        assert_eq!(
            hello.references.as_slice(),
            vec!["src/new.rs:10".to_owned()].as_slice()
        );
    }

    #[test]
    fn creates_new_items_for_new_extracted_messages() {
        let merged = merge_catalog(
            "",
            &[MergeMessageInput {
                msgid: Cow::Borrowed("fresh"),
                extracted_comments: vec![Cow::Borrowed("from extractor")],
                ..MergeMessageInput::default()
            }],
        )
        .expect("merge");
        let reparsed = parse_po(&merged).expect("reparse");

        assert_eq!(reparsed.items[0].msgid, "fresh");
        assert_eq!(reparsed.items[0].msgstr[0], "");
        assert_eq!(
            reparsed.items[0].extracted_comments.as_slice(),
            vec!["from extractor".to_owned()].as_slice()
        );
    }

    #[test]
    fn resets_msgstr_when_switching_between_singular_and_plural() {
        let existing = concat!("msgid \"count\"\n", "msgstr \"Anzahl\"\n",);
        let extracted = vec![MergeMessageInput {
            msgid: Cow::Borrowed("count"),
            msgid_plural: Some(Cow::Borrowed("counts")),
            ..MergeMessageInput::default()
        }];

        let merged = merge_catalog(existing, &extracted).expect("merge");
        let reparsed = parse_po(&merged).expect("reparse");

        assert!(reparsed.items[0].msgid_plural.is_some());
        assert_eq!(reparsed.items[0].msgstr.len(), 2);
        assert_eq!(reparsed.items[0].msgstr[0], "");
        assert_eq!(reparsed.items[0].msgstr[1], "");
    }

    #[test]
    fn merge_helpers_cover_header_and_lookup_paths() {
        assert_eq!(
            extract_quoted_bytes_cow_from(br#"msgid "plain""#, 5),
            Ok(Cow::Borrowed("plain"))
        );
        assert_eq!(
            extract_quoted_bytes_cow_from(br#"msgid "line\nbreak""#, 5),
            Ok(Cow::Owned("line\nbreak".to_owned()))
        );
        assert!(header_fragment_is_borrowable(
            br#"Language: de\nPlural-Forms: nplurals=2;\n"#
        ));
        assert!(!header_fragment_is_borrowable(br#"Language: \"de\"\n"#));
        assert_eq!(
            parse_merge_po(concat!(
                "msgid \"\"\nmsgstr \"\"\n",
                "\"Language: de\\nPlural-Forms: nplurals=3; plural=(n>1);\\n\"\n",
            ))
            .expect("borrowable multi-header fragment")
            .headers,
            vec![
                MergeHeader {
                    key: Cow::Borrowed("Language"),
                    value: Cow::Borrowed("de"),
                },
                MergeHeader {
                    key: Cow::Borrowed("Plural-Forms"),
                    value: Cow::Borrowed("nplurals=3; plural=(n>1);"),
                },
            ]
        );
        assert_eq!(
            parse_merge_po(concat!(
                "msgid \"\"\nmsgstr \"\"\n",
                "\"Language: de\\n\"\n",
                "\"Plural-Forms: nplurals = 4 ; plural = (n > 1);\\n\"\n",
            ))
            .expect("header is valid")
            .nplurals,
            4
        );
        // Duplicate (msgid, msgctxt) in existing catalog keeps the first match.
        let duplicate_existing = concat!(
            "msgctxt \"menu\"\nmsgid \"hello\"\nmsgstr \"first\"\n\n",
            "msgctxt \"menu\"\nmsgid \"hello\"\nmsgstr \"second\"\n",
        );
        let merged_dup = merge_catalog(
            duplicate_existing,
            &[MergeMessageInput {
                msgctxt: Some(Cow::Borrowed("menu")),
                msgid: Cow::Borrowed("hello"),
                ..MergeMessageInput::default()
            }],
        )
        .expect("merge duplicate");
        let reparsed_dup = parse_po(&merged_dup).expect("reparse duplicate");
        assert_eq!(reparsed_dup.items[0].msgstr[0], "first");
        assert!(
            estimate_merge_capacity(
                "msgid \"a\"\nmsgstr \"b\"\n",
                &[MergeMessageInput {
                    msgid: Cow::Borrowed("hello"),
                    references: vec![Cow::Borrowed("src/app.rs:1")],
                    ..MergeMessageInput::default()
                }],
            ) > 24
        );
    }

    #[test]
    fn merge_preserves_existing_plural_values_and_dedupes_flags() {
        let existing = concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Plural-Forms: nplurals=3; plural=(n > 1);\\n\"\n",
            "\n",
            "#, fuzzy\n",
            "msgid \"count\"\n",
            "msgid_plural \"counts\"\n",
            "msgstr[0] \"eins\"\n",
            "msgstr[1] \"zwei\"\n",
            "msgstr[2] \"viele\"\n",
        );
        let extracted = vec![MergeMessageInput {
            msgid: Cow::Borrowed("count"),
            msgid_plural: Some(Cow::Borrowed("counts")),
            flags: vec![Cow::Borrowed("fuzzy"), Cow::Borrowed("rust-format")],
            ..MergeMessageInput::default()
        }];

        let merged = merge_catalog(existing, &extracted).expect("merge plural");
        assert!(merged.contains("#, fuzzy,rust-format"));
        let reparsed = parse_po(&merged).expect("reparse merged plural");
        assert_eq!(reparsed.items[0].msgstr.len(), 3);
        assert_eq!(reparsed.items[0].msgstr[0], "eins");
        assert_eq!(reparsed.items[0].msgstr[2], "viele");
    }

    #[test]
    fn merge_normalizes_crlf_input_and_keeps_existing_single_value() {
        let existing = "msgid \"hello\"\r\nmsgstr \"world\"\r\n";
        let merged = merge_catalog(
            existing,
            &[MergeMessageInput {
                msgid: Cow::Borrowed("hello"),
                ..MergeMessageInput::default()
            }],
        )
        .expect("merge normalized crlf");

        let reparsed = parse_po(&merged).expect("reparse normalized");
        assert_eq!(reparsed.items[0].msgstr[0], "world");
        assert!(!matches!(
            BorrowedMsgStr::Singular(Cow::Borrowed("world")),
            BorrowedMsgStr::None
        ));
    }

    #[test]
    fn merge_preserves_context_comments_metadata_and_marks_removed_items_obsolete() {
        let existing = concat!(
            "# Header translator\n",
            "#. Header extracted\n",
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Language: de\\n\"\n",
            "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n",
            "\n",
            "# Translator note\n",
            "#. Old extracted note\n",
            "#@ owner: checkout\n",
            "#: old.rs:1\n",
            "#, fuzzy,php-format\n",
            "msgctxt \"button\"\n",
            "msgid \"Save\"\n",
            "\" now\"\n",
            "msgstr \"Speichern\"\n",
            "\n",
            "# Translator old only\n",
            "msgid \"Remove me\"\n",
            "msgstr \"Entferne mich\"\n",
        );
        let extracted = vec![MergeMessageInput {
            msgctxt: Some(Cow::Borrowed("button")),
            msgid: Cow::Borrowed("Save now"),
            references: vec![Cow::Borrowed("new.rs:2")],
            extracted_comments: vec![Cow::Borrowed("Fresh extracted note")],
            flags: vec![Cow::Borrowed("fuzzy"), Cow::Borrowed("rust-format")],
            ..MergeMessageInput::default()
        }];

        let merged = merge_catalog(existing, &extracted).expect("merge context item");

        assert!(merged.contains("# Header translator\n#. Header extracted\nmsgid \"\""));
        assert!(merged.contains("# Translator note\n#. Fresh extracted note\n#@ owner: checkout"));
        assert!(merged.contains("#: new.rs:2\n#, fuzzy,php-format,rust-format"));
        assert!(merged.contains("msgctxt \"button\"\nmsgid \"Save now\"\nmsgstr \"Speichern\""));
        assert!(merged.contains("#~ # Translator old only\n#~ msgid \"Remove me\""));
    }

    #[test]
    fn merge_materializes_plural_slots_from_header_and_resets_shape_changes() {
        let existing = concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Plural-Forms: nplurals=3; plural=(n > 1);\\n\"\n",
            "\n",
            "msgid \"One file\"\n",
            "msgid_plural \"{count} files\"\n",
            "msgstr \"Eine Datei\"\n",
            "\n",
            "msgid \"Plain\"\n",
            "msgid_plural \"Plains\"\n",
            "msgstr[0] \"Singular\"\n",
            "msgstr[1] \"Plural\"\n",
            "msgstr[2] \"Viele\"\n",
        );
        let extracted = vec![
            MergeMessageInput {
                msgid: Cow::Borrowed("One file"),
                msgid_plural: Some(Cow::Borrowed("{count} files")),
                ..MergeMessageInput::default()
            },
            MergeMessageInput {
                msgid: Cow::Borrowed("Plain"),
                ..MergeMessageInput::default()
            },
        ];

        let merged = merge_catalog(existing, &extracted).expect("merge shape changes");
        let reparsed = parse_po(&merged).expect("reparse");

        let files = reparsed
            .items
            .iter()
            .find(|item| item.msgid == "One file")
            .expect("files item");
        assert_eq!(files.msgstr.len(), 3);
        assert_eq!(files.msgstr[0], "Eine Datei");
        assert_eq!(files.msgstr[1], "");
        assert_eq!(files.msgstr[2], "");
        let plain = reparsed
            .items
            .iter()
            .find(|item| item.msgid == "Plain")
            .expect("plain item");
        assert!(plain.msgid_plural.is_none());
        assert_eq!(plain.msgstr.len(), 1);
        assert_eq!(plain.msgstr[0], "");
    }

    #[test]
    fn merge_writes_new_context_plural_and_obsoletes_unmatched_context_plural() {
        let existing = concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n",
            "\n",
            "# Translator plural\n",
            "#. Extracted plural\n",
            "#@ owner: files\n",
            "#: old.rs:4\n",
            "#, fuzzy\n",
            "msgctxt \"files\"\n",
            "msgid \"Old file\"\n",
            "msgid_plural \"Old files\"\n",
            "msgstr[0] \"Alte Datei\"\n",
            "msgstr[1] \"Alte Dateien\"\n",
        );
        let extracted = vec![MergeMessageInput {
            msgctxt: Some(Cow::Borrowed("files")),
            msgid: Cow::Borrowed("New file"),
            msgid_plural: Some(Cow::Borrowed("New files")),
            references: vec![Cow::Borrowed("new.rs:8")],
            extracted_comments: vec![Cow::Borrowed("Fresh plural")],
            flags: vec![Cow::Borrowed("rust-format")],
        }];

        let merged = merge_catalog(existing, &extracted).expect("merge context plural");

        assert!(merged.contains("#. Fresh plural\n#: new.rs:8\n#, rust-format\nmsgctxt \"files\"\nmsgid \"New file\"\nmsgid_plural \"New files\"\nmsgstr[0] \"\"\nmsgstr[1] \"\""));
        assert!(merged.contains("#~ # Translator plural\n#~ #. Extracted plural\n#~ #@ owner: files\n#~ #: old.rs:4\n#~ #, fuzzy\n#~ msgctxt \"files\"\n#~ msgid \"Old file\"\n#~ msgid_plural \"Old files\"\n#~ msgstr[0] \"Alte Datei\"\n#~ msgstr[1] \"Alte Dateien\""));
    }

    #[test]
    fn merge_handles_owned_header_fragments_and_missing_quotes() {
        let escaped_headers = parse_merge_po(concat!(
            "msgid \"\"\n",
            "msgstr \"Language: de\\\\nPlural-Forms: nplurals=2;\\\\n\"\n",
        ))
        .expect("owned escaped header fragment");
        assert_eq!(
            escaped_headers.headers,
            vec![MergeHeader {
                key: Cow::Owned("Language".to_owned()),
                value: Cow::Owned("de\\nPlural-Forms: nplurals=2;\\n".to_owned()),
            }]
        );
        assert_eq!(
            parse_merge_po("msgid \"\"\nmsgstr\n")
                .expect("missing quotes")
                .headers,
            Vec::new()
        );
        assert_eq!(
            extract_quoted_bytes_cow_from(b"msgid", 5).expect("missing quote"),
            ""
        );
        assert_eq!(parse_merge_po("").expect("empty catalog").nplurals, 2);
        assert_eq!(
            parse_merge_po(
                "msgid \"\"\nmsgstr \"Plural-Forms: plural=(n != 1); nplurals=two;\\n\"\n"
            )
            .expect("use default width for a nonnumeric count")
            .nplurals,
            2
        );
    }

    #[test]
    fn merge_strips_utf8_bom_and_preserves_multiline_headers() {
        let existing = concat!(
            "\u{feff}msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Language: pl\\n\"\n",
            "\"Plural-Forms: nplurals=3; plural=n==1 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 \"\n",
            "\"|| n%100>=20) ? 1 : 2;\\n\"\n",
            "\n",
            "msgid \"item\"\n",
            "msgid_plural \"items\"\n",
            "msgstr[0] \"element\"\n",
            "msgstr[1] \"elementy\"\n",
            "msgstr[2] \"elementow\"\n",
        );
        let extracted = [MergeMessageInput {
            msgid: Cow::Borrowed("item"),
            msgid_plural: Some(Cow::Borrowed("items")),
            ..MergeMessageInput::default()
        }];

        let merged = merge_catalog(existing, &extracted).expect("merge BOM + multiline header");
        let reparsed = parse_po(&merged).expect("reparse merged");

        assert_eq!(
            reparsed
                .headers
                .iter()
                .find(|header| header.key == "Plural-Forms")
                .map(|header| header.value.as_str()),
            Some(
                "nplurals=3; plural=n==1 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2;"
            )
        );
        assert_eq!(reparsed.items[0].nplurals, 3);
        assert_eq!(reparsed.items[0].msgstr.len(), 3);
        assert_eq!(reparsed.items[0].msgstr[2], "elementow");

        let obsolete_header_line = parse_merge_po(concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"#~ Language: de\\n\"\n",
        ))
        .expect("obsolete-prefixed header falls back to owned decode");
        assert_eq!(obsolete_header_line.headers[0].key, "Language");
        assert_eq!(obsolete_header_line.headers[0].value, "de");
    }

    #[test]
    fn merge_parse_errors_include_line_position_for_plural_and_context_keywords() {
        let plural_error = merge_catalog(
            "msgid \"file\"\nmsgid_plural \"bad\"quote\"\nmsgstr[0] \"\"\n",
            &[],
        )
        .expect_err("unescaped plural quote should fail");
        let plural_position = plural_error.position().expect("plural position metadata");
        assert_eq!(plural_error.message(), "unescaped quote in string literal");
        assert_eq!(plural_position.line(), 2);
        assert_eq!(plural_position.column(), 1);

        let context_error =
            merge_catalog("msgctxt \"bad\"quote\"\nmsgid \"x\"\nmsgstr \"\"\n", &[])
                .expect_err("unescaped context quote should fail");
        let context_position = context_error.position().expect("context position metadata");
        assert_eq!(context_error.message(), "unescaped quote in string literal");
        assert_eq!(context_position.line(), 1);
        assert_eq!(context_position.column(), 1);
    }

    #[test]
    fn merge_rejects_unrecognized_existing_lines() {
        let extracted = [MergeMessageInput {
            msgid: Cow::Borrowed("ok"),
            ..MergeMessageInput::default()
        }];
        let error = merge_catalog("msgid \"ok\"\nmsgstr_ \"typo\"\n", &extracted)
            .expect_err("unknown existing PO line should fail");
        let position = error.position().expect("position metadata");

        assert_eq!(error.message(), "unrecognized PO syntax");
        assert_eq!(position.line(), 2);
        assert_eq!(position.column(), 1);
    }

    #[test]
    fn merge_parses_owned_header_fragments_in_keyword_and_continuation_lines() {
        let existing = concat!(
            "msgid \"\"\n",
            "msgstr \"Project-Id-Version: ferrocat\\t1\\n\"\n",
            "\"Language: de\\t\\n\"\n",
            "\n",
            "msgid \"hello\"\n",
            "msgstr \"world\"\n",
        );
        let merged = merge_catalog(
            existing,
            &[MergeMessageInput {
                msgid: Cow::Borrowed("hello"),
                ..MergeMessageInput::default()
            }],
        )
        .expect("merge with owned header fragments");

        assert!(merged.contains("Project-Id-Version: ferrocat\\t1\\n"));
        assert!(merged.contains("Language: de\\n"));
    }
}
