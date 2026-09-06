#!/usr/bin/env bash

set -euo pipefail

script_dir=$(cd -- "$(dirname -- "$0")" && pwd)
python_dir="$script_dir/python"
node_dir="$script_dir/node"
php_dir="$script_dir/php"
venv_dir="$python_dir/.venv"

if [[ ! -d "$venv_dir" ]]; then
  python3 -m venv "$venv_dir"
fi

"$venv_dir/bin/python" -m pip install --upgrade pip
"$venv_dir/bin/python" -m pip install -r "$python_dir/requirements.txt"

(cd "$node_dir" && npm install)

if command -v composer >/dev/null 2>&1; then
  composer install --no-interaction --working-dir "$php_dir"
else
  echo "composer not found; skipping PHP adapter (install PHP 8.1+ and composer for php-gettext)" >&2
fi

echo "benchmark setup complete"
echo "python venv: $venv_dir"
