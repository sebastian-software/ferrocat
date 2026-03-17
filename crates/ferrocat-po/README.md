# ferrocat-po

Performance-first gettext PO parsing, serialization, merge helpers, and catalog workflows for `ferrocat`.

Add it with:

```bash
cargo add ferrocat-po
```

Use this crate when you want the PO-specific surface directly:

- `parse_po` / `parse_po_borrowed` for raw `.po` parsing
- `stringify_po` for serialization
- `merge_catalog` for lightweight gettext-style merges
- `parse_catalog`, `update_catalog`, and `NormalizedParsedCatalog::compile` for higher-level catalog workflows
- `compile_catalog_artifact` for requested-locale runtime artifacts with fallback resolution and missing reports

If you want the umbrella dependency instead, use [`ferrocat`](https://docs.rs/ferrocat).
