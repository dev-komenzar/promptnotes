#!/usr/bin/env bash
# Shared domain file schema lint
# Usage: ./lint-domain.sh <filepath>
# Checks: frontmatter ori block (design.md §5), H2/H3 {#id} anchors
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

FILE="${1:-}"
if [[ -z "$FILE" ]]; then
  echo "ERROR: file path required" >&2
  exit 1
fi

if [[ ! -f "$FILE" ]]; then
  echo "ERROR: $FILE not found" >&2
  exit 1
fi

ISSUES=0

# Check frontmatter has ori: block (design.md §5)
if ! grep -q '^ori:' "$FILE" 2>/dev/null; then
  echo "WARN: missing ori: frontmatter (design.md §5)"
  ((ISSUES++)) || true
fi

# Check ori: block has node_id and type fields
if grep -q '^ori:' "$FILE" 2>/dev/null; then
  if ! grep -qE '^[[:space:]]+node_id:' "$FILE" 2>/dev/null; then
    echo "WARN: ori: block missing node_id"
    ((ISSUES++)) || true
  fi
  if ! grep -qE '^[[:space:]]+type:' "$FILE" 2>/dev/null; then
    echo "WARN: ori: block missing type"
    ((ISSUES++)) || true
  fi
fi

# Check H2/H3 have {#id} anchors
BARE=$(grep -nE '^##[^#]' "$FILE" | grep -v '{#' || true)
if [[ -n "$BARE" ]]; then
  echo "WARN: H2/H3 without {#id} anchor:"
  echo "$BARE" | while read -r line; do echo "  $line"; done
  ((ISSUES++)) || true
fi

if [[ $ISSUES -eq 0 ]]; then
  echo "OK: $FILE"
else
  echo "$ISSUES issue(s) in $FILE"
fi

exit $ISSUES
