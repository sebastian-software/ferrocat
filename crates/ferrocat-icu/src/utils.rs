use std::collections::BTreeSet;

use crate::ast::{IcuMessage, IcuNode, IcuOption, IcuPluralKind};

/// Validates ICU `MessageFormat` input without returning the parsed AST.
///
/// # Errors
///
/// Returns [`crate::IcuParseError`] when the input is malformed.
pub fn validate_icu(input: &str) -> Result<(), crate::IcuParseError> {
    crate::parse_icu(input).map(|_| ())
}

/// Extracts variable names in first-seen order.
#[must_use]
pub fn extract_variables(message: &IcuMessage) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    visit_nodes(&message.nodes, &mut |name| {
        if seen.insert(name.to_owned()) {
            out.push(name.to_owned());
        }
    });
    out
}

/// Returns `true` when the message contains a cardinal plural expression.
#[must_use]
pub fn has_plural(message: &IcuMessage) -> bool {
    any_nodes(&message.nodes, &|node| {
        matches!(
            node,
            IcuNode::Plural {
                kind: IcuPluralKind::Cardinal,
                ..
            }
        )
    })
}

/// Returns `true` when the message contains a select expression.
#[must_use]
pub fn has_select(message: &IcuMessage) -> bool {
    any_nodes(&message.nodes, &|node| {
        matches!(node, IcuNode::Select { .. })
    })
}

/// Returns `true` when the message contains an ordinal plural expression.
#[must_use]
pub fn has_selectordinal(message: &IcuMessage) -> bool {
    any_nodes(&message.nodes, &|node| {
        matches!(
            node,
            IcuNode::Plural {
                kind: IcuPluralKind::Ordinal,
                ..
            }
        )
    })
}

/// Returns `true` when the message contains rich-text style tags.
#[must_use]
pub fn has_tag(message: &IcuMessage) -> bool {
    any_nodes(&message.nodes, &|node| matches!(node, IcuNode::Tag { .. }))
}

fn visit_nodes(nodes: &[IcuNode], visitor: &mut impl FnMut(&str)) {
    for node in nodes {
        match node {
            IcuNode::Literal(_) | IcuNode::Pound => {}
            IcuNode::Argument { name }
            | IcuNode::Number { name, .. }
            | IcuNode::Date { name, .. }
            | IcuNode::Time { name, .. }
            | IcuNode::List { name, .. }
            | IcuNode::Duration { name, .. }
            | IcuNode::Ago { name, .. }
            | IcuNode::Name { name, .. } => visitor(name),
            IcuNode::Select { name, options } | IcuNode::Plural { name, options, .. } => {
                visitor(name);
                visit_options(options, visitor);
            }
            IcuNode::Tag { name, children } => {
                visitor(name);
                visit_nodes(children, visitor);
            }
        }
    }
}

fn visit_options(options: &[IcuOption], visitor: &mut impl FnMut(&str)) {
    for option in options {
        visit_nodes(&option.value, visitor);
    }
}

fn any_nodes(nodes: &[IcuNode], predicate: &impl Fn(&IcuNode) -> bool) -> bool {
    nodes.iter().any(|node| match node {
        IcuNode::Select { options, .. } | IcuNode::Plural { options, .. } => {
            predicate(node)
                || options
                    .iter()
                    .any(|option| any_nodes(&option.value, predicate))
        }
        IcuNode::Tag { children, .. } => predicate(node) || any_nodes(children, predicate),
        _ => predicate(node),
    })
}

#[cfg(test)]
mod tests {
    use crate::{extract_variables, has_plural, has_select, has_selectordinal, has_tag, parse_icu};

    #[test]
    fn extracts_variables_in_first_seen_order() {
        let message = parse_icu(
            "{name} has {count, plural, one {{when, time, short}} other {{when, date, medium} in <link>{name}</link>}}",
        )
        .expect("parse");

        assert_eq!(
            extract_variables(&message),
            vec!["name", "count", "when", "link"]
        );
    }

    #[test]
    fn reports_structure_helpers() {
        let message = parse_icu(
            "{gender, select, male {{count, plural, one {<b>#</b>} other {# items}}} other {{n, selectordinal, one {#st} other {#th}}}}",
        )
        .expect("parse");

        assert!(has_select(&message));
        assert!(has_plural(&message));
        assert!(has_selectordinal(&message));
        assert!(has_tag(&message));
    }

    #[test]
    fn reports_absence_of_optional_structures() {
        let message = parse_icu("Hello {name}").expect("parse");
        assert_eq!(extract_variables(&message), vec!["name"]);
        assert!(!has_plural(&message));
        assert!(!has_select(&message));
        assert!(!has_selectordinal(&message));
        assert!(!has_tag(&message));
    }
}
