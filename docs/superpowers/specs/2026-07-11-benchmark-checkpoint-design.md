# Benchmark checkpoint and serious-profile repair

## Goal

Produce a current, reproducible publication checkpoint for Ferrocat's external
benchmark claims, while restoring the blocked `serious-v1` profile.

## Scope

- Treat `mixed-*` merge fixtures as `CatalogMode::GettextPo` when the benchmark
  harness calls `update_catalog`.
- Add a regression test covering the mixed-fixture catalog-mode selection.
- Re-run `serious-v1` after the harness repair.
- Commit the current external comparison reports for `gettext-official-v1`,
  `gettext-workflows-ecosystem-v1`, `gettext-compat-v1`, and
  `catalog-update-v1` under `benchmark/results/`.
- Refresh public benchmark wording and numbers only from stable, non-noisy
  scenarios in those reports.

## Non-goals

- Do not commit the Rust-only scheduled report or the internal storage-format
  report as publication checkpoints.
- Do not publish any scenario marked noisy as a headline benchmark number.
- Do not change catalog-update behavior outside benchmark fixture mode
  selection.

## Design

`fixture_catalog_mode` already selects `GettextPo` for `gettext-*` fixtures.
Extend that selection to `mixed-*`, whose generated PO data uses classic
gettext plural slots. This makes the harness select the matching update mode;
it does not change Ferrocat's product API or catalog semantics.

The regression test asserts that a `mixed-*` fixture resolves to `GettextPo`
and an ICU catalog fixture remains `IcuPo`.

The report commit records the exact current branch SHA and environment metadata
already embedded in the JSON. The README and performance documentation retain
the existing methodology and cite only non-noisy rows from the new reports.

## Verification

- Run the focused benchmark-harness tests and workspace formatting/lint checks.
- Run `serious-v1` in release mode after the fix.
- Verify each committed JSON report has the current head SHA and a complete
  scenario list.
- Re-run the published-number consistency check and docs build after updating
  public benchmark claims.
