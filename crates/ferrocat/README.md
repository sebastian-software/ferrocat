# ferrocat

[![crates.io](https://img.shields.io/crates/v/ferrocat.svg)](https://crates.io/crates/ferrocat)
[![docs.rs](https://img.shields.io/docsrs/ferrocat)](https://docs.rs/ferrocat)

Public umbrella crate for the `ferrocat` workspace.

Add it with:

```bash
cargo add ferrocat
```

It re-exports the stable Rust API from `ferrocat-po` and `ferrocat-icu`.

Use it when you want one dependency for:

- PO parsing and serialization
- catalog normalization, updating, and runtime compilation
- ICU `MessageFormat` parsing and inspection

Repository, docs, and contribution guidelines:

- <https://github.com/sebastian-software/ferrocat>
- <https://docs.rs/ferrocat>
