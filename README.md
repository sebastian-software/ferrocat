# ferrocat

[![crates.io](https://img.shields.io/crates/v/ferrocat.svg)](https://crates.io/crates/ferrocat)
[![docs.rs](https://img.shields.io/docsrs/ferrocat)](https://docs.rs/ferrocat)
[![CI](https://github.com/sebastian-software/ferrocat/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sebastian-software/ferrocat/actions/workflows/ci.yml)
[![codecov](https://codecov.io/github/sebastian-software/ferrocat/graph/badge.svg?branch=main)](https://app.codecov.io/github/sebastian-software/ferrocat)

`ferrocat` is Rust-native catalog infrastructure for Gettext-compatible PO workflows, ICU message semantics, and runtime formats that are predictable to ship.

Most i18n stacks force a tradeoff: Gettext gives you mature translator tooling, ICU gives you richer message semantics, and custom JSON files are easy to serve but hard to govern. Ferrocat is built for the overlap. It parses and serializes PO catalogs, merges and updates catalog data, models plural semantics explicitly, and produces compact runtime artifacts with behavior you can inspect.

The project is young, but the goal is practical: give Rust teams a catalog layer they can inspect, test, benchmark, and migrate gradually.

Ferrocat is also part of the [Palamedes](https://github.com/sebastian-software/palamedes) ecosystem. Palamedes is the OSS i18n framework for JavaScript and TypeScript apps; Ferrocat is the catalog engine underneath the workflows that need strict PO handling, deterministic updates, NDJSON storage, runtime artifact compilation, and clear semantic boundaries.

## Why Teams Use Ferrocat

- **Classic PO workflows stay first-class.** Use familiar `msgid`, `msgctxt`, `msgid_plural`, `msgstr[n]`, comments, references, flags, and obsolete entries.
- **Translator context stays in the catalog.** Ferrocat preserves the comments, references, flags, contexts, plural entries, obsolete entries, and deterministic serialization that make PO useful in real workflows.
- **Source-as-msgid is supported cleanly.** If your `msgid` is real product copy rather than an opaque key, exact identity and explicit conflicts keep catalog updates predictable.
- **NDJSON makes large-team edits easier.** One message per line keeps catalog diffs reviewable and makes merge conflicts easier to isolate, even in hosted Git workflows where you cannot rely on custom merge drivers.
- **Modern runtime delivery is built in.** Compile catalogs into a runtime model that avoids reparsing PO files on every request and fits JSON-friendly delivery workflows.
- **Palamedes can build on the same core.** Use Palamedes for application-facing JS/TS framework integrations, and Ferrocat for the catalog semantics that should stay consistent underneath them.
- **Migration can be incremental.** Use Gettext-compatible behavior where you need it, move selected catalogs toward ICU-style semantics where it helps, and keep the behavior visible in code and CI.
- **Performance is measured.** Parser, serializer, merge, combine, and runtime paths are covered by fixtures, conformance checks, and benchmark commands.

## Core Workflows

Ferrocat focuses on the work that happens around real translation catalogs:

- Parse and serialize PO files.
- Merge existing catalogs with templates or newer catalogs.
- Combine several catalogs while preserving existing translations first.
- Validate plural behavior and catalog structure.
- Compile catalogs into runtime artifacts for application delivery.
- Compare performance and conformance behavior across fixtures.

See the [Gettext task landscape](https://sebastian-software.github.io/ferrocat/reference/gettext-task-landscape) for how Ferrocat maps common Gettext-style jobs to Rust-native APIs.

## Catalog Modes

At the high-level catalog layer, `ferrocat` supports three explicit combinations of storage format and message semantics:

| Mode | Storage format | Message model | Use when you want to... |
|---|---|---|---|
| Classic Gettext catalog mode | Gettext PO | Gettext-compatible plurals | stay close to traditional gettext catalogs and `msgid_plural` / `msgstr[n]` workflows |
| ICU-native Gettext PO mode | Gettext PO | ICU MessageFormat | keep Gettext PO files and tooling, but author richer ICU plural/select/formatting messages |
| ICU-native NDJSON catalog mode | NDJSON catalog storage | ICU MessageFormat | move to one-message-per-line JSON records that are easier to diff, merge, stream, batch, and hand to external systems |

There is intentionally no `NDJSON + Gettext-compatible plurals` mode. Gettext-compatible plural behavior stays a PO concern, while NDJSON is the native high-level storage format for ICU-native catalogs. Its line-delimited shape is especially useful when large teams edit catalogs through normal Git review flows: unrelated messages stay on separate lines, conflicts are narrower, and the format does not depend on a custom merge handler being available.

The canonical documentation now lives on the docs site:

- [Docs homepage](https://sebastian-software.github.io/ferrocat/)
- [Getting started](https://sebastian-software.github.io/ferrocat/guide/getting-started)
- [Catalog modes](https://sebastian-software.github.io/ferrocat/guide/catalog-modes)
- [Ferrocat and Palamedes](https://sebastian-software.github.io/ferrocat/guide/palamedes)
- [API overview](https://sebastian-software.github.io/ferrocat/reference/api-overview)
- [Gettext task landscape](https://sebastian-software.github.io/ferrocat/reference/gettext-task-landscape)
- [Performance docs](https://sebastian-software.github.io/ferrocat/performance)
- [ADR index](https://sebastian-software.github.io/ferrocat/architecture/adr)

## Install

```bash
cargo add ferrocat
```

The public entry point is the `ferrocat` crate. It re-exports the stable Rust surface from the lower-level workspace crates:

- `ferrocat`: umbrella crate and recommended dependency for application code
- `ferrocat-po`: PO parsing, serialization, merge/combine helpers, and higher-level catalog update flows
- `ferrocat-icu`: ICU MessageFormat parsing and structural helpers

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

For the common "merge fresh extracted messages into an existing catalog" workflow, `merge_catalog` is the lean Gettext-style entry point. For N-way catalog overlays and `msgcat`-style set operations, use `combine_catalogs`. For richer high-level flows across PO and NDJSON storage, the docs site's [API overview](https://sebastian-software.github.io/ferrocat/reference/api-overview) is the best next stop.

`parse_po_borrowed` is the allocation-light PO parser for read-heavy paths. It borrows from the source buffer where possible, but it currently requires LF-only input; normalize CRLF input first or use `parse_po`, which handles line-ending normalization internally.

## Compatibility Snapshot

- **MSRV:** Rust `1.88`
- **MSRV policy:** keep support roughly 9-12 months behind current stable when practical, rather than tracking only the newest stable toolchain
- **Semver:** the public API is treated seriously, but the project is still pre-`1.0`
- **Error surface:** PO parse errors are intentionally compact today and do not yet expose source positions; adding structured positions would be a semver-relevant API change.
- **Documentation surface:** README examples, rustdoc examples, and the docs site aim to stay aligned

## Docs Paths

If you already know what kind of question you have, these are the fastest entry points:

- [Getting started](https://sebastian-software.github.io/ferrocat/guide/getting-started) for installation, quick start, and the main next steps
- [Ferrocat and Palamedes](https://sebastian-software.github.io/ferrocat/guide/palamedes) for the relationship between the catalog engine and the JS/TS i18n framework
- [API overview](https://sebastian-software.github.io/ferrocat/reference/api-overview) for choosing between PO core, catalog workflows, and ICU helpers
- [Gettext task landscape](https://sebastian-software.github.io/ferrocat/reference/gettext-task-landscape) for the workflow-level map across GNU gettext, common libraries, and Ferrocat
- [Performance docs](https://sebastian-software.github.io/ferrocat/performance) for benchmark methodology, fixtures, and history
- [Quality docs](https://sebastian-software.github.io/ferrocat/quality) for conformance and coverage
- [ADR index](https://sebastian-software.github.io/ferrocat/architecture/adr) for architecture decisions and longer-term technical direction

## Core Links

- [docs.rs crate docs](https://docs.rs/ferrocat)
- [GitHub repository](https://github.com/sebastian-software/ferrocat)
- [Palamedes i18n framework](https://github.com/sebastian-software/palamedes)
- [Contributing guide](https://github.com/sebastian-software/ferrocat/blob/main/CONTRIBUTING.md)
- [Security policy](https://github.com/sebastian-software/ferrocat/blob/main/SECURITY.md)
- [Code of Conduct](https://github.com/sebastian-software/ferrocat/blob/main/CODE_OF_CONDUCT.md)
