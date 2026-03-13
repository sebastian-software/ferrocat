# Native Prebuild Packages Design

## Goal

Replace the placeholder npm platform packages with real native prebuild packages and make the Node
wrapper, CI, and publish workflow operate on the actual `.node` artifacts.

## Scope

The implementation covers exactly these public npm packages:

- `ferrocat-darwin-arm64`
- `ferrocat-linux-x64-gnu`
- `ferrocat-linux-arm64-gnu`
- `ferrocat-winx64-msvc`

It does not add more targets such as musl, macOS x64, or universal binaries.

## Decisions

- Each platform package will again contain a real `ferrocat.node` artifact, `README.md`, and
  `CHANGELOG.md`.
- The platform packages keep a package-local `build` script which delegates to the shared
  `scripts/build-native-package.mjs`.
- `packages/ferrocat/index.js` remains the only public runtime loader and resolves the correct
  optional dependency by platform.
- `scripts/smoke-native.mjs` becomes a strict load test again; placeholder-aware skipping is
  removed.
- `publish.yml` continues to publish native packages first, then the wrapper package, then the Rust
  crate.
- CI should only run the strict native smoke test on Node lanes, because the runtime contract being
  checked is the Node wrapper and native addon load path.

## Build flow

1. The native package build runs in the target package directory.
2. `scripts/build-native-package.mjs` reads the current package name, maps it to a platform target,
   builds `ferrocat-node` via Cargo, and copies the generated library to `ferrocat.node` inside the
   package.
3. The package tarball includes `ferrocat.node`, so `pnpm publish` ships a self-contained native
   package.
4. The main `ferrocat` package loads the optional dependency matching the current platform.

## Validation

- `pnpm build`
- `cargo test`
- `node ./scripts/smoke-native.mjs`
- `pnpm pack` for the local platform package to verify `ferrocat.node` is included
