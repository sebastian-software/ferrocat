use crate::{
    ConformanceCase, ConformanceManifest, Expectation, ExpectedArtifact, IcuParseExpected, strings,
};

pub fn manifest() -> ConformanceManifest {
    ConformanceManifest::new(
        "icu-ecosystem",
        "formatjs/messageformat",
        "https://formatjs.github.io/docs/core-concepts/icu-syntax/",
        "syntax guide snapshot 2026-05-12",
        "MIT / documentation reference",
        "Representative ICU MessageFormat syntax used by JS ecosystem libraries.",
        cases(),
    )
}

fn cases() -> Vec<ConformanceCase> {
    vec![
        parse_case(
            "icu.rich_text_formatters",
            "icu/rich_text_formatters.txt",
            IcuParseExpected {
                node_kinds: strings(["literal", "tag", "literal", "tag", "literal"]),
                top_level_count: Some(5),
                ..IcuParseExpected::default()
            },
        )
        .source(
            "https://formatjs.github.io/docs/core-concepts/icu-syntax/#rich-text-formatting",
            "FormatJS: rich text formatting with embedded ICU placeholders",
        ),
        parse_case(
            "icu.select_with_nested_formatter",
            "icu/select_with_nested_formatter.txt",
            IcuParseExpected {
                node_kinds: strings(["select", "literal"]),
                top_level_count: Some(2),
                ..IcuParseExpected::default()
            },
        )
        .source(
            "https://formatjs.github.io/docs/core-concepts/icu-syntax/#select-format",
            "FormatJS: select with nested number formatter",
        ),
    ]
}

fn parse_case(id: &str, input: &str, expected: IcuParseExpected) -> ConformanceCase {
    ConformanceCase::new(id, "icu_parse", "icu_parse", Expectation::Pass, input)
        .with_expected_artifact(ExpectedArtifact::IcuParse(expected))
}
