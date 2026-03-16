# External Benchmarking

`ferrocat-bench` now exposes a manual, reproducible comparison suite for external baselines.

The internal microbenchmarks remain the fast day-to-day performance loop. The external comparison suite is for serious cross-runtime checkpoints on a documented reference host.

## Reference Host Rules

- use one documented benchmark machine for official comparisons
- keep Rust, Node, Python, and GNU gettext versions fixed across report runs
- minimize background load and network activity during a run
- keep the machine on AC power
- compare reports only within the same host and toolchain class

## Required Tooling

- Rust toolchain able to run `cargo run -p ferrocat-bench`
- Node.js plus the packages declared in `benchmark/node/package.json`
- Python 3 plus the packages declared in `benchmark/python/requirements.txt`
- GNU gettext commands `msgcat` and `msgmerge`

Suggested setup:

```bash
./benchmark/setup.sh
```

If `benchmark/python/.venv` exists, `ferrocat-bench` will automatically prefer that interpreter for `verify-benchmark-env` and `compare`, so `polib` does not need to be installed globally.

If you only want the Python side, run:

```bash
./benchmark/python/setup.sh
```

## Verify The Environment

Run:

```bash
cargo run -p ferrocat-bench -- verify-benchmark-env
```

This checks the required executables and adapter packages and prints the detected tool versions that will be captured in the report metadata.

## Benchmark Profiles

- `gettext-compat-v1`
  - official external benchmark suite
  - gettext-only realistic fixtures with mixed plural-rule locales
  - uses a conservative support matrix across `polib`, `pofile`, and `msgcat`
- `gettext-workflows-v1`
  - official external workflow suite for classic gettext merge/update paths
  - currently compares `merge_catalog` and `update_catalog` against `msgmerge`
  - intentionally limited to the conservative `gettext-ui-de-*` corpus where semantics match
- `serious-v1`
  - advanced/internal benchmark suite
  - mixed and ICU-heavy workloads
  - useful for `ferrocat`'s broader performance direction, but not the official cross-tool gettext baseline

## Run The Official Gettext Suite

Use the checked-in `gettext-compat-v1` profile and write the report outside the internal performance history:

```bash
cargo run --release -p ferrocat-bench -- compare gettext-compat-v1 --out benchmark/results/gettext-compat-v1-$(date +%Y%m%d-%H%M%S).json
```

The compare command:

- validates semantic equivalence for each comparison group before timing
- calibrates iterations per scenario to a minimum sample duration
- runs 2 warmups per scenario
- records 10 measured samples per parse/stringify scenario
- stores raw samples plus aggregated statistics in JSON

For GNU gettext CLI scenarios, the report additionally records an `empty-cli-run` baseline using a minimal header-only input. This adds:

- `baseline_elapsed_ns` and adjusted sample fields for `msgcat` / `msgmerge`
- adjusted median statistics alongside the raw end-to-end statistics

The raw timing remains the primary comparison number. The adjusted timing is a secondary estimate for understanding how much of the CLI measurement is fixed overhead versus actual fixture work.

For the workflow-oriented suite:

```bash
cargo run --release -p ferrocat-bench -- compare gettext-workflows-v1 --out benchmark/results/gettext-workflows-v1-$(date +%Y%m%d-%H%M%S).json
```

That profile covers:

- `merge_catalog` versus `msgmerge`
- `update_catalog` versus a `msgmerge`-style external workflow baseline

## Result Storage

- Internal microbenchmark history stays in `docs/performance-history.md`
- External comparison reports should be written under `benchmark/results/`
- Do not copy external compare results into the internal performance history tables

## Current `gettext-compat-v1` Coverage

- `gettext-ui-de-*`
- `gettext-commerce-pl-*`
- `gettext-saas-fr-*`
- `gettext-content-ar-*`

External baselines currently wired:

- `polib` and `pofile` on the most conservative parse/stringify corpus: `gettext-ui-de-*`
- `msgcat` on the broader gettext stringify corpora, including plural-heavier locales
- `msgmerge` on the conservative gettext workflow corpus for merge/update comparisons
- `ferrocat` internal owned vs borrowed parse baselines across the wider locale mix

This is intentional. The fixture families are all classic gettext, but the official matrix only includes a tool where the pre-validation step confirms the same normalized semantics. The advanced `mixed-*` and ICU-heavy corpora remain separate from the official gettext comparison track.
