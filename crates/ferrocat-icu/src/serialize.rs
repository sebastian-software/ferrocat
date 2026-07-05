use crate::{IcuMessage, IcuNode, IcuPluralKind};

/// Renders a parsed ICU message AST back to ICU `MessageFormat` text.
///
/// The output is canonical rather than byte-preserving: insignificant
/// whitespace and quote choices may differ from the source input, but parsing
/// the rendered string yields the same AST for the currently supported node
/// variants.
#[must_use]
pub fn stringify_icu(message: &IcuMessage) -> String {
    let mut out = String::new();
    render_nodes(&message.nodes, &mut out);
    out
}

fn render_nodes(nodes: &[IcuNode], out: &mut String) {
    for node in nodes {
        render_node(node, out);
    }
}

fn render_node(node: &IcuNode, out: &mut String) {
    match node {
        IcuNode::Literal(value) => append_escaped_literal(out, value),
        IcuNode::Argument { name } => {
            out.push('{');
            out.push_str(name);
            out.push('}');
        }
        IcuNode::Number { name, style } => render_formatter("number", name, style.as_deref(), out),
        IcuNode::Date { name, style } => render_formatter("date", name, style.as_deref(), out),
        IcuNode::Time { name, style } => render_formatter("time", name, style.as_deref(), out),
        IcuNode::List { name, style } => render_formatter("list", name, style.as_deref(), out),
        IcuNode::Duration { name, style } => {
            render_formatter("duration", name, style.as_deref(), out);
        }
        IcuNode::Ago { name, style } => render_formatter("ago", name, style.as_deref(), out),
        IcuNode::Name { name, style } => render_formatter("name", name, style.as_deref(), out),
        IcuNode::Select { name, options } => {
            out.push('{');
            out.push_str(name);
            out.push_str(", select, ");
            render_options(options, out);
            out.push('}');
        }
        IcuNode::Plural {
            name,
            kind,
            offset,
            options,
        } => {
            out.push('{');
            out.push_str(name);
            out.push_str(match kind {
                IcuPluralKind::Cardinal => ", plural, ",
                IcuPluralKind::Ordinal => ", selectordinal, ",
            });
            if *offset > 0 {
                out.push_str("offset:");
                out.push_str(&offset.to_string());
                out.push(' ');
            }
            render_options(options, out);
            out.push('}');
        }
        IcuNode::Pound => out.push('#'),
        IcuNode::Tag { name, children } => {
            out.push('<');
            out.push_str(name);
            if children.is_empty() {
                out.push_str("/>");
            } else {
                out.push('>');
                render_nodes(children, out);
                out.push_str("</");
                out.push_str(name);
                out.push('>');
            }
        }
    }
}

fn render_options(options: &[crate::IcuOption], out: &mut String) {
    for (index, option) in options.iter().enumerate() {
        if index > 0 {
            out.push(' ');
        }
        out.push_str(&option.selector);
        out.push_str(" {");
        render_nodes(&option.value, out);
        out.push('}');
    }
}

fn render_formatter(kind: &str, name: &str, style: Option<&str>, out: &mut String) {
    out.push('{');
    out.push_str(name);
    out.push_str(", ");
    out.push_str(kind);
    if let Some(style) = style {
        out.push_str(", ");
        out.push_str(style);
    }
    out.push('}');
}

fn append_escaped_literal(out: &mut String, value: &str) {
    if !value
        .as_bytes()
        .iter()
        .any(|byte| matches!(byte, b'\'' | b'{' | b'}' | b'#' | b'<' | b'>'))
    {
        out.push_str(value);
        return;
    }

    for ch in value.chars() {
        match ch {
            '\'' => out.push_str("''"),
            '{' | '}' | '#' | '<' | '>' => {
                out.push('\'');
                out.push(ch);
                out.push('\'');
            }
            _ => out.push(ch),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{parse_icu, stringify_icu};

    fn assert_roundtrip(input: &str) {
        let parsed = parse_icu(input).expect("parse source");
        let rendered = stringify_icu(&parsed);
        let reparsed = parse_icu(&rendered).expect("parse rendered");
        assert_eq!(reparsed, parsed, "rendered: {rendered}");
    }

    #[test]
    fn roundtrips_literal_escaping_arguments_and_formatters() {
        assert_roundtrip("Use '<'tag'>' with '{'braces'}', ''quotes'', and '#'.");
        assert_roundtrip(
            "{name} {count, number, integer} {created, date, short} {created, time, ::HHmm}",
        );
        assert_roundtrip("{items, list, conjunction} {elapsed, duration} {age, ago} {owner, name}");
    }

    #[test]
    fn roundtrips_select_plural_pound_and_tags() {
        assert_roundtrip(
            "{gender, select, female {She liked <b>{thing}</b>} male {He liked <b>{thing}</b>} other {They liked <b>{thing}</b>}}",
        );
        assert_roundtrip(
            "{count, plural, offset:1 =0 {No items} one {# item} other {# items and '#'}}",
        );
        assert_roundtrip(
            "{rank, selectordinal, one {#st place} two {#nd place} few {#rd place} other {#th place}}",
        );
    }

    #[test]
    fn uses_canonical_spacing_for_rendered_messages() {
        let parsed = parse_icu("{count,plural,one{# item} other{# items}}").expect("parse");

        assert_eq!(
            stringify_icu(&parsed),
            "{count, plural, one {# item} other {# items}}"
        );
    }
}
