# Native Placeholder Packages Design

## Goal

Prepare the four platform-specific npm packages as minimal placeholder packages so they can be
published at `0.0.1` and enrolled in npm Trusted Publishing before the real native artifacts are
shipped.

## Decision

- Keep the package names, versions, and platform metadata as-is.
- Reduce package contents to `README.md` and `CHANGELOG.md` only.
- Remove runtime entry points and native binary expectations from the placeholder package manifests.
- Keep a no-op `build` script so the existing workspace CI continues to run without special casing.

## Result

The placeholder packages remain publishable and platform-scoped, but they are unambiguously dummy
reservation packages until the real native package contents land in a later release.
