use crate::{
    ConformanceCase, ConformanceManifest, Expectation, ExpectedArtifact, PoItemExpected,
    PoParseExpected, PoPluralHeaderExpected, headers, strings,
};

pub fn manifest() -> ConformanceManifest {
    ConformanceManifest::new(
        "po-pofile",
        "rubenv/pofile",
        "https://github.com/rubenv/pofile",
        "master snapshot 2026-03-16",
        "MIT",
        "Secondary JS-oriented PO reference adapted from pofile tests.",
        cases(),
    )
}

#[expect(
    clippy::too_many_lines,
    reason = "Conformance fixtures are kept as one contiguous source list for traceability."
)]
fn cases() -> Vec<ConformanceCase> {
    vec![
            parse_case(
                "pofile.multiline_parse",
                "pofile/multiline.po",
                PoParseExpected {
                    item_count: Some(1),
                    header_count: Some(2),
                    headers: headers([
                        ("Project-Id-Version", "Example"),
                        (
                            "Plural-Forms",
                            "nplurals=3; plural=n==1 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2;",
                        ),
                    ]),
                    items: vec![item("The following placeholder tokens can be used in both paths and titles. When used in a path or title, they will be replaced with the appropriate values.").with_msgstr(strings(["Les ébauches de jetons suivantes peuvent être utilisées à la fois dans les chemins et dans les titres. Lorsqu'elles sont utilisées dans un chemin ou un titre, elles seront remplacées par les valeurs appropriées."]))],
                },
            )
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/multi-line.po",
                "test/fixtures/multi-line.po + test/parse.js",
            ),
            parse_case(
                "pofile.multiline_headers",
                "pofile/multiline.po",
                PoParseExpected {
                    item_count: Some(1),
                    headers: headers([(
                        "Plural-Forms",
                        "nplurals=3; plural=n==1 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2;",
                    )]),
                    ..PoParseExpected::default()
                },
            )
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/multi-line.po",
                "test/fixtures/multi-line.po + test/parse.js:Handles multi-line headers",
            ),
            parse_case(
                "pofile.multiline_project_id_header",
                "pofile/multiline.po",
                PoParseExpected {
                    headers: headers([("Project-Id-Version", "Example")]),
                    ..PoParseExpected::default()
                },
            )
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/multi-line.po",
                "test/fixtures/multi-line.po + test/parse.js:Handles multi-line headers",
            ),
            parse_case(
                "pofile.multiline_item",
                "pofile/multiline.po",
                PoParseExpected {
                    items: vec![item("The following placeholder tokens can be used in both paths and titles. When used in a path or title, they will be replaced with the appropriate values.").with_msgstr(strings(["Les ébauches de jetons suivantes peuvent être utilisées à la fois dans les chemins et dans les titres. Lorsqu'elles sont utilisées dans un chemin ou un titre, elles seront remplacées par les valeurs appropriées."]))],
                    ..PoParseExpected::default()
                },
            )
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/multi-line.po",
                "test/fixtures/multi-line.po + test/parse.js:Handles multi-line values",
            ),
            parse_case(
                "pofile.translator_comments",
                "pofile/comment.po",
                PoParseExpected {
                    item_count: Some(2),
                    items: vec![item("Title, as plain text")
                        .with_msgstr(strings(["Attribut title, en tant que texte brut"]))
                        .with_comments(strings(["Translator comment"]))
                        .with_extracted_comments(strings(["extracted from test"]))
                        .with_references(strings([".tmp/crm/controllers/map.js"]))],
                    ..PoParseExpected::default()
                },
            )
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/comment.po",
                "test/fixtures/comment.po + test/parse.js:Handles translator comments",
            ),
            parse_case(
                "pofile.extracted_comments",
                "pofile/comment.po",
                PoParseExpected {
                    item_count: Some(2),
                    items: vec![item("Empty comment")
                        .with_msgstr(strings(["Empty"]))
                        .with_comments(strings([""]))
                        .with_extracted_comments(strings(["Extracted comment", ""]))
                        .with_references(strings([""]))],
                    ..PoParseExpected::default()
                },
            )
            .with_item_start_index(1)
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/comment.po",
                "test/fixtures/comment.po + test/parse.js:Handles extracted comments",
            ),
            parse_case(
                "pofile.empty_comments",
                "pofile/comment.po",
                PoParseExpected {
                    item_count: Some(2),
                    items: vec![item("Empty comment")
                        .with_msgstr(strings(["Empty"]))
                        .with_comments(strings([""]))
                        .with_extracted_comments(strings(["Extracted comment", ""]))
                        .with_references(strings([""]))],
                    ..PoParseExpected::default()
                },
            )
            .with_item_start_index(1)
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/comment.po",
                "test/fixtures/comment.po + test/parse.js:Handle empty comments",
            ),
            parse_case(
                "pofile.references_parse",
                "pofile/reference.po",
                PoParseExpected {
                    item_count: Some(1),
                    items: vec![item("Title, as plain text")
                        .with_msgstr(strings(["Attribut title, en tant que texte brut"]))
                        .with_comments(strings(["Comment"]))
                        .with_references(strings(["src/app.js:1", "src/lib.js:2"]))],
                    ..PoParseExpected::default()
                },
            )
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/reference.po",
                "test/fixtures/reference.po + test/parse.js",
            ),
            parse_case(
                "pofile.reference_single",
                "pofile/reference_single.po",
                PoParseExpected {
                    item_count: Some(1),
                    items: vec![item("Title, as plain text")
                        .with_msgstr(strings(["Attribut title, en tant que texte brut"]))
                        .with_comments(strings(["Comment"]))
                        .with_references(strings([".tmp/crm/controllers/map.js"]))],
                    ..PoParseExpected::default()
                },
            )
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/reference.po",
                "test/fixtures/reference.po + test/parse.js:in simple cases",
            ),
            parse_case(
                "pofile.reference_stdin_combined",
                "pofile/reference_stdin.po",
                PoParseExpected {
                    item_count: Some(1),
                    items: vec![item("Z")
                        .with_msgstr(strings(["ZZ"]))
                        .with_references(strings(["standard input:12 standard input:17"]))],
                    ..PoParseExpected::default()
                },
            )
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/reference.po",
                "test/fixtures/reference.po + test/parse.js:does not process reference items",
            ),
            parse_case(
                "pofile.flags_fuzzy",
                "pofile/fuzzy.po",
                PoParseExpected {
                    item_count: Some(1),
                    items: vec![item("Sources")
                        .with_msgstr(strings(["Source"]))
                        .with_flags(strings(["fuzzy"]))],
                    ..PoParseExpected::default()
                },
            )
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/fuzzy.po",
                "test/fixtures/fuzzy.po + test/parse.js:Parses flags",
            ),
            parse_case(
                "pofile.context_disambiguation",
                "pofile/contexts.po",
                PoParseExpected {
                    item_count: Some(3),
                    items: vec![
                        item("Empty folder")
                            .with_msgctxt("folder display")
                            .with_msgstr(strings(["Leerer Ordner"])),
                        item("Empty folder")
                            .with_msgctxt("folder action")
                            .with_msgstr(strings(["Ordner leeren"])),
                    ],
                    ..PoParseExpected::default()
                },
            )
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/big.po",
                "test/fixtures/big.po + test/parse.js:Parses item context",
            ),
            parse_case(
                "pofile.context_display",
                "pofile/contexts.po",
                PoParseExpected {
                    item_count: Some(3),
                    items: vec![item("Empty folder")
                        .with_msgctxt("folder display")
                        .with_msgstr(strings(["Leerer Ordner"]))],
                    ..PoParseExpected::default()
                },
            )
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/big.po",
                "test/fixtures/big.po + test/parse.js:Parses item context",
            ),
            parse_case(
                "pofile.context_action",
                "pofile/contexts.po",
                PoParseExpected {
                    item_count: Some(3),
                    items: vec![item("Empty folder")
                        .with_msgctxt("folder action")
                        .with_msgstr(strings(["Ordner leeren"]))],
                    ..PoParseExpected::default()
                },
            )
            .with_item_start_index(1)
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/big.po",
                "test/fixtures/big.po + test/parse.js:Parses item context",
            ),
            parse_case(
                "pofile.multiline_context",
                "pofile/contexts.po",
                PoParseExpected {
                    item_count: Some(3),
                    items: vec![item("Created Date")
                        .with_msgctxt("folder meta")
                        .with_msgstr(strings(["Erstellt am"]))],
                    ..PoParseExpected::default()
                },
            )
            .with_item_start_index(2)
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/big.po",
                "test/fixtures/big.po + test/parse.js:Parses item multiline context",
            ),
            parse_case(
                "pofile.obsolete_parse",
                "pofile/commented.po",
                PoParseExpected {
                    item_count: Some(3),
                    items: vec![
                        item("{{dataLoader.data.length}} results")
                            .with_msgstr(strings(["{{dataLoader.data.length}} resultaten"])),
                        item("Add order")
                            .with_msgstr(strings(["Order toevoegen"]))
                            .with_obsolete(true),
                        item("Second commented item")
                            .with_msgstr(strings(["also not sure"]))
                            .with_comments(strings(["commented obsolete item"]))
                            .with_flags(strings(["fuzzy"]))
                            .with_obsolete(true),
                    ],
                    ..PoParseExpected::default()
                },
            )
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/commented.po",
                "test/fixtures/commented.po + test/parse.js",
            ),
            parse_case(
                "pofile.obsolete_active_item",
                "pofile/commented.po",
                PoParseExpected {
                    item_count: Some(3),
                    items: vec![item("{{dataLoader.data.length}} results")
                        .with_msgstr(strings(["{{dataLoader.data.length}} resultaten"]))],
                    ..PoParseExpected::default()
                },
            )
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/commented.po",
                "test/fixtures/commented.po + test/parse.js:keeps active item",
            ),
            parse_case(
                "pofile.obsolete_first_item",
                "pofile/commented.po",
                PoParseExpected {
                    item_count: Some(3),
                    items: vec![item("Add order")
                        .with_msgstr(strings(["Order toevoegen"]))
                        .with_obsolete(true)],
                    ..PoParseExpected::default()
                },
            )
            .with_item_start_index(1)
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/commented.po",
                "test/fixtures/commented.po + test/parse.js:parses obsolete item",
            ),
            parse_case(
                "pofile.obsolete_second_item",
                "pofile/commented.po",
                PoParseExpected {
                    item_count: Some(3),
                    items: vec![item("Second commented item")
                        .with_msgstr(strings(["also not sure"]))
                        .with_comments(strings(["commented obsolete item"]))
                        .with_flags(strings(["fuzzy"]))
                        .with_obsolete(true)],
                    ..PoParseExpected::default()
                },
            )
            .with_item_start_index(2)
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/commented.po",
                "test/fixtures/commented.po + test/parse.js:parses commented obsolete item",
            ),
            parse_case(
                "pofile.c_strings_parse",
                "pofile/c_strings.po",
                PoParseExpected {
                    item_count: Some(3),
                    items: vec![
                        item("The name field must not contain characters like \" or \\")
                            .with_msgstr(strings([""])),
                        item("%1$s\n%2$s %3$s\n%4$s\n%5$s").with_msgstr(strings([""])),
                        item("define('some/test/module', function () {\n\t'use strict';\n\treturn {};\n});\n")
                            .with_msgstr(strings([""])),
                    ],
                    ..PoParseExpected::default()
                },
            )
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/c-strings.po",
                "test/fixtures/c-strings.po + test/parse.js",
            ),
            parse_case(
                "pofile.c_strings_plural_header",
                "pofile/c_strings.po",
                PoParseExpected {
                    headers: headers([("Plural-Forms", "nplurals=2; plural=(n > 1);")]),
                    ..PoParseExpected::default()
                },
            )
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/c-strings.po",
                "test/fixtures/c-strings.po + test/parse.js:Parses headers",
            ),
            parse_case(
                "pofile.c_string_quote",
                "pofile/c_strings.po",
                PoParseExpected {
                    item_count: Some(3),
                    items: vec![item("The name field must not contain characters like \" or \\")
                        .with_msgstr(strings([""]))],
                    ..PoParseExpected::default()
                },
            )
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/c-strings.po",
                "test/fixtures/c-strings.po + test/parse.js:extract strings containing quote and backslash",
            ),
            parse_case(
                "pofile.c_string_newlines",
                "pofile/c_strings.po",
                PoParseExpected {
                    item_count: Some(3),
                    items: vec![item("%1$s\n%2$s %3$s\n%4$s\n%5$s").with_msgstr(strings([""]))],
                    ..PoParseExpected::default()
                },
            )
            .with_item_start_index(1)
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/c-strings.po",
                "test/fixtures/c-strings.po + test/parse.js:handle newline escapes",
            ),
            parse_case(
                "pofile.c_string_tabs",
                "pofile/c_strings.po",
                PoParseExpected {
                    item_count: Some(3),
                    items: vec![item("define('some/test/module', function () {\n\t'use strict';\n\treturn {};\n});\n")
                        .with_msgstr(strings([""]))],
                    ..PoParseExpected::default()
                },
            )
            .with_item_start_index(2)
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/c-strings.po",
                "test/fixtures/c-strings.po + test/parse.js:handle tab escapes",
            ),

            roundtrip_case("pofile.fuzzy_roundtrip", "pofile/fuzzy.po")
                .with_expected_fixture("pofile/fuzzy.normalized.expected.po")
                .with_notes(
                    "ferrocat-po intentionally normalizes headerless files by emitting an empty header on write.",
                )
                .source(
                    "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/fuzzy.po",
                    "test/fixtures/fuzzy.po + test/write.js",
                ),

            plural_header_case(
                "pofile.plural_header_parse",
                "pofile/plural_header.po",
                PoPluralHeaderExpected {
                    raw_value: Some(
                        "nplurals=3; plural=(n==1 ? 0 : n==2 ? 1 : 2);".to_owned(),
                    ),
                    nplurals: Some(3),
                    plural_expression: Some("(n==1 ? 0 : n==2 ? 1 : 2)".to_owned()),
                    first_item_msgstr_len: Some(3),
                },
            )
            .with_locale("fr", "en")
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/parse-plural-forms.js",
                "test/parse-plural-forms.js",
            ),
            parse_case(
                "pofile.plural_header_item_parse",
                "pofile/plural_header.po",
                PoParseExpected {
                    item_count: Some(1),
                    items: vec![item("thing")
                        .with_msgid_plural("things")
                        .with_msgstr(strings(["eins", "zwei", "viele"]))],
                    ..PoParseExpected::default()
                },
            )
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/parse-plural-forms.js",
                "test/parse-plural-forms.js + fixtures/plural header item",
            ),

            plural_header_case(
                "pofile.plural_slots_two_forms",
                "pofile/plurals_messages.po",
                PoPluralHeaderExpected {
                    raw_value: Some("nplurals=2; plural=(n != 1);".to_owned()),
                    nplurals: Some(2),
                    plural_expression: Some("(n != 1)".to_owned()),
                    first_item_msgstr_len: Some(2),
                },
            )
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/plurals/messages.po",
                "test/fixtures/plurals/messages.po + test/write.js:nplurals INTEGER",
            ),
            parse_case(
                "pofile.plural_slots_first_item",
                "pofile/plurals_messages.po",
                PoParseExpected {
                    item_count: Some(2),
                    items: vec![item("{{$count}} thing")
                        .with_msgid_plural("{{$count}} things")
                        .with_msgstr(strings(["", ""]))],
                    ..PoParseExpected::default()
                },
            )
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/plurals/messages.po",
                "test/fixtures/plurals/messages.po + test/write.js:first plural item",
            ),
            parse_case(
                "pofile.plural_slots_second_item",
                "pofile/plurals_messages.po",
                PoParseExpected {
                    item_count: Some(2),
                    items: vec![item("{{$count}} mistake")
                        .with_msgid_plural("{{$count}} mistakes")
                        .with_msgstr(strings(["", ""]))],
                    ..PoParseExpected::default()
                },
            )
            .with_item_start_index(1)
            .source(
                "https://raw.githubusercontent.com/rubenv/pofile/master/test/fixtures/plurals/messages.po",
                "test/fixtures/plurals/messages.po + test/write.js:second plural item",
            ),
        ]
}

