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
        out.push_str(&escape_string(&format!(
            "{}: {}\n",
            header.key, header.value
        )));
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
    let lines = format_keyword(keyword, value, index, options);
    for line in lines {
        out.push_str(obsolete_prefix);
        out.push_str(&line);
        out.push('\n');
    }
}

fn format_keyword(
    keyword: &str,
    text: &str,
    index: Option<usize>,
    options: &SerializeOptions,
) -> Vec<String> {
    let keyword_prefix = match index {
        Some(value) => format!("{keyword}[{value}] "),
        None => format!("{keyword} "),
    };

    if !text.contains('\n') {
        let escaped = escape_string(text);
        let full = format!("{keyword_prefix}\"{escaped}\"");
        if options.fold_length == 0 || full.len() <= options.fold_length {
            return vec![full];
        }
    }

    let parts: Vec<&str> = text.split('\n').collect();
    let escaped_parts = escape_parts(&parts);
    let folded = apply_folding(
        &escaped_parts,
        &keyword_prefix,
        options.fold_length,
        parts.len() > 1,
    );
    if folded.len() == 1 && parts.len() == 1 {
        return vec![format!("{keyword_prefix}\"{}\"", folded[0])];
    }

    let mut out = Vec::with_capacity(folded.len() + 1);
    let use_compact = options.compact_multiline && parts.first().copied().unwrap_or_default() != "";
    if use_compact {
        out.push(format!(
            "{keyword_prefix}\"{}\"",
            folded.first().cloned().unwrap_or_default()
        ));
        for segment in folded.into_iter().skip(1) {
            out.push(format!("\"{segment}\""));
        }
    } else {
        out.push(format!("{keyword_prefix}\"\""));
        for segment in folded {
            out.push(format!("\"{segment}\""));
        }
    }

    out
}

fn escape_parts(parts: &[&str]) -> Vec<String> {
    let mut escaped = Vec::with_capacity(parts.len());
    for (index, part) in parts.iter().enumerate() {
        let mut value = escape_string(part);
        if index + 1 < parts.len() {
            value.push_str("\\n");
        }
        escaped.push(value);
    }
    escaped
}

fn apply_folding(
    parts: &[String],
    keyword_prefix: &str,
    fold_length: usize,
    has_multiple_lines: bool,
) -> Vec<String> {
    if fold_length == 0 {
        return parts.to_owned();
    }

    let first_line_max = fold_length.saturating_sub(keyword_prefix.len() + 2);
    let other_line_max = fold_length.saturating_sub(2);
    let mut out = Vec::new();

    for (index, part) in parts.iter().enumerate() {
        let limit = if index == 0 && !has_multiple_lines {
            first_line_max
        } else {
            other_line_max
        };
        fold_line(part, limit.max(1), &mut out);
    }

    out
}

fn fold_line(input: &str, max_len: usize, out: &mut Vec<String>) {
    if input.len() <= max_len {
        out.push(input.to_owned());
        return;
    }

    let mut start = 0;
    while start < input.len() {
        let remaining = input.len() - start;
        if remaining <= max_len {
            out.push(input[start..].to_owned());
            break;
        }

        let end = start + max_len;
        let mut break_at = None;
        for (offset, byte) in input.as_bytes()[start..end].iter().enumerate().rev() {
            if *byte == b' ' {
                break_at = Some(start + offset + 1);
                break;
            }
        }

        let split = match break_at {
            Some(index) => index,
            None if input.as_bytes()[end - 1] == b'\\' => end - 1,
            None => end,
        };
        out.push(input[start..split].to_owned());
        start = split;
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
}
