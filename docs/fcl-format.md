# FCL — Ferrocat Catalog Lines (`.fcl`)

A line-oriented, machine-owned catalog format. You don't have to trade speed for
safety to get it: compared with the same catalog stored as PO, FCL parses about
25% faster, takes roughly 12% less disk, and gives git one canonical line per
entry so ordinary 3-way merges preserve untouched translations. One entry per
line, deterministically sorted. It is *not* meant for hand editing; the only
non-API writer it must tolerate is git's 3-way line merge.

FCL is **ICU-native only**. Plurals live inside the ICU message string
(`{count, plural, …}`), not as separate slots. Gettext plural-compat catalogs
are not representable; use PO for those.

Generate FCL through the catalog layer with `CatalogMode::IcuFcl` in
`parse_catalog`, `update_catalog`, or file-based update flows. Keep PO for
translator-facing files when external tools need gettext compatibility.

## Why a line format (and why it beats PO)

git's 3-way merge is purely line-textual and has no notion of an entry. PO
entries span multiple lines with many repeated anchor lines (`msgstr ""`, blank
separators), so diff3 mis-anchors and silently drops *unchanged* translations
on merge. FCL removes this by construction:

- **One entry == one line** → git can never split or interleave an entry.
- **Sorted by a stable key** → independent edits touch distant lines and
  auto-merge; only edits to the *same* entry conflict (correctly, both sides
  visible). An entry neither side touched is byte-identical in all three
  versions, so ordinary 3-way merges preserve it.
- **Deterministic writer** → unchanged entries serialize byte-identically, so a
  regeneration never churns lines it didn't change.

It parses faster than PO because it drops PO's per-entry overhead: no multi-line
state machine, no keyword classification, no quote-bounds scanning, and a
minimal escape set with a no-escape borrow fast path. The reader works on bytes
and leans on memchr (NEON on Apple Silicon) for field splitting and escape
scanning, the same way the PO parser is tuned. On a 10k-message ICU catalog that
lands around 25% faster than reading the equivalent PO file.

## Grammar

```
file    = header LF *( entry LF )
header  = "%FCL1" *( HT tag )            ; e.g. %FCL1\tsource=en\tlocale=de\torder=collated
entry   = id HT ctxt HT target *( HT tag )
tag     = key "=" value | flag-key       ; flag-key has no '='
```

- UTF-8, no BOM, `LF` line endings, trailing `LF`.
- Fields are separated by a single horizontal tab (`HT`, `0x09`).
- `id`, `ctxt`, `target` are always present (positional). `ctxt` empty == no
  context; `target` empty == untranslated.
- Newly written entries follow CLDR root order by `id` and then `ctxt`, declared
  as `order=collated`. Readers accept legacy files with no `order` tag only
  when they ascend by the byte sequence of `(id, ctxt)`.
- Tags appear in a fixed canonical order (see below); empty/absent tags are
  omitted (no trailing tabs).

### Header tags

| tag | required | meaning |
|---|---|---|
| `source=` | writer-required | source locale used by the catalog |
| `locale=` | optional | target locale |
| `order=collated` | writer-required | use the CLDR root order described in [ADR 0026](app/routes/architecture/adr/0026-cldr-root-catalog-order.mdx) |

The writer always emits `order=collated`. Omitting `order` identifies a legacy
file under the original bytewise `(id, ctxt)` contract and remains accepted on
read so existing files can be migrated by the next write. The only accepted
explicit value in `%FCL1` is `collated`; unknown and duplicate `order` tags are
hard errors. The collated table covers Latin text, punctuation, symbols, and
digits. Its declared ligature/digraph and out-of-repertoire limits are
documented in ADR 0026.

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
| `r=`       | `origin` (`CatalogOrigin`) | source reference `file` or `file#scope` (no line numbers) | 0..n |
| `c=`       | `comments`                 | extractor-owned note (`#.` in PO)      | 0..n |
| `tc=`      | `translator_comments`      | translator-owned note (`#` in PO)      | 0..n |
| `f=`       | `flags`                    | one opaque per-entry flag (`#,` in PO) | 0..n |
| `o`        | `obsolete`                 | obsolete marker (flag, no value)       | 0..1 |
| `lock=`    | `machine.lock`             | integrity hash; presence marks the value as machine-managed | 0..1 |
| `ai=`      | `machine.ai`               | AI provenance, `model[:confidence]`    | 0..1 |

