use std::borrow::Cow;

use ferrocat_icu::{IcuMessage, IcuParseError, parse_icu};

#[cfg(test)]
std::thread_local! {
    static ICU_PARSE_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

use super::IcuSyntaxPolicy;

/// Canonicalizes an ICU MessageFormat pattern for the selected syntax policy.
///
/// [`IcuSyntaxPolicy::Strict`] returns the input unchanged. The
/// [`IcuSyntaxPolicy::RuntimeLiteralApostrophes`] policy converts the runtime
/// dialect into an equivalent strict ICU pattern:
///
/// - ordinary apostrophes are doubled,
/// - `''` remains one escaped apostrophe,
/// - a single apostrophe starts a quote before `{`, `}`, or `<`,
/// - a single apostrophe before `#` starts a quote only inside a plural or
///   selectordinal branch,
/// - an unterminated runtime quote is closed at the end of the pattern.
///
/// The canonical form is idempotent. Strict-policy inputs and runtime-policy
/// inputs without apostrophes are returned as [`Cow::Borrowed`].
#[must_use]
pub fn canonicalize_icu_with_policy(input: &str, policy: IcuSyntaxPolicy) -> Cow<'_, str> {
    if policy == IcuSyntaxPolicy::Strict || !input.contains('\'') {
        return Cow::Borrowed(input);
    }

    Cow::Owned(canonicalize_runtime_apostrophes(input))
}

pub(super) fn parse_icu_with_syntax_policy(
    input: &str,
    policy: IcuSyntaxPolicy,
) -> Result<IcuMessage, IcuParseError> {
    #[cfg(test)]
    ICU_PARSE_COUNT.set(ICU_PARSE_COUNT.get() + 1);

    let canonical = canonicalize_icu_with_policy(input, policy);
    parse_icu(&canonical).map_err(|error| remap_icu_error(error, input, &canonical))
}

// Canonicalization only inserts apostrophes. Walk its byte sequence on the
// error path to translate the strict parser's offset back to the input; the
// successful parse does not allocate a position map.
pub(super) fn remap_icu_error(
    mut error: IcuParseError,
    input: &str,
    canonical: &str,
) -> IcuParseError {
    if input == canonical {
        return error;
    }
    let bytes = input.as_bytes();
    let mut source_offset = 0;
    for &byte in &canonical.as_bytes()[..error.position.offset] {
        if bytes.get(source_offset) == Some(&byte) {
            source_offset += 1;
        } else if byte != b'\'' {
            // Do not manufacture a location if a future canonicalizer changes
            // more than apostrophes.
            return error;
        }
    }
    if !input.is_char_boundary(source_offset) {
        return error;
    }
    error.position = IcuParseError::syntax("", input, source_offset).position;
    error
}

fn canonicalize_runtime_apostrophes(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut output = String::with_capacity(input.len() + 8);
    let mut plural_depths = vec![false];
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'\'' => {
                index = push_runtime_apostrophe(
                    input,
                    index,
                    plural_is_active(&plural_depths),
                    &mut output,
                );
            }
            b'{' => {
                let inherited = plural_is_active(&plural_depths);
                plural_depths.push(inherited || opens_plural_argument(input, index));
                output.push('{');
                index += 1;
            }
            b'}' => {
                if plural_depths.len() > 1 {
                    plural_depths.pop();
                }
                output.push('}');
                index += 1;
            }
            _ => {
                let start = index;
                index += 1;
                while index < bytes.len() && !matches!(bytes[index], b'\'' | b'{' | b'}') {
                    index += 1;
                }
                output.push_str(&input[start..index]);
            }
        }
    }

    output
}

fn plural_is_active(stack: &[bool]) -> bool {
    stack.last().copied().unwrap_or(false)
}

fn push_runtime_apostrophe(
    input: &str,
    index: usize,
    plural_is_active: bool,
    output: &mut String,
) -> usize {
    let bytes = input.as_bytes();
    let next = bytes.get(index + 1).copied();

    if next == Some(b'\'') {
        output.push_str("''");
        return index + 2;
    }

    let starts_quote = matches!(next, Some(b'{') | Some(b'}') | Some(b'<'))
        || (plural_is_active && next == Some(b'#'));
    if !starts_quote {
        output.push_str("''");
        return index + 1;
    }

    let (literal, end) = read_runtime_quoted_literal(input, index + 1);
    output.push('\'');
    for part in literal.split_inclusive('\'') {
        output.push_str(part);
        if part.ends_with('\'') {
            output.push('\'');
        }
    }
    output.push('\'');
    end
}

