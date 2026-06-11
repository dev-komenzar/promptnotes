#!/usr/bin/env bash
# Shared domain file schema lint
# Usage: ./lint-domain.sh <filepath>
# Checks: frontmatter coherence block, H2/H3 {#id} anchors
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

# Check frontmatter has coherence block
if ! grep -q '^coherence:' "$FILE" 2>/dev/null; then
  echo "WARN: missing coherence frontmatter"
  ((ISSUES++)) || true
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
