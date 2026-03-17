## Summary

- what changed
- why it changed
- any notable tradeoffs

## Verification

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- [ ] `cargo test --workspace --locked`

## Notes

- public API or semver-relevant changes documented
- benchmark impact considered for hot-path changes
- follow-up issues or ADR links, if applicable
