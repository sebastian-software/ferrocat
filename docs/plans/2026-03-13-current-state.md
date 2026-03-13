# Current State and Open Items

This document is the current-state snapshot for the standalone Ferrocat repository as of
2026-03-13. The other documents in `docs/plans/` capture design decisions and rollout phases; this
file is the short source of truth for what is already done, what is currently true on `main`, and
what remains open.

## Current state

- The Rust workspace has been renamed and split cleanly into:
  - `crates/ferrocat`
  - `crates/ferrocat-node`
- The repository is now a combined Rust + `pnpm` monorepo.
- Public npm package layout is:
  - `packages/ferrocat`
  - `packages/ferrocat-darwin-arm64`
  - `packages/ferrocat-linux-x64-gnu`
  - `packages/ferrocat-linux-arm64-gnu`
  - `packages/ferrocat-winx64-msvc`
- CI exists and covers macOS, Linux, and Windows across Node 22, Node 24, and Bun.
- `release-please` is configured and npm + crates.io Trusted Publishing are working.
- The first shared public release line has been established as `0.1.0`.
- The internal crate `crates/ferrocat-node` is unpublished, but is intentionally kept on the same
  version stream as the public artifacts.

## What has already been done

- Repo and crate naming have been migrated from `pofile` / `pofile-node` to `ferrocat` /
  `ferrocat-node`.
- The Windows native npm package was renamed from `ferrocat-win32-x64-msvc` to
  `ferrocat-winx64-msvc` after npm spam heuristics blocked the original name.
- Placeholder npm platform packages were temporarily introduced to reserve package names and enable
  Trusted Publishing.
- The release and publish workflows were wired up and successfully exercised.
- After the first `0.1.0` release, the platform packages on `main` were switched back from
  placeholders to real native package manifests and build scripts.

## Important current nuance

`main` is now ahead of the already-published `0.1.0` release in a meaningful way:

- The current tree on `main` expects real native package artifacts again.
- The next release from `main` is the one that should publish the actual native package contents
  matching that layout.

In other words: the release infrastructure is live, but the current `main` branch is already the
post-`0.1.0` product state.

## Open items

### 1. Cut the next release from current `main`

This is the most important open step.

- The next release should publish the real native package contents, not the earlier placeholder
  package state.
- Release-please should be allowed to cut the next version from the current commit history on
  `main`.
- After that release, npm should be rechecked to confirm each platform package actually contains
  `ferrocat.node`.

### 2. Verify the full cross-platform native path in CI

Local validation only proves the current development machine path.

- macOS arm64 has been validated locally.
- Linux x64 glibc, Linux arm64 glibc, and Windows x64 MSVC must still be trusted through CI on
  their respective runners.
- The publish workflow now checks for `ferrocat.node` before publishing, but that still needs
  observation on a real release run.

### 3. Improve top-level documentation

The repo works better than the docs currently explain.

- The root `README.md` is still primarily Rust-first.
- The npm install path and supported Node platform matrix should be documented explicitly.
- The relationship between the Rust crate, the Node wrapper, and the platform packages should be
  explained in one short repo-level architecture section.

### 4. Decide what to do next about target coverage

These are not blockers, but they remain open product decisions:

- Linux musl support
- macOS x64 support
- any future non-Node bindings

## Known non-issues

- `publish.yml` running on every push to `main` is expected.
  - Only the release creation probe runs on normal pushes.
  - Actual publishing is guarded behind `releases_created == true`.
- `crates/ferrocat-node` appearing in release-please output is intentional.
  - It stays unpublished on crates.io.
  - It is included only to keep Rust workspace versions aligned.

## Recommended next move

The best next step is not more repo scaffolding.

The best next step is to let release-please open the next release PR from the current `main` state,
merge it, and watch one complete end-to-end publish of the real native package layout.
