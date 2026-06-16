use ferrocat_icu::{parse_icu, stringify_icu};
use proptest::prelude::*;

fn identifier_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,12}"
}

fn literal_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 .,!?_-]{0,32}"
}

fn simple_message_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        literal_strategy(),
        (
            literal_strategy(),
            identifier_strategy(),
            literal_strategy()
        )
            .prop_map(|(prefix, name, suffix)| format!("{prefix}{{{name}}}{suffix}")),
        identifier_strategy().prop_map(|name| format!("{{{name}, number, integer}}")),
        (
            identifier_strategy(),
            literal_strategy(),
            literal_strategy(),
        )
            .prop_map(|(name, one, other)| {
                format!("{{{name}, plural, one {{{one}}} other {{{other}}}}}")
            }),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        ..ProptestConfig::default()
    })]

    #[test]
    fn stringify_icu_roundtrips_generated_messages(input in simple_message_strategy()) {
        let parsed = parse_icu(&input).expect("generated ICU message should parse");
        let rendered = stringify_icu(&parsed);
        let reparsed = parse_icu(&rendered).expect("rendered ICU message should parse");

        prop_assert_eq!(reparsed, parsed);
    }
}
