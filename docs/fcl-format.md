# FCL — Ferrocat Catalog Lines (`.fcl`)

A line-oriented, machine-owned catalog format optimized for **git mergeability**
and **fast parsing**. One entry per line, deterministically sorted. It is *not*
meant for hand editing; the only non-API writer it must tolerate is git's
3-way line merge.

FCL is **ICU-native only** (like the NDJSON catalog format). Plurals live inside
the ICU message string (`{count, plural, …}`), not as separate slots. Gettext
plural-compat catalogs are not representable; use PO for those.

## Why a line format (and why it beats PO)

git's 3-way merge is purely line-textual and has no notion of an entry. PO
entries span multiple lines with many repeated anchor lines (`msgstr ""`, blank
separators), so diff3 mis-anchors and silently drops *unchanged* translations
on merge. FCL removes this by construction:

- **One entry == one line** → git can never split or interleave an entry.
- **Sorted by a stable key** → independent edits touch distant lines and
  auto-merge; only edits to the *same* entry conflict (correctly, both sides
  visible). An entry neither side touched is byte-identical in all three
  versions and therefore cannot be altered or lost.
- **Deterministic writer** → unchanged entries serialize byte-identically, so a
  regeneration never churns lines it didn't change.

It parses faster than PO because it drops PO's per-entry overhead: no multi-line
state machine, no keyword classification, no quote-bounds scanning, and a
minimal escape set with a no-escape borrow fast path.

## Grammar

```
file    = header LF *( entry LF )
header  = "%FCL1" *( HT tag )            ; e.g. %FCL1\tsource=en\tlocale=de
entry   = id HT ctxt HT target *( HT tag )
tag     = key "=" value | flag-key       ; flag-key has no '='
```

- UTF-8, no BOM, `LF` line endings, trailing `LF`.
- Fields are separated by a single horizontal tab (`HT`, `0x09`).
- `id`, `ctxt`, `target` are always present (positional). `ctxt` empty == no
  context; `target` empty == untranslated.
- Entries are sorted ascending by the byte sequence of `(id, ctxt)`.
- Tags appear in a fixed canonical order (see below); empty/absent tags are
  omitted (no trailing tabs).

### Escaping

Applied to every field and every tag value. Nothing else is escaped:

| char | escape |
|------|--------|
| `\`  | `\\`   |
| tab  | `\t`   |
| LF   | `\n`   |

A field containing no `\` is taken verbatim (zero-copy borrow on parse).

### Tags

| tag | source field | meaning | cardinality |
|-----|--------------|---------|-------------|
| `r=`       | `origin` (`CatalogOrigin`) | source reference `file` or `file:line` | 0..n |
| `c=`       | `comments`                 | extracted comment (`#.` in PO)         | 0..n |
| `tc=`      | `extra.translator_comments`| translator comment (`#` in PO)         | 0..n |
| `f=`       | `extra.flags`              | a PO flag (`c-format`, …)              | 0..n |
| `o`        | `obsolete`                 | obsolete marker (flag, no value)       | 0..1 |
| `mt.model=`| `machine_translation.model`| MT model id                            | 0..1 |
| `mt.conf=` | `machine_translation.confidence` | MT confidence 0..100             | 0..1 |
| `mt.hash=` | `machine_translation.hash` | MT change-detection hash               | 0..1 |

Canonical tag order: `r` (sorted), `c`, `tc`, `f` (sorted), `o`, `mt.model`,
`mt.conf`, `mt.hash`.

**Deliberately omitted:** `machine_translation.modified` (a timestamp).
Timestamps churn every line on regeneration and poison merges; change/staleness
detection is hash-based (`mt.hash`). FCL is therefore lossy for `modified` by
design.

## Robustness

- A line beginning with a git conflict marker (`<<<<<<<`, `=======`, `>>>>>>>`)
  is a hard parse error with position — never silently mis-parsed.
- Duplicate `(id, ctxt)` (adjacent after sort) is a hard error.
- Unknown tag keys are a hard error (the versioned `%FCL1` magic gates
  forward-compatible additions).

## Relationship to the other formats

- **FCL** — canonical, git-merged, fast machine artifact. Treat as generated
  (`.gitattributes linguist-generated`); do not hand-edit.
- **PO** — gettext interop / human-readable export.
- **NDJSON** — superseded by FCL for the machine/merge role (FCL is faster,
  more compact, and merges as well while needing no JSON tooling). Slated for
  removal once FCL is established.
