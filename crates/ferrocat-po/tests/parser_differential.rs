use std::borrow::Cow;

use ferrocat_po::{MergeExtractedMessage, PoFile, merge_catalog, parse_po, parse_po_borrowed};

const CASES: &[(&str, &str)] = &[
    (
        "context plural with metadata",
        concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Language: de\\n\"\n",
            "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n",
            "\n",
            "# Translator note\n",
            "#. Extracted note\n",
            "#: src/app.rs:1 src/lib.rs:2\n",
            "#, fuzzy, rust-format\n",
            "#@ owner: checkout\n",
            "msgctxt \"menu\"\n",
            "msgid \"file\"\n",
            "msgid_plural \"files\"\n",
            "msgstr[0] \"Datei\"\n",
            "msgstr[1] \"Dateien\"\n",
            "\n",
            "msgid \"Save\"\n",
            "msgstr \"Speichern\"\n",
        ),
    ),
    (
        "multiline escaped content",
        concat!(
            "msgid \"multi\"\n",
            "\" line\\n\"\n",
            "\"source\"\n",
            "msgstr \"mehr\"\n",
            "\" zeilig\\t\"\n",
            "\n",
            "msgctxt \"cta\"\n",
            "msgid \"Quote \\\"here\\\"\"\n",
            "msgstr \"Zitat \\\"hier\\\"\"\n",
        ),
    ),
];

#[test]
fn owned_and_borrowed_parsers_match_on_shared_lf_inputs() {
    for (name, input) in CASES {
        let owned = parse_po(input).unwrap_or_else(|error| panic!("{name}: owned parse: {error}"));
        let borrowed = parse_po_borrowed(input)
            .unwrap_or_else(|error| panic!("{name}: borrowed parse: {error}"))
            .into_owned();

        assert_eq!(borrowed, owned, "{name}");
    }
}

#[test]
fn merge_parser_preserves_matching_existing_messages() {
    for (name, input) in CASES {
        let owned = parse_po(input).unwrap_or_else(|error| panic!("{name}: owned parse: {error}"));
        let extracted = extracted_messages_from(&owned);
        let merged = merge_catalog(input, &extracted)
            .unwrap_or_else(|error| panic!("{name}: merge parse: {error}"));
        let reparsed =
            parse_po(&merged).unwrap_or_else(|error| panic!("{name}: reparse merged: {error}"));

        assert_eq!(reparsed.items, owned.items, "{name}\nmerged:\n{merged}");
    }
}

fn extracted_messages_from(file: &PoFile) -> Vec<MergeExtractedMessage<'_>> {
    file.items
        .iter()
        .filter(|item| !item.obsolete)
        .map(|item| MergeExtractedMessage {
            msgctxt: item.msgctxt.as_deref().map(Cow::Borrowed),
            msgid: Cow::Borrowed(item.msgid.as_str()),
            msgid_plural: item.msgid_plural.as_deref().map(Cow::Borrowed),
            references: item
                .references
                .iter()
                .map(|reference| Cow::Borrowed(reference.as_str()))
                .collect(),
            extracted_comments: item
                .extracted_comments
                .iter()
                .map(|comment| Cow::Borrowed(comment.as_str()))
                .collect(),
            flags: item
                .flags
                .iter()
                .map(|flag| Cow::Borrowed(flag.as_str()))
                .collect(),
        })
        .collect()
}
