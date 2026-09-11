# ferrocat-cli

Command-line interface for Ferrocat catalog workflows.

The first supported workflow is `ferrocat audit`, a CI-oriented release gate
for source and target catalogs:

```bash
ferrocat audit \
  --source-locale en \
  --source locales/en.po \
  --target de=locales/de.po \
  --format text
```

`ferrocat-cli` releases include a prebuilt `x86_64-unknown-linux-musl` archive
named `ferrocat-<version>-x86_64-unknown-linux-musl.tar.gz`. The release
workflow smoke-tests the packaged `ferrocat` binary before uploading it.

Exit codes:

- `0`: audit completed with no error diagnostics
- `1`: audit completed and reported at least one error diagnostic
- `2`: command usage, I/O, parse, or serialization failure

Use `--format json` when CI should consume the structured
`CatalogAuditReport`.

<!-- ferramenta-family:start -->
**ferrocat** is part of the [Ferramenta](https://ferramenta.dev) family — A family of Rust tools.

Siblings: [ferroni](https://sebastian-software.github.io/ferroni/) — Oniguruma-compatible regex engine · [ferriki](https://github.com/sebastian-software/ferriki) — Shiki-compatible syntax highlighting · [ferromark](https://sebastian-software.github.io/ferromark/) — Markdown to HTML with a secure default and every GFM extension included. · [ferrolex](https://github.com/sebastian-software/ferrolex) — Spell checking for text and code · [palamedes](https://palamedes.dev) — Internationalization for TypeScript applications · [ferrovia](https://github.com/sebastian-software/ferrovia) — SVGO-compatible SVG optimizer · [ferralk](https://github.com/sebastian-software/ferralk) — Glob matching and parallel filesystem walking · [ferrugo](https://github.com/sebastian-software/ferrugo) — PDF previews for untrusted files.
<!-- ferramenta-family:end -->
