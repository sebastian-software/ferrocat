# ferrocat-icu

Compact, Rust-native rich-message parsing and diagnostics for `ferrocat`, based on ICU `MessageFormat` v1.

This crate is for messages that contain runtime values: placeholders, number/date/time formatting, plural logic, select branches, or rich-text tags. It helps catalog tooling understand those structures before translations ship.

Add it with:

```bash
cargo add ferrocat-icu
```

Use this crate when you want the ICU-specific surface directly:

- `parse_icu` / `parse_icu_with_options` for parsing
- `validate_icu` for lightweight validation
- `analyze_icu` for structured argument, formatter, plural, select, and tag summaries
- `compare_icu_messages` for source/translation compatibility diagnostics
- `validate_icu_formatter_support` for mapping consumer runtime support policies to diagnostics
- `validate_icu_formatter_support_from_analysis` when the caller already has an `IcuAnalysis` and wants to avoid a second message walk
- `normalize_message_metadata` for progressive source-side metadata around `msgid + msgctxt`
- `extract_argument_names` and `extract_tag_names` when tags should not be mixed with data arguments
- `extract_variables`, `has_plural`, `has_select`, and related helpers for AST inspection

```rust
use ferrocat_icu::{IcuCompatibilityOptions, compare_icu_messages, parse_icu};

fn main() -> Result<(), ferrocat_icu::IcuParseError> {
    let source = parse_icu("Hello {name}, you have {count, number, integer} files.")?;
    let translation = parse_icu("Hallo, du hast {count, number, integer} Dateien.")?;
    let report = compare_icu_messages(
        &source,
        &translation,
        &IcuCompatibilityOptions::default(),
    );

    assert!(report.has_errors());
    Ok(())
}
```

Semantic metadata stays small for simple messages and expands only when the
message or extractor knows more:

```rust
use ferrocat_icu::{
    MessageArgumentKind, MessageMetadataInput, normalize_message_metadata,
};

fn main() -> Result<(), ferrocat_icu::IcuParseError> {
    let metadata = normalize_message_metadata(MessageMetadataInput::new(
        "{count, plural, one {One item} other {# items}}",
    ))?;

    assert_eq!(
        metadata.args.get("count").map(|argument| argument.kind),
        Some(MessageArgumentKind::Number)
    );
    assert!(metadata.selectors.contains_key("count"));
    Ok(())
}
```

If you want the combined public entry point instead, use [`ferrocat`](https://docs.rs/ferrocat).

<!-- ferramenta-family:start -->
**ferrocat** is part of the [Ferramenta](https://ferramenta.dev) family — Rust-native developer tools that keep the APIs the ecosystem already knows.

Siblings: [ferroni](https://sebastian-software.github.io/ferroni/) · [ferriki](https://github.com/sebastian-software/ferriki) · [ferromark](https://sebastian-software.github.io/ferromark/) · [ferrolex](https://github.com/sebastian-software/ferrolex) · [palamedes](https://palamedes.dev) · [ferrovia](https://github.com/sebastian-software/ferrovia) · [ferralk](https://github.com/sebastian-software/ferralk) · [ferrugo](https://github.com/sebastian-software/ferrugo).
<!-- ferramenta-family:end -->
