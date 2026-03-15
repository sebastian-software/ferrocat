use std::borrow::Cow;
use std::collections::HashMap;

use crate::serialize::{write_keyword, write_prefixed_line};
use crate::text::escape_string_into;
use crate::{
    BorrowedHeader, BorrowedMsgStr, BorrowedPoFile, BorrowedPoItem, ParseError, SerializeOptions,
    parse_po_borrowed,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtractedMessage<'a> {
    pub msgctxt: Option<Cow<'a, str>>,
    pub msgid: Cow<'a, str>,
    pub msgid_plural: Option<Cow<'a, str>>,
    pub references: Vec<Cow<'a, str>>,
    pub extracted_comments: Vec<Cow<'a, str>>,
    pub flags: Vec<Cow<'a, str>>,
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

    let existing = parse_po_borrowed(input)?;
    let nplurals = parse_nplurals(&existing.headers).unwrap_or(2);
    let options = SerializeOptions::default();
    let mut out = String::with_capacity(estimate_merge_capacity(input, extracted_messages));
    let mut scratch = String::new();

    write_file_preamble(&mut out, &existing);

    let mut existing_index: HashMap<&str, Vec<(Option<&str>, usize)>> =
        HashMap::with_capacity(existing.items.len());
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
        write_borrowed_item(&mut out, &mut scratch, item, true, &options);
        out.push('\n');
        wrote_item = true;
    }

    Ok(out)
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

fn write_file_preamble(out: &mut String, file: &BorrowedPoFile<'_>) {
    for comment in &file.comments {
        write_prefixed_line(out, "", "#", comment.as_ref());
    }
    for comment in &file.extracted_comments {
        write_prefixed_line(out, "", "#.", comment.as_ref());
    }

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

fn find_existing_index(
    existing_index: &HashMap<&str, Vec<(Option<&str>, usize)>>,
    msgctxt: Option<&str>,
    msgid: &str,
) -> Option<usize> {
    let candidates = existing_index.get(msgid)?;
    candidates
        .iter()
        .find_map(|(candidate_ctxt, index)| (*candidate_ctxt == msgctxt).then_some(*index))
}

fn write_merged_existing_item(
    out: &mut String,
    scratch: &mut String,
    existing: &BorrowedPoItem<'_>,
    extracted: &ExtractedMessage<'_>,
    nplurals: usize,
    options: &SerializeOptions,
) {
    let obsolete_prefix = "";

    write_cow_prefixed_lines(out, obsolete_prefix, "#", &existing.comments);
    write_cow_prefixed_lines(out, obsolete_prefix, "#.", &extracted.extracted_comments);
    write_metadata_lines(out, obsolete_prefix, &existing.metadata);
    write_cow_prefixed_lines(out, obsolete_prefix, "#:", &extracted.references);
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

    write_cow_prefixed_lines(out, obsolete_prefix, "#.", &extracted.extracted_comments);
    write_cow_prefixed_lines(out, obsolete_prefix, "#:", &extracted.references);
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

fn write_borrowed_item(
    out: &mut String,
    scratch: &mut String,
    item: &BorrowedPoItem<'_>,
    obsolete: bool,
    options: &SerializeOptions,
) {
    let obsolete_prefix = if obsolete { "#~ " } else { "" };

    write_cow_prefixed_lines(out, obsolete_prefix, "#", &item.comments);
    write_cow_prefixed_lines(out, obsolete_prefix, "#.", &item.extracted_comments);
    write_metadata_lines(out, obsolete_prefix, &item.metadata);
    write_cow_prefixed_lines(out, obsolete_prefix, "#:", &item.references);
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

fn write_cow_prefixed_lines(
    out: &mut String,
    obsolete_prefix: &str,
    prefix: &str,
    values: &[Cow<'_, str>],
) {
    for value in values {
        write_prefixed_line(out, obsolete_prefix, prefix, value.as_ref());
    }
}

fn write_metadata_lines(
    out: &mut String,
    obsolete_prefix: &str,
    values: &[(Cow<'_, str>, Cow<'_, str>)],
) {
    for (key, value) in values {
        out.push_str(obsolete_prefix);
        out.push_str("#@ ");
        out.push_str(key.as_ref());
        out.push_str(": ");
        out.push_str(value.as_ref());
        out.push('\n');
    }
}

fn write_flags_line(out: &mut String, obsolete_prefix: &str, values: &[Cow<'_, str>]) {
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
    existing: &[Cow<'_, str>],
    extracted: &[Cow<'_, str>],
) {
    if existing.is_empty() && extracted.is_empty() {
        return;
    }

    out.push_str(obsolete_prefix);
    out.push_str("#, ");

    let mut wrote_any = false;
    let mut seen = Vec::with_capacity(existing.len() + extracted.len());
    for value in existing.iter().chain(extracted.iter()) {
        let flag = value.as_ref();
        if seen.iter().any(|existing: &&str| *existing == flag) {
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
        let count = nplurals.max(1);
        for index in 0..count {
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

fn parse_nplurals(headers: &[BorrowedHeader<'_>]) -> Option<usize> {
    let plural_forms = headers
        .iter()
        .find(|header| header.key.as_ref() == "Plural-Forms")?
        .value
        .as_ref();

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

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{ExtractedMessage, merge_catalog};
    use crate::{SerializeOptions, parse_po, stringify_po};

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
        assert_eq!(
            merged,
            stringify_po(&reparsed, &SerializeOptions::default())
        );
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
