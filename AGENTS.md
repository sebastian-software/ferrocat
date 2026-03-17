# Agent Rules

These rules apply to automated coding agents working in this repository.

## Commit Messages

- Always use Conventional Commits for any commit you create.
- Treat this as a release requirement, not a style preference: `release-please` depends on commit prefixes such as `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `perf:`, `style:`, or `chore:`.
- Prefer the format `type(scope): summary` when the scope adds clarity.
- Before pushing rewritten history or new commits, double-check the final `git log --oneline` output to confirm the prefixes are correct.
- If you rewrite pushed history, use `git push --force-with-lease`, never plain `--force`.
- If you rewrite commits that were already referenced in issues, PRs, or notes, update those references to the new commit hashes.

Examples:

- `feat(po): add compiled catalog runtime API`
- `fix(api): avoid panic on poisoned plural cache`
- `docs(adr): document compiled key strategy`
- `chore(bench): stabilize benchmark compare measurements`

## Verification

- Keep the existing Rust hook expectations green before push:
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- If you add or change dependencies, make sure `Cargo.lock` is updated and the locked checks still pass before push.

## Public API Changes

- When adding a new public API or a semver-relevant behavior change, update user-facing docs in the same change unless it is clearly internal-only.
- For this repo, that usually means updating the relevant Rust docs plus `README.md` or `docs/api-overview.md`.
- If the change introduces a durable behavioral contract, format, or compatibility rule, add or update an ADR.
- Prefer explicit options and safe defaults over implicit fallback behavior when silent fallback could hide user mistakes.

## Performance And Benchmarking

- Treat benchmark harness changes as product changes: validate them with before/after measurements, not just code inspection.
- When benchmark stability is the concern, prefer reporting noise metrics such as variation or span instead of relying only on one median number.

## Stable Derived Keys

- If the repo introduces a stable derived ID, key, or hash contract, document the exact format and collision behavior in code and an ADR.
