use std::borrow::Cow;

use ferrox_po::{ExtractedMessage, parse_po};

const TINY_FIXTURE: &str = include_str!("../fixtures/tiny.po");
const REALISTIC_FIXTURE: &str = include_str!("../fixtures/realistic.po");
const STRESS_FIXTURE: &str = include_str!("../fixtures/stress.po");

#[derive(Debug, Clone, Copy)]
pub struct FixtureStats {
    pub entries: usize,
    pub plural_entries: usize,
    pub translator_comments: usize,
    pub extracted_comments: usize,
    pub references: usize,
    pub contexts: usize,
    pub metadata_comments: usize,
    pub obsolete_entries: usize,
    pub multiline_entries: usize,
    pub escaped_entries: usize,
}

pub struct Fixture {
    name: Cow<'static, str>,
    kind: &'static str,
    content: Cow<'static, str>,
    stats: FixtureStats,
}

pub struct MergeFixture {
    name: Cow<'static, str>,
    kind: &'static str,
    existing_po: Cow<'static, str>,
    extracted_messages: Vec<ExtractedMessage<'static>>,
    existing_entries: usize,
}

impl MergeFixture {
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn kind(&self) -> &str {
        self.kind
    }

    pub fn existing_po(&self) -> &str {
        self.existing_po.as_ref()
    }

    pub fn extracted_messages(&self) -> &[ExtractedMessage<'static>] {
        &self.extracted_messages
    }

    pub fn existing_entries(&self) -> usize {
        self.existing_entries
    }

    pub fn extracted_entries(&self) -> usize {
        self.extracted_messages.len()
    }
}

impl Fixture {
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn kind(&self) -> &str {
        self.kind
    }

    pub fn content(&self) -> &str {
        self.content.as_ref()
    }

    pub fn stats(&self) -> FixtureStats {
        self.stats
    }
}

pub fn fixture_by_name(name: &str) -> Option<Fixture> {
    match name {
        "tiny" => Some(static_fixture("tiny", TINY_FIXTURE)),
        "realistic" => Some(static_fixture("realistic", REALISTIC_FIXTURE)),
        "stress" => Some(static_fixture("stress", STRESS_FIXTURE)),
        "mixed-1000" => Some(generated_fixture(1_000)),
        "mixed-10000" => Some(generated_fixture(10_000)),
        _ => None,
    }
}

pub fn merge_fixture_by_name(name: &str) -> Option<MergeFixture> {
    match name {
        "mixed-1000" => Some(generated_merge_fixture(1_000)),
        "mixed-10000" => Some(generated_merge_fixture(10_000)),
        _ => None,
    }
}

fn static_fixture(name: &'static str, content: &'static str) -> Fixture {
    Fixture {
        name: Cow::Borrowed(name),
        kind: "static",
        content: Cow::Borrowed(content),
        stats: scan_stats(content),
    }
}

fn generated_fixture(entries: usize) -> Fixture {
    let content = build_mixed_fixture(entries);
    let stats = scan_stats(&content);
    Fixture {
        name: Cow::Owned(format!("mixed-{entries}")),
        kind: "generated",
        content: Cow::Owned(content),
        stats,
    }
}

fn generated_merge_fixture(entries: usize) -> MergeFixture {
    let existing_po = build_mixed_fixture(entries);
    let parsed = parse_po(&existing_po).expect("generated merge fixture must parse");

    let mut extracted_messages = Vec::with_capacity((entries * 9) / 10);
    let mut active_index = 0usize;
    for item in &parsed.items {
        if item.obsolete {
            continue;
        }
        active_index += 1;
        if active_index % 5 == 0 {
            continue;
        }

        extracted_messages.push(ExtractedMessage {
            msgctxt: item.msgctxt.as_ref().map(|value| Cow::Owned(value.clone())),
            msgid: Cow::Owned(item.msgid.clone()),
            msgid_plural: item
                .msgid_plural
                .as_ref()
                .map(|value| Cow::Owned(value.clone())),
            references: vec![Cow::Owned(format!(
                "src/merged_{:04}.rs:{}",
                active_index,
                (active_index % 200) + 1
            ))],
            extracted_comments: if active_index % 7 == 0 {
                vec![Cow::Owned(format!(
                    "Merged extractor comment {}",
                    active_index % 13
                ))]
            } else {
                Vec::new()
            },
            flags: if active_index % 11 == 0 {
                vec![Cow::Borrowed("c-format")]
            } else {
                Vec::new()
            },
        });
    }

    for index in 0..(entries / 10).max(1) {
        let message_index = entries + index;
        extracted_messages.push(ExtractedMessage {
            msgctxt: (message_index % 9 == 0)
                .then(|| Cow::Owned(format!("merge-context-{}", message_index % 5))),
            msgid: Cow::Owned(format!("Merged message {}", message_index)),
            msgid_plural: (message_index % 8 == 0)
                .then(|| Cow::Owned(format!("Merged messages {}", message_index))),
            references: vec![Cow::Owned(format!(
                "src/new_merge_{:04}.rs:{}",
                message_index,
                (message_index % 200) + 1
            ))],
            extracted_comments: if message_index % 6 == 0 {
                vec![Cow::Borrowed("newly extracted")]
            } else {
                Vec::new()
            },
            flags: if message_index % 10 == 0 {
                vec![Cow::Borrowed("fuzzy")]
            } else {
                Vec::new()
            },
        });
    }

    MergeFixture {
        name: Cow::Owned(format!("merge-mixed-{entries}")),
        kind: "generated",
        existing_entries: parsed.items.len(),
        existing_po: Cow::Owned(existing_po),
        extracted_messages,
    }
}

