# ADR 0007: Do Not Support `previous_msgid` History

- Status: Accepted
- Date: 2026-03-16

## Context

GNU gettext toolchains can attach previous-source history to PO entries by writing comment-style lines such as:

```po
#| msgctxt "Old menu context"
#| msgid "Old file label"
msgctxt "menu"
msgid "File"
msgstr "Datei"
```

These lines are useful in classic translator-centered merge workflows because they preserve a hint about the previous source text after fuzzy matching or `msgmerge` updates.

`ferrox` is intentionally optimizing for a different center of gravity:

- current source text is the canonical identity surface
- memory efficiency and hot-path performance matter more than preserving historical merge metadata
- ICU-oriented and model-assisted localization workflows are the long-term target
- gettext remains a compatibility bridge, not the architectural center

In that product direction, `previous_msgid` history is legacy workflow metadata rather than core message semantics.

## Decision

`ferrox` will not model, preserve, or roundtrip `previous_msgid` / previous-`msgctxt` history.

Concretely:

- the PO parser may ignore `#| ...` history lines
- the PO data model will not gain dedicated fields for previous-source history
- serializing a parsed PO file will drop that history
- conformance reporting should treat this as intentionally unsupported, not as a future-compatibility gap

We still parse the current active PO item that follows such history lines.

## Consequences

Positive:

- simpler and smaller PO item model
- less parser and serializer complexity
- no extra memory cost for metadata that is not part of the current semantic source of truth
- conformance metrics stay aligned with the product direction instead of rewarding legacy fidelity for its own sake

Negative:

- `ferrox` is not a lossless roundtrip engine for PO files that rely on previous-source history
- users migrating from traditional gettext workflows may lose those hints after rewrite
- full historical parity with tools like `msgmerge` is explicitly out of scope

If demand appears later, this can be revisited as an optional compatibility layer rather than a default core feature.
