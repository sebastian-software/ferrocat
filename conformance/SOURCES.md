# Conformance Sources

This directory stores the first hermetic conformance snapshot for `ferrox`.

## Snapshot Policy

- Source snapshots were selected on 2026-03-16.
- Fixtures are compact semantic adaptations of upstream tests unless noted otherwise.
- Small structured expectations are stored inline in the Rust case definitions; external fixtures are reserved for realistic inputs and full text outputs.
- GNU `gettext` is intentionally excluded from phase 1.
- Current snapshot size on 2026-03-16: `55` cases / `442` assertions.

## Upstream Sources

- `izimobil/polib`
  - Upstream: <https://github.com/izimobil/polib>
  - License: MIT
  - Role: primary PO edge-case baseline
  - Snapshot here: `12` cases / `88` assertions
  - Coverage here: comment ordering, UTF-8 BOM handling, strict invalid quoting rejects, wrapping, merge semantics, and merge output semantics

- `rubenv/pofile`
  - Upstream: <https://github.com/rubenv/pofile>
  - License: MIT
  - Role: secondary JS PO parser/writer cross-check
  - Snapshot here: `30` cases / `301` assertions
  - Coverage here: multiline values, structured references, comments, contexts, obsolete entries, C-string escapes, fuzzy roundtrip, and `Plural-Forms` parsing

- `python-babel/babel`
  - Upstream: <https://github.com/python-babel/babel>
  - License: BSD-3-Clause
  - Role: targeted supplemental PO cases
  - Snapshot here: `5` cases / `32` assertions
  - Coverage here: unknown locale roundtrip, irregular multiline `msgstr`, and enclosed location parsing with structured references

- `unicode-org/icu`
  - Upstream: <https://github.com/unicode-org/icu>
  - License: Unicode License
  - Role: official MessageFormat parser reference for `ferrox-icu`
  - Snapshot here: `8` cases / `21` assertions
  - Coverage here: simple arguments, plural/selectordinal, nested tags, apostrophe escaping, and parser-visible failure cases
