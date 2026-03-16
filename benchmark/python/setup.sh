#!/bin/zsh

set -euo pipefail

script_dir=$(cd -- "$(dirname -- "$0")" && pwd)
venv_dir="$script_dir/.venv"

if [[ ! -d "$venv_dir" ]]; then
  python3 -m venv "$venv_dir"
fi

"$venv_dir/bin/python" -m pip install --upgrade pip
"$venv_dir/bin/python" -m pip install -r "$script_dir/requirements.txt"

echo "python benchmark environment ready: $venv_dir"
