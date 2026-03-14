use crate::{PoFile, PoItem, SerializeOptions, escape_string};

pub fn stringify_po(file: &PoFile, options: &SerializeOptions) -> String {
    let mut out = String::with_capacity(estimate_capacity(file));

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
        write_item(&mut out, item, options);
        if iter.peek().is_some() {
            out.push('\n');
        }
        out.push('\n');
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

fn write_item(out: &mut String, item: &PoItem, options: &SerializeOptions) {
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
        write_keyword(out, obsolete_prefix, "msgctxt", context, None, options);
    }
    write_keyword(out, obsolete_prefix, "msgid", &item.msgid, None, options);
    if let Some(plural) = &item.msgid_plural {
        write_keyword(out, obsolete_prefix, "msgid_plural", plural, None, options);
    }

    if item.msgid_plural.is_some() && item.msgstr.is_empty() {
        let count = item.nplurals.max(1);
        for index in 0..count {
            write_keyword(out, obsolete_prefix, "msgstr", "", Some(index), options);
        }
        return;
    }

    if item.msgstr.is_empty() {
        write_keyword(out, obsolete_prefix, "msgstr", "", None, options);
        return;
    }

    let indexed = item.msgid_plural.is_some() || item.msgstr.len() > 1;
    for (index, value) in item.msgstr.iter().enumerate() {
        write_keyword(
            out,
            obsolete_prefix,
            "msgstr",
            value,
            if indexed { Some(index) } else { None },
            options,
        );
    }
}

fn write_prefixed_line(out: &mut String, obsolete_prefix: &str, prefix: &str, value: &str) {
    out.push_str(obsolete_prefix);
    out.push_str(prefix);
    if !value.is_empty() {
        out.push(' ');
        out.push_str(value);
    }
    out.push('\n');
}

fn write_keyword(
    out: &mut String,
    obsolete_prefix: &str,
    keyword: &str,
    value: &str,
    index: Option<usize>,
    options: &SerializeOptions,
) {
    if try_write_simple_keyword(out, obsolete_prefix, keyword, value, index, options) {
        return;
    }

    write_complex_keyword(out, obsolete_prefix, keyword, value, index, options);
}

fn try_write_simple_keyword(
    out: &mut String,
    obsolete_prefix: &str,
    keyword: &str,
    value: &str,
    index: Option<usize>,
    options: &SerializeOptions,
) -> bool {
    if value.contains('\n') {
        return false;
    }

    let escaped = escape_string(value);
    let line_len = obsolete_prefix.len() + keyword_prefix_len(keyword, index) + escaped.len() + 2;
    if options.fold_length > 0 && line_len > options.fold_length {
        return false;
    }

    out.push_str(obsolete_prefix);
    push_keyword_prefix(out, keyword, index);
    out.push('"');
    out.push_str(&escaped);
    out.push_str("\"\n");
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
    let escaped = escape_string(input);
    out.push_str(&escaped);
}

fn write_complex_keyword(
    out: &mut String,
    obsolete_prefix: &str,
    keyword: &str,
    text: &str,
    index: Option<usize>,
    options: &SerializeOptions,
) {
    let prefix_len = keyword_prefix_len(keyword, index);
    let has_multiple_lines = text.contains('\n');
    let use_compact = options.compact_multiline && text.split('\n').next().unwrap_or_default() != "";
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
    let mut wrote_first_value_line = false;

    if !use_compact {
        out.push_str(obsolete_prefix);
        push_keyword_prefix(out, keyword, index);
        out.push_str("\"\"\n");
        wrote_first_value_line = true;
    }

    for (part, has_next) in parts_with_has_next(text) {
        let mut escaped = escape_string(part);
        if has_next {
            escaped.push_str("\\n");
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
            &escaped,
            limit,
            &mut wrote_first_value_line,
        );
    }
}

fn parts_with_has_next(input: &str) -> impl Iterator<Item = (&str, bool)> {
    let mut parts = input.split('\n').peekable();
    std::iter::from_fn(move || {
        let part = parts.next()?;
        Some((part, parts.peek().is_some()))
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
        write_quoted_segment(out, obsolete_prefix, keyword, index, &input[start..end], wrote_first_value_line);
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

fn folded_split_point(input: &str, start: usize, max_len: usize) -> usize {
    let remaining = input.len() - start;
    if remaining <= max_len {
        return input.len();
    }

    let end = start + max_len;
    let mut break_at = None;
    for (offset, byte) in input.as_bytes()[start..end].iter().enumerate().rev() {
        if *byte == b' ' {
            break_at = Some(start + offset + 1);
            break;
        }
    }

    match break_at {
        Some(index) => index,
        None if input.as_bytes()[end - 1] == b'\\' => end - 1,
        None => end,
    }
}

#[cfg(test)]
mod tests {
    use crate::{Header, PoFile, PoItem, SerializeOptions};

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
                msgstr: vec!["Zeile1\nZeile2".to_owned()],
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
                msgstr: vec!["\nUebersetzt".to_owned()],
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
                msgstr: vec!["Uno dos tres cuatro".to_owned()],
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
}
