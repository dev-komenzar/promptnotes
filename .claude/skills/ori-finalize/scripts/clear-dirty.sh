#!/usr/bin/env bash
# ori-finalize: clear dirty marks and update status
# Usage: ./clear-dirty.sh <slice-id>
#
# Guard: review.md must exist with verdict=PASS before dirty is cleared.
# Bypass (human override): ./clear-dirty.sh <slice-id> --skip-review-check
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
SKIP_REVIEW="${2:-}"

if [[ -z "$ID" ]]; then
  echo "ERROR: slice-id required" >&2
  exit 1
fi

STATUS=".ori/slices/$ID/status.yaml"
if [[ ! -f "$STATUS" ]]; then
  echo "ERROR: $STATUS not found" >&2
  exit 1
fi

# --- Review gate ---
if [[ "$SKIP_REVIEW" != "--skip-review-check" ]]; then
  REVIEW_MD=".ori/slices/$ID/review.md"

  if [[ ! -f "$REVIEW_MD" ]]; then
    echo "❌ Cannot clear dirty: review.md does not exist for $ID" >&2
    echo "   Run /ori-review $ID first (or /ori-flow $ID to run all phases)." >&2
    echo "   If you absolutely need to bypass this guard, use:" >&2
    echo "     $0 $ID --skip-review-check" >&2
    exit 1
  fi

  # Extract verdict from review.md (look for verdict=PASS in ## Pass sections)
  VERDICT=$(grep -oP 'verdict\s*[=:]\s*\K(PASS|NEEDS_FIX|REJECT)' "$REVIEW_MD" | tail -1)
  if [[ -z "$VERDICT" ]]; then
    echo "❌ Cannot clear dirty: no verdict found in $REVIEW_MD" >&2
    echo "   Run /ori-review $ID to complete review." >&2
    exit 1
  fi

  if [[ "$VERDICT" != "PASS" ]]; then
    echo "❌ Cannot clear dirty: review verdict is '$VERDICT', must be 'PASS'" >&2
    echo "   Address review findings, re-run /ori-review $ID, then finalize." >&2
    exit 1
  fi

  echo "✅ review gate: verdict=PASS confirmed for $ID"
fi

# Replace dirty list with empty
sed -i 's/^dirty:.*/dirty: []/' "$STATUS"
echo "cleared dirty marks for $ID"
