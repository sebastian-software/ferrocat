#!/usr/bin/env bash
# Single source for the coverage gate.
#
# CI, CONTRIBUTING.md, and AGENTS.md all invoke this script, so the crate
# thresholds and the llvm-cov filters live in exactly one place instead of
# being restated (and drifting) in three.
#
# The umbrella `ferrocat` crate is reported by the gate but intentionally not
# threshold-gated while it only re-exports the lower-level crates plus smoke
# tests. Add it to THRESHOLDS when executable umbrella code grows.
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

IGNORE_REGEX='crates/ferrocat-(bench|conformance)/'
THRESHOLDS=(ferrocat-cli=85 ferrocat-po=95 ferrocat-icu=95)

cargo llvm-cov --workspace --all-features --locked \
  --lcov --output-path target/lcov.info \
  --ignore-filename-regex "$IGNORE_REGEX"

cargo llvm-cov report --json --summary-only \
  --output-path target/coverage-summary.json \
  --ignore-filename-regex "$IGNORE_REGEX"

node scripts/coverage-gate.mjs target/coverage-summary.json "${THRESHOLDS[@]}"
