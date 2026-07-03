#!/usr/bin/env bash
set -euo pipefail

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

packages=(
  ferrocat
  ferrocat-po
  ferrocat-icu
)

for package in "${packages[@]}"; do
  snapshot="api-snapshots/${package}.txt"
  generated="${tmpdir}/${package}.txt"

  cargo public-api -p "$package" --all-features -sss --color=never > "$generated"
  diff -u "$snapshot" "$generated"
done