fn read_runtime_quoted_literal(input: &str, start: usize) -> (String, usize) {
    let bytes = input.as_bytes();
    let mut literal = String::new();
    let mut index = start;

    while index < bytes.len() {
        if bytes[index] != b'\'' {
            let chunk_start = index;
            index += 1;
            while index < bytes.len() && bytes[index] != b'\'' {
                index += 1;
            }
            literal.push_str(&input[chunk_start..index]);
            continue;
        }

        if bytes.get(index + 1).copied() == Some(b'\'') {
            literal.push('\'');
            index += 2;
            continue;
        }

        return (literal, index + 1);
    }

    (literal, index)
}

fn opens_plural_argument(input: &str, index: usize) -> bool {
    let rest = &input[index + 1..];
    let Some((_, after_name)) = split_argument_part(rest) else {
        return false;
    };
    let Some((kind, _)) = split_argument_part(after_name) else {
        return false;
    };

    matches!(kind.trim(), "plural" | "selectordinal")
}

fn split_argument_part(input: &str) -> Option<(&str, &str)> {
    let stop = input.find([',', '}', '{'])?;
    if input.as_bytes()[stop] != b',' {
        return None;
    }

    Some((&input[..stop], &input[stop + 1..]))
}

#[cfg(test)]
pub(super) fn reset_icu_parse_count() {
    ICU_PARSE_COUNT.set(0);
}

#[cfg(test)]
pub(super) fn icu_parse_count() -> usize {
    ICU_PARSE_COUNT.get()
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use ferrocat_icu::IcuNode;

    use super::{canonicalize_icu_with_policy, parse_icu_with_syntax_policy};
    use crate::api::IcuSyntaxPolicy;

    #[test]
    fn runtime_policy_canonicalizes_natural_apostrophes_and_borrows_plain_input() {
        assert_eq!(
            canonicalize_icu_with_policy("don't", IcuSyntaxPolicy::RuntimeLiteralApostrophes),
            Cow::Owned::<str>("don''t".to_owned())
        );
        assert!(matches!(
            canonicalize_icu_with_policy(
                "Hello {name}",
                IcuSyntaxPolicy::RuntimeLiteralApostrophes
            ),
            Cow::Borrowed("Hello {name}")
        ));
        assert!(matches!(
            canonicalize_icu_with_policy("don't", IcuSyntaxPolicy::Strict),
            Cow::Borrowed("don't")
        ));
    }

    #[test]
    fn runtime_policy_preserves_meaningful_icu_apostrophe_quoting() {
        let quoted_argument =
            parse_icu_with_syntax_policy("L'{title}", IcuSyntaxPolicy::RuntimeLiteralApostrophes)
                .expect("runtime quote");
        assert_eq!(
            quoted_argument.nodes,
            vec![IcuNode::Literal("L{title}".to_owned())]
        );

        let escaped_apostrophe = parse_icu_with_syntax_policy(
            "rock ''n'' roll",
            IcuSyntaxPolicy::RuntimeLiteralApostrophes,
        )
        .expect("escaped apostrophes");
        assert_eq!(
            escaped_apostrophe.nodes,
            vec![IcuNode::Literal("rock 'n' roll".to_owned())]
        );
    }

    #[test]
    fn runtime_policy_only_treats_quoted_pound_as_syntax_inside_plural_branches() {
        let plural = parse_icu_with_syntax_policy(
            "{count, plural, other {'#' items}}",
            IcuSyntaxPolicy::RuntimeLiteralApostrophes,
        )
        .expect("plural");
        let IcuNode::Plural { options, .. } = &plural.nodes[0] else {
            panic!("expected plural");
        };
        assert_eq!(
            options[0].value,
            vec![IcuNode::Literal("# items".to_owned())]
        );

        assert_eq!(
            canonicalize_icu_with_policy("'#' items", IcuSyntaxPolicy::RuntimeLiteralApostrophes),
            "''#'' items"
        );
    }

    #[test]
    fn runtime_policy_canonicalization_is_idempotent() {
        for input in [
            "don't",
            "L'{title} est prêt",
            "{count, plural, other {'#' items}}",
            "'#' items",
            "''",
        ] {
            let once =
                canonicalize_icu_with_policy(input, IcuSyntaxPolicy::RuntimeLiteralApostrophes);
            let twice = canonicalize_icu_with_policy(
                once.as_ref(),
                IcuSyntaxPolicy::RuntimeLiteralApostrophes,
            );
            assert_eq!(twice, once, "input: {input}");
        }
    }
}

