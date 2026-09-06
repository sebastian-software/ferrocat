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
**ferrocat** is part of the [Ferramenta](https://ferramenta.dev) family — Rust-native developer tools that keep the APIs the ecosystem already knows.

Siblings: [ferroni](https://sebastian-software.github.io/ferroni/) · [ferriki](https://github.com/sebastian-software/ferriki) · [ferromark](https://sebastian-software.github.io/ferromark/) · [ferrolex](https://github.com/sebastian-software/ferrolex) · [palamedes](https://palamedes.dev) · [ferrovia](https://github.com/sebastian-software/ferrovia) · [ferralk](https://github.com/sebastian-software/ferralk) · [ferrugo](https://github.com/sebastian-software/ferrugo).
<!-- ferramenta-family:end -->
