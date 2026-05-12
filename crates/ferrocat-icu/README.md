# ferrocat-icu

Compact, Rust-native ICU `MessageFormat` parsing primitives for `ferrocat`.

Add it with:

```bash
cargo add ferrocat-icu
```

Use this crate when you want the ICU-specific surface directly:

- `parse_icu` / `parse_icu_with_options` for parsing
- `validate_icu` for lightweight validation
- `analyze_icu` for structured argument, formatter, plural, select, and tag summaries
- `compare_icu_messages` for source/translation compatibility diagnostics
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

If you want the combined public entry point instead, use [`ferrocat`](https://docs.rs/ferrocat).
