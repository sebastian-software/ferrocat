# ferrocat

[![crates.io](https://img.shields.io/crates/v/ferrocat.svg)](https://crates.io/crates/ferrocat)
[![docs.rs](https://img.shields.io/docsrs/ferrocat)](https://docs.rs/ferrocat)
[![CI](https://github.com/sebastian-software/ferrocat/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sebastian-software/ferrocat/actions/workflows/ci.yml)
[![codecov](https://codecov.io/github/sebastian-software/ferrocat/graph/badge.svg?branch=main)](https://app.codecov.io/github/sebastian-software/ferrocat)

`ferrocat` is a Rust-native translation catalog engine. It treats localized copy as product data: parse it, update it, review it, validate it, audit it, and compile it into runtime payloads your application can ship with confidence.

The practical problem is simple: translations change constantly, and most projects need more than "load a JSON file at runtime." Teams need source identity, translator context, reviewable diffs, release checks, fallback behavior, and a runtime shape that does not hide catalog problems until production. Ferrocat is built for that middle layer.

Under the hood, Ferrocat builds on proven ideas from PO catalogs, ICU MessageFormat, line-delimited JSON, and deterministic runtime compilation. You do not need to start with all of that terminology. The important part is that the catalog remains inspectable, testable, benchmarked, and portable across host-language adapters.

Ferrocat is also part of the [Palamedes](https://github.com/sebastian-software/palamedes) ecosystem. Palamedes is the OSS i18n framework for JavaScript and TypeScript apps; Ferrocat supplies the shared catalog engine underneath it: exact updates, storage choices, release QA, runtime artifact compilation, and clear boundaries between catalog data and application integration. JavaScript and TypeScript access is a Palamedes concern, including the `palamedes-node` N-API bridge in that repository; this repository stays the pure-Rust catalog engine.

## What Ferrocat Gives You

- **One reliable catalog core.** Keep source text, contexts, translations, comments, references, flags, plural forms, and obsolete entries in a model that application code can reason about.
- **Predictable updates.** Merge newly extracted messages into existing catalogs without fuzzy guessing, hidden identity changes, or silent conflict resolution.
- **Release-ready QA.** Audit catalog sets for missing locales, missing translations, empty translations, stale target messages, ICU mistakes, metadata conflicts, obsolete entries, and visible `fuzzy` flags.
- **Safer rich messages.** Analyze placeholders, formatters, plural/select branches, and rich-text tags so a translation cannot accidentally drop a required runtime value.
- **AI translation metadata ready.** Track machine-generated translations with model, modification time, confidence, and a change-detection hash, then drop stale metadata automatically when a human edits the text.
- **Runtime artifacts.** Compile catalogs into host-neutral payloads with stable keys, fallback behavior, missing reports, and optional ICU compatibility diagnostics.
- **Reviewable storage.** Use PO when translator tooling matters, or NDJSON when large teams and automation need one message per line for cleaner diffs.
- **Room for host frameworks.** Palamedes can own JS/TS extraction, bindings, and framework integration while Ferrocat owns the catalog behavior that should stay consistent underneath.
- **Measured behavior.** Parser, serializer, merge, combine, audit, and runtime paths are covered by fixtures, conformance checks, and benchmark commands.

## Technical Foundation

Ferrocat is a new catalog layer, but it is not invented in a vacuum. It keeps the useful parts of established translation workflows and makes them available through a Rust API:

- **PO catalogs** for translator-friendly source, translation, context, comment, reference, flag, plural, and obsolete-entry handling.
- **ICU MessageFormat v1** for richer messages with arguments, formatting, plurals, selects, and rich-text tags.
- **NDJSON catalogs** for line-oriented storage that works well with Git review, automation, and external systems.
- **Machine-translation metadata** for AI-assisted localization workflows that need to know which model produced a translation and whether that metadata still matches the current text.
- **Structured diagnostics** instead of ad hoc text output, so CI, editors, and host frameworks can consume the same report.

## Where Ferrocat Fits

Ferrocat is not trying to replace every part of an i18n stack. If you already know the existing landscape, this is the gap it fills:

| Common approach | What works well | Where Ferrocat helps |
|---|---|---|
| GNU gettext-style tooling | Mature PO conventions, translator metadata, broad ecosystem familiarity | Rust-native APIs, explicit conflict policy, structured diagnostics, ICU-native workflows, and app-ready runtime artifacts |
| Framework-specific i18n packages | Great authoring ergonomics and runtime adapters inside one host ecosystem | Shared catalog semantics that can be reused by Palamedes or other adapters instead of reimplemented per framework |
| Custom JSON catalogs | Easy loading and deployment | Stronger update semantics, reviewable NDJSON storage, source/translation QA, and a path back to translator-friendly PO workflows |
| ICU-only message handling | Powerful plural, select, formatter, and rich-text syntax | Structural analysis and compatibility checks that catch missing arguments, formatter drift, tag mismatch, and branch changes before shipping |

Ferrocat does not currently ship first-party WebAssembly, N-API, Python, Go, or
other host-language bindings from this repository. Host integration is layered
on top of the Rust API: JS/TS applications should look to Palamedes and its
`palamedes-node` bridge, while other ecosystems can use the Rust crates
directly, the `ferrocat` CLI audit gate, or host-specific bindings maintained
at that layer.

## Core Workflows

Ferrocat focuses on the work that happens around real translation catalogs:

- Parse and serialize PO files.
- Merge existing catalogs with templates or newer catalogs.
- Combine several catalogs while preserving existing translations first.
- Validate plural behavior and catalog structure.
- Analyze ICU message structure and compare source/translation compatibility.
- Normalize semantic message metadata around `msgid + msgctxt`.
- Preserve AI translation metadata in PO and NDJSON catalogs, including stale-metadata cleanup when translations are edited.
- Audit catalog sets before release with structured diagnostics.
- Compile catalogs into runtime artifacts for application delivery.
- Compare performance and conformance behavior across fixtures.

See the [API overview](https://sebastian-software.github.io/ferrocat/reference/api-overview) when you want the Rust entry points. The [Gettext task landscape](https://sebastian-software.github.io/ferrocat/reference/gettext-task-landscape) is a deeper reference for readers mapping long-standing command-line workflows to Ferrocat APIs.

## Catalog Modes

At the high-level catalog layer, `ferrocat` supports three explicit combinations of storage format and message semantics. You do not need to choose every mode on day one; the point is that migrations stay visible in code.

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

The public Rust entry point is the `ferrocat` crate. It re-exports the stable Rust surface from the lower-level workspace crates:

- `ferrocat`: umbrella crate and recommended dependency for application code
- `ferrocat-po`: PO parsing, serialization, merge/combine helpers, and higher-level catalog update flows
- `ferrocat-icu`: ICU MessageFormat parsing, structural helpers, source/translation compatibility diagnostics, and semantic message metadata helpers

For non-Rust CI release gates, install the CLI package and run `ferrocat audit`:

```bash
cargo install ferrocat-cli
ferrocat audit --source-locale en --source locales/en.po --target de=locales/de.po
```

## Quick Start

```rust
use ferrocat::{SerializeOptions, parse_po, stringify_po};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = parse_po(
        r#"
msgid "hello"
msgstr "world"
"#,
    )?;

    file.items[0].msgstr = "Welt".to_owned().into();

    let rendered = stringify_po(&file, &SerializeOptions::default());
    assert!(rendered.contains(r#"msgstr "Welt""#));

    Ok(())
}
```

For the common "merge fresh extracted messages into an existing catalog" workflow, `merge_catalog` is the lean Gettext-style entry point. For N-way catalog overlays and `msgcat`-style set operations, use `combine_catalogs`. For release checks across a source catalog and target catalogs, use `audit_catalogs`. For application delivery, compile requested-locale artifacts with fallback and ICU diagnostics. For richer high-level flows across PO and NDJSON storage, the docs site's [API overview](https://sebastian-software.github.io/ferrocat/reference/api-overview) is the best next stop.

`parse_po_borrowed` is the allocation-light PO parser for read-heavy paths. It borrows from the source buffer where possible, but it currently requires LF-only input; normalize CRLF input first or use `parse_po`, which handles line-ending normalization internally.

ICU-native workflows can use `analyze_icu`, `compare_icu_messages`, and `validate_icu_formatter_support` to catch missing arguments, formatter changes, unsupported runtime formatter kinds or styles, tag mismatches, select/plural branch drift, and discouraged pattern-style formatters. Runtime artifact compilation can opt into the same source/translation checks with `icu_compatibility`.

Semantic metadata workflows can use `normalize_message_metadata` to keep simple source-as-msgid records small while deriving argument, tag, and selector facts from ICU MessageFormat v1 when needed. The metadata model keeps `msgid + msgctxt` as catalog identity, so it fits both Palamedes-style source strings and ID-style catalogs.

AI translation workflows can attach `MachineTranslationMetadata` to catalog entries. Ferrocat stores `model`, optional `modified`, optional `confidence`, and a translation hash in PO (`#@ ferrocat-mt ...`) or NDJSON (`mt`), and high-level writers remove that metadata when the hash no longer matches the current translation.

Catalog QA workflows can use `audit_catalogs` to produce a read-only report over source and target catalogs. The default checks cover missing locales, missing or empty translations, extra target-only messages, ICU syntax, ICU source/translation compatibility, semantic metadata conflicts, obsolete entries, and visible `fuzzy` flags. The audit API reports what is shippable; it does not infer fuzzy matches or rewrite catalogs.

Ferrocat's ICU scope is currently MessageFormat v1. MessageFormat 2 is tracked as a future standard, but it is not a near-term implementation target because the implementation surface is still transitional and MF1 authoring diagnostics solve the more immediate catalog problems.

## Compatibility Snapshot

- **MSRV:** Rust `1.93.0`
- **MSRV policy:** align with OXC when practical, while avoiding churn from tracking only the newest stable toolchain
- **Semver:** the public API is treated seriously, but the project is still pre-`1.0`
- **Error surface:** PO parse errors are intentionally compact today and do not yet expose source positions; adding structured positions would be a semver-relevant API change.
- **Documentation surface:** README examples, rustdoc examples, and the docs site aim to stay aligned

## Docs Paths

If you already know what kind of question you have, these are the fastest entry points:

- [Getting started](https://sebastian-software.github.io/ferrocat/guide/getting-started) for installation, quick start, and the main next steps
- [Ferrocat and Palamedes](https://sebastian-software.github.io/ferrocat/guide/palamedes) for the relationship between the catalog engine and the JS/TS i18n framework
- [Releases and upgrading](https://sebastian-software.github.io/ferrocat/guide/upgrading) for changelogs, lockstep versions, and pre-`1.0` upgrade notes
- [API overview](https://sebastian-software.github.io/ferrocat/reference/api-overview) for choosing between PO core, catalog workflows, and ICU helpers
- [Gettext task landscape](https://sebastian-software.github.io/ferrocat/reference/gettext-task-landscape) for the workflow-level map across GNU gettext, common libraries, and Ferrocat
- [Performance docs](https://sebastian-software.github.io/ferrocat/performance) for benchmark methodology, fixtures, and history
- [Quality docs](https://sebastian-software.github.io/ferrocat/quality) for conformance and coverage
- [ADR index](https://sebastian-software.github.io/ferrocat/architecture/adr) for architecture decisions and longer-term technical direction

## Core Links

- [docs.rs crate docs](https://docs.rs/ferrocat)
- [`ferrocat` changelog](https://github.com/sebastian-software/ferrocat/blob/main/crates/ferrocat/CHANGELOG.md)
- [GitHub Releases](https://github.com/sebastian-software/ferrocat/releases)
- [GitHub repository](https://github.com/sebastian-software/ferrocat)
- [Palamedes i18n framework](https://github.com/sebastian-software/palamedes)
- [Contributing guide](https://github.com/sebastian-software/ferrocat/blob/main/CONTRIBUTING.md)
- [Security policy](https://github.com/sebastian-software/ferrocat/blob/main/SECURITY.md)
- [Code of Conduct](https://github.com/sebastian-software/ferrocat/blob/main/CODE_OF_CONDUCT.md)
