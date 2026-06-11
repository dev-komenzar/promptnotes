#!/usr/bin/env bash
# ori-finalize: update spec.md hash values to match current upstream hashes
# Usage: ./update-hash.sh <slice-id>
set -euo pipefail

# Auto-detect project root
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null || true)"
if [ -z "$PROJECT_ROOT" ]; then
  d="$SCRIPT_DIR"
  while [ "$d" != "/" ]; do
    if [ -d "$d/.ori" ]; then PROJECT_ROOT="$d"; break; fi
    d="$(dirname "$d")"
  done
fi
if [ -z "$PROJECT_ROOT" ]; then echo "ERROR: cannot find project root (.ori/ not found)" >&2; exit 1; fi
cd "$PROJECT_ROOT"

ID="${1:-}"
if [[ -z "$ID" ]]; then
  echo "ERROR: slice-id required" >&2
  exit 1
fi

SPEC=".ori/slices/$ID/spec.md"
if [[ ! -f "$SPEC" ]]; then
  echo "ERROR: $SPEC not found" >&2
  exit 1
fi

# Extract upstream paths from spec.md frontmatter (hash: section)
upstreams=$(sed -n '/^  upstream:/,/^  [a-z]/p' "$SPEC" 2>/dev/null | grep '^    - ' | sed 's/    - //' | sed 's/#.*//' | sort -u || true)

for up in $upstreams; do
  path=".ori/$up"
  if [[ -f "$path" ]]; then
    hash=$(sha256sum "$path" | cut -c1-12)
    escaped=$(echo "$up" | sed 's/[\/&]/\\&/g')
    sed -i "s|    $escaped#.*: .*|    $escaped#.*: $hash|" "$SPEC"
  fi
done

echo "updated hashes in $SPEC"
