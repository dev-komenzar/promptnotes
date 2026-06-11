#!/usr/bin/env bash
# ori-feature-status: list all pages and their status
# Usage: ./list-pages.sh
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

for dir in .ori/pages/*/; do
  [[ -d "$dir" ]] || continue
  id=$(basename "$dir")
  status="$dir/status.yaml"

  if [[ -f "$status" ]]; then
    dirty=$(grep 'dirty:' "$status" 2>/dev/null | sed 's/dirty: //' || echo "[]")
    echo "$id dirty=$dirty"
  else
    echo "$id status=scaffold"
  fi
done
