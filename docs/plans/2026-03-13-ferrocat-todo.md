# Ferrocat Rollout Plan

## Decisions already made

- `ferrocat` is the public product name for both crates.io and npm.
- The current `pofile` crate will be renamed to `ferrocat`.
- The current `pofile-node` crate will be renamed to `ferrocat-node`.
- This repository will become the single monorepo for Rust crates and JavaScript packages.
- JavaScript package management uses `pnpm` via Corepack.
- npm publishing uses Trusted Publishing with OIDC.
- GitHub Actions publish jobs must use Node.js 24.
- General development and CI can target Node.js 22 as the minimum supported version.
- Releases should use `release-please`, not `release-it`.
- npm distribution should use one public wrapper package plus platform-specific optional native packages.

## Target package layout

### Rust workspace

- `crates/ferrocat`
  - Public Rust crate published to crates.io as `ferrocat`
  - Contains PO parsing, ICU compilation, catalog compilation, runtime helpers, and the stable Rust API
- `crates/ferrocat-node`
  - Internal N-API bridge crate
  - Produces the native binary consumed by the JavaScript wrapper packages
  - Stays unpublished on crates.io

### pnpm workspace

- `packages/ferrocat`
  - Public npm package
  - Thin Node.js wrapper that loads the correct platform package
  - Declares platform packages as `optionalDependencies`
- `packages/ferrocat-darwin-arm64`
- `packages/ferrocat-linux-x64-gnu`
- `packages/ferrocat-linux-arm64-gnu`
- `packages/ferrocat-win32-x64-msvc`
  - Publishable platform packages containing the `.node` binary
  - Built in CI on their matching runners

## Immediate cleanup required

- Rename all Rust crate names, paths, imports, examples, tests, docs, and metadata from `pofile` to `ferrocat`.
- Rename all Node binding references from `pofile-node` to `ferrocat-node`.
- Replace the current root README with a repo-level Ferrocat README instead of an adapted crate README.
- Rewrite crate/module docs that still describe the project as an early extracted port or as primarily a host-binding backend.
- Clean up `.gitignore` to remove `pofile-ts`-specific leftovers.
- Reset the crate changelog/history wording so the repo no longer reads like a transplant from another package.

## CI and release requirements

### Validation CI

- Add `cargo test`
- Add `cargo fmt --check`
- Add `cargo clippy --all-targets --all-features -- -D warnings`
- Add `pnpm install --frozen-lockfile`
- Add package build/test/typecheck steps once the JS workspace exists
- Run general CI on Node.js 22

### Release automation

- Add `.release-please-config.json`
- Add `.release-please-manifest.json`
- Add `.github/workflows/release-pr.yml`
- Add `.github/workflows/publish.yml`
- Configure publish workflows to use Node.js 24
- Configure publish workflows with `id-token: write`
- Publish native platform packages first, then publish `packages/ferrocat`
- Use `release-please` release commits as the publish trigger

## Recommended implementation order

### Phase 1: Hard rename and repo cleanup

- Rename `crates/pofile` to `crates/ferrocat`
- Rename `crates/pofile-node` to `crates/ferrocat-node`
- Update workspace members in `Cargo.toml`
- Update crate names, descriptions, documentation URLs, examples, tests, and imports
- Rewrite the root README around Ferrocat as a product/repo
- Clean `.gitignore`
- Normalize formatting with `cargo fmt`

Outcome:
- The repo stops presenting itself as a transitional split and becomes internally consistent under the Ferrocat name

### Phase 2: Add pnpm/Corepack monorepo scaffolding

- Add root `package.json`
- Add `pnpm-workspace.yaml`
- Add `pnpm-lock.yaml`
- Add shared scripts for build, test, and native build orchestration
- Add a base TypeScript config only if the wrapper packages need compilation

Outcome:
- The repository can manage Rust and JavaScript artifacts from one root

### Phase 3: Add wrapper and native packages

- Add `packages/ferrocat`
- Add the four platform packages
- Add a shared native build script that builds `crates/ferrocat-node` and copies the resulting binary into the target package
- Implement runtime package resolution in `packages/ferrocat`
- Document supported targets and failure modes when no matching package exists

Outcome:
- The npm distribution model is in place and matches the intended public install path

### Phase 4: Add CI and release automation

- Add validation workflow for Rust and pnpm workspace checks
- Add release-please workflow
- Add publish workflow with Node.js 24
- Set up matrix publishing for the four native packages
- Publish the wrapper package after native packages succeed

Outcome:
- Releases become reproducible and consistent with Trusted Publishing requirements

### Phase 5: Final documentation and positioning pass

- Add a short architecture note for crate/package responsibilities
- Document the relationship between `pofile-ts` and `ferrocat`
- Document the supported Node platforms and version floor
- Add badges only after workflows and releases are live

Outcome:
- Public messaging matches the actual repo structure and release behavior

## First three commits to make

1. `chore: rename pofile workspace to ferrocat`
   - Rust crate rename
   - Path rename
   - import/test/example/doc updates

2. `chore: initialize pnpm monorepo for ferrocat packages`
   - root `package.json`
   - `pnpm-workspace.yaml`
   - base package scaffolding

3. `ci: add validation and release-please workflows`
   - CI workflow
   - release PR workflow
   - publish workflow

## Follow-up decisions that can wait

- Whether to add Linux musl builds later
- Whether to support macOS x64 in addition to arm64
- Whether to split runtime functionality into a later dedicated crate
- How aggressively to align version numbers between crates.io and npm beyond the shared product name

## Explicitly deferred for now

- Benchmarks against `pofile-ts`
- Launch copy and announcement materials
- Additional host bindings beyond Node.js
- Any repo split between Rust core and JS packaging
