use crate::{IcuMessage, IcuNode, IcuOption, IcuParseError, parse_icu, stringify_icu};

/// Options controlling ICU-aware pseudolocalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IcuPseudolocalizationOptions<'a> {
    /// Literal prefix inserted at the start of the message when absent.
    pub prefix: &'a str,
    /// Literal suffix inserted at the end of the message when absent.
    pub suffix: &'a str,
    /// Percentage of ASCII alphabetic literal characters to append as filler.
    pub expansion_percent: u8,
    /// Literal filler character used for expansion.
    pub expansion_char: char,
}

impl Default for IcuPseudolocalizationOptions<'_> {
    fn default() -> Self {
        Self {
            prefix: "[!! ",
            suffix: " !!]",
            expansion_percent: 30,
            expansion_char: '·',
        }
    }
}

impl<'a> IcuPseudolocalizationOptions<'a> {
    /// Creates pseudolocalization options with the default visible wrapper.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns options with the provided wrapper literals.
    #[must_use]
    pub const fn with_wrapper(mut self, prefix: &'a str, suffix: &'a str) -> Self {
        self.prefix = prefix;
        self.suffix = suffix;
        self
    }

    /// Returns options with the provided expansion percentage.
    #[must_use]
    pub const fn with_expansion_percent(mut self, expansion_percent: u8) -> Self {
        self.expansion_percent = expansion_percent;
        self
    }

    /// Returns options with the provided expansion filler character.
    #[must_use]
    pub const fn with_expansion_char(mut self, expansion_char: char) -> Self {
        self.expansion_char = expansion_char;
        self
    }
}

/// Pseudolocalizes an ICU message string while preserving ICU syntax.
///
/// Only literal text nodes are transformed. Argument names, formatter kinds and
/// styles, plural/select selectors, plural offsets, `#` placeholders, and tag
/// names are preserved.
///
/// # Errors
///
/// Returns [`IcuParseError`] when the input is not valid ICU MessageFormat for
/// Ferrocat's parser.
pub fn pseudolocalize_icu(
    input: &str,
    options: &IcuPseudolocalizationOptions<'_>,
) -> Result<String, IcuParseError> {
    let message = parse_icu(input)?;
    Ok(stringify_icu(&pseudolocalize_icu_message(
        &message, options,
    )))
}

/// Pseudolocalizes a parsed ICU message AST.
#[must_use]
pub fn pseudolocalize_icu_message(
    message: &IcuMessage,
    options: &IcuPseudolocalizationOptions<'_>,
) -> IcuMessage {
    let mut nodes = pseudolocalize_nodes(&message.nodes, options);
    apply_wrapper(&mut nodes, options);
    IcuMessage { nodes }
}

fn pseudolocalize_nodes(
    nodes: &[IcuNode],
    options: &IcuPseudolocalizationOptions<'_>,
) -> Vec<IcuNode> {
    nodes
        .iter()
        .map(|node| pseudolocalize_node(node, options))
        .collect()
}

fn pseudolocalize_node(node: &IcuNode, options: &IcuPseudolocalizationOptions<'_>) -> IcuNode {
    match node {
        IcuNode::Literal(value) => IcuNode::Literal(pseudolocalize_literal(value, options)),
        IcuNode::Argument { name } => IcuNode::Argument { name: name.clone() },
        IcuNode::Number { name, style } => IcuNode::Number {
            name: name.clone(),
            style: style.clone(),
        },
        IcuNode::Date { name, style } => IcuNode::Date {
            name: name.clone(),
            style: style.clone(),
        },
        IcuNode::Time { name, style } => IcuNode::Time {
            name: name.clone(),
            style: style.clone(),
        },
        IcuNode::List { name, style } => IcuNode::List {
            name: name.clone(),
            style: style.clone(),
        },
        IcuNode::Duration { name, style } => IcuNode::Duration {
            name: name.clone(),
            style: style.clone(),
        },
        IcuNode::Ago { name, style } => IcuNode::Ago {
            name: name.clone(),
            style: style.clone(),
        },
        IcuNode::Name { name, style } => IcuNode::Name {
            name: name.clone(),
            style: style.clone(),
        },
        IcuNode::Select {
            name,
            options: cases,
        } => IcuNode::Select {
            name: name.clone(),
            options: pseudolocalize_options(cases, options),
        },
        IcuNode::Plural {
            name,
            kind,
            offset,
            options: cases,
        } => IcuNode::Plural {
            name: name.clone(),
            kind: kind.clone(),
            offset: *offset,
            options: pseudolocalize_options(cases, options),
        },
        IcuNode::Pound => IcuNode::Pound,
        IcuNode::Tag { name, children } => IcuNode::Tag {
            name: name.clone(),
            children: pseudolocalize_nodes(children, options),
        },
    }
}

fn pseudolocalize_options(
    cases: &[IcuOption],
    options: &IcuPseudolocalizationOptions<'_>,
) -> Vec<IcuOption> {
    cases
        .iter()
        .map(|case| IcuOption {
            selector: case.selector.clone(),
            value: pseudolocalize_nodes(&case.value, options),
        })
        .collect()
}

