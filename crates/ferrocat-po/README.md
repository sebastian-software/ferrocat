# ferrocat-po

Performance-first Gettext PO parsing, serialization, merge helpers, and high-level catalog workflows for `ferrocat`.

Add it with:

```bash
cargo add ferrocat-po
```

This crate covers both the low-level Gettext PO surface and the higher-level catalog layer.

At the catalog layer, it supports three explicit modes:

- classic Gettext catalog mode: Gettext PO + gettext-compatible plurals
- ICU-native Gettext PO mode: Gettext PO + ICU MessageFormat
- ICU-native NDJSON catalog mode: NDJSON catalog storage + ICU MessageFormat

`NDJSON + gettext-compatible plurals` is intentionally unsupported.

Use this crate when you want that surface directly:

- `parse_po` / `parse_po_borrowed` for raw Gettext PO parsing
- `stringify_po` for Gettext PO serialization
- `merge_catalog` for lightweight gettext-style merges
- `parse_catalog`, `update_catalog`, and `NormalizedParsedCatalog::compile` for higher-level catalog workflows across Gettext PO and NDJSON storage
- `compile_catalog_artifact` for requested-locale runtime artifacts with fallback resolution and missing reports
- `compile_catalog_artifact_selected` for selected compiled-ID subsets of those runtime artifacts

If you want the umbrella dependency instead, use [`ferrocat`](https://docs.rs/ferrocat).
