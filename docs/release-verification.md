# Release Verification

This checklist verifies a real Rust-only `ferrocat` release after the publish workflow succeeds.

## 1. Confirm automation state

- Open the latest `Publish` run and confirm the `release-please` job updated or closed the expected release PR cleanly.
- For merged release PRs, confirm the same `Publish` run also completed the `publish-rust` job successfully.

## 2. Confirm workspace versions

- Check crates.io for `ferrocat-icu`, `ferrocat-po`, and `ferrocat`.
- Confirm the published crates resolved to the same release version.
- Confirm the workspace-only crates in the repo are also version-aligned with the published crates.
- Confirm internal `ferrocat*` path dependencies in the crate manifests were updated to matching versions where applicable.
- Confirm the GitHub release tag matches the published version created by Release Please.

## 3. Confirm install and docs surface

- In a clean scratch project, run `cargo add ferrocat`.
- Build a tiny smoke example that calls `parse_po` and `parse_icu` through the umbrella crate.
- Confirm docs.rs builds for `ferrocat` and that the README example still matches the published surface.

## 4. Record outcome

- If the release is good, record the workflow URL and version in the relevant status or changelog notes.
- If publishing failed partway through, capture the exact crate, version, and workflow URL before retrying so the next release does not repeat the same blind spot.

## 5. Rollback guidance

- If only the GitHub release failed, fix the workflow cause and rerun from the release commit.
- If `ferrocat-icu` or `ferrocat-po` published but `ferrocat` failed, do not delete tags. Cut a follow-up release with the fix and let Release Please advance the version.
- If a published crate is materially broken, use `cargo yank` for the affected version and note the yank in the release incident log.
