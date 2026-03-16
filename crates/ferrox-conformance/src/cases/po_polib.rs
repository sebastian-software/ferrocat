use crate::{
    ConformanceCase, ConformanceManifest, Expectation, ExpectedArtifact, PoItemExpected,
    PoParseExpected, PoRejectExpected, headers, strings,
};

pub fn manifest() -> ConformanceManifest {
    ConformanceManifest::new(
        "po-polib",
        "izimobil/polib",
        "https://github.com/izimobil/polib",
        "master snapshot 2026-03-16",
        "MIT",
        "Primary PO edge-case reference adapted from polib tests.",
        cases(),
    )
    .with_notes([
        "Fixtures are compact semantic snapshots derived from upstream tests, not raw bulk copies.",
    ])
}

fn cases() -> Vec<ConformanceCase> {
    vec![
        roundtrip_case("polib.comment_ordering_roundtrip", "polib/comment_ordering.po")
            .with_expected_fixture("polib/comment_ordering.expected.po")
            .source(
                "https://raw.githubusercontent.com/izimobil/polib/master/tests/test_comment_ordering.po",
                "tests/test_comment_ordering.po",
            ),

        parse_case(
            "polib.comment_ordering_parse",
            "polib/comment_ordering.po",
            PoParseExpected {
                item_count: Some(1),
                header_count: Some(1),
                headers: headers([("Content-Type", "text/plain; charset=UTF-8")]),
                items: vec![PoItemExpected {
                    msgid: "foo".to_owned(),
                    msgstr: strings(["oof"]),
                    comments: strings(["First comment line"]),
                    extracted_comments: strings(["Second comment line"]),
                    ..PoItemExpected::default()
                }],
            },
        )
        .source(
            "https://raw.githubusercontent.com/izimobil/polib/master/tests/test_comment_ordering.po",
            "tests/test_comment_ordering.po + tests.py:test_comment_ordering",
        ),

        roundtrip_known_gap_case("polib.wrapwidth_50", "polib/wrap_input.po")
            .with_expected_fixture("polib/wrap_expected.po")
            .with_fold_length(50)
            .with_notes("ferrox-po wraps long msgid lines differently from polib's serializer today.")
            .source(
                "https://raw.githubusercontent.com/izimobil/polib/master/tests/test_wrap.po",
                "tests/test_wrap.po + tests.py:test_wrapping",
            ),

        parse_case(
            "polib.wrap_second_item_parse",
            "polib/wrap_input.po",
            PoParseExpected {
                item_count: Some(3),
                items: vec![PoItemExpected {
                    msgid: "Some line that contain special characters \" and that \t is very, very, very long...: %s \n".to_owned(),
                    msgstr: strings([""]),
                    ..PoItemExpected::default()
                }],
                ..PoParseExpected::default()
            },
        )
        .with_item_start_index(1)
        .source(
            "https://raw.githubusercontent.com/izimobil/polib/master/tests/test_wrap.po",
            "tests/test_wrap.po + tests.py:test_wrapping",
        ),

        parse_case(
            "polib.wrap_third_item_parse",
            "polib/wrap_input.po",
            PoParseExpected {
                item_count: Some(3),
                items: vec![PoItemExpected {
                    msgid: "Some line that contain special characters \"foobar\" and that contains whitespace at the end          ".to_owned(),
                    msgstr: strings([""]),
                    ..PoItemExpected::default()
                }],
                ..PoParseExpected::default()
            },
        )
        .with_item_start_index(2)
        .source(
            "https://raw.githubusercontent.com/izimobil/polib/master/tests/test_wrap.po",
            "tests/test_wrap.po + tests.py:test_wrapping",
        ),

        reject_known_gap_case(
            "polib.invalid_quote_reject",
            "polib/unescaped_quote.po",
            PoRejectExpected {
                message_contains: "unescaped".to_owned(),
            },
        )
        .with_notes("The permissive parser currently accepts this invalid quoting pattern.")
        .source(
            "https://raw.githubusercontent.com/izimobil/polib/master/tests/tests.py",
            "tests.py:test_unescaped_double_quote2",
        ),

        merge_case("polib.merge_basic", "polib/merge_before.po", "polib/merge_template.pot")
            .with_expected_fixture("polib/merge_expected.po")
            .source(
                "https://raw.githubusercontent.com/izimobil/polib/master/tests/test_merge_before.po",
                "tests/test_merge_before.po + tests/test_merge_after.po",
            ),

        parse_case(
            "polib.merge_keep_item",
            "polib/merge_expected.po",
            PoParseExpected {
                item_count: Some(3),
                items: vec![PoItemExpected {
                    msgid: "keep".to_owned(),
                    msgstr: strings(["Behalten"]),
                    references: strings(["src/new.rs:10"]),
                    ..PoItemExpected::default()
                }],
                ..PoParseExpected::default()
            },
        )
        .source(
            "https://raw.githubusercontent.com/izimobil/polib/master/tests/test_merge_after.po",
            "tests/test_merge_after.po:preserves existing translation and updates references",
        ),

        parse_case(
            "polib.merge_add_item",
            "polib/merge_expected.po",
            PoParseExpected {
                item_count: Some(3),
                items: vec![PoItemExpected {
                    msgid: "add".to_owned(),
                    msgstr: strings([""]),
                    references: strings(["src/new.rs:20"]),
                    ..PoItemExpected::default()
                }],
                ..PoParseExpected::default()
            },
        )
        .with_item_start_index(1)
        .source(
            "https://raw.githubusercontent.com/izimobil/polib/master/tests/test_merge_after.po",
            "tests/test_merge_after.po:adds new template message",
        ),

        parse_case(
            "polib.merge_drop_item",
            "polib/merge_expected.po",
            PoParseExpected {
                item_count: Some(3),
                items: vec![PoItemExpected {
                    msgid: "drop".to_owned(),
                    msgstr: strings(["Entfernen"]),
                    obsolete: true,
                    ..PoItemExpected::default()
                }],
                ..PoParseExpected::default()
            },
        )
        .with_item_start_index(2)
        .source(
            "https://raw.githubusercontent.com/izimobil/polib/master/tests/test_merge_after.po",
            "tests/test_merge_after.po:marks missing message obsolete",
        ),

        parse_case(
            "polib.previous_msgid_current_item",
            "polib/previous_msgid.po",
            PoParseExpected {
                item_count: Some(1),
                items: vec![PoItemExpected {
                    msgid: "File".to_owned(),
                    msgctxt: Some("menu".to_owned()),
                    msgstr: strings(["Datei"]),
                    ..PoItemExpected::default()
                }],
                ..PoParseExpected::default()
            },
        )
        .source(
            "https://raw.githubusercontent.com/izimobil/polib/master/tests/test_previous_msgid.po",
            "tests/test_previous_msgid.po:parses current item body",
        ),

        parse_known_gap_case("polib_utf8_bom", "polib/bom.po")
            .with_notes("Owned parser does not yet normalize UTF-8 BOM-prefixed PO content.")
            .source(
                "https://raw.githubusercontent.com/izimobil/polib/master/tests/test_ufeff.po",
                "tests/test_ufeff.po",
            ),
    ]
}

