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

Siblings: [ferroni](https://ferroni.dev) — Oniguruma-compatible regex engine · [ferriki](https://ferriki.dev) — Shiki-compatible syntax highlighting · [ferromark](https://ferromark.dev) — Markdown to HTML, sanitized by default · [ferrolex](https://github.com/sebastian-software/ferrolex) — Spell checking for text and code · [ferralk](https://github.com/sebastian-software/ferralk) — Glob matching and parallel filesystem walking · [ferrugo](https://github.com/sebastian-software/ferrugo) — PDF previews for untrusted files · [ferrolink](https://github.com/sebastian-software/ferrolink) — Links, anchors and assets checked against the finished build · [ferrigura](https://github.com/sebastian-software/ferrigura) — Mermaid diagrams to SVG without a browser · [palamedes](https://palamedes.dev) — Internationalization for TypeScript applications · [dalo](https://dalo.sh) — Your team's agent setup, versioned like code · [ardo](https://ardo-docs.dev) — Documentation sites built with React
<!-- ferramenta-family:end -->
