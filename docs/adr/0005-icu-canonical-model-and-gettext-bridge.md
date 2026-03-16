# ADR 0005: Treat ICU as the Canonical Model and Gettext as a Compatibility Bridge

- Status: Accepted
- Date: 2026-03-16

## Context

`ferrocat` now has a high-level catalog API layered on top of the PO core:

- `parse_catalog`
- `update_catalog`
- `update_catalog_file`

That API needs to handle two overlapping, but not identical, plural worlds:

- ICU/CLDR-style plural categories and message structure
- gettext `Plural-Forms`, `msgid_plural`, and `msgstr[n]`

These are related, but they are not the same semantic model.

ICU/MessageFormat is the richer long-term target:

- structured plurals and selects
- better future fit for modern i18n workflows
- a stronger basis for validation and later compiler work

Gettext remains important because real projects still need to:

- read and update existing `.po` catalogs
- migrate from gettext-style plurals toward ICU
- export back into gettext-based toolchains when required

The project already uses `icu_plurals` for locale-aware plural categories and now has a conservative `PluralProfile` bridge for gettext slot ordering and `Plural-Forms` handling.

## Decision

Treat ICU/MessageFormat v1 as the canonical internal model.

Treat gettext as a compatibility bridge around that model.

Concretely:

- `PluralEncoding::Icu` remains the default for the high-level API
- internal message projection should prefer ICU-oriented structure
- gettext import and export remain supported, but conservatively
- existing `Plural-Forms` metadata should be respected where possible
- automatic gettext header generation should only happen for clearly safe cases
- unclear, lossy, or mismatched gettext plural situations should produce diagnostics instead of speculative rewrites

We explicitly do not make full gettext plural/header parity a short-term architectural goal.

Instead, the next major semantic milestone after stabilizing the bridge is a real `ferrocat-icu` MessageFormat v1 parser.

## Consequences

Positive:

- the long-term semantic center of the library is clearer
- ICU-focused future work has a cleaner foundation
- gettext support stays useful without dominating internal design
- diagnostics become the preferred tool for bridge ambiguity

Negative:

- gettext support is intentionally "good and conservative", not maximal historical parity
- some locales or headers will remain partially manual instead of fully auto-generated
- roundtrip fidelity for gettext edge cases depends more on existing metadata than on aggressive inference
