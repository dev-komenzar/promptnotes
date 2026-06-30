#!/usr/bin/env bash
# ori-doctor: check domain document schema
# Validates: frontmatter coherence block, H2/H3 {#id} anchors, required sections
set -euo pipefail

# Auto-detect project root (PWD-first; SCRIPT_DIR fallback last).
# Why: when ori is installed inside a user project, SCRIPT_DIR resolves to the
# ori repo so git toplevel misses the user's .ori/ (ori-fzr.15).
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

DIR="${1:-.ori/domain}"
ISSUES=0

for f in "$DIR"/*.md "$DIR"/**/*.md; do
  [[ -f "$f" ]] || continue
  rel="${f#.ori/}"

  # Check frontmatter has coherence block
  if ! grep -q '^coherence:' "$f" 2>/dev/null; then
    echo "  WARN  $rel: missing coherence frontmatter"
    ((ISSUES++)) || true
  fi

  # Check H2/H3 have {#id} anchors
  BARE=$(grep -nE '^##[^#]' "$f" | grep -v '{#' || true)
  if [[ -n "$BARE" ]]; then
    echo "  WARN  $rel: H2/H3 without {#id} anchor:"
    echo "$BARE" | while read -r line; do echo "         $line"; done
    ((ISSUES++)) || true
  fi
done

echo "  domain schema: $ISSUES issue(s)"
exit $ISSUES