fn parse_case(id: &str, input: &str, expected: PoParseExpected) -> ConformanceCase {
    ConformanceCase::new(id, "parse", "po_parse", Expectation::Pass, input)
        .with_expected_artifact(ExpectedArtifact::PoParse(expected))
}

fn parse_known_gap_case(id: &str, input: &str) -> ConformanceCase {
    ConformanceCase::new(id, "parse", "po_parse", Expectation::KnownGap, input)
}

fn reject_known_gap_case(id: &str, input: &str, expected: PoRejectExpected) -> ConformanceCase {
    ConformanceCase::new(id, "diagnostics", "po_reject", Expectation::KnownGap, input)
        .with_expected_artifact(ExpectedArtifact::PoReject(expected))
}

fn roundtrip_case(id: &str, input: &str) -> ConformanceCase {
    ConformanceCase::new(id, "roundtrip", "po_roundtrip", Expectation::Pass, input)
}

fn roundtrip_known_gap_case(id: &str, input: &str) -> ConformanceCase {
    ConformanceCase::new(
        id,
        "roundtrip",
        "po_roundtrip",
        Expectation::KnownGap,
        input,
    )
}

fn merge_case(id: &str, input: &str, companion_input: &str) -> ConformanceCase {
    ConformanceCase::new(id, "merge", "po_merge", Expectation::Pass, input)
        .with_companion_input(companion_input)
}
