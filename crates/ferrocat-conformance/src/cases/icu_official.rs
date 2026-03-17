use crate::{
    ConformanceCase, ConformanceManifest, Expectation, ExpectedArtifact, IcuParseExpected,
    IcuRejectExpected, strings,
};

pub fn manifest() -> ConformanceManifest {
    ConformanceManifest::new(
        "icu-official",
        "unicode-org/icu",
        "https://github.com/unicode-org/icu",
        "main snapshot 2026-03-16",
        "Unicode License",
        "Official ICU MessageFormat parser reference focused on parser-visible behavior.",
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
            "icu.simple_argument",
            "icu/simple_argument.txt",
            IcuParseExpected {
                node_kinds: strings(["literal", "argument", "literal"]),
                top_level_count: Some(3),
                first_literal: Some("Hello ".to_owned()),
                first_argument_name: Some("name".to_owned()),
                ..IcuParseExpected::default()
            },
        )
        .source(
            "https://raw.githubusercontent.com/unicode-org/icu/main/icu4j/main/common_tests/src/test/java/com/ibm/icu/dev/test/format/TestMessageFormat.java",
            "TestMessageFormat: basic argument parsing",
        ),

        parse_case(
            "icu.plural_and_selectordinal",
            "icu/plural_and_selectordinal.txt",
            IcuParseExpected {
                top_level_count: Some(4),
                first_plural_kind: Some("cardinal".to_owned()),
                first_plural_offset: Some(1),
                first_plural_option_count: Some(3),
                second_plural_kind: Some("ordinal".to_owned()),
                second_plural_option_count: Some(2),
                ..IcuParseExpected::default()
            },
        )
        .source(
            "https://raw.githubusercontent.com/unicode-org/icu/main/icu4j/main/common_tests/src/test/java/com/ibm/icu/dev/test/format/TestMessageFormat.java",
            "TestMessageFormat: parses plural/selectordinal",
        ),

        parse_case(
            "icu.tags_nested",
            "icu/tags_nested.txt",
            IcuParseExpected {
                top_level_count: Some(2),
                ..IcuParseExpected::default()
            },
        )
        .source(
            "https://raw.githubusercontent.com/unicode-org/icu/main/icu4j/main/common_tests/src/test/java/com/ibm/icu/dev/test/format/TestMessageFormat.java",
            "TestMessageFormat: parses tags and nested content",
        ),

        parse_case(
            "icu.apostrophe_escape",
            "icu/apostrophe_escape.txt",
            IcuParseExpected {
                node_kinds: strings(["literal", "argument", "literal"]),
                top_level_count: Some(3),
                first_literal: Some("{".to_owned()),
                first_argument_name: Some("name".to_owned()),
                ..IcuParseExpected::default()
            },
        )
        .source(
            "https://raw.githubusercontent.com/unicode-org/icu/main/icu4j/main/common_tests/src/test/java/com/ibm/icu/dev/test/format/TestMessageFormat.java",
            "TestMessageFormat: apostrophe escaping",
        ),

        reject_case(
            "icu.missing_other_reject",
            "icu/missing_other.txt",
            IcuRejectExpected {
                message_contains: "other".to_owned(),
                ..IcuRejectExpected::default()
            },
        )
        .source(
            "https://raw.githubusercontent.com/unicode-org/icu/main/icu4j/main/common_tests/src/test/java/com/ibm/icu/dev/test/format/TestMessageFormat.java",
            "TestMessageFormat: missing other clause",
        ),

        reject_case(
            "icu.invalid_offset_reject",
            "icu/invalid_offset.txt",
            IcuRejectExpected {
                message_contains: "integer".to_owned(),
                ..IcuRejectExpected::default()
            },
        )
        .source(
            "https://raw.githubusercontent.com/unicode-org/icu/main/icu4j/main/common_tests/src/test/java/com/ibm/icu/dev/test/format/TestMessageFormat.java",
            "TestMessageFormat: invalid offset",
        ),

        reject_case(
            "icu.mismatched_tag_reject",
            "icu/mismatched_tag.txt",
            IcuRejectExpected {
                message_contains: "Mismatched".to_owned(),
                ..IcuRejectExpected::default()
            },
        )
        .source(
            "https://raw.githubusercontent.com/unicode-org/icu/main/icu4j/main/common_tests/src/test/java/com/ibm/icu/dev/test/format/TestMessageFormat.java",
            "TestMessageFormat: mismatched closing tag",
        ),

        reject_case(
            "icu.error_position_reject",
            "icu/error_position.txt",
            IcuRejectExpected {
                message_contains: "Expected".to_owned(),
                line: Some(3),
                min_column: Some(1),
            },
        )
        .source(
            "https://raw.githubusercontent.com/unicode-org/icu/main/icu4j/main/common_tests/src/test/java/com/ibm/icu/dev/test/format/TestMessageFormat.java",
            "TestMessageFormat: error positions are reported",
        ),
    ]
}

fn parse_case(id: &str, input: &str, expected: IcuParseExpected) -> ConformanceCase {
    ConformanceCase::new(id, "icu_parse", "icu_parse", Expectation::Pass, input)
        .with_expected_artifact(ExpectedArtifact::IcuParse(expected))
}

fn reject_case(id: &str, input: &str, expected: IcuRejectExpected) -> ConformanceCase {
    ConformanceCase::new(id, "icu_parse", "icu_reject", Expectation::Reject, input)
        .with_expected_artifact(ExpectedArtifact::IcuReject(expected))
}
