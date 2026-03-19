# ADR 0009: Use Versioned, Truncated SHA-256 Keys for Runtime Catalog Compilation

## Status

Accepted

## Context

`ferrocat` now exposes a runtime-oriented catalog compilation step above `parse_catalog` and `NormalizedParsedCatalog`.

That layer needs a stable derived key for runtime lookup maps:

- source identity remains gettext-native: `msgid + msgctxt`
- runtime identity should be compact and opaque
- downstream tools need a deterministic contract they can reproduce outside Rust
- frontend bundle size matters, so key length should stay small

At the same time, the key format must avoid silently papering over collisions or mixing multiple key schemes without a compatibility story.

## Decision

`ferrocat` uses a single built-in key strategy for the first compile API:

- strategy name: `CompiledKeyStrategy::FerrocatV1`
- hash function: SHA-256
- input: a versioned, length-delimited payload derived from `msgctxt` and `msgid`
- versioning: included in the hash input as domain separation, not exposed as a visible key prefix
- output: the first 64 bits of the SHA-256 digest
- encoding: unpadded Base64URL

This yields compact ASCII-safe runtime keys of 11 characters.

The same default key contract is also exposed publicly through a small helper
that accepts `msgid` and optional `msgctxt`, so downstream transforms and host
adapters can derive the exact same runtime identity without reimplementing the
algorithm locally.

Collision handling is strict:

- if two distinct source identities produce the same compiled key, compilation fails
- `ferrocat` does not auto-extend, overwrite, or silently continue

The compile API also defaults to no source fallback:

- runtime compilation should not silently replace missing translations with source text
- if a caller wants source-locale fallback behavior, it must be requested explicitly

## Consequences

Positive:

- keys are short enough for runtime bundles and generated artifacts
- the format is easy to reproduce in other ecosystems
- no visible version prefix wastes output bytes
- SHA-256 is a familiar and low-surprise choice for downstream implementers
- hard collision failure keeps the contract trustworthy

Negative:

- keys are opaque and not intended for human inspection
- truncating to 64 bits accepts a very small theoretical collision risk
- callers that want fallback-filled runtime artifacts must opt in explicitly

## Alternatives Considered

### Visible version prefixes such as `fc1_`

Rejected because they spend bytes on every emitted key and mostly help debugging rather than correctness. The version still exists, but only inside the hashed input.

### Longer hashes such as 96 or 128 bits

Rejected for `v1` because they increase bundle and artifact size without much practical benefit for the expected catalog sizes.

### Shorter hashes such as 32 or 48 bits

Rejected as the default because they raise collision probability more aggressively than needed. `64` bits is the chosen middle ground.

### Non-ASCII or higher-base encodings

Rejected because UTF-8 byte size, escaping behavior, and tooling portability are worse than a conservative ASCII-safe Base64URL output.

### Non-cryptographic hashes such as FNV

Rejected because SHA-256 is easier to describe, validate, and reproduce across ecosystems while still being cheap enough for this non-hot-path compile step.
