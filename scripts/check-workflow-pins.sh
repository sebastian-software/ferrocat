#!/usr/bin/env bash
# Fails when a workflow references a third-party action by a mutable ref.
#
# Tags and branches can be moved to point at different code, so every `uses:`
# must name a full 40-character commit SHA and carry a trailing comment with the
# human-readable version it was resolved from. Local (`./…`) and container
# (`docker://…`) references are exempt because they are not fetched by ref.
set -euo pipefail

workflow_directory="${1:-.github/workflows}"

if [[ ! -d "$workflow_directory" ]]; then
  echo "Workflow directory does not exist or is not a directory: $workflow_directory" >&2
  exit 2
fi

pinned='^[^[:space:]@]+@[0-9a-f]{40}[[:space:]]+#[[:space:]]*[^[:space:]]' # owner/repo@sha # vN
exempt='^(\./|docker://)'

status=0
checked=0

while IFS= read -r match; do
  location="${match%%:uses:*}"
  value="${match#*:uses:}"
  value="${value#"${value%%[![:space:]]*}"}"
  checked=$((checked + 1))

  if [[ "$value" =~ $exempt ]] || [[ "$value" =~ $pinned ]]; then
    continue
  fi

  echo "$location: uses: $value" >&2
  status=1
done < <(grep -rnE '^[[:space:]]*(-[[:space:]]+)?uses:' "$workflow_directory" |
  sed -E 's/^([^:]+:[0-9]+):[[:space:]]*(-[[:space:]]+)?uses:/\1:uses:/')

if [[ "$status" -ne 0 ]]; then
  echo "" >&2
  echo "Workflow actions must be pinned to a full 40-character commit SHA with a" >&2
  echo "trailing version comment, for example:" >&2
  echo "  uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1" >&2
  exit 1
fi

echo "All $checked workflow action references are pinned to a commit SHA."
