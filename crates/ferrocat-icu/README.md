# ferrocat-icu

Compact, Rust-native ICU `MessageFormat` parsing primitives for `ferrocat`.

Add it with:

```bash
cargo add ferrocat-icu
```

Use this crate when you want the ICU-specific surface directly:

- `parse_icu` / `parse_icu_with_options` for parsing
- `validate_icu` for lightweight validation
- `extract_variables`, `has_plural`, `has_select`, and related helpers for AST inspection

If you want the combined public entry point instead, use [`ferrocat`](https://docs.rs/ferrocat).
