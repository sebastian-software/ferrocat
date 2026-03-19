# ADR 0004: Model `msgstr` by Semantics Instead of Always Using `Vec<String>`

- Status: Accepted
- Date: 2026-03-15

## Context

Most PO entries are singular.

Representing every translation set as a `Vec<String>` imposes avoidable overhead:

- per-item vector allocation pressure
- extra branching and resizing for the common singular case
- less direct expression of PO semantics

## Decision

Represent `msgstr` with semantic variants:

- `None`
- `Singular`
- `Plural`

The owned model uses `MsgStr`, and the borrowed model uses `BorrowedMsgStr<'a>`.

## Consequences

Positive:

- better fit for actual PO structure
- lower overhead for common singular items
- cleaner basis for both owned and borrowed parsing

Negative:

- API changed from a plain `Vec<String>` shape
- serializer and parser logic need variant-aware handling
- interoperability code may need small adjustments
