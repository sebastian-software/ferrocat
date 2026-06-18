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

Exit codes:

- `0`: audit completed with no error diagnostics
- `1`: audit completed and reported at least one error diagnostic
- `2`: command usage, I/O, parse, or serialization failure

Use `--format json` when CI should consume the structured
`CatalogAuditReport`.
