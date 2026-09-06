# ferrocat

[![crates.io](https://img.shields.io/crates/v/ferrocat.svg)](https://crates.io/crates/ferrocat)
[![docs.rs](https://img.shields.io/docsrs/ferrocat)](https://docs.rs/ferrocat)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/sebastian-software/ferrocat/blob/main/LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.93.0-blue.svg)](https://github.com/sebastian-software/ferrocat#compatibility-snapshot)

Public umbrella crate for the `ferrocat` workspace.

Ferrocat is a Rust-native translation catalog engine. It gives applications and host-language adapters one place to parse, update, audit, validate, and compile translation catalogs into runtime-ready data.

Add it with:

```bash
cargo add ferrocat
```

It re-exports the stable Rust API from `ferrocat-po` and `ferrocat-icu`.

## Feature flags

`ferrocat` enables `full` by default. `full` currently means `catalog` plus
`serde`, so the default build exposes the complete current API surface.

- `catalog`: high-level catalog parse, update, combine, conversion, audit,
  metadata, FCL, plural, and runtime compile APIs
- `serde`: serde support in the re-exported `ferrocat-po` and `ferrocat-icu`
  types
- `compile`, `mt`, and `plurals`: reserved subsystem aliases that currently
  imply `catalog`; they do not reduce or split the catalog API surface yet

Use `default-features = false` for low-level PO and ICU parsing without the
catalog layer. Enabling `compile`, `mt`, or `plurals` currently has the same
dependency effect as enabling `catalog`.

Use the umbrella crate when you want one dependency for the full catalog workflow:

- translator-friendly catalog parsing and serialization
- deterministic catalog updates, N-way combination, and deletion-aware three-way merges
- explicit ICU-native PO ↔ FCL conversion for content and files
- AI translation metadata for machine-generated entries, including stale-metadata cleanup after manual edits
- release QA through structured audit reports
- rich-message diagnostics for placeholders, plural/select logic, formatters, and tags
- policy-aware ICU apostrophe canonicalization and matching compiled-key derivation
- host-neutral runtime artifact compilation

At the catalog layer, `ferrocat` supports three explicit modes:

- classic Gettext catalog mode: Gettext PO + gettext-compatible plurals
- ICU-native Gettext PO mode: Gettext PO + ICU MessageFormat
- ICU-native FCL catalog mode: FCL (Ferrocat Catalog Lines) storage + ICU MessageFormat

`FCL + gettext-compatible plurals` is intentionally unsupported.

See the repository's [FCL format specification](../../docs/fcl-format.md) for the line format and escaping rules behind ICU-native FCL storage.

Repository, docs, and contribution guidelines:

- <https://github.com/sebastian-software/ferrocat>
- <https://docs.rs/ferrocat>

Part of the Ferramenta family of Rust-native developer tools: https://ferramenta.dev

Siblings: ferroni, ferriki, ferromark, ferrolex, ferralk, palamedes
