# Issues 236-241 Maintenance Design

## Goal

Resolve issues #236 through #241 in one reviewable pull request. Keep the work in small Conventional Commits, preserve existing public behavior and report formats, and post useful progress updates to the pull request and each issue.

## Scope

The pull request covers three related maintenance areas:

- performance: avoid repeated ICU parsing in catalog audits and extend the PR-visible benchmark profile to catalog QA and runtime workflows;
- structure: split the oversized comparison harness into focused private modules without changing its CLI or JSON contracts;
- documentation and project safeguards: restore the blocking semver check, update shipped 2.2 guidance, and keep published conformance and coverage claims aligned with CI.

No unrelated API redesign, benchmark framework replacement, or release automation work is included.

## Delivery Strategy

Use one branch and one draft pull request. Land each independently verifiable slice as a small Conventional Commit, in this order:

1. `fix(ci): restore blocking semver checks` closes #239.
2. `docs(guide): align upgrade guidance with 2.2` closes #240.
3. `test(quality): guard published quality metrics` closes #241.
4. `refactor(bench): split comparison harness modules` closes #238.
5. `perf(audit): reuse parsed ICU messages` closes #236.
6. `feat(bench): cover catalog workflow regressions` closes #237.
7. Add a separate docs commit only if the benchmark surface changes a user-facing claim that cannot be kept accurate in commit 6.

The pull request description will reference all six issues and track completed slices and verification results. Each issue will receive a short start update and a completion update tied to the relevant commit or pull request state.

## Design

### Semver and release guidance

Remove the non-blocking override from the semver compatibility job so failures block CI again. Replace stale references to a planned 2.1 or upcoming 2.2 cleanup with wording that reflects the shipped 2.2 line. Version examples should use the current supported major version and should not predict an uncommitted future release.

### Published quality metrics

Treat conformance counts and coverage policy differently:

- Conformance counts are deterministic repository facts. Update the published snapshot and add a test that compares the generated summary with the documented values.
- Exact coverage percentages are volatile implementation snapshots. Keep stable enforced thresholds and the CI command documented, but remove exact percentages from sections presented as the current gate unless CI generates and checks them automatically.

The guard must fail with a clear message that names the stale documentation and the expected current values.

### Benchmark harness structure

Keep the existing command-line interface, profile schema, scheduling behavior, measurements, semantic digests, and JSON report schema intact. Split `compare.rs` along existing responsibilities into private modules such as:

- profile loading and validation;
- operation execution and adapters;
- semantic digest generation;
- report construction and serialization;
- regression evaluation;
- environment metadata;
- focused tests next to the behavior they cover.

Public entry points remain stable. Module boundaries should follow data flow already present in the file rather than introduce new abstraction layers.

### ICU audit parsing reuse

When syntax and compatibility checks run together, parse each source ICU message once and reuse that result for compatibility checks. Parse each translated message once per audited target and reuse it within that target's checks. Preserve diagnostic contents, ordering, locale handling, failure behavior, and the behavior of syntax-only and compatibility-only configurations.

The cache should remain local to one audit invocation and borrow existing message keys or values where practical. It must not introduce global state or change public types.

### Workflow benchmark coverage

Extend the PR regression profile with representative operations for catalog combination, audit, coverage/review, and compiled runtime access. Prefer realistic small fixtures and deterministic semantic digests. Reuse the current profile and report machinery instead of creating a second framework.

New scenarios must be visible in the machine-readable report and participate in the existing regression decision path. Their runtime should remain suitable for pull-request CI.

## Compatibility and Error Handling

This work must not change public Rust APIs, benchmark CLI flags, profile parsing compatibility, or report field meanings. Invalid profiles and failed operations should continue to use the existing error types and exit behavior. The audit optimization must return the same diagnostics for equivalent inputs before and after the change.

## Verification

Run focused tests after every slice. Before marking the pull request ready, run:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --all-features --locked`
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked`
- `cargo +1.93.0 check --workspace --all-targets --all-features --locked`
- the repository coverage commands and coverage gate from `AGENTS.md`
- `cargo package -p ferrocat-icu -p ferrocat-po -p ferrocat -p ferrocat-cli --locked`
- `pnpm install --frozen-lockfile` and `pnpm build` from `docs/`

For #236, record a release-mode before/after measurement using the same generated catalog, locales, check configuration, warmup, and sample count. For #237 and #238, verify that existing profiles still parse, existing report snapshots remain compatible, and every added workflow appears in the PR regression report.

## Completion Criteria

All six issues have an implemented and verified slice, the pull request body maps commits and checks back to each issue, documentation matches shipped behavior, and benchmark results contain enough detail to judge the performance change without overstating noisy measurements.
