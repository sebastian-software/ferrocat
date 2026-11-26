# ferrocat

[![crates.io](https://img.shields.io/crates/v/ferrocat.svg)](https://crates.io/crates/ferrocat)
[![docs.rs](https://img.shields.io/docsrs/ferrocat)](https://docs.rs/ferrocat)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/sebastian-software/ferrocat/blob/main/LICENSE-MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.94.0-blue.svg)](https://github.com/sebastian-software/ferrocat#compatibility-snapshot)

Public umbrella crate for the `ferrocat` workspace.

Documentation, interactive examples, and benchmarks: **[ferrocat.dev](https://ferrocat.dev)**.

Ferrocat is a Rust-native translation catalog engine. It gives applications and host-language adapters one place to parse, update, review, audit, validate, and compile Gettext PO, ICU MessageFormat, and FCL catalogs into runtime-ready payloads — at 1218 MiB/s borrowed PO parsing and 410 MiB/s plain catalog updates on the reference 10k-message benchmark.

Add it with:

```bash
cargo add ferrocat
```

It re-exports the stable Rust API from `ferrocat-po` and `ferrocat-icu` under the namespaced modules `ferrocat::po`, `ferrocat::catalog`, `ferrocat::icu`, and `ferrocat::collation` (plus top-level re-exports).

## Quick start

```rust
use ferrocat::catalog::{
    CatalogAuditOptions, ParseCatalogOptions, audit_catalogs, parse_catalog_for_review,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = parse_catalog_for_review(
        ParseCatalogOptions::new(
            "msgid \"Hello {name}\"\nmsgstr \"Hello {name}\"\n",
            "en",
        )
        .with_locale("en"),
    )?;
    let target = parse_catalog_for_review(
        ParseCatalogOptions::new("msgid \"Hello {name}\"\nmsgstr \"Hallo\"\n", "en")
            .with_locale("de"),
    )?;

    let report = audit_catalogs(&[&source, &target], &CatalogAuditOptions::new("en"))?;
    assert!(report.has_errors()); // missing `{name}` placeholder in German translation
    Ok(())
}
```

## Feature flags

`ferrocat` enables `full` by default. `full` currently means `catalog` plus
`serde`, so the default build exposes the complete current API surface.

- `catalog`: high-level catalog parse, update, combine, conversion, audit,
  metadata, FCL, plural, and runtime compile APIs
- `collation`: compact CLDR root comparison keys through `ferrocat::collation`
  without catalog-layer dependencies (`catalog` also enables it)
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

- <https://ferrocat.dev>
- <https://github.com/sebastian-software/ferrocat>
- <https://docs.rs/ferrocat>

<!-- ferramenta-family:start -->
**ferrocat** is part of the [Ferramenta](https://ferramenta.dev) family — A family of Rust tools.

Siblings: [ferroni](https://ferroni.dev) — Oniguruma-compatible regex engine · [ferriki](https://ferriki.dev) — Shiki-compatible syntax highlighting · [ferromark](https://ferromark.dev) — Markdown to HTML, sanitized by default · [ferrolex](https://ferrolex.dev) — Spell checking for text and code · [ferralk](https://ferralk.dev) — Glob matching and parallel filesystem walking · [ferrugo](https://sebastian-software.github.io/ferrugo/) — PDF previews for untrusted files · [ferrolink](https://ferrolink.dev) — Links, anchors and assets checked against the finished build · [ferrigura](https://ferrigura.dev) — Mermaid diagrams to SVG without a browser · [palamedes](https://palamedes.dev) — Internationalization for TypeScript applications · [dalo](https://dalo.sh) — Your team's agent setup, versioned like code · [ardo](https://ardo-docs.dev) — Documentation sites built with React
<!-- ferramenta-family:end -->
