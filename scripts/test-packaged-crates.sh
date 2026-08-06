#!/usr/bin/env bash
set -euo pipefail

target_root="${CARGO_TARGET_DIR:-target}"
package_root="${target_root}/package"
package_test_target="${target_root}/packaged-crate-tests"

cargo package \
  -p ferrocat-icu \
  -p ferrocat-po \
  -p ferrocat \
  -p ferrocat-cli \
  --locked

package_dir() {
  local package="$1"
  local package_id
  local version
  local directory

  package_id="$(cargo pkgid -p "$package")"
  version="${package_id##*#}"
  version="${version##*@}"
  directory="${package_root}/${package}-${version}"

  if [[ ! -f "${directory}/Cargo.toml" ]]; then
    echo "missing packaged manifest: ${directory}/Cargo.toml" >&2
    return 1
  fi

  (cd "$directory" && pwd -P)
}

icu_dir="$(package_dir ferrocat-icu)"
po_dir="$(package_dir ferrocat-po)"
ferrocat_dir="$(package_dir ferrocat)"
cli_dir="$(package_dir ferrocat-cli)"

icu_patch="patch.crates-io.ferrocat-icu.path=\"${icu_dir}\""
po_patch="patch.crates-io.ferrocat-po.path=\"${po_dir}\""

cargo test \
  --manifest-path "${icu_dir}/Cargo.toml" \
  --all-features \
  --target-dir "$package_test_target"

cargo test \
  --manifest-path "${po_dir}/Cargo.toml" \
  --all-features \
  --target-dir "$package_test_target" \
  --config "$icu_patch"

cargo test \
  --manifest-path "${ferrocat_dir}/Cargo.toml" \
  --all-features \
  --target-dir "$package_test_target" \
  --config "$icu_patch" \
  --config "$po_patch"

cargo test \
  --manifest-path "${cli_dir}/Cargo.toml" \
  --all-features \
  --target-dir "$package_test_target" \
  --config "$icu_patch" \
  --config "$po_patch"
