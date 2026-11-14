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
/// - a single apostrophe starts a quote only before `{`, `}`, or `#` inside a
///   plural or selectordinal branch,
/// - an unterminated runtime quote is closed at the end of the pattern.
///
/// [`IcuSyntaxPolicy::RuntimeLiteralApostrophesAndTags`] additionally starts a
/// quoted literal after an apostrophe before any `<`, including comparisons.
/// This is an explicit opt-in: existing runtime-policy text and keys can change.
///
/// The canonical form is idempotent. Strict-policy inputs and runtime-policy
/// inputs without apostrophes are returned as [`Cow::Borrowed`].
#[must_use]
pub fn canonicalize_icu_with_policy(input: &str, policy: IcuSyntaxPolicy) -> Cow<'_, str> {
    if policy == IcuSyntaxPolicy::Strict || !input.contains('\'') {
        return Cow::Borrowed(input);
    }

    Cow::Owned(canonicalize_runtime_apostrophes(
        input,
        policy == IcuSyntaxPolicy::RuntimeLiteralApostrophesAndTags,
    ))
}

pub(super) fn parse_icu_with_syntax_policy(
    input: &str,
    policy: IcuSyntaxPolicy,
) -> Result<IcuMessage, IcuParseError> {
    #[cfg(test)]
    ICU_PARSE_COUNT.set(ICU_PARSE_COUNT.get() + 1);

    let canonical = canonicalize_icu_with_policy(input, policy);
    parse_icu(&canonical)
}

fn canonicalize_runtime_apostrophes(input: &str, quote_tags: bool) -> String {
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
                    quote_tags,
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
    quote_tags: bool,
    output: &mut String,
) -> usize {
    let bytes = input.as_bytes();
    let next = bytes.get(index + 1).copied();

    if next == Some(b'\'') {
        output.push_str("''");
        return index + 2;
    }

    let starts_quote = matches!(next, Some(b'{') | Some(b'}'))
        || (plural_is_active && next == Some(b'#'))
        || (quote_tags && next == Some(b'<'));
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
    fn runtime_tag_quote_preserves_literal_tag_text() {
        let parsed = parse_icu_with_syntax_policy(
            "Use '<name>' here",
            IcuSyntaxPolicy::RuntimeLiteralApostrophesAndTags,
        )
        .expect("quoted tag text must parse");
        assert_eq!(
            parsed.nodes,
            vec![IcuNode::Literal("Use <name> here".to_owned())]
        );
    }

    #[test]
    fn tag_quoting_is_additive_and_preserves_real_tags() {
        let policy = IcuSyntaxPolicy::RuntimeLiteralApostrophesAndTags;
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
            parse_icu_with_syntax_policy(input, policy).expect("valid pattern");
        }
        let quoted = "'<0>Hi</0>'";
        let old = parse_icu_with_syntax_policy(quoted, IcuSyntaxPolicy::RuntimeLiteralApostrophes)
            .expect("old policy retains the tag");
        let new = parse_icu_with_syntax_policy(quoted, policy).expect("new policy quotes the tag");
        assert!(
            old.nodes
                .iter()
                .any(|node| matches!(node, IcuNode::Tag { .. }))
        );
        assert_eq!(new.nodes, vec![IcuNode::Literal("<0>Hi</0>".into())]);
        assert!(parse_icu_with_syntax_policy("<0>broken</1>", policy).is_err());
        assert_eq!(
            canonicalize_icu_with_policy(quoted, IcuSyntaxPolicy::Strict),
            quoted
        );
    }

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
