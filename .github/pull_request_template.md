## Summary

- what changed
- why it changed
- any notable tradeoffs

## Changes

<!-- The notable changes, one bullet each. Call out user-visible or breaking behavior. -->

## Validation

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- [ ] `cargo test --workspace --all-features --locked`
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked`
- [ ] MSRV check with the toolchain named by `rust-version` in `Cargo.toml`
- [ ] coverage gate (`cargo llvm-cov` plus `node scripts/coverage-gate.mjs`), when Rust code changed
- [ ] `cargo package --locked` for the published crates, when crate metadata or packaging changed
- [ ] `pnpm install --frozen-lockfile && pnpm build` from `docs/`, when the docs site changed

## Issue

<!-- Closes #123, Refs #123, or a short note on why no issue exists. -->

## Notes

- public API or semver-relevant changes documented
- benchmark impact considered for hot-path changes
- follow-up issues or ADR links, if applicable
