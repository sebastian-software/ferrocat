# ferrocat

[![codecov](https://codecov.io/github/sebastian-software/ferrocat/graph/badge.svg?branch=main)](https://app.codecov.io/github/sebastian-software/ferrocat)

`ferrocat` is a Rust-first gettext and ICU toolkit built around predictable performance, explicit crate boundaries, and source-attributed compatibility coverage.

The public entry point is the `ferrocat` crate. It re-exports the stable Rust surface from the lower-level workspace crates:

- `ferrocat`: umbrella crate and recommended dependency for application code
- `ferrocat-po`: PO parsing, serialization, merge helpers, and catalog update APIs
- `ferrocat-icu`: ICU MessageFormat parsing and structural helpers
- `ferrocat-bench`: workspace-only benchmark harness
- `ferrocat-conformance`: workspace-only upstream-derived conformance fixtures

## Installation

```bash
cargo add ferrocat
```

If you want a narrower dependency, `ferrocat-po` and `ferrocat-icu` remain publishable secondary crates.

## Why `ferrocat`

Most PO tooling still makes at least one uncomfortable tradeoff:

- compatibility with legacy gettext workflows comes first, modern semantics second
- convenience wins over predictable allocation behavior and hot-path efficiency
- tests exist, but they are not tied back to independently maintained upstream suites

`ferrocat` aims to be stronger on all three axes:

- Rust-native implementation instead of line-by-line translation
- performance-first parser and serializer architecture
- compatibility measured against real upstream behavior

At the catalog layer, `ferrocat` treats ICU-style structure as the long-term semantic model and gettext as the compatibility bridge.

## Example

```rust
use ferrocat::{SerializeOptions, parse_po, stringify_po};

let mut file = parse_po(
    r#"
msgid "hello"
msgstr "world"
"#,
)?;

file.items[0].msgstr = "Welt".to_owned().into();

let rendered = stringify_po(&file, &SerializeOptions::default());
assert!(rendered.contains(r#"msgstr "Welt""#));
```

For the common "merge fresh extracted messages into an existing catalog" workflow:

```rust
use std::borrow::Cow;

use ferrocat::{MergeExtractedMessage, merge_catalog};

let updated = merge_catalog(
    existing_po,
    &[MergeExtractedMessage {
        msgid: Cow::Borrowed("hello"),
        references: vec![Cow::Borrowed("src/app.rs:10")],
        ..MergeExtractedMessage::default()
    }],
)?;
```

## Current Highlights

- owned parse via `parse_po`
- borrowed parse via `parse_po_borrowed`
- serialization via `stringify_po`
- merge workflow via `merge_catalog`
- high-level catalog APIs via `parse_catalog`, `update_catalog`, and `update_catalog_file`
- ICU parsing via `parse_icu` plus helpers such as `extract_variables`
- conservative gettext compatibility with diagnostics where ambiguity matters

The borrowed parser exists because real PO workflows are often read-heavy and transformation-heavy. In those paths, avoiding unnecessary allocation is the difference between a pleasant API and a scalable one.

## Conformance

`ferrocat` includes a hermetic, source-attributed conformance snapshot under [`conformance`](conformance).

As of 2026-03-16, the current scoreboard is:

- `55` upstream-derived conformance cases
- `442` concrete assertions
- `50` expected passes
- `5` expected rejects
- `0` `known_gap`

Run the full verification with:

```bash
cargo test --workspace
cargo run -p ferrocat-bench -- conformance-report
```

`ferrocat-po` intentionally normalizes headerless PO files on write by emitting an explicit empty header entry. That behavior is documented and is not counted as a compatibility gap.

## Test Coverage

Coverage reporting is wired through workspace-local Cargo aliases backed by `cargo-llvm-cov`.

Useful commands:

```bash
cargo coverage-summary
cargo coverage
cargo coverage-lcov
```

The coverage setup focuses on `ferrocat`, `ferrocat-po`, and `ferrocat-icu`, while excluding the workspace-only benchmark and conformance crates.

See [docs/test-coverage.md](docs/test-coverage.md) for local setup, Codecov wiring, and artifact locations.

## Benchmarks And Profiling

Useful benchmark commands:

```bash
cargo run --release -p ferrocat-bench -- parse mixed-10000 200
cargo run --release -p ferrocat-bench -- parse-borrowed mixed-10000 200
cargo run --release -p ferrocat-bench -- stringify mixed-10000 200
cargo run --release -p ferrocat-bench -- merge mixed-10000 100
cargo run -p ferrocat-bench -- describe mixed-1000
cargo run -p ferrocat-bench -- describe gettext-ui-de-1000
cargo run -p ferrocat-bench -- verify-benchmark-env
cargo run --release -p ferrocat-bench -- compare gettext-compat-v1 --out benchmark/results/gettext-compat-v1.json
```

Historical benchmark results live in [docs/performance-history.md](docs/performance-history.md).

The manual external comparison suite, including the official gettext-only benchmark profile and reference-host rules, is documented in [docs/benchmarking.md](docs/benchmarking.md).

The official `gettext-compat-v1` profile uses a conservative support matrix: `polib` and `pofile` are only included where the generated classic gettext corpus validates to the same normalized semantics, while broader plural-heavy scenarios are still benchmarked via `ferrocat` and GNU gettext tooling.

For workflow-style benchmarking there is now also a separate `gettext-workflows-v1` profile, which compares `merge_catalog` and `update_catalog` against a conservative `msgmerge` baseline on the `gettext-ui-de-*` corpus.

Current external gettext snapshot from [benchmark/results/gettext-compat-v1-first-run.json](benchmark/results/gettext-compat-v1-first-run.json):

The important number is throughput, not `median-ms`. The compare runner calibrates each sample to roughly the same wall-clock duration, so `median-ms` is mainly useful inside one scenario run. For cross-tool reading, compare `items/s`.

### Parse winners

| Fixture | Compared implementations | Fastest | Throughput |
|---|---|---:|---:|
| `gettext-ui-de-1000` | `ferrocat`, `ferrocat-borrowed`, `pofile`, `polib` | **`ferrocat`** | **1.47M items/s** |
| `gettext-ui-de-10000` | `ferrocat`, `ferrocat-borrowed`, `pofile`, `polib` | **`ferrocat`** | **1.36M items/s** |
| `gettext-saas-fr-1000` | `ferrocat`, `ferrocat-borrowed` | **`ferrocat`** | **1.41M items/s** |
| `gettext-saas-fr-10000` | `ferrocat`, `ferrocat-borrowed` | **`ferrocat`** | **1.33M items/s** |
| `gettext-commerce-pl-1000` | `ferrocat`, `ferrocat-borrowed` | **`ferrocat`** | **1.39M items/s** |
| `gettext-commerce-pl-10000` | `ferrocat`, `ferrocat-borrowed` | **`ferrocat`** | **1.31M items/s** |
| `gettext-content-ar-1000` | `ferrocat`, `ferrocat-borrowed` | **`ferrocat`** | **1.19M items/s** |
| `gettext-content-ar-10000` | `ferrocat`, `ferrocat-borrowed` | **`ferrocat`** | **1.11M items/s** |

For the conservative UI/de corpus with external parsers included, the first run came out like this:

| Fixture | `ferrocat` | `ferrocat-borrowed` | `pofile` | `polib` |
|---|---:|---:|---:|---:|
| `gettext-ui-de-1000` | **1.47M** | 1.41M | 266k | 59k |
| `gettext-ui-de-10000` | **1.36M** | 1.30M | 11.9k | 59k |

### Stringify winners

| Fixture | Compared implementations | Fastest | Throughput |
|---|---|---:|---:|
| `gettext-ui-de-1000` | `ferrocat`, `pofile`, `polib`, `msgcat` | **`ferrocat`** | **6.38M items/s** |
| `gettext-ui-de-10000` | `ferrocat`, `pofile`, `polib`, `msgcat` | **`ferrocat`** | **6.02M items/s** |
| `gettext-saas-fr-1000` | `ferrocat`, `msgcat` | **`ferrocat`** | **6.32M items/s** |
| `gettext-saas-fr-10000` | `ferrocat`, `msgcat` | **`ferrocat`** | **5.93M items/s** |
| `gettext-commerce-pl-1000` | `ferrocat`, `msgcat` | **`ferrocat`** | **6.82M items/s** |
| `gettext-commerce-pl-10000` | `ferrocat`, `msgcat` | **`ferrocat`** | **6.37M items/s** |
| `gettext-content-ar-1000` | `ferrocat`, `msgcat` | **`ferrocat`** | **4.92M items/s** |
| `gettext-content-ar-10000` | `ferrocat`, `msgcat` | **`ferrocat`** | **4.64M items/s** |

For the conservative UI/de corpus with all classic stringify baselines included:

| Fixture | `ferrocat` | `pofile` | `polib` | `msgcat` |
|---|---:|---:|---:|---:|
| `gettext-ui-de-1000` | **6.38M** | 730k | 100k | 25k |
| `gettext-ui-de-10000` | **6.02M** | 679k | 99k | 30k |

So the first official gettext benchmark does not show `pofile` as the overall winner. `pofile` is very quick on some calibrated sample medians, but once you normalize by work done per sample, `ferrocat` is currently fastest in every official comparison group of this run.

For profiling on macOS:

```bash
cargo instruments --no-open -t "Time Profiler" --bin ferrocat-bench -- parse mixed-10000 2000
```

## Project Docs

- [docs/conformance.md](docs/conformance.md)
- [docs/performance-history.md](docs/performance-history.md)
- [docs/benchmarking.md](docs/benchmarking.md)
- [docs/benchmark-fixtures.md](docs/benchmark-fixtures.md)
- [docs/release-verification.md](docs/release-verification.md)
- [docs/test-coverage.md](docs/test-coverage.md)
- [docs/notes/2026-03-14-scan-architecture.md](docs/notes/2026-03-14-scan-architecture.md)
- [docs/adr](docs/adr)

Contribution conventions, including Conventional Commits, are documented in [CONTRIBUTING.md](CONTRIBUTING.md).
