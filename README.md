# ferrox

`ferrox` is a Rust-native, performance-first toolkit for gettext PO handling, with ICU planned alongside it.

The current workspace contains:

- `ferrox-po`: PO parsing and serialization
- `ferrox-icu`: ICU placeholder crate for upcoming work
- `ferrox-bench`: repeatable benchmark harness and realistic generated fixtures

## Goals

- idiomatic Rust instead of a line-by-line port
- strong baseline performance on common PO workloads
- a clear path toward SIMD/NEON-backed structural scanning where it actually helps
- APIs that support both ergonomic usage and allocation-aware fast paths

## Current State

`ferrox-po` currently provides:

- owned parsing via `parse_po`
- borrowed parsing via `parse_po_borrowed`
- serialization via `stringify_po`
- catalog merging via `merge_catalog`
- high-level catalog APIs via `parse_catalog`, `update_catalog`, and `update_catalog_file`
- C-style escape/unescape handling
- comments, metadata, references, flags, contexts, plurals, headers, and obsolete entries

The borrowed parser exists because many real workflows are read-heavy and transformation-heavy, but do not need a fully owned AST immediately.

At the high-level catalog layer, ICU is the default semantic target and gettext is treated as a compatibility bridge for import, export, and migration-oriented workflows.

## Conformance

`ferrox` includes a hermetic, source-attributed conformance snapshot under [`conformance`](conformance).

As of 2026-03-16, the snapshot covers `55` upstream-derived conformance cases and `442` concrete assertions across:

- `izimobil/polib`
- `rubenv/pofile`
- `python-babel/babel`
- `unicode-org/icu`

Use:

```bash
cargo test --workspace
cargo run -p ferrox-bench -- conformance-report
```

The report breaks coverage down into `pass`, `reject`, and `known_gap` and is intended to provide publishable, source-backed compatibility numbers.

`ferrox-po` intentionally normalizes headerless PO files on write by emitting an explicit empty header entry, so this behavior is not counted as a conformance gap.

Current scoreboard: `50` expected passes, `5` expected rejects, `0` `known_gap`.

## Parse Modes

### Owned parse

Use this when the parsed catalog needs to outlive the input buffer, cross API boundaries, or be mutated freely.

```rust
use ferrox_po::parse_po;

let file = parse_po(input)?;
```

### Borrowed parse

Use this when you want to keep allocations low and can keep the input string alive for the duration of the operation.

```rust
use ferrox_po::parse_po_borrowed;

let file = parse_po_borrowed(input)?;
let owned = file.into_owned();
```

Today, borrowed parsing requires LF-only input. The owned parser still handles CRLF normalization.

## Why Both Modes Exist

These two modes are intentional, not temporary duplication.

- `parse_po` is the ergonomic default API
- `parse_po_borrowed` is the allocation-aware fast path

For Node/N-API style integrations, the likely long-term model is:

- expose simple owned or task-oriented APIs externally
- use borrowed parsing internally where it buys measurable speed

## Example

```rust
use ferrox_po::{SerializeOptions, parse_po, stringify_po};

let mut file = parse_po(
    r#"
msgid "hello"
msgstr "world"
"#,
)?;

file.items[0].msgstr = "Welt".to_owned().into();

let rendered = stringify_po(&file, &SerializeOptions::default());
```

## Merge Workflow

For the common "read an existing catalog, merge fresh extracted messages, write the updated PO back" workflow, use `merge_catalog`.

```rust
use std::borrow::Cow;

use ferrox_po::{ExtractedMessage, merge_catalog};

let updated = merge_catalog(
    existing_po,
    &[ExtractedMessage {
        msgid: Cow::Borrowed("hello"),
        references: vec![Cow::Borrowed("src/app.rs:10")],
        ..ExtractedMessage::default()
    }],
)?;
```

This keeps matching translations, refreshes extractor-owned fields like references and extracted comments, adds new messages, and marks removed ones obsolete.

## High-Level Catalog API

For product-style workflows, prefer the high-level catalog API over direct PO item manipulation.

- `parse_catalog` projects a PO catalog into a structured catalog model
- `update_catalog` updates catalog content in memory
- `update_catalog_file` wraps the same flow around file I/O

`PluralEncoding::Icu` is the default. `PluralEncoding::Gettext` exists as a compatibility mode for existing gettext-based catalogs and toolchains, but it is not the canonical internal model.

This means:

- catalog parsing and updates are modeled around structured semantics first
- export format is an output decision, not the internal source of truth
- gettext `Plural-Forms` handling is intentionally conservative and diagnostic-driven

## Benchmarks

The benchmark harness lives in `crates/ferrox-bench`.

Useful commands:

```bash
cargo run --release -p ferrox-bench -- parse mixed-10000 200
cargo run --release -p ferrox-bench -- parse-borrowed mixed-10000 200
cargo run --release -p ferrox-bench -- stringify mixed-10000 200
cargo run --release -p ferrox-bench -- merge mixed-10000 100
cargo run -p ferrox-bench -- describe mixed-1000
```

Historical benchmark results are tracked append-only in [docs/performance-history.md](docs/performance-history.md).

## Profiling

The project is set up to use Apple Instruments and `cargo-instruments`/`xctrace`.

Typical workflow:

```bash
cargo instruments --no-open -t "Time Profiler" --bin ferrox-bench -- parse mixed-10000 2000
```

Or directly:

```bash
xcrun xctrace record --template "Time Profiler" --output target/instruments/parse.trace --launch -- target/release/ferrox-bench parse mixed-10000 2000
```

## Architecture Notes

Important design decisions live in [docs/adr](docs/adr).

Commit message conventions are documented in [CONTRIBUTING.md](CONTRIBUTING.md).

The highest-signal supporting docs today are:

- [docs/performance-history.md](docs/performance-history.md)
- [docs/benchmark-fixtures.md](docs/benchmark-fixtures.md)
- [docs/notes/2026-03-14-scan-architecture.md](docs/notes/2026-03-14-scan-architecture.md)
- [docs/plans/2026-03-14-ferrox-porting-plan.md](docs/plans/2026-03-14-ferrox-porting-plan.md)
