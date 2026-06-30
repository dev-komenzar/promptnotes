#!/usr/bin/env bash
# ori-doctor: master health check runner
# Runs all individual checks and aggregates results
set -euo pipefail

# Auto-detect project root (PWD-first; SCRIPT_DIR fallback last).
# Why: when ori is installed inside a user project, SCRIPT_DIR resolves to the
# ori repo so git toplevel misses the user's .ori/ (ori-fzr.15).
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
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
  PROJECT_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null || true)"
fi
if [ -z "$PROJECT_ROOT" ] || [ ! -d "$PROJECT_ROOT/.ori" ]; then echo "ERROR: cannot find project root (.ori/ not found)" >&2; exit 1; fi
cd "$PROJECT_ROOT"

echo "=== ori-doctor health check ==="
echo ""

TOTAL=0

for check in "$SCRIPT_DIR"/check-*.sh; do
  name=$(basename "$check" .sh | sed 's/^check-//')
  echo "[$name]"
  "$check" 2>&1 || true
  TOTAL=$((TOTAL + ${PIPESTATUS[0]:-0}))
  echo ""
done

echo "[lint]"
if command -v node >/dev/null 2>&1 && [ -f "$SCRIPT_DIR/lint.js" ]; then
  node "$SCRIPT_DIR/lint.js" 2>&1 || true
  TOTAL=$((TOTAL + ${PIPESTATUS[0]:-0}))
else
  echo "  lint.js not found or node unavailable, skipping JS lint check"
fi
echo ""

echo "=== Total issues: $TOTAL ==="
exit $(( TOTAL > 0 ? 1 : 0 ))
