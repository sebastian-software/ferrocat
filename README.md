# ferrox

`ferrox` is a Rust-native, performance-first toolkit for gettext PO handling, built for teams that want modern i18n workflows without being trapped in legacy toolchain shapes.

It is not a literal port of an existing JavaScript library. It is a deliberate Rust implementation with:

- a low-level PO core that respects ownership, borrowing, and hot-path efficiency
- a high-level catalog API that treats ICU-style structure as the long-term semantic target
- source-attributed conformance coverage against real upstream parser and writer behavior

## Why `ferrox`

Most PO tooling still makes at least one of these tradeoffs:

- it is tied to historical gettext workflows first, and modern semantics second
- it is convenient, but not built around predictable performance
- it has tests, but not source-backed conformance evidence

`ferrox` is trying to be stronger on all three axes:

- Rust-native implementation instead of line-by-line translation
- performance-first parser and serializer architecture
- compatibility measured against independently maintained upstream suites

## What Is In The Workspace

- `ferrox-po`: owned and borrowed PO parsing, serialization, merge support, and a higher-level catalog API
- `ferrox-icu`: the beginning of the ICU/MessageFormat layer
- `ferrox-bench`: repeatable benchmark and conformance reporting harness

## Current Highlights

- Owned parse via `parse_po`
- Borrowed parse via `parse_po_borrowed`
- Serialization via `stringify_po`
- Merge workflow via `merge_catalog`
- High-level catalog APIs via `parse_catalog`, `update_catalog`, and `update_catalog_file`
- Support for comments, metadata, references, flags, contexts, plurals, headers, and obsolete entries
- Conservative gettext compatibility with diagnostics where ambiguity matters

The borrowed parser exists because real PO workflows are often read-heavy and transformation-heavy. In those paths, avoiding unnecessary allocation is not a micro-optimization. It is the difference between a nice API and a scalable one.

## Modern Direction

At the catalog layer, `ferrox` treats ICU-style structure as the canonical semantic model and gettext as a compatibility bridge.

That means:

- internal modeling is aimed at structured messages rather than legacy slot arrays alone
- gettext import and export remain important, but do not define the architecture
- future-facing i18n work has a cleaner base than classic PO-only tooling usually offers

If your world is still gettext today, `ferrox` is meant to help there too. The point is not to abandon compatibility. The point is to avoid making historical compatibility the architectural center forever.

## Conformance

`ferrox` includes a hermetic, source-attributed conformance snapshot under [`conformance`](conformance).

As of 2026-03-16, the current scoreboard is:

- `55` upstream-derived conformance cases
- `442` concrete assertions
- `50` expected passes
- `5` expected rejects
- `0` `known_gap`

The snapshot currently draws from:

- `izimobil/polib`
- `rubenv/pofile`
- `python-babel/babel`
- `unicode-org/icu`

This matters because the tests are not just local inventions. They are tied back to behavior from real upstream ecosystems.

Run the full verification with:

```bash
cargo test --workspace
cargo run -p ferrox-bench -- conformance-report
```

`ferrox-po` intentionally normalizes headerless PO files on write by emitting an explicit empty header entry. That behavior is documented and is not counted as a compatibility gap.

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

For the common "read an existing catalog, merge fresh extracted messages, write the updated PO back" workflow:

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

This keeps existing translations, refreshes extractor-owned fields such as references and extracted comments, adds new messages, and marks removed ones obsolete.

## Parse Modes

### Owned parse

Use `parse_po` when the parsed catalog needs to outlive the input buffer, move across API boundaries, or be freely mutated.

### Borrowed parse

Use `parse_po_borrowed` when allocation pressure matters and the input buffer can stay alive for the lifetime of the parsed structure.

Today, borrowed parsing requires LF-only input. Owned parsing still handles CRLF normalization.

## Benchmarks And Profiling

The benchmark harness lives in `crates/ferrox-bench`.

Useful commands:

```bash
cargo run --release -p ferrox-bench -- parse mixed-10000 200
cargo run --release -p ferrox-bench -- parse-borrowed mixed-10000 200
cargo run --release -p ferrox-bench -- stringify mixed-10000 200
cargo run --release -p ferrox-bench -- merge mixed-10000 100
cargo run -p ferrox-bench -- describe mixed-1000
```

Historical benchmark results live in [docs/performance-history.md](docs/performance-history.md).

For profiling on macOS:

```bash
cargo instruments --no-open -t "Time Profiler" --bin ferrox-bench -- parse mixed-10000 2000
```

## Design Notes

The highest-signal project decisions are recorded in [docs/adr](docs/adr).

Useful supporting docs:

- [docs/conformance.md](docs/conformance.md)
- [docs/performance-history.md](docs/performance-history.md)
- [docs/benchmark-fixtures.md](docs/benchmark-fixtures.md)
- [docs/notes/2026-03-14-scan-architecture.md](docs/notes/2026-03-14-scan-architecture.md)

Contribution conventions, including Conventional Commits, are documented in [CONTRIBUTING.md](CONTRIBUTING.md).
