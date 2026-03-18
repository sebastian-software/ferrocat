# ADR 0011: Add an NDJSON Storage Format for High-Level Catalog Workflows

- Status: Accepted
- Date: 2026-03-18

## Context

`ferrocat` already has a clean separation between:

- the low-level PO core
- the higher-level canonical catalog API

That split makes it possible to add a second storage format without rewriting the
PO hot path or forcing the public catalog model back into gettext-centric shapes.

There is also growing interest in a storage format that is:

- simpler to parse than PO
- easy to diff and stream
- friendlier to AI/pipeline-style tooling
- already close to likely translation exchange payloads

For this use case, the important semantic center is not raw PO fidelity. It is
the canonical catalog model built around:

- `msgid`
- optional `msgctxt`
- current translation value
- comments, origins, flags, and obsolete state
- ICU-oriented plural structure

## Decision

Add a second high-level catalog storage format: `CatalogStorageFormat::Ndjson`.

This format is intentionally:

- available only through the high-level catalog API
- explicit rather than inferred from file extension
- canonical-model-oriented rather than PO-roundtrip-oriented

The wire shape is:

1. a small frontmatter header between `---` lines
2. one JSON object per message line

Required frontmatter:

- `format: ferrocat.ndjson.v1`

Supported frontmatter keys in `v1`:

- `format`
- `locale`
- `source_locale`

Message records use:

- `id` for `msgid`
- `ctx` for `msgctxt`
- `str` for the translation string
- optional `comments`, `origin`, `obsolete`, and `extra`

Plural messages stay flattened as ICU strings in `id` and `str`.

We explicitly do not introduce:

- file-extension autodetect in `v1`
- a low-level borrowed NDJSON document API
- full PO header fidelity inside the NDJSON format

## Consequences

Positive:

- high-level catalog workflows can now read and write a simpler storage format
- message payloads become easier to stream, diff, batch, and hand to external tooling
- the PO core stays specialized and performance-focused
- NDJSON stays aligned with the canonical model instead of re-encoding gettext slot details

Negative:

- NDJSON is a new public file contract that must now be versioned and documented
- some PO-specific fidelity, especially arbitrary headers, is intentionally not represented
- the public catalog API now carries an explicit storage-format switch

## Alternatives Considered

### Replace PO with NDJSON

Rejected because the low-level PO parser and serializer are still core product
features with strong performance and compatibility goals.

### Infer storage format from file extension

Rejected for `v1` because explicit format choice is easier to reason about and
safer for public APIs and programmatic callers.

### Use YAML or TOML for the new high-level storage format

Rejected for this first experiment because line-delimited JSON is simpler to
parse, easier to stream, and naturally aligned with one-message-per-record
workflows.
