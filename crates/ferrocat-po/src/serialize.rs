use crate::scan::find_escapable_byte;
use crate::text::{escape_string_into, escape_string_into_with_first_escape};
use crate::{PoFile, PoItem, SerializeOptions};

pub fn stringify_po(file: &PoFile, options: &SerializeOptions) -> String {
    let mut out = String::with_capacity(estimate_capacity(file));
    let mut scratch = String::new();

    for comment in &file.comments {
        push_prefixed_comment(&mut out, "#", comment);
    }
    for comment in &file.extracted_comments {
        push_prefixed_comment(&mut out, "#.", comment);
    }

    out.push_str("msgid \"\"\n");
    out.push_str("msgstr \"\"\n");
    for header in &file.headers {
        out.push('"');
        append_escaped(&mut out, &header.key);
        out.push_str(": ");
        append_escaped(&mut out, &header.value);
        out.push_str("\\n");
        out.push_str("\"\n");
    }
    out.push('\n');

    let mut iter = file.items.iter().peekable();
    while let Some(item) = iter.next() {
        write_item(&mut out, &mut scratch, item, options);
        if iter.peek().is_some() {
            out.push('\n');
        }
    }

    out
}

fn estimate_capacity(file: &PoFile) -> usize {
    let headers_len: usize = file
        .headers
        .iter()
        .map(|header| header.key.len() + header.value.len() + 8)
        .sum();
    let items_len: usize = file
        .items
        .iter()
        .map(|item| {
            item.msgid.len()
                + item.msgctxt.as_ref().map_or(0, String::len)
                + item.msgid_plural.as_ref().map_or(0, String::len)
                + item.msgstr.iter().map(String::len).sum::<usize>()
                + item.comments.iter().map(String::len).sum::<usize>()
                + item
                    .extracted_comments
                    .iter()
                    .map(String::len)
                    .sum::<usize>()
                + item.references.iter().map(String::len).sum::<usize>()
                + item.flags.iter().map(String::len).sum::<usize>()
        })
        .sum();

    headers_len + items_len + 256
}

fn push_prefixed_comment(out: &mut String, prefix: &str, comment: &str) {
    out.push_str(prefix);
    if !comment.is_empty() {
        out.push(' ');
        out.push_str(comment);
    }
    out.push('\n');
}

