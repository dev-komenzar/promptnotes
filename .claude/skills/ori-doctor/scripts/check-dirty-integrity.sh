#!/usr/bin/env bash
# ori-doctor: detect unauthorized dirty clears
# A slice with empty dirty[] but no review.md or review verdict != PASS
# has likely been manually tampered with (status.yaml directly edited).
set -euo pipefail

# Auto-detect project root (PWD-first; SCRIPT_DIR fallback last).
PWD_DIR="$(pwd)"
PROJECT_ROOT="$(git -C "$PWD_DIR" rev-parse --show-toplevel 2>/dev/null || true)"
if [ -n "$PROJECT_ROOT" ] && [ ! -d "$PROJECT_ROOT/.ori" ]; then PROJECT_ROOT=""; fi
if [ -z "$PROJECT_ROOT" ]; then
  d="$PWD_DIR"
  while [ "$d" != "/" ]; do
    if [ -d "$d/.ori" ]; then PROJECT_ROOT="$d"; break; fi
    d="$(dirname "$d")"
  done
fi
if [ -z "$PROJECT_ROOT" ]; then
  SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
  PROJECT_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null || true)"
fi
if [ -z "$PROJECT_ROOT" ] || [ ! -d "$PROJECT_ROOT/.ori" ]; then echo "ERROR: cannot find project root (.ori/ not found)" >&2; exit 1; fi
cd "$PROJECT_ROOT"

ISSUES=0

for dir in .ori/slices/*/; do
  [[ -d "$dir" ]] || continue
  id=$(basename "$dir")
  status="$dir/status.yaml"
  review="$dir/review.md"
  spec="$dir/spec.md"

  [[ -f "$status" ]] || continue

  # Only check slices that have no dirty marks — i.e., supposedly "finished"
  # If dirty is non-empty, the slice is legitimately in progress
  dirty_count=$(grep -oP '^dirty:\s*\[(.*)\]' "$status" | grep -oP '\[(.*)\]' | sed 's/[][]//g' | tr ',' '\n' | sed '/^$/d' | wc -l)
  if [[ "$dirty_count" -gt 0 ]]; then
    continue  # legitimately dirty
  fi

  # No dirty marks — check if review was properly completed
  if [[ ! -f "$review" ]]; then
    echo "  ✗ slices/$id: dirty=[] but review.md not found (possible manual tampering with status.yaml)"
    echo "    fix: /ori-flow $id (run full workflow including review)"
    ((ISSUES++)) || true
    continue
  fi

  # Check verdict
  verdict=$(grep -oP 'verdict\s*[=:]\s*\K(PASS|NEEDS_FIX|REJECT)' "$review" 2>/dev/null | tail -1 || true)
  if [[ -z "$verdict" ]]; then
    echo "  ⚠ slices/$id: dirty=[] but review.md has no verdict (incomplete review?)"
    ((ISSUES++)) || true
    continue
  fi

  if [[ "$verdict" != "PASS" ]]; then
    echo "  ✗ slices/$id: dirty=[] but review verdict is '$verdict' (verdict must be PASS to clear dirty)"
    echo "    fix: /ori-review $id → address findings → re-review → finalize"
    ((ISSUES++)) || true
  fi
done

echo "  dirty integrity: $ISSUES issue(s)"
exit $ISSUES