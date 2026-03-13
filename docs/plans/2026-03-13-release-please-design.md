# Release-Please Design

## Goal

Set up `release-please` so that Ferrocat manages one shared version stream across:

- the public Rust crate `crates/ferrocat`
- the public npm wrapper `packages/ferrocat`
- the four publishable native npm packages

Merging a release PR should trigger one publish workflow that:

1. creates GitHub releases/tags via `release-please`
2. publishes native npm packages first
3. publishes the `ferrocat` npm wrapper
4. publishes the `ferrocat` Rust crate to crates.io

## Decisions

- Reset the old Rust `5.0.0-beta.0` line back to the standalone Ferrocat baseline
  - The public crates and npm packages in-repo stay at `0.0.1` until the first real release PR is merged.
  - The first real Ferrocat release should be forced to `0.1.0`.
- Keep `crates/ferrocat-node` internal and unpublished
  - It participates in the shared version stream so all Rust crates stay aligned.
  - It remains `publish = false` and is not published to crates.io by `publish.yml`.
- Use manifest-driven `release-please`
  - This is the recommended mode for monorepos and required for Rust workspaces.
- Use both `node-workspace` and `cargo-workspace` plugins with `merge: false`
  - Official release-please docs note this is necessary when combining workspace plugins with `linked-versions`.
- Use one `linked-versions` group for all public artifacts
  - Rust crate and npm packages stay on the same version at all times.
- Use `bootstrap-sha` to cut off the imported split history
  - Release notes should start from the standalone Ferrocat repo state, not from inherited `pofile-ts` history.
- Split release PR creation and publishing into two workflows
  - `release-pr.yml` manages PRs only
  - `publish.yml` performs release tagging and publication only
- Use Node.js 24 in both release workflows
  - Required for npm Trusted Publishing and aligned with current GitHub Actions runtime changes.
- Use npm Trusted Publishing for npm packages
  - `publish.yml` is the trusted publisher workflow.
- Use crates.io Trusted Publishing for the Rust crate
  - The publish workflow will authenticate via OIDC using `rust-lang/crates-io-auth-action`.

## Workflow design

### `release-pr.yml`

- Trigger: push to `main`
- Purpose: create or update the combined release PR
- Mechanism: `googleapis/release-please-action@v4` with `skip-github-release: true`
- Permissions:
  - `contents: write`
  - `issues: write`
  - `pull-requests: write`

### `publish.yml`

- Trigger: push to `main`
- Purpose: detect merged release PRs, create tags/releases, then publish artifacts
- Mechanism:
  - first job runs `googleapis/release-please-action@v4` with `skip-github-pull-request: true`
  - downstream jobs only run when releases were actually created
- Publish order:
  - native npm packages
  - `packages/ferrocat`
  - `crates/ferrocat`

## Risks and constraints

- npm Trusted Publisher must be configured for every package that will be published, not only for the main `ferrocat` package.
- crates.io Trusted Publishing requires repository trust to be configured on crates.io for `ferrocat`.
- `release-please` will only open a release PR when conventional commits imply a release.
- Multiple GitHub releases/tags will be created, one per release component, because the Rust crate and npm wrapper share the same public name but remain separate release components.
- Because `0.0.1` was only used as a reservation/dummy release, the first actual release should be triggered explicitly with a `Release-As: 0.1.0` commit body.

## Validation plan

- Check JSON/YAML syntax locally
- Run `pnpm install --frozen-lockfile`
- Run `pnpm build`
- Run `pnpm test:rust`
- Review `git diff` for release-please managed paths and workflow filenames

## First release bootstrap

Once Trusted Publishing is configured for every npm package and crates.io trust is enabled for
`ferrocat`, create the first actual release PR with an empty conventional commit like:

```text
chore: release ferrocat 0.1.0

Release-As: 0.1.0
```

That lets `release-please` produce the first standalone Ferrocat release as `0.1.0` while keeping
the repository state aligned with the already-reserved `0.0.1` package placeholders.
