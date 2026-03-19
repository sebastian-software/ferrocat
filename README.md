# ferrocat

[![crates.io](https://img.shields.io/crates/v/ferrocat.svg)](https://crates.io/crates/ferrocat)
[![docs.rs](https://img.shields.io/docsrs/ferrocat)](https://docs.rs/ferrocat)
[![CI](https://github.com/sebastian-software/ferrocat/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sebastian-software/ferrocat/actions/workflows/ci.yml)
[![codecov](https://codecov.io/github/sebastian-software/ferrocat/graph/badge.svg?branch=main)](https://app.codecov.io/github/sebastian-software/ferrocat)

`ferrocat` is a performance-first toolkit for translation catalogs that need to span classic GNU gettext PO workflows, ICU MessageFormat semantics, and JSON-friendly runtime delivery.

The canonical documentation now lives on the docs site:

- [Docs homepage](https://sebastian-software.github.io/ferrocat/)
- [Getting started](https://sebastian-software.github.io/ferrocat/guide/getting-started)
- [Catalog modes](https://sebastian-software.github.io/ferrocat/guide/catalog-modes)
- [API overview](https://sebastian-software.github.io/ferrocat/reference/api-overview)
- [Performance docs](https://sebastian-software.github.io/ferrocat/performance)
- [ADR index](https://sebastian-software.github.io/ferrocat/architecture/adr)

## Install

```bash
cargo add ferrocat
```

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

## Core Links

- [docs.rs crate docs](https://docs.rs/ferrocat)
- [GitHub repository](https://github.com/sebastian-software/ferrocat)
- [Contributing guide](https://github.com/sebastian-software/ferrocat/blob/main/CONTRIBUTING.md)
- [Security policy](https://github.com/sebastian-software/ferrocat/blob/main/SECURITY.md)
- [Code of Conduct](https://github.com/sebastian-software/ferrocat/blob/main/CODE_OF_CONDUCT.md)
