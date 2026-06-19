# ferrocat

[![crates.io](https://img.shields.io/crates/v/ferrocat.svg)](https://crates.io/crates/ferrocat)
[![docs.rs](https://img.shields.io/docsrs/ferrocat)](https://docs.rs/ferrocat)

Public umbrella crate for the `ferrocat` workspace.

Ferrocat is a Rust-native translation catalog engine. It gives applications and host-language adapters one place to parse, update, audit, validate, and compile translation catalogs into runtime-ready data.

Patch releases may also carry workspace dependency and release-maintenance refreshes that keep the published crate set reproducible.

Add it with:

```bash
cargo add ferrocat
```

It re-exports the stable Rust API from `ferrocat-po` and `ferrocat-icu`.

Use the umbrella crate when you want one dependency for the full catalog workflow:

- translator-friendly catalog parsing and serialization
- deterministic catalog updates and combination
- AI translation metadata for machine-generated entries, including stale-metadata cleanup after manual edits
- release QA through structured audit reports
- rich-message diagnostics for placeholders, plural/select logic, formatters, and tags
- host-neutral runtime artifact compilation

At the catalog layer, `ferrocat` supports three explicit modes:

- classic Gettext catalog mode: Gettext PO + gettext-compatible plurals
- ICU-native Gettext PO mode: Gettext PO + ICU MessageFormat
- ICU-native NDJSON catalog mode: NDJSON catalog storage + ICU MessageFormat

`NDJSON + gettext-compatible plurals` is intentionally unsupported.

Repository, docs, and contribution guidelines:

- <https://github.com/sebastian-software/ferrocat>
- <https://docs.rs/ferrocat>
