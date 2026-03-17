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

- `gettext-official-v1`
  - the smallest official benchmark profile
  - intentionally benchmark-focused rather than test-focused
  - one conservative primary locale: `de`
  - one second normal locale: `fr`
  - one more complex plural locale: `pl`
  - one representative large corpus size per scenario
- `gettext-official-quick-v1`
  - the fast companion to `gettext-official-v1`
  - keeps the same fixture and external-tool matrix
  - lowers the minimum sample duration
  - uses fewer warmup and measured runs
  - useful for local iteration and regression checks, but not the publication-grade profile
- `gettext-compat-v1`
  - extended external benchmark suite
  - broader gettext-only matrix with additional locale/family coverage
  - useful when you want more detail than the slim official profile
- `gettext-workflows-v1`
  - focused workflow suite for classic gettext merge/update paths
  - compares `merge_catalog` and `update_catalog` against `msgmerge`
  - kept separate from the slim official profile so workflow tuning does not dominate the main benchmark story
- `gettext-workflows-ecosystem-v1`
  - extended workflow suite for classic gettext merge/update paths
  - compares `merge_catalog` and `update_catalog` against `msgmerge`, `pofile`, `pofile-ts`, and `polib`
  - external library numbers are measured as reconstructed parse -> merge -> serialize pipelines
  - useful when you want workflow numbers across the broader gettext ecosystem
- `serious-v1`
  - advanced/internal benchmark suite
  - mixed and ICU-heavy workloads
  - useful for `ferrocat`'s broader performance direction, but not the official cross-tool gettext baseline

## Run The Official Gettext Suite

Use the checked-in `gettext-official-v1` profile and write the report outside the internal performance history:

```bash
cargo run --release -p ferrocat-bench -- compare gettext-official-v1 --out benchmark/results/gettext-official-v1-$(date +%Y%m%d-%H%M%S).json
```

The compare command:

- validates semantic equivalence for each comparison group before timing
- calibrates iterations per scenario to a minimum sample duration
- runs 2 warmups per scenario
- records 10 measured samples per parse/stringify scenario
- stores raw samples plus aggregated statistics in JSON

For a quicker checkpoint with the same comparison matrix:

```bash
cargo run --release -p ferrocat-bench -- compare gettext-official-quick-v1 --out benchmark/results/gettext-official-quick-v1-$(date +%Y%m%d-%H%M%S).json
```

That profile currently uses:

- `minimum_sample_millis: 100`
- 1 warmup and 3 measured samples for parse/stringify scenarios
- 1 warmup and 2 measured samples for workflow scenarios

Use it for faster day-to-day checks. Keep `gettext-official-v1` as the primary report for published comparisons.

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

For the broader workflow ecosystem suite:

```bash
cargo run --release -p ferrocat-bench -- compare gettext-workflows-ecosystem-v1 --out benchmark/results/gettext-workflows-ecosystem-v1-$(date +%Y%m%d-%H%M%S).json
```

That profile extends the workflow comparison with:

- `pofile`
- `pofile-ts`
- `polib`

These are measured as fair reconstructed workflows using each library's parse and stringify APIs around the same extracted-message merge step.

For the broader compatibility/detail suite:

```bash
cargo run --release -p ferrocat-bench -- compare gettext-compat-v1 --out benchmark/results/gettext-compat-v1-$(date +%Y%m%d-%H%M%S).json
```

Use this when you want more fixture variety than the slim official profile provides.

## Result Storage

- Internal microbenchmark history stays in `docs/performance-history.md`
- External comparison reports should be written under `benchmark/results/`
- Do not copy external compare results into the internal performance history tables

## Current `gettext-official-v1` Shape

- `gettext-ui-de-10000`
- `gettext-saas-fr-10000`
- `gettext-commerce-pl-10000`

External baselines currently wired:

- `polib`, `pofile`, `pofile-ts`, and `gettext-parser` on the classic gettext parse/stringify corpora: `gettext-ui-de-10000`, `gettext-saas-fr-10000`, `gettext-commerce-pl-10000`
- `msgcat` on stringify comparisons
- `msgmerge` on the conservative workflow corpus
- `ferrocat` internal owned vs borrowed parse baselines on `de`, `fr`, and `pl`

Workflow-only baselines currently wired:

- `pofile`, `pofile-ts`, and `polib` on `gettext-workflows-ecosystem-v1`
- each measured as parse -> merge -> serialize pipelines on `gettext-ui-de-1000` and `gettext-ui-de-10000`
- `gettext-parser` is intentionally excluded from workflow benchmarking for now because its PO compile/parse path does not preserve obsolete entries in a way that is semantically fair for `msgmerge`-style workflows

This is intentional. The official profile is meant to answer the small, understandable benchmark question first. The broader `gettext-compat-v1` profile is still available when you want more detail, and the advanced `mixed-*` / ICU-heavy corpora remain separate from the official gettext comparison track.

## Reporting Expectations

When you share benchmark results from the external suite, include the environment block from the JSON report together with the throughput table. At minimum, keep these fields visible:

- `host_identifier`
- `os`
- `cpu_model`
- `rustc_version`
- `node_version`
- `python_version`
- `msgcat_version` / `msgmerge_version` when GNU gettext numbers are shown

This keeps published numbers tied to the machine and toolchain they were measured on.
