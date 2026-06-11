#!/usr/bin/env bash
# ori-sync: detect changes in domain files and mark affected slices dirty
# Usage: ./detect-changes.sh [--since <git-ref>]
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

SINCE="${2:-HEAD~1}"

# Find changed files in .ori/domain/
CHANGED=$(git diff --name-only "$SINCE" -- .ori/domain/ 2>/dev/null || true)

if [[ -z "$CHANGED" ]]; then
  echo "no domain changes detected since $SINCE"
  exit 0
fi

echo "changed domain files:"
echo "$CHANGED"

# Find slices that derive_from changed files
for dir in .ori/slices/*/; do
  [[ -d "$dir" ]] || continue
  id=$(basename "$dir")
  manifest="$dir/manifest.yaml"
  [[ -f "$manifest" ]] || continue

  for changed in $CHANGED; do
    rel="${changed#.ori/}"
    if grep -q "$rel" "$manifest" 2>/dev/null; then
      echo "  dirty: $id (derives from $rel)"
    fi
  done
done
