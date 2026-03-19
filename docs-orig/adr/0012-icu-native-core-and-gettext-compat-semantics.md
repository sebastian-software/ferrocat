# ADR 0012: Make the High-Level Catalog API ICU-Native by Default and Move Gettext Plurals Behind an Explicit Compat Mode

- Status: Accepted
- Date: 2026-03-18

## Context

The high-level catalog API had gradually accumulated two different semantic models:

- an ICU-oriented internal view
- a classic gettext plural bridge

In practice this meant the parse and update hot paths tried to eagerly project
ICU plural strings into structured plural data while also accepting classic
`msgid_plural` / `msgstr[n]` PO input.

That mixed model had three costs:

- unnecessary parse-time work for the common ICU-native case
- harder-to-reason-about semantics for downstream compile/runtime code
- increasing pressure to handle more mixed ICU/gettext edge cases inside one path

The introduction of NDJSON as a native catalog storage format made the split
more obvious: NDJSON is naturally ICU/text-first, while classic gettext plural
slots are a separate compatibility concern.

## Decision

The high-level catalog API now exposes two explicit semantic modes:

- `CatalogSemantics::IcuNative` as the default
- `CatalogSemantics::GettextCompat` as the explicit PO interoperability mode

This is a semantic split, not just a formatting option.

The public contracts are:

- `CatalogSemantics::IcuNative` requires `PluralEncoding::Icu`
- `CatalogSemantics::GettextCompat` requires `PluralEncoding::Gettext`
- `CatalogStorageFormat::Ndjson` is only supported with `CatalogSemantics::IcuNative`
- invalid combinations are rejected with `ApiError::InvalidArguments` or `ApiError::Unsupported`

Behavior by mode:

### `IcuNative`

- PO and NDJSON parse `msgid` / `msgstr` or `id` / `str` directly as text
- top-level ICU plurals are no longer eagerly projected into `TranslationShape::Plural`
- `CatalogUpdateInput::SourceFirst` stays source-text-first and does not auto-project ICU plurals
- PO write emits raw ICU/text strings and never writes `msgid_plural`
- `NormalizedParsedCatalog::compile` produces singular runtime strings for native ICU messages

### `GettextCompat`

- PO parse accepts classic `msgid_plural` / `msgstr[n]`
- PO write emits classic gettext plural slots
- ICU projection is not part of this compat parse path
- NDJSON is not supported
- `NormalizedParsedCatalog::compile` can still return structured plural runtime values

`compile_catalog_artifact` remains a string artifact API. Because of that,
`GettextCompat` is allowed to bridge plural structure to final ICU strings only
at the artifact boundary.

## Consequences

Positive:

- the default high-level path is simpler and cheaper
- native ICU workflows no longer pay for eager plural projection
- NDJSON and native PO storage now share one clearer semantic model
- compat behavior is explicit instead of being mixed into the default path

Negative:

- this is a public behavior change for callers that previously relied on eager
  ICU plural projection in `parse_catalog`
- callers that want classic gettext plural semantics must now opt into
  `CatalogSemantics::GettextCompat`
- some tests, benchmarks, and tooling need to pass semantics explicitly

## Alternatives Considered

### Keep one high-level path with more conditional logic

Rejected because it would preserve the mixed-model complexity and make both
performance work and semantics harder to reason about.

### Continue eager ICU plural projection in the default path

Rejected because the common ICU-native workflow benefits more from keeping raw
text intact and projecting only on demand.

### Make compat mode only a write option

Rejected because parse, update, and compile semantics also differ materially;
the split needs to exist throughout the high-level API, not only at export.
