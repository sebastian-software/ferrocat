# ferrocat-po

Performance-first catalog parsing, serialization, merge helpers, release audits, and runtime compilation for `ferrocat`.

This crate contains the PO core plus the feature-gated catalog workflow layer. Use it when translation files need to be parsed, serialized, updated, reviewed, checked for release readiness, or compiled into application-facing payloads.

Add it with:

```bash
cargo add ferrocat-po
```

This crate covers both the low-level PO surface and the higher-level catalog layer. For PO-only dependency profiles, disable default features.

At the catalog layer, it supports three explicit modes:

- classic Gettext catalog mode: Gettext PO + gettext-compatible plurals
- ICU-native Gettext PO mode: Gettext PO + ICU MessageFormat
- ICU-native FCL catalog mode: FCL (Ferrocat Catalog Lines) storage + ICU MessageFormat

`FCL + gettext-compatible plurals` is intentionally unsupported.

See the repository's [FCL format specification](../../docs/fcl-format.md) for the line format and escaping rules behind ICU-native FCL storage.

Use this crate when you want that surface directly:

- `parse_po` / `parse_po_borrowed` for raw PO parsing
- `stringify_po` for PO serialization
- `merge_catalog` for lightweight catalog merges
- `parse_catalog`, `update_catalog`, and `NormalizedParsedCatalog::compile` for higher-level catalog workflows across Gettext PO and FCL storage
- `MachineMetadata` (integrity `lock` plus optional `AiProvenance`) and `machine_translation_hash` for marking machine-managed values in PO or FCL and detecting later by-hand edits
- `audit_catalogs` for read-only release QA across source and target catalogs
- `compile_catalog_artifact` for requested-locale runtime artifacts with fallback resolution and missing reports
- `compile_catalog_artifact_selected` for selected compiled-ID subsets of those runtime artifacts

`audit_catalogs` answers whether a catalog set is ready to ship without rewriting
anything. The default report checks completeness, target-only stale messages,
ICU syntax, ICU source/translation compatibility, semantic metadata conflicts,
and obsolete entries.

`parse_po_borrowed` keeps many fields as references into the input buffer and accepts the same LF, CRLF, and bare CR line-ending forms as the owned `parse_po` parser.

If you want the umbrella dependency instead, use [`ferrocat`](https://docs.rs/ferrocat).
