#!/bin/zsh

set -euo pipefail

script_dir=$(cd -- "$(dirname -- "$0")" && pwd)
python_dir="$script_dir/python"
node_dir="$script_dir/node"
venv_dir="$python_dir/.venv"

if [[ ! -d "$venv_dir" ]]; then
  python3 -m venv "$venv_dir"
fi

"$venv_dir/bin/python" -m pip install --upgrade pip
"$venv_dir/bin/python" -m pip install -r "$python_dir/requirements.txt"

(cd "$node_dir" && npm install)

echo "benchmark setup complete"
echo "python venv: $venv_dir"
