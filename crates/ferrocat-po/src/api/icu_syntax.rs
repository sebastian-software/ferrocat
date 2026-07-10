use ferrocat_icu::{IcuMessage, IcuParseError, parse_icu};

#[cfg(test)]
std::thread_local! {
    static ICU_PARSE_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

use super::IcuSyntaxPolicy;

pub(super) fn parse_icu_with_syntax_policy(
    input: &str,
    policy: IcuSyntaxPolicy,
) -> Result<IcuMessage, IcuParseError> {
    #[cfg(test)]
    ICU_PARSE_COUNT.set(ICU_PARSE_COUNT.get() + 1);

    match policy {
        IcuSyntaxPolicy::Strict => parse_icu(input),
        IcuSyntaxPolicy::RuntimeLiteralApostrophes => {
            if input.contains('\'') {
                parse_icu(&input.replace('\'', "''"))
            } else {
                parse_icu(input)
            }
        }
    }
}

#[cfg(test)]
pub(super) fn reset_icu_parse_count() {
    ICU_PARSE_COUNT.set(0);
}

#[cfg(test)]
pub(super) fn icu_parse_count() -> usize {
    ICU_PARSE_COUNT.get()
}
