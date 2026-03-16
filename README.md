# ferrocat

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

## Benchmarks And Profiling

Useful benchmark commands:

```bash
cargo run --release -p ferrocat-bench -- parse mixed-10000 200
cargo run --release -p ferrocat-bench -- parse-borrowed mixed-10000 200
cargo run --release -p ferrocat-bench -- stringify mixed-10000 200
cargo run --release -p ferrocat-bench -- merge mixed-10000 100
cargo run -p ferrocat-bench -- describe mixed-1000
```

Historical benchmark results live in [docs/performance-history.md](docs/performance-history.md).

For profiling on macOS:

```bash
cargo instruments --no-open -t "Time Profiler" --bin ferrocat-bench -- parse mixed-10000 2000
```

## Project Docs

- [docs/conformance.md](docs/conformance.md)
- [docs/performance-history.md](docs/performance-history.md)
- [docs/benchmark-fixtures.md](docs/benchmark-fixtures.md)
- [docs/release-verification.md](docs/release-verification.md)
- [docs/notes/2026-03-14-scan-architecture.md](docs/notes/2026-03-14-scan-architecture.md)
- [docs/adr](docs/adr)

Contribution conventions, including Conventional Commits, are documented in [CONTRIBUTING.md](CONTRIBUTING.md).