fn build_mixed_fixture(entries: usize) -> String {
    let mut out = String::with_capacity(entries * 120);
    out.push_str("# Benchmark corpus for ferrox\n");
    out.push_str("# Mixed feature distribution, deterministic generation\n");
    out.push_str("msgid \"\"\n");
    out.push_str("msgstr \"\"\n");
    out.push_str("\"Project-Id-Version: ferrox benchmark\\n\"\n");
    out.push_str("\"Language: de\\n\"\n");
    out.push_str("\"Content-Type: text/plain; charset=UTF-8\\n\"\n");
    out.push_str("\"Content-Transfer-Encoding: 8bit\\n\"\n");
    out.push_str("\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\n");

    for index in 0..entries {
        let is_plural = index % 10 == 0;
        let has_comment = index % 20 == 0;
        let has_extracted = index % 25 == 0;
        let has_references = index % 3 == 0;
        let has_context = index % 12 == 0;
        let has_metadata = index % 50 == 0;
        let is_obsolete = index > 0 && index % 100 == 0;
        let is_multiline = index % 33 == 0;
        let has_escape = index % 40 == 0;
        let prefix = if is_obsolete { "#~ " } else { "" };

        if has_comment {
            push_line(&mut out, prefix, "# Translator note for entry");
        }
        if has_extracted {
            push_line(&mut out, prefix, "#. Extracted from benchmark source");
        }
        if has_metadata {
            push_line(&mut out, prefix, "#@ domain: benchmark");
        }
        if has_references {
            push_line(
                &mut out,
                prefix,
                &format!("#: src/feature_{:04}.rs:{}", index, (index % 200) + 1),
            );
        }
        if index % 18 == 0 {
            push_line(&mut out, prefix, "#, fuzzy");
        }
        if has_context {
            push_keyword(
                &mut out,
                prefix,
                "msgctxt",
                &format!("context-{}", index % 7),
            );
        }

        let msgid = if is_multiline {
            format!("Entry {index} first line\nEntry {index} second line with placeholder {{name}}")
        } else if has_escape {
            format!("Entry {index} contains \\\"quotes\\\" and \\\\slashes\\\\")
        } else {
            format!("Entry {index} simple benchmark message")
        };

        push_keyword(&mut out, prefix, "msgid", &msgid);

        if is_plural {
            let plural = if is_multiline {
                format!("Entry {index} plural first line\nEntry {index} plural second line")
            } else {
                format!("Entry {index} plural benchmark messages")
            };
            push_keyword(&mut out, prefix, "msgid_plural", &plural);

            let singular_translation = if is_multiline {
                format!("Eintrag {index} erste Zeile\nEintrag {index} zweite Zeile")
            } else {
                format!("Eintrag {index} einzelne Benchmark-Nachricht")
            };
            let plural_translation = if has_escape {
                format!("Eintrag {index} plural mit \\\"Zitat\\\" und \\\\Pfad\\\\")
            } else {
                format!("Eintrag {index} mehrere Benchmark-Nachrichten")
            };

            push_indexed_keyword(&mut out, prefix, "msgstr", 0, &singular_translation);
            push_indexed_keyword(&mut out, prefix, "msgstr", 1, &plural_translation);
        } else {
            let msgstr = if is_multiline {
                format!("Eintrag {index} erste Zeile\nEintrag {index} zweite Zeile")
            } else if has_escape {
                format!("Eintrag {index} mit \\\"Zitat\\\" und \\\\Pfad\\\\")
            } else {
                format!("Eintrag {index} einfache Benchmark-Nachricht")
            };
            push_keyword(&mut out, prefix, "msgstr", &msgstr);
        }

        out.push('\n');
    }

    out
}

fn push_line(out: &mut String, prefix: &str, line: &str) {
    out.push_str(prefix);
    out.push_str(line);
    out.push('\n');
}

