# Agent Rules

These rules apply to automated coding agents working in this repository.

## Commit Messages

- Always use Conventional Commits for any commit you create.
- Treat this as a release requirement, not a style preference: `release-please` depends on commit prefixes such as `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `perf:`, `style:`, or `chore:`.
- Prefer the format `type(scope): summary` when the scope adds clarity.
- Before pushing rewritten history or new commits, double-check the final `git log --oneline` output to confirm the prefixes are correct.

Examples:

- `feat(po): add compiled catalog runtime API`
- `fix(api): avoid panic on poisoned plural cache`
- `docs(adr): document compiled key strategy`
- `chore(bench): stabilize benchmark compare measurements`

## Verification

- Keep the existing Rust hook expectations green before push:
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
