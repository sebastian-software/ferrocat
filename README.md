# ferrocat

[![crates.io](https://img.shields.io/crates/v/ferrocat.svg)](https://crates.io/crates/ferrocat)
[![docs.rs](https://img.shields.io/docsrs/ferrocat)](https://docs.rs/ferrocat)
[![CI](https://github.com/sebastian-software/ferrocat/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sebastian-software/ferrocat/actions/workflows/ci.yml)
[![codecov](https://codecov.io/github/sebastian-software/ferrocat/graph/badge.svg?branch=main)](https://app.codecov.io/github/sebastian-software/ferrocat)

`ferrocat` is a modern, performance-first toolkit for the translation formats that serious localization workflows still rely on: gettext PO files today, with a clear path toward richer ICU-aware workflows tomorrow.

If your mental model for translations starts with JSON files, `ferrocat` is the bridge back to what a lot of real-world i18n systems have used for decades: PO-based catalogs, gettext-style workflows, translator-friendly metadata, and tooling that has to be both fast and trustworthy.

`ferrocat` brings that world into a Rust-native architecture with explicit crate boundaries, source-attributed conformance coverage, and performance work grounded in borrowing, byte-oriented hot paths, and profiling instead of wishful thinking.

## Why People Get Excited About `ferrocat`

- **It speaks the real language of translation workflows.** PO files are still a durable standard across gettext-based pipelines, translator tooling, comments, references, contexts, and plural handling. `ferrocat` is built for that reality, not just for toy key-value dictionaries.
- **The performance story has reasons behind it.** The fast path is shaped by Rust-native design decisions: owned and borrowed APIs, byte-oriented scanning, explicit crate boundaries, and repeated profiling work to remove avoidable allocation and parsing overhead.
- **It aims for trust, not just speed.** Conformance is tied back to independently maintained upstream behavior instead of vague compatibility claims.
- **It is built for migration, not lock-in.** At the high level, `ferrocat` treats ICU-style structure as the long-term semantic model while keeping gettext as the compatibility bridge many teams still need today.
- **It starts in Rust, but it does not stop there.** The core is being shaped so future Node.js/N-API and other bindings can sit on top of a stable engine instead of re-implementing the same translation logic per ecosystem.

## Installation

```bash
cargo add ferrocat
```

The public entry point is the `ferrocat` crate. It re-exports the stable Rust surface from the lower-level workspace crates:

- `ferrocat`: umbrella crate and recommended dependency for application code
- `ferrocat-po`: PO parsing, serialization, merge helpers, and catalog update APIs
- `ferrocat-icu`: ICU MessageFormat parsing and structural helpers
- `ferrocat-bench`: workspace-only benchmark harness
- `ferrocat-conformance`: workspace-only upstream-derived conformance fixtures

If you want a narrower dependency, `ferrocat-po` and `ferrocat-icu` remain publishable secondary crates.

## Compatibility Policy

- **MSRV:** `ferrocat` currently targets Rust `1.85`.
- **Semver:** the public API is treated seriously, but the project is still pre-`1.0`. Minor releases may still contain carefully documented breaking changes when the API contract needs to improve.
- **Docs surface:** published crates aim to keep README examples, rustdoc examples, and repository docs aligned.

## Why `ferrocat` Exists

Many teams end up choosing between two unsatisfying extremes:

- translation data in simple app-local formats that are easy to start with, but weak once real localization workflows show up
- legacy PO tooling that preserves old workflows, but carries historical performance and API tradeoffs forward
- libraries with tests, but without strong evidence that their edge-case behavior matches the upstream ecosystems people already depend on

`ferrocat` exists to close that gap. It aims to give you the things people actually want at the same time:

- Rust-native implementation instead of line-by-line translation
- performance-first parser, serializer, and merge architecture
- compatibility measured against real upstream behavior
- a cleaner long-term semantic center for ICU-aware catalog work

That last point matters because gettext is still everywhere, but many teams want something better than being trapped forever in legacy shapes. At the catalog layer, `ferrocat` treats ICU-style structure as the long-term semantic model and gettext as the compatibility bridge.

## Quick Start

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

If that matches what you are building, this repo is worth trying now and worth watching as the cross-ecosystem story grows.

## API Overview

The current public surface falls into three practical layers, depending on whether you want raw PO access, higher-level catalog workflows, or ICU parsing:

| Layer | Functions | Use when you want to... |
|---|---|---|
| PO core | `parse_po`, `parse_po_borrowed`, `stringify_po` | parse and write classic `.po` files directly |
| Catalog workflows | `merge_catalog` | do the lean gettext-style merge step against fresh extracted messages |
| Catalog workflows | `parse_catalog` | read a `.po` file into the higher-level canonical catalog model |
| Catalog workflows | `compiled_key` | derive the default stable runtime lookup key from `msgid` and optional `msgctxt` |
| Catalog workflows | `NormalizedParsedCatalog::compile` | compile a normalized catalog into runtime lookup entries with stable derived keys |
| Catalog workflows | `compile_catalog_artifact` | compile one requested-locale runtime artifact with fallback resolution, missing reports, and final ICU strings |
| Catalog workflows | `compile_catalog_artifact_selected` | compile only a selected subset of compiled runtime IDs into a locale artifact slice |
| Catalog workflows | `CompiledCatalogIdIndex` | build deterministic compiled-ID metadata, export the ordered ID map, or describe selected IDs against a catalog set |
| Catalog workflows | `update_catalog` | run the full in-memory catalog update flow with headers, plurals, sorting, and diagnostics |
| Catalog workflows | `update_catalog_file` | run the same full update flow directly against a file on disk |
| ICU | `parse_icu`, `validate_icu`, `extract_variables` | parse or inspect ICU MessageFormat structure |

See [docs/api-overview.md](docs/api-overview.md) for the fuller "what should I use when?" guide.

Across all of these layers, `ferrocat` keeps a conservative gettext-compatibility stance and surfaces diagnostics where ambiguity matters.

The borrowed parser exists because real PO workflows are often read-heavy and transformation-heavy. In those paths, avoiding unnecessary allocation is the difference between a pleasant API and a scalable one.

## Runtime Catalog Compilation

`ferrocat` now also exposes a first runtime-oriented compile step on top of the parsed catalog API:

```rust
use ferrocat::{CompileCatalogOptions, ParseCatalogOptions, parse_catalog};

let parsed = parse_catalog(ParseCatalogOptions {
    content: "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
    source_locale: "en",
    locale: Some("de"),
    ..ParseCatalogOptions::default()
})?;
let normalized = parsed.into_normalized_view()?;
let compiled = normalized.compile(&CompileCatalogOptions::default())?;

assert_eq!(compiled.len(), 1);
```

This layer is intentionally small:

- it starts from `NormalizedParsedCatalog`, so source identity is still `msgid + msgctxt`
- it produces typed runtime values instead of flattening plurals into strings
- it derives compact stable lookup keys for runtime maps
- it does **not** silently fill source text by default

Current key contract:

- built-in strategy: `CompiledKeyStrategy::FerrocatV1`
- hash: SHA-256
- output: first 64 bits, encoded as unpadded Base64URL
- visible version prefix: none
- versioning: internal domain-separation input to the hash, not part of the emitted key
- collisions: compile-time error, never overwrite

If a host adapter or source transform needs that same default runtime key
without compiling a whole catalog first, use `compiled_key(msgid, msgctxt)`.

The goal is to give downstream runtimes small, reproducible lookup keys without turning the library into a code generator. If you need JS/TS/Rust module generation, `CompiledCatalog` is the intended handoff point.

For the next layer up, `ferrocat` also exposes a host-neutral artifact compile step for one requested locale:

```rust
use ferrocat::{
    CompileCatalogArtifactOptions, ParseCatalogOptions, compile_catalog_artifact, parse_catalog,
};

let source = parse_catalog(ParseCatalogOptions {
    content: "msgid \"Hello\"\nmsgstr \"Hello\"\n",
    source_locale: "en",
    locale: Some("en"),
    ..ParseCatalogOptions::default()
})?
.into_normalized_view()?;
let requested = parse_catalog(ParseCatalogOptions {
    content: "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
    source_locale: "en",
    locale: Some("de"),
    ..ParseCatalogOptions::default()
})?
.into_normalized_view()?;

let artifact = compile_catalog_artifact(
    &[&requested, &source],
    &CompileCatalogArtifactOptions {
        requested_locale: "de",
        source_locale: "en",
        ..CompileCatalogArtifactOptions::default()
    },
)?;

assert_eq!(artifact.messages.len(), 1);
assert!(artifact.missing.is_empty());
```

This higher-level compile path is meant for runtime/export tooling that wants:

- one requested-locale runtime map keyed by Ferrocat's compiled lookup keys
- locale fallback resolution before host-specific code generation
- explicit missing-message reporting for non-source locales
- final ICU-string validation diagnostics

Use `NormalizedParsedCatalog::compile` when you only want the smaller typed runtime lookup layer for one normalized catalog. Use `compile_catalog_artifact` when you need the fully resolved locale-specific runtime artifact that downstream loaders or bundlers can consume directly. Use `compile_catalog_artifact_selected` when a host adapter already knows the exact compiled runtime IDs it needs and wants only that subset. Use `CompiledCatalogIdIndex` when a host adapter wants to cache the ordered `compiled_id -> source_key` mapping or ask Ferrocat which requested IDs are known, available in a given catalog set, and singular vs plural before compiling payloads.

## Project Status

`ferrocat` is an actively developed pre-`1.0` Rust project.

Current strengths:

- high-performance PO parsing, serialization, and merge/update workflows
- ICU `MessageFormat` parsing with structural helpers
- normalized catalog APIs plus runtime compilation and requested-locale artifact generation
- conformance and benchmark infrastructure treated as product features, not afterthoughts

Current roadmap themes:

- deeper runtime/catalog workflows on top of the compiled catalog layer
- broader cross-ecosystem tooling and bindings
- continued conformance growth and benchmark publication discipline

## Conformance

`ferrocat` includes a hermetic, source-attributed conformance snapshot under [`conformance`](conformance).

This is part of the core product story: compatibility should be demonstrated against real upstream suites, not hand-waved.

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

## Open Source And Community

- [Contributing guide](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [ADR index](docs/adr/README.md)
- [Release verification checklist](docs/release-verification.md)

If you are evaluating `ferrocat` for production, the most relevant repo signals are the published docs.rs pages, the conformance snapshot, and the benchmark methodology documentation. If you want to contribute, the issue templates, Conventional Commit rules, and Rust verification commands are already wired for that workflow.

## Benchmarks And Profiling

The benchmark section exists to support the main claim, not replace it: `ferrocat` is fast because the library was designed and profiled for predictable hot-path behavior.

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

For workflow-style benchmarking there is now also a separate `gettext-workflows-v1` profile, which compares `merge_catalog` against a conservative `msgmerge` baseline on the `gettext-ui-de-*` corpus.

Current official gettext snapshot from [benchmark/results/gettext-official-v1-20260318-094344.json](benchmark/results/gettext-official-v1-20260318-094344.json) (`generated_at: 2026-03-18T09:50:53Z`):

Environment snapshot for that report:

| System | Rust | Node.js | Python | GNU gettext |
|---|---|---|---|---|
| `Apple M1 Pro (32 GB RAM, macOS arm64)` | `rustc 1.94.0` | `v24.14.0` | `3.9.6` | `gettext-tools 1.0` |

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
| UI strings (DE, 10k)<br>(`gettext-ui-de-10000`) | 1.35M | **1.67M** | 570k | 105k | 9.6k | 59.4k |
| SaaS strings (FR, 10k)<br>(`gettext-saas-fr-10000`) | 1.31M | **1.58M** | 537k | 112k | 8.5k | 58.2k |
| Commerce strings (PL, 10k)<br>(`gettext-commerce-pl-10000`) | 1.29M | **1.63M** | 593k | 102k | 7.9k | 59.9k |

### Stringify throughput

| Fixture | ferrocat (Rust)<br>`stringify_po` | pofile-ts (Node.js)<br>`stringifyPo` | gettext-parser (Node.js)<br>`po.compile` | pofile (Node.js)<br>`serialize` | polib (Python)<br>`serialize` | GNU gettext (C)<br>`msgcat` |
|---|---:|---:|---:|---:|---:|---:|
| UI strings (DE, 10k)<br>(`gettext-ui-de-10000`) | **6.06M** | 1.25M | 194k | 696k | 99.9k | 29.7k |
| SaaS strings (FR, 10k)<br>(`gettext-saas-fr-10000`) | **5.96M** | 1.04M | 243k | 649k | 114k | 30.7k |
| Commerce strings (PL, 10k)<br>(`gettext-commerce-pl-10000`) | **6.43M** | 1.12M | 225k | 621k | 113k | 29.3k |

`merge_catalog` is the leaner gettext-style merge step. It works like a fast-path merge:

- keep matching translations
- add new entries
- mark removed entries as obsolete
- preserve the classic PO shape instead of re-projecting through the higher-level catalog model

Workflow ecosystem snapshot from [benchmark/results/gettext-workflows-ecosystem-v1-20260318-095059.json](benchmark/results/gettext-workflows-ecosystem-v1-20260318-095059.json) (`generated_at: 2026-03-18T09:52:12Z`):

`pofile`, `pofile-ts`, and `polib` now also run as reconstructed `msgmerge`-style pipelines: parse existing `.po`, merge against the generated `.pot`, then serialize again. This is intentionally a workflow comparison, not just a raw parser benchmark.

Those reconstructed external workflows currently do the lean merge behavior:

- parse existing `.po`
- parse generated `.pot`
- match entries by context + `msgid` + `msgid_plural`
- keep existing translations where the key still matches
- create empty translations for new entries
- mark unmatched old entries as obsolete
- serialize the result back to `.po`

They do **not** try to reproduce the higher-level `update_catalog` feature set such as canonical catalog projection, header-default completion, diagnostic collection, or the broader export rules. This workflow table is intentionally only about the lean merge step.

`gettext-parser` is not part of this workflow table yet. Its current PO compile/parse model is fine for parse/stringify benchmarking, but it does not preserve obsolete entries in a way that makes a `msgmerge`-style workflow semantically fair.

In these lean `msgmerge`-style workflows, `ferrocat` is not just faster than GNU gettext in this comparison set, but also substantially faster than the common Node.js and Python libraries included here.

`update_catalog` still exists as a higher-level API, but it is no longer part of the public cross-tool benchmark table because it does broader catalog-maintenance work and does not have a clean direct equivalent in the external comparison set.

The broader `gettext-compat-v1` and `gettext-workflows-v1` reports are still useful when you want more detail. If you publish or quote benchmark numbers, include the report's environment block so the device and toolchain are visible alongside the throughput table.

### Catalog Merge Throughput Across Ecosystem Tools

| Fixture | ferrocat (Rust)<br>`merge_catalog` | pofile-ts (Node.js)<br>`parsePo` + merge + `stringifyPo` | pofile (Node.js)<br>`parse` + merge + `serialize` | GNU gettext (C)<br>`msgmerge` | polib (Python)<br>`pofile` + merge + `str()` |
|---|---:|---:|---:|---:|---:|
| UI strings (DE, 1k)<br>(`gettext-ui-de-1000`) | **1.97M** | 169k | 77.8k | 23.3k | 18.1k |
| UI strings (DE, 10k)<br>(`gettext-ui-de-10000`) | **1.84M** | 155k | 2.7k | 26.8k | 18.1k |

For profiling on macOS:

```bash
cargo instruments --no-open -t "Time Profiler" --bin ferrocat-bench -- parse mixed-10000 2000
```

## Future Direction

Today `ferrocat` is a Rust-first library. The broader goal is bigger than that: a fast, trustworthy translation core that can power multiple ecosystems from one implementation.

That is why the architecture already keeps future Node.js/N-API-friendly boundaries in mind. The Rust crate is the first delivery vehicle, not the final limit of the project.

If this direction matches what you want from translation tooling, try the crate today and star or watch the repo to follow the broader ecosystem story as it expands.

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
