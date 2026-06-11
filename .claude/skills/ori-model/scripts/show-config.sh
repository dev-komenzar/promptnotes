#!/usr/bin/env bash
# ori-model: show current model configuration
# Reads .apm/agents/*.md files and displays capability→model mapping
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

AGENT_DIR=".apm/agents"

if [[ ! -d "$AGENT_DIR" ]]; then
  echo "no .apm/agents/ directory"
  exit 0
fi

echo "capability    model"
echo "────────────────────────────────────"

for f in "$AGENT_DIR"/*.md; do
  [[ -f "$f" ]] || continue
  name=$(basename "$f" .md)
  capability=$(grep '^capability:' "$f" 2>/dev/null | sed 's/capability: *//' || echo "-")
  model=$(grep '^model:' "$f" 2>/dev/null | sed 's/model: *//' || echo "-")
  echo "$name  capability=$capability  model=$model"
done
