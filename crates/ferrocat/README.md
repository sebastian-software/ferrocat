# ferrocat

[![crates.io](https://img.shields.io/crates/v/ferrocat.svg)](https://crates.io/crates/ferrocat)
[![docs.rs](https://img.shields.io/docsrs/ferrocat)](https://docs.rs/ferrocat)

Public umbrella crate for the `ferrocat` workspace.

Add it with:

```bash
cargo add ferrocat
```

It re-exports the stable Rust API from `ferrocat-po` and `ferrocat-icu`.

At the catalog layer, `ferrocat` supports three explicit modes:

- classic Gettext catalog mode: Gettext PO + gettext-compatible plurals
- ICU-native Gettext PO mode: Gettext PO + ICU MessageFormat
- ICU-native NDJSON catalog mode: NDJSON catalog storage + ICU MessageFormat

`NDJSON + gettext-compatible plurals` is intentionally unsupported.

Use it when you want one dependency for:

- Gettext PO parsing and serialization
- catalog normalization, updating, runtime compilation, and requested-locale artifact generation
- ICU `MessageFormat` parsing and inspection

Repository, docs, and contribution guidelines:

- <https://github.com/sebastian-software/ferrocat>
- <https://docs.rs/ferrocat>
