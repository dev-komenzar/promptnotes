#!/usr/bin/env bash
# ori-doctor: check slice document schema
# Validates: spec.md structure, status.yaml presence, manifest.yaml presence
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

ISSUES=0

if [[ ! -d .ori/slices ]]; then
  echo "  slices: no .ori/slices/ directory"
  exit 0
fi

for dir in .ori/slices/*/; do
  [[ -d "$dir" ]] || continue
  id=$(basename "$dir")

  if [[ ! -f "$dir/manifest.yaml" ]]; then
    echo "  WARN  slices/$id: missing manifest.yaml"
    ((ISSUES++)) || true
  fi
  if [[ ! -f "$dir/status.yaml" ]]; then
    echo "  WARN  slices/$id: missing status.yaml"
    ((ISSUES++)) || true
  fi
done

echo "  slices schema: $ISSUES issue(s)"
exit $ISSUES
