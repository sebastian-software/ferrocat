use ferrocat_icu::{IcuMessage, IcuParseError, parse_icu};

use super::IcuSyntaxPolicy;

pub(super) fn parse_icu_with_syntax_policy(
    input: &str,
    policy: IcuSyntaxPolicy,
) -> Result<IcuMessage, IcuParseError> {
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
