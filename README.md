# Ferrocat

Rust-native translation catalog engine for Gettext PO, ICU MessageFormat, and FCL catalogs.

[![crates.io](https://img.shields.io/crates/v/ferrocat.svg)](https://crates.io/crates/ferrocat)
[![docs.rs](https://img.shields.io/docsrs/ferrocat)](https://docs.rs/ferrocat)
[![CI](https://github.com/sebastian-software/ferrocat/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sebastian-software/ferrocat/actions/workflows/ci.yml)
[![codecov](https://codecov.io/github/sebastian-software/ferrocat/graph/badge.svg?branch=main)](https://app.codecov.io/github/sebastian-software/ferrocat)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.94.0-blue.svg)](#compatibility-snapshot)

`ferrocat` is a Rust-native translation catalog engine. It treats localized copy as product data: parse it, update it, review it, measure it, audit it, and compile it into runtime payloads your application can ship with confidence.

The practical problem is simple: translations change constantly, and most projects need more than "load a JSON file at runtime." Teams need source identity, translator context, reviewable diffs, coverage numbers, release checks, fallback behavior, and a runtime shape that does not hide catalog problems until production. Ferrocat is built for that middle layer.

You do not need to know PO files, ICU MessageFormat, or FCL to get started. What matters is that the catalog stays inspectable, testable, benchmarked, and portable across host-language adapters, instead of disappearing into format-specific tooling.

## Install

```bash
cargo add ferrocat
```

The public Rust entry point is the `ferrocat` crate. It re-exports the stable Rust surface from the lower-level workspace crates:

- `ferrocat`: umbrella crate and recommended dependency for application code
- `ferrocat-po`: PO parsing, serialization, low-level merge helpers, and feature-gated higher-level catalog workflows
- `ferrocat-icu`: ICU MessageFormat parsing, structural helpers, source/translation compatibility diagnostics, and semantic message metadata helpers

For non-Rust CI release gates, install the CLI package and run `ferrocat audit`:

```bash
cargo install ferrocat-cli
ferrocat audit --source-locale en --source locales/en.po --target de=locales/de.po
```

The `ferrocat-cli` GitHub release also publishes a prebuilt Linux musl archive for `x86_64-unknown-linux-musl`. The archive is named `ferrocat-<version>-x86_64-unknown-linux-musl.tar.gz` and contains the `ferrocat` binary plus license and README metadata.

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

For the common "merge fresh extracted messages into an existing catalog" workflow, `merge_catalog` is the lean Gettext-style entry point. For N-way catalog overlays and `msgcat`-style set operations, use `combine_catalogs`; when catalogs already live on disk, `combine_catalog_files` adds format inference and atomic output replacement around the same semantics. For version-control workflows, `merge_catalogs_three_way` accepts explicit ancestor, ours, and theirs roles and preserves deletions instead of treating the ancestor as another overlay. Use `convert_catalog` or `convert_catalog_file` for explicit ICU-native PO ↔ FCL conversion without treating conversion as a one-input combine. For release checks across a source catalog and target catalogs, use `audit_catalogs`. For dashboards or translator handoffs, use `measure_catalog_coverage` and `review_catalogs`. For application delivery, compile requested-locale artifacts with fallback and ICU diagnostics; use `compile_catalog_artifact_report` when host tooling also needs per-message resolution provenance without changing the runtime artifact shape.

Coverage, audit, and review inputs should be parsed with
`parse_catalog_for_review`. It returns a normalized catalog that retains only
semantic fuzzy identities, so active `#, fuzzy` PO and `f=fuzzy` FCL entries do
not count as translated. The ordinary `parse_catalog` path continues to skip
opaque flags and translator comments for high-throughput consumers that do not
request report state.

Beyond the basics, Ferrocat exposes byte-oriented and allocation-light borrowed parsers for hot paths, ICU analysis and source/translation compatibility checks (`analyze_icu`, `compare_icu_messages`, `validate_icu_formatter_support`), policy-aware apostrophe canonicalization and compiled-key derivation, ICU-aware pseudolocalization, semantic metadata normalization around `msgid + msgctxt`, and AI-translation metadata that high-level writers clear automatically once a human edits the text. ICU scope is MessageFormat v1; MessageFormat 2 is tracked as a future standard but is not a near-term target.

## What Ferrocat Gives You

- **One reliable catalog core.** Keep source text, contexts, translations, notes, source origins such as `src/App.tsx#CheckoutButton` or `src/i18n.ts#formatInvoiceStatus`, plural forms, and obsolete entries in a model that application code can reason about.
- **Predictable updates.** Merge newly extracted messages into existing catalogs without fuzzy guessing, hidden identity changes, or silent conflict resolution.
- **Deletion-aware collaboration.** Merge explicit ancestor, ours, and theirs catalogs while preserving deletions and reporting modify/delete conflicts through structured errors or diagnostics.
- **Explicit format conversion.** Convert ICU-native PO and FCL content or files with separate source and target modes, message-level metadata preservation, and atomic file replacement.
- **Lingui-compatible catalog order.** Sort message and context identities with the CLDR root order used by `Intl.Collator("en-US")` across PO, FCL, update, and combine output. PO can group by source origin while retaining the same identity tie-break; FCL always keeps its own collated line-order invariant.
- **Release-ready QA.** Audit catalog sets for missing locales, missing translations, empty translations, stale target messages, ICU mistakes, metadata conflicts, and obsolete entries.
- **Coverage and review reports.** Turn catalog state into completion counters, translator handoff diffs, and machine-managed value freshness checks instead of rebuilding those rules in every host tool.
- **Safer rich messages.** Analyze placeholders, formatters, plural/select branches, and rich-text tags so a translation cannot accidentally drop a required runtime value.
- **AI-native metadata.** Tag any machine-written value with a top-level integrity lock plus optional model and confidence. Ship it as-is by default; when a human corrects one, the lock stops matching, so the next re-translation run will not silently overwrite their fix.
- **Runtime artifacts you can explain.** Compile catalogs into host-neutral payloads with stable keys, fallback behavior, missing reports, optional ICU diagnostics, and provenance rows that show where each runtime string came from.
- **Pseudo-locale QA.** Generate ICU-aware pseudolocalized messages and runtime artifacts without damaging placeholders, plural selectors, rich-text tags, or formatter syntax.
- **Storage that survives a merge.** Use PO when translator tooling reads the file directly, or FCL when several people and jobs touch the same locale: one canonical entry per line keeps ordinary git merges from losing untouched translations.
- **Room for host frameworks.** Palamedes can own JS/TS extraction, bindings, and framework integration while Ferrocat owns the catalog behavior that should stay consistent underneath.
- **Measured behavior.** PR-visible regression scenarios cover parsing, serialization, merge, combine, audit, coverage/review, and runtime artifact compilation, backed by fixtures, upstream-derived conformance cases, and coverage gates.

Ferrocat is part of the [Palamedes](https://github.com/sebastian-software/palamedes) ecosystem. Palamedes is the OSS i18n framework for JavaScript and TypeScript apps; Ferrocat supplies the shared catalog engine underneath it. JavaScript and TypeScript access is a Palamedes concern, including the `palamedes-node` N-API bridge in that repository; this repository stays the pure-Rust catalog engine.

## Technical Foundation

Ferrocat is a new catalog layer, but it is not invented in a vacuum. It keeps the useful parts of established translation workflows and makes them available through a Rust API:

- **PO catalogs** for translator-friendly source, translation, context, comment, origin, plural, and obsolete-entry handling.
- **ICU MessageFormat v1** for richer messages with arguments, formatting, plurals, selects, and rich-text tags.
- **FCL catalogs** (Ferrocat Catalog Lines) for Git review, automation, and pipelines: one entry per line, deterministically sorted, and designed to preserve unrelated edits during ordinary 3-way merges.
- **Machine-managed value metadata** for AI-assisted workflows: a top-level integrity lock that detects later by-hand edits to any machine-set value, plus optional AI provenance.
- **Structured diagnostics** instead of ad hoc text output, so CI, editors, and host frameworks can consume the same report.

## Where Ferrocat Fits

Ferrocat is not trying to replace every part of an i18n stack. If you already know the existing landscape, this is the gap it fills:

| Common approach | What works well | Where Ferrocat helps |
|---|---|---|
| GNU gettext-style tooling | Mature PO conventions, translator metadata, broad ecosystem familiarity | Rust-native APIs, explicit conflict policy, structured diagnostics, ICU-native workflows, and app-ready runtime artifacts |
| Framework-specific i18n packages | Great authoring ergonomics and runtime adapters inside one host ecosystem | Shared catalog semantics that can be reused by Palamedes or other adapters instead of reimplemented per framework |
| Custom JSON catalogs | Easy loading and deployment | Stronger update semantics, reviewable FCL storage, source/translation QA, and a path back to translator-friendly PO workflows |
| ICU-only message handling | Powerful plural, select, formatter, and rich-text syntax | Structural analysis and compatibility checks that catch missing arguments, formatter drift, tag mismatch, and branch changes before shipping |

Ferrocat does not currently ship first-party WebAssembly, N-API, Python, Go, or other host-language bindings from this repository. Host integration is layered on top of the Rust API: JS/TS applications should look to Palamedes and its `palamedes-node` bridge, while other ecosystems can use the Rust crates directly, the `ferrocat` CLI audit gate, or host-specific bindings maintained at that layer.

## Performance

Ferrocat's benchmark suite compares file-to-file catalog workloads against common PO libraries and tools on the same fixtures. The published table below uses the 10k-message gettext catalog from the checked-in benchmark reports (Apple M1 Ultra, median throughput):

| Workload | Ferrocat | pofile-ts (Node) | gettext/gettext (PHP) | Babel (Python) | polib (Python) | GNU msgmerge |
|---|---:|---:|---:|---:|---:|---:|
| Parse | **725 MiB/s** | 149 | 53 | - | 22 | - |
| Update with new strings | **291 MiB/s** | 43 | 16 | - | 8.1 | 4.8 |
| Full catalog update | **150 MiB/s** | 42 | - | 8.9 | 8.2 | 4.8 |

The Node baseline is [pofile-ts](https://github.com/sebastian-software/pofile-ts), a speed-focused fork of the widely used `pofile`, so the JavaScript comparison targets a fast implementation rather than an easy one. The update benchmark parses existing and freshly extracted files, merges by exact identity, and serializes the result, which is where Ferrocat's byte-oriented scanning and move-not-clone catalog paths matter most.

The full catalog update row is Ferrocat's high-level `update_catalog` on the same files: on top of the plain update it analyzes ICU message structure, tracks placeholders, and produces deterministic output. The comparison now includes Babel's real `Catalog.update` path for Python; even with that extra work, Ferrocat's full update runs about 3.4x ahead of the fastest non-Ferrocat workflow update and roughly 31x ahead of GNU `msgmerge`.

See the [benchmark methodology](https://ferrocat.dev/performance/benchmarking) and the checked-in reports under [`benchmark/results/`](benchmark/results) for host details, fixture definitions, noise handling, and the full matrix.

## Catalog Modes

At the high-level catalog layer, `ferrocat` supports three explicit combinations of storage format and message semantics. You do not need to choose every mode on day one; the point is that migrations stay visible in code.

| Mode | Storage format | Message model | Use when you want to... |
|---|---|---|---|
| Classic Gettext catalog mode | Gettext PO | Gettext-compatible plurals | stay close to traditional gettext catalogs and `msgid_plural` / `msgstr[n]` workflows |
| ICU-native Gettext PO mode | Gettext PO | ICU MessageFormat | keep Gettext PO files and tooling, but author richer ICU plural/select/formatting messages |
| ICU-native FCL catalog mode | FCL catalog storage | ICU MessageFormat | move to one-entry-per-line, tab-separated records that are easier to diff, merge, batch, and hand to external systems |

There is intentionally no FCL + gettext-plural mode; gettext plural behavior stays a PO concern, while FCL is the ICU-native machine storage format for ICU-native catalogs. Generate FCL through the catalog layer by choosing `CatalogMode::IcuFcl` in `parse_catalog`, `update_catalog`, conversion, or file-based update flows; keep PO when external translator tools need gettext compatibility. The in-repo [FCL format specification](docs/fcl-format.md) documents the exact line format, escaping rules, and architecture decisions behind that storage mode.

## Feature Profiles

The published crates default to the full current API surface. Use
`default-features = false` when you want the low-level PO and ICU parsers without
the catalog workflow layer.

- `catalog` enables high-level catalog parsing, updates, combining, conversion,
  audits, machine-translation metadata, plural handling, FCL storage, and
  runtime artifact compilation.
- `serde` enables serialization support for tooling, cache, schema, report, and
  runtime artifact shapes.
- `compile`, `mt`, and `plurals` are reserved subsystem aliases that currently
  imply `catalog`; they do not reduce or split the catalog API surface yet.

See the [API overview](https://ferrocat.dev/reference/api-overview#feature-profiles)
for the crate-by-crate feature profile details.

## Compatibility Snapshot

- **MSRV:** Rust `1.94.0`
- **MSRV policy:** align with OXC when practical, while avoiding churn from tracking only the newest stable toolchain
- **MSRV bumps:** raising the MSRV is treated as a minor-version change and called out in the changelog; patch releases do not raise the MSRV
- **Prebuilt CLI target:** `x86_64-unknown-linux-musl` is validated in CI and published as a smoke-tested GitHub Release archive for `ferrocat-cli`
- **Semver:** the public API follows semantic versioning. Until the API surface settles and the crate has more downstream consumers, minor releases may still ship reviewed breaking cleanups (renames, stricter types, `#[non_exhaustive]` additions); every break is listed in the changelog with migration notes
- **Error surface:** PO parse errors stay intentionally compact but expose `message()` plus optional `position()` metadata with zero-based byte offset and one-based line/column when source context is available
- **Documentation surface:** README examples, rustdoc examples, and the docs site aim to stay aligned

## Docs Paths

If you already know what kind of question you have, these are the fastest entry points:

- [Getting started](https://ferrocat.dev/guide/getting-started) for installation, quick start, and the main next steps
- [Catalog modes](https://ferrocat.dev/guide/catalog-modes) for choosing PO or FCL storage with gettext or ICU message semantics
- [FCL format specification](docs/fcl-format.md) for the repository-local line format and escaping reference
- [Ferrocat and Palamedes](https://ferrocat.dev/guide/palamedes) for the relationship between the catalog engine and the JS/TS i18n framework
- [Releases and upgrading](https://ferrocat.dev/guide/upgrading) for changelogs, lockstep versions, and safe upgrades
- [API overview](https://ferrocat.dev/reference/api-overview) for choosing between PO core, catalog workflows, and ICU helpers
- [Gettext task landscape](https://ferrocat.dev/reference/gettext-task-landscape) for the workflow-level map across GNU gettext, common libraries, and Ferrocat
- [Performance docs](https://ferrocat.dev/performance) for benchmark methodology, fixtures, and history
- [Quality docs](https://ferrocat.dev/quality) for conformance and coverage
- [ADR index](https://ferrocat.dev/architecture/adr) for architecture decisions and longer-term technical direction

## Core Links

- [Documentation site](https://ferrocat.dev)
- [docs.rs crate docs](https://docs.rs/ferrocat)
- [`ferrocat` changelog](https://github.com/sebastian-software/ferrocat/blob/main/crates/ferrocat/CHANGELOG.md)
- [GitHub Releases](https://github.com/sebastian-software/ferrocat/releases)
- [GitHub repository](https://github.com/sebastian-software/ferrocat)
- [Palamedes i18n framework](https://github.com/sebastian-software/palamedes)
- [Contributing guide](https://github.com/sebastian-software/ferrocat/blob/main/CONTRIBUTING.md)
- [Security policy](https://github.com/sebastian-software/ferrocat/blob/main/SECURITY.md)
- [Code of Conduct](https://github.com/sebastian-software/ferrocat/blob/main/CODE_OF_CONDUCT.md)
- [Support guide](https://github.com/sebastian-software/ferrocat/blob/main/SUPPORT.md)

<!-- ferramenta-family:start -->
## The Ferramenta family

This project is part of [Ferramenta](https://ferramenta.dev) — the family of Rust-native developer tools by [Sebastian Software](https://oss.sebastian-software.com) that keep the APIs the ecosystem already knows:

| Tool | Job |
| --- | --- |
| [ferroni](https://github.com/sebastian-software/ferroni) | Oniguruma-compatible regex engine |
| [ferriki](https://github.com/sebastian-software/ferriki) | Shiki-compatible syntax highlighting |
| [ferromark](https://github.com/sebastian-software/ferromark) | CommonMark/GFM Markdown to HTML |
| [ferrovia](https://github.com/sebastian-software/ferrovia) | SVGO-compatible SVG optimizer |
| **[ferrocat](https://github.com/sebastian-software/ferrocat)** | Translation catalog engine |
| [ferrolex](https://github.com/sebastian-software/ferrolex) | Spell, dictionary, and brand validation |
| [ferrugo](https://github.com/sebastian-software/ferrugo) | Rust-native PDF previews |

ferrocat and ferrolex are also the Rust foundation of [Palamedes](https://github.com/sebastian-software/palamedes), i18n tooling for JavaScript and TypeScript teams that want one translation model to survive framework changes.
<!-- ferramenta-family:end -->

---

<!-- sebastian-software-branding:start -->

<p align="center">
  <a href="https://oss.sebastian-software.com">
    <img src="https://sebastian-brand.vercel.app/sebastian-software/logo-software.svg" alt="Sebastian Software" width="240" />
  </a>
</p>

<p align="center">
  <strong>Built by Sebastian Software</strong> — consulting for TypeScript, React &amp; Rust.<br />
  <a href="https://sebastian-software.de">Work with us</a> · <a href="https://oss.sebastian-software.com">More open source</a>
</p>

<p align="center">Copyright &copy; 2026 Sebastian Software GmbH</p>

<!-- sebastian-software-branding:end -->
