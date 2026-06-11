#!/usr/bin/env bash
# ori-derive: check if a slice exists
# Usage: ./check-slice-exists.sh <slice-id>
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

if [[ -f ".ori/slices/$ID/manifest.yaml" ]]; then
  echo "$ID"
  exit 0
fi

# Fuzzy match
CANDIDATES=$(find .ori/slices -maxdepth 2 -name 'manifest.yaml' 2>/dev/null | sed 's|.ori/slices/||;s|/manifest.yaml||' | grep -i "$ID" || true)
if [[ -n "$CANDIDATES" ]]; then
  echo "not found, candidates:" >&2
  echo "$CANDIDATES" >&2
  exit 2
fi

echo "not found" >&2
exit 1