fn write_item(out: &mut String, scratch: &mut String, item: &PoItem, options: &SerializeOptions) {
    let obsolete_prefix = if item.obsolete { "#~ " } else { "" };

    for comment in &item.comments {
        write_prefixed_line(out, obsolete_prefix, "#", comment);
    }
    for comment in &item.extracted_comments {
        write_prefixed_line(out, obsolete_prefix, "#.", comment);
    }
    for (key, value) in &item.metadata {
        out.push_str(obsolete_prefix);
        out.push_str("#@ ");
        out.push_str(key);
        out.push_str(": ");
        out.push_str(value);
        out.push('\n');
    }
    for reference in &item.references {
        out.push_str(obsolete_prefix);
        out.push_str("#: ");
        out.push_str(reference);
        out.push('\n');
    }
    if !item.flags.is_empty() {
        out.push_str(obsolete_prefix);
        out.push_str("#, ");
        for (index, flag) in item.flags.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str(flag);
        }
        out.push('\n');
    }

    if let Some(context) = &item.msgctxt {
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
        &item.msgid,
        None,
        options,
    );
    if let Some(plural) = &item.msgid_plural {
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

    if item.msgid_plural.is_some() && item.msgstr.is_empty() {
        let count = item.nplurals.max(1);
        for index in 0..count {
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

    if item.msgstr.is_empty() {
        write_keyword(out, scratch, obsolete_prefix, "msgstr", "", None, options);
        return;
    }

    let indexed = item.msgid_plural.is_some() || item.msgstr.len() > 1;
    for (index, value) in item.msgstr.iter().enumerate() {
        write_keyword(
            out,
            scratch,
            obsolete_prefix,
            "msgstr",
            value,
            if indexed { Some(index) } else { None },
            options,
        );
    }
}

pub(crate) fn write_prefixed_line(
    out: &mut String,
    obsolete_prefix: &str,
    prefix: &str,
    value: &str,
) {
    out.push_str(obsolete_prefix);
    out.push_str(prefix);
    if !value.is_empty() {
        out.push(' ');
        out.push_str(value);
    }
    out.push('\n');
}

pub(crate) fn write_keyword(
    out: &mut String,
    scratch: &mut String,
    obsolete_prefix: &str,
    keyword: &str,
    value: &str,
    index: Option<usize>,
    options: &SerializeOptions,
) {
    if try_write_simple_keyword(out, obsolete_prefix, keyword, value, index, options) {
        return;
    }

    write_complex_keyword(
        out,
        scratch,
        obsolete_prefix,
        keyword,
        value,
        index,
        options,
    );
}

fn try_write_simple_keyword(
    out: &mut String,
    obsolete_prefix: &str,
    keyword: &str,
    value: &str,
    index: Option<usize>,
    options: &SerializeOptions,
) -> bool {
    let first_escape = find_escapable_byte(value.as_bytes());
    if matches!(first_escape, Some(index) if value.as_bytes()[index] == b'\n') {
        return false;
    }

    let prefix_len = keyword_prefix_len(keyword, index);
    if options.fold_length > 0
        && value.len()
            > options
                .fold_length
                .saturating_sub(obsolete_prefix.len() + prefix_len + 2)
    {
        return false;
    }

    let start_len = out.len();
    out.reserve(obsolete_prefix.len() + prefix_len + value.len() + 3);
    out.push_str(obsolete_prefix);
    push_keyword_prefix(out, keyword, index);
    out.push('"');
    escape_string_into_with_first_escape(out, value, first_escape);
    out.push_str("\"\n");

    if options.fold_length > 0 && out.len() - start_len - 1 > options.fold_length {
        out.truncate(start_len);
        return false;
    }

    true
}

fn keyword_prefix_len(keyword: &str, index: Option<usize>) -> usize {
    match index {
        Some(value) => keyword.len() + digits(value) + 3,
        None => keyword.len() + 1,
    }
}

fn push_keyword_prefix(out: &mut String, keyword: &str, index: Option<usize>) {
    out.push_str(keyword);
    if let Some(value) = index {
        out.push('[');
        push_usize(out, value);
        out.push(']');
    }
    out.push(' ');
}

fn push_usize(out: &mut String, mut value: usize) {
    if value == 0 {
        out.push('0');
        return;
    }

    let mut buf = [0u8; 20];
    let mut len = 0usize;
    while value > 0 {
        buf[len] = b'0' + (value % 10) as u8;
        len += 1;
        value /= 10;
    }
    for index in (0..len).rev() {
        out.push(char::from(buf[index]));
    }
}

fn digits(mut value: usize) -> usize {
    let mut count = 1usize;
    while value >= 10 {
        value /= 10;
        count += 1;
    }
    count
}

fn append_escaped(out: &mut String, input: &str) {
    escape_string_into(out, input);
}

fn write_complex_keyword(
    out: &mut String,
    scratch: &mut String,
    obsolete_prefix: &str,
    keyword: &str,
    text: &str,
    index: Option<usize>,
    options: &SerializeOptions,
) {
    let prefix_len = keyword_prefix_len(keyword, index);
    let has_multiple_lines = text.contains('\n');
    let first_line_max = if options.fold_length == 0 {
        usize::MAX
    } else {
        options.fold_length.saturating_sub(prefix_len + 2).max(1)
    };
    let other_line_max = if options.fold_length == 0 {
        usize::MAX
    } else {
        options.fold_length.saturating_sub(2).max(1)
    };
    let parts = parts_with_has_next(text).collect::<Vec<_>>();
    let requires_folding = options.fold_length > 0
        && parts.iter().any(|(part, has_next)| {
            let escaped_len = escaped_part_len(part, *has_next);
            let limit = if has_multiple_lines {
                other_line_max
            } else {
                first_line_max
            };
            escaped_len > limit
        });
    let use_compact = options.compact_multiline
        && text.split('\n').next().unwrap_or_default() != ""
        && !requires_folding;
    let mut wrote_first_value_line = false;

    if !use_compact {
        out.push_str(obsolete_prefix);
        push_keyword_prefix(out, keyword, index);
        out.push_str("\"\"\n");
        wrote_first_value_line = true;
    }

    for (part, has_next) in parts {
        scratch.clear();
        escape_string_into(scratch, part);
        if has_next {
            scratch.push_str("\\n");
        }

        let limit = if wrote_first_value_line || has_multiple_lines {
            other_line_max
        } else {
            first_line_max
        };

        write_folded_segments(
            out,
            obsolete_prefix,
            keyword,
            index,
            scratch,
            limit,
            &mut wrote_first_value_line,
        );
    }
}

fn parts_with_has_next(input: &str) -> impl Iterator<Item = (&str, bool)> {
    input
        .split_inclusive('\n')
        .map(|part| match part.strip_suffix('\n') {
            Some(stripped) => (stripped, true),
            None => (part, false),
        })
}

fn write_folded_segments(
    out: &mut String,
    obsolete_prefix: &str,
    keyword: &str,
    index: Option<usize>,
    input: &str,
    max_len: usize,
    wrote_first_value_line: &mut bool,
) {
    let mut start = 0;
    loop {
        let end = folded_split_point(input, start, max_len);
        write_quoted_segment(
            out,
            obsolete_prefix,
            keyword,
            index,
            &input[start..end],
            wrote_first_value_line,
        );
        if end == input.len() {
            break;
        }
        start = end;
    }
}

fn write_quoted_segment(
    out: &mut String,
    obsolete_prefix: &str,
    keyword: &str,
    index: Option<usize>,
    segment: &str,
    wrote_first_value_line: &mut bool,
) {
    out.push_str(obsolete_prefix);
    if !*wrote_first_value_line {
        push_keyword_prefix(out, keyword, index);
        *wrote_first_value_line = true;
    }
    out.push('"');
    out.push_str(segment);
    out.push_str("\"\n");
}

fn escaped_part_len(part: &str, has_next: bool) -> usize {
    let escaped_len = match find_escapable_byte(part.as_bytes()) {
        Some(_) => {
            let mut escaped = String::new();
            escape_string_into(&mut escaped, part);
            escaped.len()
        }
        None => part.len(),
    };

    escaped_len + if has_next { 2 } else { 0 }
}

fn folded_split_point(input: &str, start: usize, max_len: usize) -> usize {
    let remaining = input.len() - start;
    if remaining <= max_len {
        return input.len();
    }

    let mut end = start;
    while end < input.len() {
        let chunk_end = next_fold_chunk_end(input, end);
        let next_len = chunk_end - start;
        if next_len > max_len {
            break;
        }
        end = chunk_end;
    }

    if end > start {
        return end;
    }

    let end = clamp_char_boundary(input, start, start + max_len);
    if input.as_bytes()[end - 1] == b'\\' {
        end - 1
    } else {
        end
    }
}

fn next_fold_chunk_end(input: &str, start: usize) -> usize {
    let bytes = input.as_bytes();
    let is_space = bytes[start] == b' ';
    let mut end = start + 1;
    while end < bytes.len() && (bytes[end] == b' ') == is_space {
        end += 1;
    }
    end
}

fn clamp_char_boundary(input: &str, start: usize, requested_end: usize) -> usize {
    let mut end = requested_end.min(input.len());
    while end > start && !input.is_char_boundary(end) {
        end -= 1;
    }
    if end > start {
        return end;
    }

    let mut end = requested_end.min(input.len());
    while end < input.len() && !input.is_char_boundary(end) {
        end += 1;
    }
    end
}

#[cfg(test)]
mod tests {
    use crate::{Header, MsgStr, PoFile, PoItem, SerializeOptions, parse_po};

    use super::stringify_po;

    #[test]
    fn serializes_comments_headers_and_items() {
        let file = PoFile {
            comments: vec!["Translator comment".to_owned()],
            extracted_comments: vec!["Extracted".to_owned()],
            headers: vec![
                Header {
                    key: "Language".to_owned(),
                    value: "de".to_owned(),
                },
                Header {
                    key: "Plural-Forms".to_owned(),
                    value: "nplurals=2; plural=(n != 1);".to_owned(),
                },
            ],
            items: vec![PoItem {
                msgid: "Line1\nLine2".to_owned(),
                msgstr: MsgStr::from(vec!["Zeile1\nZeile2".to_owned()]),
                ..PoItem::new(2)
            }],
        };

        let output = stringify_po(&file, &SerializeOptions::default());
        assert!(output.contains("# Translator comment\n"));
        assert!(output.contains("#. Extracted\n"));
        assert!(output.contains("\"Language: de\\n\"\n"));
        assert!(output.contains("msgid \"Line1\\n\"\n\"Line2\"\n"));
        assert!(output.contains("msgstr \"Zeile1\\n\"\n\"Zeile2\"\n"));
    }

    #[test]
    fn serializes_empty_plural_translations() {
        let file = PoFile {
            headers: vec![],
            comments: vec![],
            extracted_comments: vec![],
            items: vec![PoItem {
                msgid: "item".to_owned(),
                msgid_plural: Some("items".to_owned()),
                nplurals: 3,
                ..PoItem::new(3)
            }],
        };

        let output = stringify_po(&file, &SerializeOptions::default());
        assert!(output.contains("msgstr[0] \"\"\n"));
        assert!(output.contains("msgstr[1] \"\"\n"));
        assert!(output.contains("msgstr[2] \"\"\n"));
    }

    #[test]
    fn serializes_non_compact_multiline_values() {
        let file = PoFile {
            headers: vec![],
            comments: vec![],
            extracted_comments: vec![],
            items: vec![PoItem {
                msgid: "\nIndented".to_owned(),
                msgstr: MsgStr::from(vec!["\nUebersetzt".to_owned()]),
                ..PoItem::new(2)
            }],
        };

        let output = stringify_po(
            &file,
            &SerializeOptions {
                compact_multiline: false,
                ..SerializeOptions::default()
            },
        );

        assert!(output.contains("msgid \"\"\n\"\\n\"\n\"Indented\"\n"));
        assert!(output.contains("msgstr \"\"\n\"\\n\"\n\"Uebersetzt\"\n"));
    }

    #[test]
    fn does_not_fold_when_fold_length_is_zero() {
        let file = PoFile {
            headers: vec![],
            comments: vec![],
            extracted_comments: vec![],
            items: vec![PoItem {
                msgid: "Alpha beta gamma delta".to_owned(),
                msgstr: MsgStr::from(vec!["Uno dos tres cuatro".to_owned()]),
                ..PoItem::new(2)
            }],
        };

        let output = stringify_po(
            &file,
            &SerializeOptions {
                fold_length: 0,
                compact_multiline: true,
            },
        );

        assert!(output.contains("msgid \"Alpha beta gamma delta\"\n"));
        assert!(output.contains("msgstr \"Uno dos tres cuatro\"\n"));
    }

    #[test]
    fn folds_utf8_without_splitting_codepoints() {
        let file = PoFile {
            headers: vec![],
            comments: vec![],
            extracted_comments: vec![],
            items: vec![PoItem {
                msgid: "Grüße aus Köln".to_owned(),
                msgstr: MsgStr::from(vec!["Übermäßig höflich".to_owned()]),
                ..PoItem::new(2)
            }],
        };

        let output = stringify_po(
            &file,
            &SerializeOptions {
                fold_length: 12,
                compact_multiline: true,
            },
        );

        let reparsed = parse_po(&output).expect("reparse folded utf8 output");
        assert_eq!(reparsed.items[0].msgid, "Grüße aus Köln");
        assert_eq!(reparsed.items[0].msgstr[0], "Übermäßig höflich");
    }

    #[test]
    fn drops_previous_msgid_history_on_roundtrip() {
        let input = r#"#| msgctxt "Old menu context"
#| msgid "Old file label"
msgctxt "menu"
msgid "File"
msgstr "Datei"
"#;

        let parsed = parse_po(input).expect("parse previous-msgid input");
        assert_eq!(parsed.items.len(), 1);
        assert_eq!(parsed.items[0].msgctxt.as_deref(), Some("menu"));
        assert_eq!(parsed.items[0].msgid, "File");

        let output = stringify_po(&parsed, &SerializeOptions::default());
        assert!(!output.contains("#| "));
        assert!(output.contains("msgctxt \"menu\"\n"));
        assert!(output.contains("msgid \"File\"\n"));
    }

    #[test]
    fn normalizes_headerless_files_with_explicit_empty_header() {
        let file = PoFile {
            headers: vec![],
            comments: vec![],
            extracted_comments: vec![],
            items: vec![PoItem {
                msgid: "Save".to_owned(),
                msgstr: MsgStr::from("Speichern".to_owned()),
                flags: vec!["fuzzy".to_owned()],
                ..PoItem::new(2)
            }],
        };

        let output = stringify_po(&file, &SerializeOptions::default());
        assert!(output.starts_with("msgid \"\"\nmsgstr \"\"\n\n"));
        assert!(output.contains("#, fuzzy\nmsgid \"Save\"\nmsgstr \"Speichern\"\n"));
    }

    #[test]
    fn folds_single_line_values_like_gettext_style_multiline_entries() {
        let file = PoFile {
            headers: vec![],
            comments: vec!["test wrapping".to_owned()],
            extracted_comments: vec![],
            items: vec![
                PoItem {
                    msgid: "Some line that contain special characters \" and that \t is very, very, very long...: %s \n".to_owned(),
                    msgstr: MsgStr::from(vec!["".to_owned()]),
                    ..PoItem::new(2)
                },
                PoItem {
                    msgid: "Some line that contain special characters \"foobar\" and that contains whitespace at the end          ".to_owned(),
                    msgstr: MsgStr::from(vec!["".to_owned()]),
                    ..PoItem::new(2)
                },
            ],
        };

        let output = stringify_po(
            &file,
            &SerializeOptions {
                fold_length: 50,
                compact_multiline: true,
            },
        );

        assert!(output.contains("msgid \"\"\n\"Some line that contain special characters \\\" and\"\n\" that \\t is very, very, very long...: %s \\n\"\n"));
        assert!(output.contains("msgid \"\"\n\"Some line that contain special characters \"\n\"\\\"foobar\\\" and that contains whitespace at the \"\n\"end          \"\n"));
    }
}
