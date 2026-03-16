# Conformance

`ferrox` now carries a hermetic conformance snapshot under [`/Users/sebastian/Workspace/ferrox/conformance`](/Users/sebastian/Workspace/ferrox/conformance).

Phase 1 intentionally excludes GNU `gettext`. The current snapshot uses:

- `izimobil/polib` as the primary PO edge-case baseline
- `rubenv/pofile` as a secondary JS-oriented PO cross-check
- Babel as a targeted PO supplement
- the official ICU MessageFormat tests as the parser reference for `ferrox-icu`

## Current Counts

Current snapshot totals as of 2026-03-16:

- `56` source-attributed conformance cases
- `423` concrete assertions checked by the harness
- `45` expected passes
- `4` expected rejects
- `7` documented `known_gap` cases

Per suite:

- `po-pofile`: `30` cases / `301` assertions
- `po-polib`: `13` cases / `78` assertions
- `po-babel`: `5` cases / `23` assertions
- `icu-official`: `8` cases / `21` assertions

The case count tracks individually addressable upstream-derived scenarios. The assertion count tracks the concrete field- and structure-level comparisons performed by the harness, which is the better number to use when communicating weight and breadth.

## Snapshot Scope

- `po-polib`: comment ordering, wrapping, invalid quoting, merge semantics, merge output parsing, and known gaps for previous-message history and UTF-8 BOM handling
- `po-pofile`: multiline values, references, comments, contexts, obsolete entries, C-string escapes, fuzzy roundtrip, and `Plural-Forms`
- `po-babel`: unknown locale roundtrip, irregular multiline `msgstr`, enclosed location parsing, and a known gap for structured location splitting
- `icu-official`: simple arguments, plural/selectordinal, nested tags, apostrophe escaping, and parser-visible failure cases

## Local Coverage Mapping

Existing local tests still provide broad regression coverage in:

- `parse`, `serialize`, `merge`, and `api` behavior inside `ferrox-po`
- parser and utility behavior inside `ferrox-icu`

The conformance layer is intentionally narrower and source-attributed. It exists to answer a different question: whether `ferrox` matches independently maintained reference behavior on representative upstream cases.

## Scoreboard

Use:

```bash
cargo test --workspace
cargo run -p ferrox-bench -- conformance-report
```

The report prints totals per suite and capability, broken down into `pass`, `reject`, and `known_gap`.

It also prints assertion totals, so we can talk about both "how many source-attributed cases" and "how many concrete checks" without inflating fixture counts.

Known gaps are counted and documented, but they do not fail CI.

## Phase 1 Exclusion

GNU `gettext` is not part of the phase 1 scoreboard. The main reason is repository hygiene: its tests are powerful, but much harder to adopt hermetically without either GPL test vendoring or a much heavier adaptation layer. The current snapshot is intentionally built from MIT/BSD/Unicode-licensed sources first.