fn parse_case(id: &str, input: &str, expected: PoParseExpected) -> ConformanceCase {
    ConformanceCase::new(id, "parse", "po_parse", Expectation::Pass, input)
        .with_expected_artifact(ExpectedArtifact::PoParse(expected))
}

fn plural_header_case(id: &str, input: &str, expected: PoPluralHeaderExpected) -> ConformanceCase {
    ConformanceCase::new(
        id,
        "plural_header",
        "po_plural_header",
        Expectation::Pass,
        input,
    )
    .with_expected_artifact(ExpectedArtifact::PoPluralHeader(expected))
}

fn roundtrip_case(id: &str, input: &str) -> ConformanceCase {
    ConformanceCase::new(id, "roundtrip", "po_roundtrip", Expectation::Pass, input)
}

fn item(msgid: &str) -> PoItemExpected {
    PoItemExpected {
        msgid: msgid.to_owned(),
        ..PoItemExpected::default()
    }
}

trait PoItemExpectedExt {
    fn with_msgctxt(self, value: &str) -> Self;
    fn with_msgid_plural(self, value: &str) -> Self;
    fn with_msgstr(self, values: Vec<String>) -> Self;
    fn with_comments(self, values: Vec<String>) -> Self;
    fn with_extracted_comments(self, values: Vec<String>) -> Self;
    fn with_references(self, values: Vec<String>) -> Self;
    fn with_flags(self, values: Vec<String>) -> Self;
    fn with_obsolete(self, value: bool) -> Self;
}

impl PoItemExpectedExt for PoItemExpected {
    fn with_msgctxt(mut self, value: &str) -> Self {
        self.msgctxt = Some(value.to_owned());
        self
    }

    fn with_msgid_plural(mut self, value: &str) -> Self {
        self.msgid_plural = Some(value.to_owned());
        self
    }

    fn with_msgstr(mut self, values: Vec<String>) -> Self {
        self.msgstr = values;
        self
    }

    fn with_comments(mut self, values: Vec<String>) -> Self {
        self.comments = values;
        self
    }

    fn with_extracted_comments(mut self, values: Vec<String>) -> Self {
        self.extracted_comments = values;
        self
    }

    fn with_references(mut self, values: Vec<String>) -> Self {
        self.references = values;
        self
    }

    fn with_flags(mut self, values: Vec<String>) -> Self {
        self.flags = values;
        self
    }

    fn with_obsolete(mut self, value: bool) -> Self {
        self.obsolete = value;
        self
    }
}
