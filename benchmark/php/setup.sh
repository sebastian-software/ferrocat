#!/usr/bin/env bash

set -euo pipefail

script_dir=$(cd -- "$(dirname -- "$0")" && pwd)

if ! command -v php >/dev/null 2>&1; then
  echo "php not found; install PHP 8.1+ (e.g. 'brew install php')" >&2
  exit 1
fi

if ! command -v composer >/dev/null 2>&1; then
  echo "composer not found; install it (e.g. 'brew install composer')" >&2
  exit 1
fi

composer install --no-interaction --working-dir "$script_dir"

echo "php benchmark environment ready: $script_dir/vendor"
