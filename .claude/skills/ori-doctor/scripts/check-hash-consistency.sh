#!/usr/bin/env bash
# ori-doctor: check hash consistency between derived specs and upstream domain docs
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
  spec="$dir/spec.md"

  [[ -f "$spec" ]] || continue

  # Extract upstream references from spec.md frontmatter
  upstreams=$(sed -n '/^  upstream:/,/^  [a-z]/p' "$spec" 2>/dev/null | grep '^    - ' | sed 's/    - //' | sed 's/#.*//' | sort -u || true)

  for up in $upstreams; do
    upfile=".ori/$up"
    if [[ ! -f "$upfile" ]]; then
      echo "  WARN  slices/$id: upstream '$up' not found"
      ((ISSUES++)) || true
    fi
  done
done

echo "  hash consistency: $ISSUES issue(s)"
exit $ISSUES
