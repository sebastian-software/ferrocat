# ferrocat-po

Performance-first catalog parsing, serialization, merge helpers, release audits, and runtime compilation for `ferrocat`.

This crate is the catalog-workflow layer. Use it when translation files need to be updated, reviewed, checked for release readiness, or compiled into application-facing payloads.

Add it with:

```bash
cargo add ferrocat-po
```

This crate covers both the low-level PO surface and the higher-level catalog layer.

At the catalog layer, it supports three explicit modes:

- classic Gettext catalog mode: Gettext PO + gettext-compatible plurals
- ICU-native Gettext PO mode: Gettext PO + ICU MessageFormat
- ICU-native NDJSON catalog mode: NDJSON catalog storage + ICU MessageFormat

`NDJSON + gettext-compatible plurals` is intentionally unsupported.

Use this crate when you want that surface directly:

- `parse_po` / `parse_po_borrowed` for raw PO parsing
- `stringify_po` for PO serialization
- `merge_catalog` for lightweight catalog merges
- `parse_catalog`, `update_catalog`, and `NormalizedParsedCatalog::compile` for higher-level catalog workflows across Gettext PO and NDJSON storage
- `MachineTranslationMetadata` and `machine_translation_hash` for AI translation metadata that can be preserved in PO or NDJSON and removed when the translated text changes
- `audit_catalogs` for read-only release QA across source and target catalogs
- `compile_catalog_artifact` for requested-locale runtime artifacts with fallback resolution and missing reports
- `compile_catalog_artifact_selected` for selected compiled-ID subsets of those runtime artifacts

`audit_catalogs` answers whether a catalog set is ready to ship without rewriting
anything. The default report checks completeness, target-only stale messages,
ICU syntax, ICU source/translation compatibility, semantic metadata conflicts,
obsolete entries, and visible `fuzzy` flags.

`parse_po_borrowed` keeps many fields as references into the input buffer and accepts the same LF, CRLF, and bare CR line-ending forms as the owned `parse_po` parser.

If you want the umbrella dependency instead, use [`ferrocat`](https://docs.rs/ferrocat).
