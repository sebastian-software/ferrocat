# Contributing

## Git Hooks

This repository ships tracked Git hooks under `.githooks/`.

Enable it once per clone with:

```bash
git config core.hooksPath .githooks
```

`pre-commit` runs for staged Rust or Cargo-related changes.

`pre-push` always runs for the full workspace before a push.

Both hooks mirror the Rust linting commands from CI:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Commit Messages

This repository uses Conventional Commits.

Use the format:

```text
type(scope): summary
```

Examples:

- `feat(conformance): add reporting command`
- `fix(po): support UTF-8 BOM in parsing`
- `refactor(conformance): move cases into Rust modules`
- `docs(adr): declare headerless PO normalization`
- `test(po): cover invalid quote rejection`

Guidelines:

- use lowercase commit types such as `feat`, `fix`, `refactor`, `docs`, `test`, `perf`, `style`, or `chore`
- keep the summary imperative and concise
- add a scope when it clarifies the affected area
- prefer one logical change per commit