Canonical tag order: `r` (sorted), `c`, `tc`, `f`, `o`, `lock`, `ai`.

`r` carries the file and an optional stable `#scope` anchor. The file identifies
the source file; the scope identifies the nearest stable named source container.
Use names a developer would recognize in source code:

| Origin | Typical scope |
|---|---|
| `src/App.tsx#CheckoutButton` | component name |
| `src/i18n.ts#formatInvoiceStatus` | function name |
| `src/routes/settings.tsx#SettingsPage` | route component or route handler |
| `src/domain/invoice.ts#InvoiceStatus` | class, enum, or module-level authoring unit |

Scope is metadata for review and tooling, not message identity, and it is not a
replacement for gettext context / `msgctxt`; downstream tools should not derive
it from `msgctxt` by default. See
[ADR 0024](app/routes/architecture/adr/0024-origin-scope-anchor.mdx).

`c`, `tc`, and `f` all hold free-form values for round-tripping. The split
exists so the gettext comment kinds and per-entry flags can be written back the
way they arrived: `c` is extractor-owned and gets refreshed by an update, while
`tc` and `f` are translator-owned and are preserved verbatim against the entry
identity. The opt-in review projection recognizes exact `fuzzy` as a
review-needed status; unknown flags such as `x-custom` remain semantically
opaque and round-trip unchanged (see
[ADR 0023](app/routes/architecture/adr/0023-drop-gettext-flags-merge-comments.mdx)
and [ADR 0028](app/routes/architecture/adr/0028-fuzzy-review-state-projection.mdx)).

`lock` is the fingerprint of the value when a machine (AI engine, TMS, script)
set it; if `hash(current value) != lock`, a human edited it and high-level
writers drop the block. `ai` is optional provenance: an opaque `model` id, then
an optional `:confidence` decimal in `[0, 1]`. The model id is free-form and may
contain `/` or `:`; only a trailing `[0, 1]` suffix after the last `:` is read as
confidence (see [ADR 0022](app/routes/architecture/adr/0022-machine-managed-value-integrity-and-ai-provenance.mdx)).

**Deliberately omitted:** a machine-translation timestamp. Timestamps churn every
line on regeneration and poison merges; staleness is detected by the `lock` hash
instead.

**Also omitted: line numbers.** References carry the file only. A line number
shifts whenever anything above a message changes, so it churns diffs and merges
without identifying anything the `(id, ctxt)` key does not. This is a
catalog-layer decision shared with PO output; the low-level `parse_po` /
`stringify_po` round-trip stays faithful to whatever references a PO file holds.

## Robustness

- A line beginning with a git conflict marker (`<<<<<<<`, `=======`, `>>>>>>>`)
  is a hard parse error with position — never silently mis-parsed.
- Duplicate `(id, ctxt)` (adjacent under either supported order) is a hard
  parse error. The shared FCL export boundary also rejects duplicate serialized
  identities with `ApiError::Conflict`, so `update_catalog`, catalog conversion,
  and catalog combine cannot emit an FCL file that its reader would reject.
- Structured plural messages use their synthesized ICU source string as the
  serialized `id`. A literal singular ID that matches that string is therefore
  a collision, just like active and obsolete messages that share an identity.
- Entries that violate the legacy bytewise order or declared collated order are
  a hard error.
- Duplicate singleton tags and tags outside canonical order are hard errors.
- Unknown tag keys are a hard error (the versioned `%FCL1` magic gates
  forward-compatible additions).

File-based catalog workflows finish this validation and render the complete FCL
content before atomically replacing the destination. An identity conflict
therefore leaves an existing destination unchanged.

Entry tags are additive the same way the `order=collated` header tag was: a
reader that knows a tag accepts files with and without it, but a file that
*carries* a newer tag is rejected by an older reader, because unknown keys are
an error by design. `tc=` and `f=` were added this way; a file written without
them parses under any reader that understands the rest of `%FCL1`.

## FCL vs PO

- **FCL** — canonical, git-merged, fast machine artifact. Treat as generated
  (`.gitattributes linguist-generated`); do not hand-edit.
- **PO** — gettext interop / human-readable export.

Pick FCL for the machine/merge role and PO when a translator tool reads the file
directly. The two cover different jobs; you don't have to choose one globally.
