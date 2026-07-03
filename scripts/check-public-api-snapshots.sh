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
  expected="${tmpdir}/${package}.expected.txt"
  generated="${tmpdir}/${package}.txt"

  cargo +nightly public-api -p "$package" --all-features -sss --color=never \
    | sed 's/core::io::error::Error/std::io::error::Error/g' \
    > "$generated"
  LC_ALL=C sort "$snapshot" > "$expected"
  LC_ALL=C sort "$generated" -o "$generated"
  diff -u "$expected" "$generated"
done
