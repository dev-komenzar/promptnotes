#!/usr/bin/env bash
# ori-finalize: clear dirty marks and update status
# Usage: ./clear-dirty.sh <slice-id>
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

STATUS=".ori/slices/$ID/status.yaml"
if [[ ! -f "$STATUS" ]]; then
  echo "ERROR: $STATUS not found" >&2
  exit 1
fi

# Replace dirty list with empty
sed -i 's/^dirty:.*/dirty: []/' "$STATUS"
echo "cleared dirty marks for $ID"
