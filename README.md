# ferrocat

[![crates.io](https://img.shields.io/crates/v/ferrocat.svg)](https://crates.io/crates/ferrocat)
[![docs.rs](https://img.shields.io/docsrs/ferrocat)](https://docs.rs/ferrocat)
[![CI](https://github.com/sebastian-software/ferrocat/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sebastian-software/ferrocat/actions/workflows/ci.yml)
[![codecov](https://codecov.io/github/sebastian-software/ferrocat/graph/badge.svg?branch=main)](https://app.codecov.io/github/sebastian-software/ferrocat)

`ferrocat` is a performance-first toolkit for translation catalogs that need to span classic GNU gettext PO workflows, ICU MessageFormat semantics, and JSON-friendly runtime delivery.

Serious localization systems still depend on Gettext PO catalogs, translator-facing metadata, comments, references, and mature gettext-shaped workflows. At the same time, modern product stacks often want richer ICU-native messages and runtime-friendly data structures instead of forcing everything through PO as the only interchange format.

`ferrocat` brings those concerns into one Rust-native architecture with explicit catalog modes, stable crate boundaries, source-attributed conformance work, and performance tuning grounded in borrowing, byte-oriented hot paths, and profiling instead of hand-waving.

## What Ferrocat Optimizes For

- **Real translation workflows, not toy dictionaries.** Gettext PO still matters in production, and `ferrocat` treats comments, references, contexts, and plural behavior as first-class concerns.
- **Performance with reasons behind it.** The fast path is shaped by Rust-native design choices such as owned and borrowed APIs, byte-oriented scanning, and explicit profiling-driven iteration.
- **Trust, not just speed claims.** Conformance is tied back to upstream behavior instead of vague “mostly compatible” positioning.
- **Migration instead of lock-in.** Teams can stay close to classic gettext, move to ICU-native messages inside PO storage, or adopt NDJSON when external systems want JSON-first records.

## Three Catalog Modes

At the high-level catalog layer, `ferrocat` supports three explicit combinations of storage format and message semantics:

| Mode | Storage format | Message model | Use when you want to... |
|---|---|---|---|
| Classic Gettext catalog mode | Gettext PO | Gettext-compatible plurals | stay close to traditional gettext catalogs and `msgid_plural` / `msgstr[n]` workflows |
| ICU-native Gettext PO mode | Gettext PO | ICU MessageFormat | keep Gettext PO files and tooling, but author richer ICU plural/select/formatting messages |
| ICU-native NDJSON catalog mode | NDJSON catalog storage | ICU MessageFormat | move to line-oriented JSON records that are easier to diff, stream, batch, and hand to external systems |

There is intentionally no `NDJSON + Gettext-compatible plurals` mode. Gettext-compatible plural behavior stays a PO concern, while NDJSON is the native high-level storage format for ICU-native catalogs.

The canonical documentation now lives on the docs site:

- [Docs homepage](https://sebastian-software.github.io/ferrocat/)
- [Getting started](https://sebastian-software.github.io/ferrocat/guide/getting-started)
- [Catalog modes](https://sebastian-software.github.io/ferrocat/guide/catalog-modes)
- [API overview](https://sebastian-software.github.io/ferrocat/reference/api-overview)
- [Performance docs](https://sebastian-software.github.io/ferrocat/performance)
- [ADR index](https://sebastian-software.github.io/ferrocat/architecture/adr)

## Install

```bash
cargo add ferrocat
```

The public entry point is the `ferrocat` crate. It re-exports the stable Rust surface from the lower-level workspace crates:

- `ferrocat`: umbrella crate and recommended dependency for application code
- `ferrocat-po`: PO parsing, serialization, merge helpers, and higher-level catalog update flows
- `ferrocat-icu`: ICU MessageFormat parsing and structural helpers

## Project Goals

`ferrocat` exists for teams that are unhappy with the usual tradeoff triangle:

- simple app-local translation formats that are easy to start with but weak once real localization workflows arrive
- legacy PO tooling that preserves translator workflows but carries old performance and API tradeoffs forward
- libraries that have tests but not much evidence that their edge-case behavior actually matches the upstream ecosystems people depend on

The project goal is to close that gap with a Rust-native implementation that gives you:

- a fast PO parser, serializer, and merge/update engine
- a cleaner semantic center for ICU-aware catalog work
- explicit runtime-oriented compile layers for downstream adapters and bundlers
- compatibility evidence and benchmark methodology treated as part of the product surface

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
# Ok::<(), Box<dyn std::error::Error>>(())
```

For the common “merge fresh extracted messages into an existing catalog” workflow, `merge_catalog` is the lean gettext-style entry point. For richer high-level flows across PO and NDJSON storage, the docs site’s [API overview](https://sebastian-software.github.io/ferrocat/reference/api-overview) is the best next stop.

## Compatibility Snapshot

- **MSRV:** Rust `1.85`
- **Semver:** the public API is treated seriously, but the project is still pre-`1.0`
- **Documentation surface:** README examples, rustdoc examples, and the docs site aim to stay aligned

## Docs Paths

If you already know what kind of question you have, these are the fastest entry points:

- [Getting started](https://sebastian-software.github.io/ferrocat/guide/getting-started) for installation, quick start, and the main next steps
- [API overview](https://sebastian-software.github.io/ferrocat/reference/api-overview) for choosing between PO core, catalog workflows, and ICU helpers
- [Performance docs](https://sebastian-software.github.io/ferrocat/performance) for benchmark methodology, fixtures, and history
- [Quality docs](https://sebastian-software.github.io/ferrocat/quality) for conformance and coverage
- [ADR index](https://sebastian-software.github.io/ferrocat/architecture/adr) for architecture decisions and longer-term technical direction

## Core Links

- [docs.rs crate docs](https://docs.rs/ferrocat)
- [GitHub repository](https://github.com/sebastian-software/ferrocat)
- [Contributing guide](https://github.com/sebastian-software/ferrocat/blob/main/CONTRIBUTING.md)
- [Security policy](https://github.com/sebastian-software/ferrocat/blob/main/SECURITY.md)
- [Code of Conduct](https://github.com/sebastian-software/ferrocat/blob/main/CODE_OF_CONDUCT.md)
