#!/usr/bin/env bash
# ori-feature-status: list all slices and their status
# Usage: ./list-slices.sh [--dirty]
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

DIRTY_ONLY=false
if [[ "${1:-}" == "--dirty" ]]; then
  DIRTY_ONLY=true
fi

for dir in .ori/slices/*/; do
  [[ -d "$dir" ]] || continue
  id=$(basename "$dir")
  status="$dir/status.yaml"

  if [[ -f "$status" ]]; then
    dirty=$(grep 'dirty:' "$status" 2>/dev/null | sed 's/dirty: //' || echo "[]")
    if [[ "$DIRTY_ONLY" == "true" ]] && [[ "$dirty" == "[]" ]]; then
      continue
    fi
    echo "$id dirty=$dirty"
  else
    echo "$id status=scaffold"
  fi
done
