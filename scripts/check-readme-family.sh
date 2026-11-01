#!/usr/bin/env bash
# Check or refresh the compact crate README blocks from the installed family registry.
# The dependency pin in docs/package.json is the only registry version for this check.
set -euo pipefail
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root/docs"
node scripts/readme-family.mjs "${1:---check}"
