# Ferrocat Fuzz Targets

This directory contains local `cargo-fuzz` harnesses for parser and catalog
entry points that accept untrusted catalog content.

The target set runs locally and in the scheduled GitHub Actions fuzz workflow.
CI uses short, bounded libFuzzer runs so parser panics, timeouts, OOMs, and
crashes become a stability signal without turning every pull request into a
long-running fuzz campaign.

Install the local runner with:

```bash
rustup toolchain install nightly
cargo install cargo-fuzz
```

Run a target with:

```bash
cargo +nightly fuzz run parse_po fuzz/seed-corpus/po -- -max_total_time=60
```

Available targets:

- `parse_po`
- `parse_po_borrowed`
- `parse_catalog_po`
- `parse_catalog_ndjson`
- `parse_icu`
- `merge_catalog`
- `update_catalog`

The checked-in seed corpus lives under `fuzz/seed-corpus/`. Generated corpus
growth remains ignored under `fuzz/corpus/`; promote only reduced, stable crash
inputs or especially useful fixtures into the seed corpus.

When fuzzing finds a crash:

1. Reproduce it locally with the failing target and artifact path from the CI
   artifact.
2. Minimize the input with `cargo +nightly fuzz tmin <target> <artifact>`.
3. Add the minimized input as a regression test or seed-corpus fixture.
4. Keep the new fixture small enough to review in a normal pull request.
