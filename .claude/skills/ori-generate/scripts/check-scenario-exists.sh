#!/usr/bin/env bash
# ori-generate: check if a scenario exists
# Usage: ./check-scenario-exists.sh <scenario-id>
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
if [[ -z "$ID" ]]; then
  echo "ERROR: scenario-id required" >&2
  exit 1
fi

if [[ -f ".ori/scenarios/$ID/manifest.yaml" ]]; then
  echo "$ID"
  exit 0
fi

# Fuzzy match among scaffolded scenarios
CANDIDATES=$(find .ori/scenarios -maxdepth 2 -name 'manifest.yaml' 2>/dev/null | sed 's|.ori/scenarios/||;s|/manifest.yaml||' | grep -i "$ID" || true)

# Validation section anchors (scenario id = validation.md H2 anchor, 1:1 —
# see scenario.instructions.md §id-convention). These are the scaffold source
# of truth: an exact anchor hit means the scenario exists upstream but has
# not been scaffolded yet.
VAL_ANCHORS=""
if [[ -f ".ori/domain/validation.md" ]]; then
  VAL_ANCHORS=$(sed -n 's/^## Scenario.*{#\([a-z][a-z0-9-]*\)}[[:space:]]*$/\1/p' .ori/domain/validation.md)
fi
EXACT_ANCHOR=$(printf '%s\n' "$VAL_ANCHORS" | grep -xF "$ID" || true)
FUZZY_ANCHORS=$(printf '%s\n' "$VAL_ANCHORS" | grep -i "$ID" | grep -vx -- "$ID" || true)

if [[ -n "$CANDIDATES" ]]; then
  echo "not found, candidates:" >&2
  echo "$CANDIDATES" >&2
  if [[ -n "$FUZZY_ANCHORS" ]]; then
    echo "validation.md anchors:" >&2
    echo "$FUZZY_ANCHORS" >&2
  fi
  exit 2
fi

if [[ -n "$EXACT_ANCHOR" ]]; then
  echo "not scaffolded, but validation.md has section anchor '$ID' (scenario id = anchor, 1:1)" >&2
  echo "scaffold via the ori-flow skill bundle: node <skill-bundle>/scripts/new-scenario.js $ID (after user confirmation)" >&2
  exit 3
fi

if [[ -n "$FUZZY_ANCHORS" ]]; then
  echo "not found, validation.md anchor candidates:" >&2
  echo "$FUZZY_ANCHORS" >&2
  exit 2
fi

echo "not found" >&2
exit 1