fn push_keyword(out: &mut String, prefix: &str, keyword: &str, value: &str) {
    if !value.contains('\n') {
        out.push_str(prefix);
        out.push_str(keyword);
        out.push_str(" \"");
        out.push_str(&escape_po(value));
        out.push_str("\"\n");
        return;
    }

    let mut parts = value.split('\n').peekable();
    out.push_str(prefix);
    out.push_str(keyword);
    out.push_str(" \"\"\n");
    while let Some(part) = parts.next() {
        out.push_str(prefix);
        out.push('"');
        out.push_str(&escape_po(part));
        if parts.peek().is_some() {
            out.push_str("\\n");
        }
        out.push_str("\"\n");
    }
}

fn push_indexed_keyword(out: &mut String, prefix: &str, keyword: &str, index: usize, value: &str) {
    if !value.contains('\n') {
        out.push_str(prefix);
        out.push_str(keyword);
        out.push('[');
        out.push_str(&index.to_string());
        out.push_str("] \"");
        out.push_str(&escape_po(value));
        out.push_str("\"\n");
        return;
    }

    let mut parts = value.split('\n').peekable();
    out.push_str(prefix);
    out.push_str(keyword);
    out.push('[');
    out.push_str(&index.to_string());
    out.push_str("] \"\"\n");
    while let Some(part) = parts.next() {
        out.push_str(prefix);
        out.push('"');
        out.push_str(&escape_po(part));
        if parts.peek().is_some() {
            out.push_str("\\n");
        }
        out.push_str("\"\n");
    }
}

fn escape_po(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
    out
}

fn scan_stats(content: &str) -> FixtureStats {
    let mut stats = FixtureStats {
        entries: 0,
        plural_entries: 0,
        translator_comments: 0,
        extracted_comments: 0,
        references: 0,
        contexts: 0,
        metadata_comments: 0,
        obsolete_entries: 0,
        multiline_entries: 0,
        escaped_entries: 0,
    };

    let mut in_header = true;
    let mut saw_multiline_for_current = false;
    let mut saw_escape_for_current = false;
    for line in content.lines() {
        if line.starts_with("msgid \"\"") && in_header {
            continue;
        }
        if line.starts_with("msgstr \"\"") && in_header {
            continue;
        }
        if in_header && line.starts_with('"') {
            continue;
        }
        if in_header && line.is_empty() {
            in_header = false;
            continue;
        }

        if line.starts_with("msgid ") || line.starts_with("#~ msgid ") {
            if stats.entries > 0 {
                stats.multiline_entries += usize::from(saw_multiline_for_current);
                stats.escaped_entries += usize::from(saw_escape_for_current);
            }
            stats.entries += 1;
            stats.obsolete_entries += usize::from(line.starts_with("#~ "));
            saw_multiline_for_current = line.ends_with("\"\"");
            saw_escape_for_current = line.contains("\\\"") || line.contains("\\\\");
            continue;
        }

        if line.starts_with("msgid_plural ") || line.starts_with("#~ msgid_plural ") {
            stats.plural_entries += 1;
            saw_multiline_for_current |= line.ends_with("\"\"");
            saw_escape_for_current |= line.contains("\\\"") || line.contains("\\\\");
            continue;
        }

        if line.starts_with("# ") || line == "#" || line.starts_with("#~ # ") || line == "#~ #" {
            stats.translator_comments += 1;
            continue;
        }
        if line.starts_with("#. ") || line == "#." || line.starts_with("#~ #. ") || line == "#~ #."
        {
            stats.extracted_comments += 1;
            continue;
        }
        if line.starts_with("#: ") || line.starts_with("#~ #: ") {
            stats.references += 1;
            continue;
        }
        if line.starts_with("#@ ") || line.starts_with("#~ #@ ") {
            stats.metadata_comments += 1;
            continue;
        }
        if line.starts_with("msgctxt ") || line.starts_with("#~ msgctxt ") {
            stats.contexts += 1;
            continue;
        }
        if line.starts_with('"') || line.starts_with("#~ \"") {
            saw_multiline_for_current = true;
            saw_escape_for_current |= line.contains("\\\"") || line.contains("\\\\");
        }
    }

    if stats.entries > 0 {
        stats.multiline_entries += usize::from(saw_multiline_for_current);
        stats.escaped_entries += usize::from(saw_escape_for_current);
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::fixture_by_name;

    #[test]
    fn builds_mixed_1000_fixture_with_expected_shape() {
        let fixture = fixture_by_name("mixed-1000").expect("fixture exists");
        let stats = fixture.stats();

        assert_eq!(fixture.kind(), "generated");
        assert_eq!(stats.entries, 1000);
        assert_eq!(stats.plural_entries, 100);
        assert!(stats.translator_comments >= 50);
        assert!(stats.extracted_comments >= 40);
        assert!(stats.references >= 300);
        assert!(stats.contexts >= 80);
        assert!(stats.metadata_comments >= 20);
        assert!(stats.obsolete_entries >= 9);
        assert!(stats.multiline_entries >= 30);
        assert!(stats.escaped_entries >= 20);
    }
}
