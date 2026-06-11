#!/usr/bin/env bash
# ori-doctor: check cross-reference integrity
# Validates all derives_from and upstream references point to existing files
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

for dir in .ori/slices/*/; do
  [[ -d "$dir" ]] || continue
  id=$(basename "$dir")
  manifest="$dir/manifest.yaml"
  [[ -f "$manifest" ]] || continue

  # Check derives_from references
  refs=$(sed -n '/^derives_from:/,/^[a-z]/p' "$manifest" 2>/dev/null | grep '^  - ' | sed 's/  - //' | sed 's/#.*//' || true)
  for ref in $refs; do
    if [[ ! -f ".ori/$ref" ]]; then
      echo "  WARN  slices/$id: derives_from '$ref' not found"
      ((ISSUES++)) || true
    fi
  done
done

echo "  cross-reference: $ISSUES issue(s)"
exit $ISSUES