fn apply_wrapper(nodes: &mut Vec<IcuNode>, options: &IcuPseudolocalizationOptions<'_>) {
    if !options.prefix.is_empty() && !starts_with_literal(nodes, options.prefix) {
        nodes.insert(0, IcuNode::Literal(options.prefix.to_owned()));
    }
    if !options.suffix.is_empty() && !ends_with_literal(nodes, options.suffix) {
        nodes.push(IcuNode::Literal(options.suffix.to_owned()));
    }
}

fn starts_with_literal(nodes: &[IcuNode], prefix: &str) -> bool {
    matches!(nodes.first(), Some(IcuNode::Literal(value)) if value.starts_with(prefix))
}

fn ends_with_literal(nodes: &[IcuNode], suffix: &str) -> bool {
    matches!(nodes.last(), Some(IcuNode::Literal(value)) if value.ends_with(suffix))
}

fn pseudolocalize_literal(value: &str, options: &IcuPseudolocalizationOptions<'_>) -> String {
    let ascii_letters = value.chars().filter(char::is_ascii_alphabetic).count();
    let mut out = String::with_capacity(value.len() + ascii_letters);
    for ch in value.chars() {
        out.push(accent_char(ch));
    }
    let filler_count = (ascii_letters * usize::from(options.expansion_percent)).div_ceil(100);
    for _ in 0..filler_count {
        out.push(options.expansion_char);
    }
    out
}

const fn accent_char(ch: char) -> char {
    match ch {
        'A' => 'À',
        'B' => 'Ɓ',
        'C' => 'Ç',
        'D' => 'Ð',
        'E' => 'É',
        'F' => 'Ƒ',
        'G' => 'Ğ',
        'H' => 'Ĥ',
        'I' => 'Ï',
        'J' => 'Ĵ',
        'K' => 'Ķ',
        'L' => 'Ļ',
        'M' => 'Ṁ',
        'N' => 'Ñ',
        'O' => 'Ö',
        'P' => 'Þ',
        'Q' => 'Q',
        'R' => 'Ŕ',
        'S' => 'Š',
        'T' => 'Ŧ',
        'U' => 'Û',
        'V' => 'Ṽ',
        'W' => 'Ŵ',
        'X' => 'Ẋ',
        'Y' => 'Ý',
        'Z' => 'Ž',
        'a' => 'à',
        'b' => 'ƀ',
        'c' => 'ç',
        'd' => 'ð',
        'e' => 'é',
        'f' => 'ƒ',
        'g' => 'ğ',
        'h' => 'ĥ',
        'i' => 'ï',
        'j' => 'ĵ',
        'k' => 'ķ',
        'l' => 'ļ',
        'm' => 'ṁ',
        'n' => 'ñ',
        'o' => 'ö',
        'p' => 'þ',
        'q' => 'q',
        'r' => 'ŕ',
        's' => 'š',
        't' => 'ŧ',
        'u' => 'û',
        'v' => 'ṽ',
        'w' => 'ŵ',
        'x' => 'ẋ',
        'y' => 'ý',
        'z' => 'ž',
        _ => ch,
    }
}

#[cfg(test)]
mod tests {
    use crate::{IcuPseudolocalizationOptions, parse_icu, pseudolocalize_icu, stringify_icu};

    fn no_expansion() -> IcuPseudolocalizationOptions<'static> {
        IcuPseudolocalizationOptions::new().with_expansion_percent(0)
    }

    #[test]
    fn pseudolocalize_icu_preserves_arguments() {
        let output = pseudolocalize_icu("Hello {name}", &no_expansion()).expect("pseudo");

        assert_eq!(output, "[!! Ĥéļļö {name} !!]");
    }

    #[test]
    fn pseudolocalize_icu_preserves_plural_structure() {
        let output = pseudolocalize_icu(
            "{count, plural, offset:1 =0 {No files} one {# file} other {# files}}",
            &no_expansion(),
        )
        .expect("pseudo");

        assert_eq!(
            output,
            "[!! {count, plural, offset:1 =0 {Ñö ƒïļéš} one {# ƒïļé} other {# ƒïļéš}} !!]"
        );
    }

    #[test]
    fn pseudolocalize_icu_preserves_tag_names() {
        let output =
            pseudolocalize_icu("Click <link>{name}</link>", &no_expansion()).expect("pseudo");

        assert_eq!(output, "[!! Çļïçķ <link>{name}</link> !!]");
    }

    #[test]
    fn pseudolocalize_icu_escapes_transformed_literal_text() {
        let output = pseudolocalize_icu("Use '<'tag'>' and '#'.", &no_expansion()).expect("pseudo");
        let reparsed = parse_icu(&output).expect("parse pseudolocalized");

        assert_eq!(stringify_icu(&reparsed), output);
    }

    #[test]
    fn pseudolocalize_icu_is_stable_when_applied_twice() {
        let options = IcuPseudolocalizationOptions::new();
        let once = pseudolocalize_icu("Hello {name}", &options).expect("first pseudo");
        let twice = pseudolocalize_icu(&once, &options).expect("second pseudo");

        assert_eq!(twice, once);
    }
}
