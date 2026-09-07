#!/usr/bin/env bash
# Renders (or verifies) the Ferramenta family block in every README that
# advertises the family.
#
# The block comes from `src/family.ts` in sebastian-software/ferramenta, the
# single source of truth for the family's tools, jobs, groups, and links. A
# hand-copied block drifts; this one follows the registry.
#
#   scripts/check-readme-family.sh            # verify, exits 1 on drift
#   scripts/check-readme-family.sh --check    # same, spelled out
#   scripts/check-readme-family.sh --write    # regenerate the blocks in place
#
# `--check` compares content, not whitespace, so a Markdown formatter is free to
# pad table cells or add blank lines around the HTML comment markers.
#
# Bump FERRAMENTA_PIN and re-run with `--write` to adopt a registry change; see
# CONTRIBUTING.md. Requires Node >= 22.13 and pnpm.
set -euo pipefail

# Pinned so the same commit renders the same block on every run. A branch ref
# would re-check against whatever landed in the registry since.
FERRAMENTA_PIN="3225743e20818805a27e3e1a77cf1726bfb6f939"
GENERATOR="github:sebastian-software/ferramenta#${FERRAMENTA_PIN}&path:/packages/family"

mode="${1:---check}"

case "$mode" in
  --check | --write) ;;
  *)
    echo "usage: ${0##*/} [--check|--write]" >&2
    exit 2
    ;;
esac

repo_root="$(cd "$(dirname "$0")/.." && pwd)"

# Corepack resolves the pinned pnpm from `docs/package.json`, so pnpm runs from
# `docs/` (see CONTRIBUTING.md). `pnpm dlx` builds in a temporary directory and
# leaves that package alone; the README paths are absolute for the same reason.
run_generator() {
  # `set -o pipefail` keeps the generator's exit status; sed only trims the
  # absolute paths back to repository-relative ones for readable output.
  (cd "$repo_root/docs" && pnpm dlx "$GENERATOR" "$@") 2>&1 | sed "s|$repo_root/||g"
}

# The repository README carries the full grouped tables; the four published
# crates carry the two-line flavor that crates.io renders. `--current ferrocat`
# marks this project as the family member the reader is already looking at.
render() {
  local variant="$1" readme="$2"
  run_generator --current ferrocat --variant "$variant" "$mode" "$repo_root/$readme"
}

render github README.md

for crate in ferrocat ferrocat-po ferrocat-icu ferrocat-cli; do
  render registry "crates/$crate/README.md"
done
