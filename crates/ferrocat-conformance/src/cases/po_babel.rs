use crate::{
    ConformanceCase, ConformanceManifest, Expectation, ExpectedArtifact, PoItemExpected,
    PoParseExpected, headers, strings,
};

pub fn manifest() -> ConformanceManifest {
    ConformanceManifest::new(
        "po-babel",
        "python-babel/babel",
        "https://github.com/python-babel/babel",
        "master snapshot 2026-03-16",
        "BSD-3-Clause",
        "Targeted PO supplement for locale and modern roundtrip edge cases.",
        cases(),
    )
}

fn cases() -> Vec<ConformanceCase> {
    vec![
        roundtrip_case("babel.unknown_language_roundtrip", "babel/unknown_language.po")
            .with_expected_fixture("babel/unknown_language.po")
            .source(
                "https://raw.githubusercontent.com/python-babel/babel/master/tests/messages/test_pofile.py",
                "test_pofile.py:test_unknown_language_roundtrip",
            ),

        parse_case(
            "babel.unknown_language_header",
            "babel/unknown_language.po",
            PoParseExpected {
                headers: headers([("Language", "sr_SP")]),
                ..PoParseExpected::default()
            },
        )
        .source(
            "https://raw.githubusercontent.com/python-babel/babel/master/tests/messages/test_pofile.py",
            "test_pofile.py:test_unknown_language_roundtrip",
        ),

        parse_case(
            "babel.irregular_multiline_msgstr",
            "babel/irregular_multiline.po",
            PoParseExpected {
                item_count: Some(1),
                items: vec![PoItemExpected {
                    msgid: "foo".to_owned(),
                    msgstr: strings(["multi-line\n translation"]),
                    ..PoItemExpected::default()
                }],
                ..PoParseExpected::default()
            },
        )
        .source(
            "https://raw.githubusercontent.com/python-babel/babel/master/tests/messages/test_pofile.py",
            "test_pofile.py:test_denormalize_on_msgstr_without_empty_first_line",
        ),

        parse_case(
            "babel.enclosed_location_comment",
            "babel/enclosed_locations.po",
            PoParseExpected {
                item_count: Some(1),
                items: vec![PoItemExpected {
                    msgid: "foo".to_owned(),
                    msgstr: strings(["bar"]),
                    references: strings(["main 1.py:1", "other.py:2"]),
                    ..PoItemExpected::default()
                }],
                ..PoParseExpected::default()
            },
        )
        .source(
            "https://raw.githubusercontent.com/python-babel/babel/master/tests/messages/test_pofile.py",
            "test_pofile.py:test_extract_locations_valid_location_comment",
        ),

        parse_case(
            "babel.enclosed_location_message",
            "babel/enclosed_locations.po",
            PoParseExpected {
                item_count: Some(1),
                items: vec![PoItemExpected {
                    msgid: "foo".to_owned(),
                    msgstr: strings(["bar"]),
                    references: strings(["main 1.py:1", "other.py:2"]),
                    ..PoItemExpected::default()
                }],
                ..PoParseExpected::default()
            },
        )
        .source(
            "https://raw.githubusercontent.com/python-babel/babel/master/tests/messages/test_pofile.py",
            "test_pofile.py:test_extract_locations_valid_location_comment",
        ),
    ]
}

fn parse_case(id: &str, input: &str, expected: PoParseExpected) -> ConformanceCase {
    ConformanceCase::new(id, "parse", "po_parse", Expectation::Pass, input)
        .with_expected_artifact(ExpectedArtifact::PoParse(expected))
}

fn roundtrip_case(id: &str, input: &str) -> ConformanceCase {
    ConformanceCase::new(id, "roundtrip", "po_roundtrip", Expectation::Pass, input)
}