#[cfg(test)]
mod tag_quote_tests {
    use super::{canonicalize_icu_with_policy, parse_icu_with_syntax_policy};
    use crate::api::IcuSyntaxPolicy;
    use ferrocat_icu::IcuNode;

    #[test]
    fn runtime_errors_point_into_original_unicode_input() {
        for input in ["don't {name", "é don't {name", "é don't\nother {name"] {
            let error =
                parse_icu_with_syntax_policy(input, IcuSyntaxPolicy::RuntimeLiteralApostrophes)
                    .expect_err("unterminated argument");
            assert_eq!(error.position.offset, input.len(), "{input}");
            let final_line = input.rsplit('\n').next().expect("line");
            assert_eq!(error.position.line, input.lines().count());
            assert_eq!(error.position.column, final_line.chars().count() + 1);
        }
    }

    #[test]
    fn remaps_offsets_after_apostrophe_escapes_and_quotes() {
        for input in [
            "don't {bad}",
            "é ''<x>' don't {bad}",
            "line\ndon't <tag>{bad}",
        ] {
            let canonical =
                canonicalize_icu_with_policy(input, IcuSyntaxPolicy::RuntimeLiteralApostrophes);
            let expected = input.find("{bad}").expect("marker");
            let canonical_offset = canonical.find("{bad}").expect("canonical marker");
            let error = ferrocat_icu::IcuParseError::syntax("marker", &canonical, canonical_offset);
            let mapped = super::remap_icu_error(error, input, &canonical);
            assert_eq!(mapped.position.offset, expected, "{input}");
        }
    }

    #[test]
    fn runtime_quotes_preserve_literals_and_real_tags() {
        let policy = IcuSyntaxPolicy::RuntimeLiteralApostrophes;
        for (input, expected) in [
            ("Use '<name>' here", "Use '<name>' here"),
            ("Use '<name>", "Use '<name>'"),
            ("'<a>it''s</a>'", "'<a>it''s</a>'"),
            ("don't '<x>'", "don''t '<x>'"),
            ("'< less'", "'< less'"),
            (
                "{n, plural, other {'<x>' #}}",
                "{n, plural, other {'<x>' #}}",
            ),
            ("<0>'<x>'</0>", "<0>'<x>'</0>"),
        ] {
            let canonical = canonicalize_icu_with_policy(input, policy);
            assert_eq!(canonical, expected, "{input}");
            assert_eq!(canonicalize_icu_with_policy(&canonical, policy), canonical);
            parse_icu_with_syntax_policy(input, policy).expect("valid runtime pattern");
        }
        let literal = parse_icu_with_syntax_policy("Use '<name>' here", policy).expect("literal");
        assert_eq!(
            literal.nodes,
            vec![IcuNode::Literal("Use <name> here".into())]
        );
        let plural = parse_icu_with_syntax_policy("{n, plural, other {'<x>' #}}", policy)
            .expect("plural with quoted tag");
        let IcuNode::Plural { options, .. } = &plural.nodes[0] else {
            panic!("expected plural");
        };
        assert!(
            options[0]
                .value
                .iter()
                .any(|node| matches!(node, IcuNode::Pound))
        );
        assert!(
            options[0]
                .value
                .iter()
                .any(|node| matches!(node, IcuNode::Literal(text) if text.contains("<x>")))
        );
        let real = parse_icu_with_syntax_policy("<link>{name}</link>", policy).expect("real tag");
        assert!(matches!(&real.nodes[0], IcuNode::Tag { name, .. } if name == "link"));
        assert_eq!(
            canonicalize_icu_with_policy("don't '<name>'", IcuSyntaxPolicy::Strict),
            "don't '<name>'"
        );
    }
}
