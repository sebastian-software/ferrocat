# Ferrocat Fuzz Targets

This directory contains local `cargo-fuzz` harnesses for parser and catalog
entry points that accept untrusted catalog content.

The first target set is intentionally local-only. Scheduled CI fuzzing should
be added after these harnesses have produced stable, useful corpora.

Run a target with:

```bash
cargo fuzz run parse_po
```

Available targets:

- `parse_po`
- `parse_po_borrowed`
- `parse_catalog_po`
- `parse_catalog_ndjson`
- `parse_icu`
- `merge_catalog`
- `update_catalog`
