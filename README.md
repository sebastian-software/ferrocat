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

## API Overview

The current public surface falls into three practical layers:

| Layer | Functions | Use when you want to... |
|---|---|---|
| PO core | `parse_po`, `parse_po_borrowed`, `stringify_po` | parse and write classic `.po` files directly |
| Catalog workflows | `merge_catalog` | do the lean gettext-style merge step against fresh extracted messages |
| Catalog workflows | `parse_catalog` | read a `.po` file into the higher-level canonical catalog model |
| Catalog workflows | `update_catalog` | run the full in-memory catalog update flow with headers, plurals, sorting, and diagnostics |
| Catalog workflows | `update_catalog_file` | run the same full update flow directly against a file on disk |
| ICU | `parse_icu`, `validate_icu`, `extract_variables` | parse or inspect ICU MessageFormat structure |

See [docs/api-overview.md](docs/api-overview.md) for the fuller "what should I use when?" guide.

Across all of these layers, `ferrocat` keeps a conservative gettext-compatibility stance and surfaces diagnostics where ambiguity matters.

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
cargo run --release -p ferrocat-bench -- compare gettext-official-v1 --out benchmark/results/gettext-official-v1.json
cargo run --release -p ferrocat-bench -- compare gettext-official-quick-v1 --out benchmark/results/gettext-official-quick-v1.json
```

Historical benchmark results live in [docs/performance-history.md](docs/performance-history.md).

The manual external comparison suite, including the official gettext-only benchmark profile and reference-host rules, is documented in [docs/benchmarking.md](docs/benchmarking.md).

The smallest official benchmark profile is `gettext-official-v1`: one conservative main locale (`de`), one second normal locale (`fr`), one more complex plural locale (`pl`), and one representative larger corpus size per scenario. Broader profiles still exist for deeper analysis, but the main benchmark story now stays intentionally small.

For quicker day-to-day checks there is also `gettext-official-quick-v1`. It keeps the same fixture and tool matrix, but uses fewer warmups, fewer measured samples, and a lower minimum sample duration. That makes it useful as a fast regression check while `gettext-official-v1` stays the publication-grade profile.

For workflow-style benchmarking there is now also a separate `gettext-workflows-v1` profile, which compares `merge_catalog` and `update_catalog` against a conservative `msgmerge` baseline on the `gettext-ui-de-*` corpus.

Current official gettext snapshot from [benchmark/results/gettext-official-v1-with-gettext-parser-and-borrowed-de.json](benchmark/results/gettext-official-v1-with-gettext-parser-and-borrowed-de.json):

Environment snapshot for that report:

| Host | OS | CPU | Rust | Node.js | Python | GNU gettext |
|---|---|---|---|---|---|---|
| `MacBook-Pro-von-Sebastian.local` | `macos-aarch64` | `arm64` | `rustc 1.94.0` | `v24.14.0` | `3.9.6` | `gettext-tools 1.0` |

The important number is throughput, not `median-ms`. The compare runner calibrates each sample to roughly the same wall-clock duration, so `median-ms` is mainly useful inside one scenario run. For cross-tool reading, compare `items/s`.

For GNU gettext CLI tools, the JSON report now also includes an `empty-cli-run` baseline measured with a minimal header-only input. That gives each `msgcat`/`msgmerge` sample both:

- a raw end-to-end value
- an adjusted value with the minimal CLI baseline subtracted

The raw value remains the primary benchmark number. The adjusted value is a secondary estimate that helps separate command startup and tiny fixed costs from the actual workload.

Column labels:

- `ferrocat (Rust)`: native Rust implementations from this repo
- `pofile-ts (Node.js)`: the TypeScript rewrite / optimized successor in the same ecosystem
- `gettext-parser (Node.js)`: the long-standing Node gettext parser/compiler package
- `pofile (Node.js)`: the JavaScript/Node gettext parser package
- `polib (Python)`: the Python gettext library
- `GNU gettext (C)`: command-line tools from the classic gettext toolchain
- `—`: not part of that official comparison group

### Parse throughput

| Fixture | ferrocat (Rust)<br>`parse_po` | ferrocat (Rust)<br>`parse_po_borrowed` | pofile-ts (Node.js)<br>`parsePo` | gettext-parser (Node.js)<br>`po.parse` | pofile (Node.js)<br>`parse` | polib (Python)<br>`parse` |
|---|---:|---:|---:|---:|---:|---:|
| UI strings (DE, 10k)<br>(`gettext-ui-de-10000`) | 1.34M | **1.64M** | 562k | 104k | 9.0k | 58.5k |
| SaaS strings (FR, 10k)<br>(`gettext-saas-fr-10000`) | 1.31M | **1.57M** | 537k | 109k | 8.4k | 56.9k |
| Commerce strings (PL, 10k)<br>(`gettext-commerce-pl-10000`) | 1.29M | **1.62M** | 578k | 102k | 7.7k | 59.1k |

### Stringify throughput

| Fixture | ferrocat (Rust)<br>`stringify_po` | pofile-ts (Node.js)<br>`stringifyPo` | gettext-parser (Node.js)<br>`po.compile` | pofile (Node.js)<br>`serialize` | polib (Python)<br>`serialize` | GNU gettext (C)<br>`msgcat` |
|---|---:|---:|---:|---:|---:|---:|
| UI strings (DE, 10k)<br>(`gettext-ui-de-10000`) | **6.04M** | 1.26M | 195k | 552k | 99.5k | 29.9k |
| SaaS strings (FR, 10k)<br>(`gettext-saas-fr-10000`) | **5.98M** | 997k | 247k | 655k | 113k | 31.0k |
| Commerce strings (PL, 10k)<br>(`gettext-commerce-pl-10000`) | **6.37M** | 1.05M | 217k | 502k | 112k | 29.3k |

Workflow snapshot from [benchmark/results/gettext-official-v1-with-gettext-parser-and-borrowed-de.json](benchmark/results/gettext-official-v1-with-gettext-parser-and-borrowed-de.json):

### Basic Catalog Merge throughput

`merge_catalog` is the leaner gettext-style merge step. `msgmerge` is the nearest GNU gettext baseline for that workflow.

| Fixture | ferrocat (Rust)<br>`merge_catalog` | GNU gettext (C)<br>`msgmerge` |
|---|---:|---:|
| UI strings (DE, 10k)<br>(`gettext-ui-de-10000`) | **1.70M** | 26.1k |

### Full Catalog Update throughput

`update_catalog` is the higher-level end-to-end catalog update flow. We still compare it to `msgmerge` as the closest classic gettext workflow baseline, but it is a broader `ferrocat` operation than the lean `merge_catalog` step above.

| Fixture | ferrocat (Rust)<br>`update_catalog` | GNU gettext (C)<br>`msgmerge` |
|---|---:|---:|
| UI strings (DE, 10k)<br>(`gettext-ui-de-10000`) | **344k** | 26.3k |

The broader `gettext-compat-v1` and `gettext-workflows-v1` reports are still useful when you want more detail, but the table above is now aligned with the smaller official benchmark profile. If you publish or quote benchmark numbers, include the report's environment block so the device and toolchain are visible alongside the throughput table.

For profiling on macOS:

```bash
cargo instruments --no-open -t "Time Profiler" --bin ferrocat-bench -- parse mixed-10000 2000
```

## Project Docs

- [docs/conformance.md](docs/conformance.md)
- [docs/api-overview.md](docs/api-overview.md)
- [docs/performance-history.md](docs/performance-history.md)
- [docs/benchmarking.md](docs/benchmarking.md)
- [docs/benchmark-fixtures.md](docs/benchmark-fixtures.md)
- [docs/release-verification.md](docs/release-verification.md)
- [docs/test-coverage.md](docs/test-coverage.md)
- [docs/notes/2026-03-14-scan-architecture.md](docs/notes/2026-03-14-scan-architecture.md)
- [docs/adr](docs/adr)

Contribution conventions, including Conventional Commits, are documented in [CONTRIBUTING.md](CONTRIBUTING.md).
