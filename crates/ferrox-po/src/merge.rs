use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use crate::{
    BorrowedHeader, BorrowedPoItem, MsgStr, ParseError, PoFile, PoItem, SerializeOptions,
    parse_po_borrowed, stringify_po,
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
    let mut file = PoFile {
        comments: existing
            .comments
            .iter()
            .map(|value| value.as_ref().to_owned())
            .collect(),
        extracted_comments: existing
            .extracted_comments
            .iter()
            .map(|value| value.as_ref().to_owned())
            .collect(),
        headers: existing
            .headers
            .iter()
            .cloned()
            .map(BorrowedHeader::into_owned)
            .collect(),
        items: Vec::with_capacity(existing.items.len().max(extracted_messages.len())),
    };

    let mut existing_index = HashMap::with_capacity(existing.items.len());
    for (index, item) in existing.items.iter().enumerate() {
        existing_index.insert(
            OwnedMessageKey::new(item.msgctxt.as_deref(), item.msgid.as_ref()),
            index,
        );
    }

    let mut matched = HashSet::with_capacity(extracted_messages.len());

    for extracted in extracted_messages {
        let key = OwnedMessageKey::new(extracted.msgctxt.as_deref(), extracted.msgid.as_ref());
        matched.insert(key.clone());

        let item = match existing_index.get(&key) {
            Some(index) => merge_existing_item(&existing.items[*index], extracted, nplurals),
            None => new_item_from_extracted(extracted, nplurals),
        };
        file.items.push(item);
    }

    for item in &existing.items {
        let key = OwnedMessageKey::new(item.msgctxt.as_deref(), item.msgid.as_ref());
        if matched.contains(&key) {
            continue;
        }

        let mut obsolete = item.clone().into_owned();
        obsolete.obsolete = true;
        file.items.push(obsolete);
    }

    Ok(stringify_po(&file, &SerializeOptions::default()))
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OwnedMessageKey {
    msgctxt: Option<String>,
    msgid: String,
}

impl OwnedMessageKey {
    fn new(msgctxt: Option<&str>, msgid: &str) -> Self {
        Self {
            msgctxt: msgctxt.map(str::to_owned),
            msgid: msgid.to_owned(),
        }
    }
}

fn merge_existing_item(
    existing: &BorrowedPoItem<'_>,
    extracted: &ExtractedMessage<'_>,
    nplurals: usize,
) -> PoItem {
    let mut item = existing.clone().into_owned();
    let was_plural = item.msgid_plural.is_some();
    let is_plural = extracted.msgid_plural.is_some();

    item.msgctxt = extracted
        .msgctxt
        .as_ref()
        .map(|value| value.as_ref().to_owned());
    item.msgid = extracted.msgid.as_ref().to_owned();
    item.msgid_plural = extracted
        .msgid_plural
        .as_ref()
        .map(|value| value.as_ref().to_owned());
    item.references = extracted
        .references
        .iter()
        .map(|value| value.as_ref().to_owned())
        .collect();
    item.extracted_comments = extracted
        .extracted_comments
        .iter()
        .map(|value| value.as_ref().to_owned())
        .collect();
    item.flags = merge_flags(&item.flags, &extracted.flags);
    item.obsolete = false;
    item.nplurals = nplurals;

    normalize_msgstr(&mut item.msgstr, was_plural, is_plural, nplurals);
    item
}

fn new_item_from_extracted(extracted: &ExtractedMessage<'_>, nplurals: usize) -> PoItem {
    let mut item = PoItem::new(nplurals);
    item.msgctxt = extracted
        .msgctxt
        .as_ref()
        .map(|value| value.as_ref().to_owned());
    item.msgid = extracted.msgid.as_ref().to_owned();
    item.msgid_plural = extracted
        .msgid_plural
        .as_ref()
        .map(|value| value.as_ref().to_owned());
    item.references = extracted
        .references
        .iter()
        .map(|value| value.as_ref().to_owned())
        .collect();
    item.extracted_comments = extracted
        .extracted_comments
        .iter()
        .map(|value| value.as_ref().to_owned())
        .collect();
    item.flags = extracted
        .flags
        .iter()
        .map(|value| value.as_ref().to_owned())
        .collect();
    item.msgstr = default_msgstr(item.msgid_plural.is_some(), nplurals);
    item
}

fn merge_flags(existing: &[String], extracted: &[Cow<'_, str>]) -> Vec<String> {
    let mut merged = existing.to_vec();
    for flag in extracted {
        if merged.iter().any(|existing| existing == flag.as_ref()) {
            continue;
        }
        merged.push(flag.as_ref().to_owned());
    }
    merged
}

fn normalize_msgstr(msgstr: &mut MsgStr, was_plural: bool, is_plural: bool, nplurals: usize) {
    if was_plural != is_plural {
        *msgstr = default_msgstr(is_plural, nplurals);
        return;
    }

    if is_plural {
        let mut values = std::mem::take(msgstr).into_vec();
        values.resize(nplurals.max(1), String::new());
        *msgstr = MsgStr::Plural(values);
        return;
    }

    *msgstr = match std::mem::take(msgstr) {
        MsgStr::None => MsgStr::Singular(String::new()),
        MsgStr::Singular(value) => MsgStr::Singular(value),
        MsgStr::Plural(mut values) => {
            MsgStr::Singular(values.drain(..1).next().unwrap_or_default())
        }
    };
}

fn default_msgstr(is_plural: bool, nplurals: usize) -> MsgStr {
    if is_plural {
        MsgStr::Plural(vec![String::new(); nplurals.max(1)])
    } else {
        MsgStr::Singular(String::new())
    }
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
