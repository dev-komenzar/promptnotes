#!/usr/bin/env bash
# ori-init: create .ori/ skeleton in the current project.
#
# Skill-owned implementation per ori-execution-model-shift-2026-06-03
# (CLI 廃止 → skill のスクリプト実行ベース). This script is the single
# source of truth for `.ori/` initialization — invoked directly by the
# /ori-init skill; no npm library dependency.
#
# Usage: create-skeleton.sh [--force] [--dest <dir>] [--app-name <name>] [--agent <name>]
#
# Exit codes:
#   0  success
#   1  --dest invalid, missing template, or filesystem error
#   2  usage error (unknown flag, invalid --app-name, or invalid --agent)
set -euo pipefail

FORCE=false
DEST=""
APP_NAME_ARG=""
AGENT_ARG=""

# Supported agents — must match the keys under `ori.agents.*` in
# templates/config.yaml. Single source of truth for both --agent
# validation and auto-detection.
SUPPORTED_AGENTS=(claude codex opencode gemini cursor copilot)

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TPL_DIR="$SCRIPT_DIR/templates"

usage() {
  cat >&2 <<'EOF'
Usage: create-skeleton.sh [options]

Options:
  --force             Overwrite existing .ori/ files when present
  --dest <dir>        Destination directory (default: current working directory)
  --app-name <name>   App name written to .ori/config.yaml workspace.apps[0].name
                      (default: derived from --dest folder basename).
                      Sanitized to [a-z0-9-]; empty after sanitize → exit 2.
  --agent <name>      Active agent written to .ori/config.yaml current_agent
                      (one of: claude codex opencode gemini cursor copilot).
                      Default: auto-detected from --dest markers
                      (.claude/, .opencode/, .codex/, .gemini/, .cursor/,
                       .cursorrules, .github/copilot-instructions.md,
                       .github/copilot/) with priority claude > opencode >
                      codex > gemini > cursor > copilot; falls back to
                      claude when nothing is detected.
  -h, --help          Show this help and exit
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --force)     FORCE=true; shift ;;
    --dest)      DEST="${2:-}"; shift 2 ;;
    --app-name)  APP_NAME_ARG="${2:-}"; shift 2 ;;
    --agent)     AGENT_ARG="${2:-}"; shift 2 ;;
    -h|--help)   usage; exit 0 ;;
    *) echo "ERROR: unknown argument: $1" >&2; usage; exit 2 ;;
  esac
done

DEST="${DEST:-$PWD}"
if [[ ! -d "$DEST" ]]; then
  echo "ERROR: --dest does not exist: $DEST" >&2
  exit 1
fi
DEST="$(cd "$DEST" && pwd)"

if [[ ! -f "$TPL_DIR/config.yaml" || ! -f "$TPL_DIR/domain-scaffold.md.tpl" ]]; then
  echo "ERROR: skill templates missing under $TPL_DIR" >&2
  exit 1
fi

# Resolve app name. Default derives from --dest folder basename, but
# --app-name overrides it so the /ori-init skill can let the user
# customize when the cwd basename is too long or word-split differently
# than they want (ori-gag). Same sanitization applies to either source:
# lowercase, non-[a-z0-9-] → '-', collapse '-', trim hyphens. The
# basename path falls back to "app" on empty; a user-supplied value
# that sanitizes to empty is rejected (exit 2) instead of silently
# becoming "app" — that would mask a typo.
sanitize_app_name() {
  printf '%s' "$1" | tr '[:upper:]' '[:lower:]' \
    | sed -e 's/[^a-z0-9-]\{1,\}/-/g' -e 's/-\{2,\}/-/g' -e 's/^-//' -e 's/-$//'
}

if [[ -n "$APP_NAME_ARG" ]]; then
  app_name="$(sanitize_app_name "$APP_NAME_ARG")"
  if [[ -z "$app_name" ]]; then
    echo "ERROR: --app-name '$APP_NAME_ARG' sanitizes to empty (allowed: [a-z0-9-])" >&2
    exit 2
  fi
else
  folder="$(basename "$DEST")"
  app_name="$(sanitize_app_name "$folder")"
  [[ -z "$app_name" ]] && app_name="app"
fi

# Resolve current_agent. Hard-coding claude (ori-zpy) silently mis-tagged
# projects whose primary harness was opencode/codex/etc., because the rest
# of ori uses current_agent to pick capability_to_model and spawn args.
# Resolution order:
#   1. explicit --agent <name>  (validated against SUPPORTED_AGENTS)
#   2. directory markers under $DEST in priority order
#   3. fallback "claude" (matches the historical default so existing
#      tests / behavior on a bare directory stay stable)
agent_supported() {
  local cand="$1" a
  for a in "${SUPPORTED_AGENTS[@]}"; do
    [[ "$a" == "$cand" ]] && return 0
  done
  return 1
}

# Detection markers — listed in priority order. Each row is
# "agent|test-spec" where test-spec is a space-separated list of paths
# (relative to $DEST); presence of ANY one is a positive detection.
# Priority rationale: claude first preserves the historical default for
# projects with multiple harnesses; opencode/codex follow per ori-zpy.
AGENT_MARKERS=(
  "claude|.claude"
  "opencode|.opencode"
  "codex|.codex"
  "gemini|.gemini"
  "cursor|.cursor .cursorrules"
  "copilot|.github/copilot-instructions.md .github/copilot"
)

detect_agents() {
  local row agent specs spec
  detected=()
  for row in "${AGENT_MARKERS[@]}"; do
    agent="${row%%|*}"
    specs="${row#*|}"
    for spec in $specs; do
      if [[ -e "$DEST/$spec" ]]; then
        detected+=("$agent")
        break
      fi
    done
  done
}

if [[ -n "$AGENT_ARG" ]]; then
  if ! agent_supported "$AGENT_ARG"; then
    echo "ERROR: --agent '$AGENT_ARG' not supported (allowed: ${SUPPORTED_AGENTS[*]})" >&2
    exit 2
  fi
  current_agent="$AGENT_ARG"
  echo "OK: using --agent override: $current_agent"
else
  detected=()
  detect_agents
  case "${#detected[@]}" in
    0)
      current_agent="claude"
      echo "INFO: no agent markers found under $DEST — defaulting current_agent=claude" >&2
      ;;
    1)
      current_agent="${detected[0]}"
      echo "OK: detected agent: $current_agent"
      ;;
    *)
      current_agent="${detected[0]}"
      echo "WARN: multiple agent markers detected (${detected[*]}); picking '$current_agent' by priority. Use --agent to override." >&2
      ;;
  esac
fi

# Directories — mirrors the (now-removed) CLI's DIRS table.
DIRS=(
  ".ori/domain/workflows"
  ".ori/domain/ui-fields"
  ".ori/domain/code"
  ".ori/slices"
  ".ori/pages"
  ".ori/proposals"
  ".ori/state"
)
for d in "${DIRS[@]}"; do
  mkdir -p "$DEST/$d"
done

# .gitkeep — keep empty VCS-tracked dirs visible.
for d in ".ori/slices" ".ori/pages" ".ori/proposals"; do
  target="$DEST/$d/.gitkeep"
  [[ -e "$target" ]] || : > "$target"
done

# .ori/config.yaml
CONFIG="$DEST/.ori/config.yaml"
if [[ -e "$CONFIG" && "$FORCE" != true ]]; then
  echo "WARN: .ori/config.yaml already exists. Use --force to overwrite." >&2
else
  # Use sentinel placeholders (__APP_NAME__, __CURRENT_AGENT__) substituted
  # via sed. Both values come from a constrained set (app_name sanitized to
  # [a-z0-9-]; current_agent validated against SUPPORTED_AGENTS), so no sed
  # metachar escaping is needed.
  sed -e "s/__APP_NAME__/$app_name/g" -e "s/__CURRENT_AGENT__/$current_agent/g" \
    "$TPL_DIR/config.yaml" > "$CONFIG"
  echo "OK: wrote .ori/config.yaml (app: $app_name, current_agent: $current_agent)"
fi

# .ori/.gitignore
# Seed with default ignore patterns to prevent build artifacts from phase 10
# (ori-ddd-10-types) from being accidentally committed. Real-world incident:
# `.ori/domain/code/rust/target/` was committed with 596 files because cargo
# check was run before .gitignore existed.
GITIGNORE="$DEST/.ori/.gitignore"
if [[ ! -e "$GITIGNORE" ]]; then
  cat > "$GITIGNORE" <<'EOF'
# ori runtime state
state/

# phase 10 (ori-ddd-10-types) build artifacts —
# cargo / npm / gradle / python can generate these under domain/code/<lang>/.
# Add language-specific .gitignore inside each code/<lang>/ as well.
domain/code/*/target/
domain/code/*/node_modules/
domain/code/*/dist/
domain/code/*/build/
domain/code/*/.gradle/
domain/code/*/__pycache__/
domain/code/*/.venv/
domain/code/**/*.tsbuildinfo
domain/code/**/*.pyc
domain/code/**/*.class
EOF
  echo "OK: wrote .ori/.gitignore (state/ + phase 10 build artifact patterns)"
fi

# Domain scaffolds — 12 phase outputs (DDD phase 1..11a + indexes).
# Format: path|title|phase. Order matches the original CLI table.
SCAFFOLDS=(
  ".ori/domain/discovery.md|Discovery|ori-ddd-1-discovery"
  ".ori/domain/event-storming.md|Event Storming|ori-ddd-2-event-storming"
  ".ori/domain/bounded-contexts.md|Bounded Contexts|ori-ddd-3-bounded-contexts"
  ".ori/domain/context-map.md|Context Map|ori-ddd-4-context-map"
  ".ori/domain/aggregates.md|Aggregates|ori-ddd-5-aggregates"
  ".ori/domain/domain-events.md|Domain Events|ori-ddd-6-domain-events"
  ".ori/domain/validation.md|Validation|ori-ddd-7-validation"
  ".ori/domain/glossary.md|Glossary|ori-ddd-8-glossary"
  ".ori/domain/workflows/index.md|Workflows Index|ori-ddd-9-workflows"
  ".ori/domain/types.md|Types Index|ori-ddd-10-types"
  ".ori/domain/code/index.md|Code Index|ori-ddd-10-types"
  ".ori/domain/ui-fields/index.md|UI Fields Index|ori-ddd-11a-ui-fields"
)

written=0
skipped=0
for entry in "${SCAFFOLDS[@]}"; do
  IFS='|' read -r rel title phase <<< "$entry"
  target="$DEST/$rel"
  if [[ -e "$target" && "$FORCE" != true ]]; then
    skipped=$((skipped + 1))
    continue
  fi
  mkdir -p "$(dirname "$target")"
  sed -e "s/__TITLE__/$title/g" -e "s/__PHASE__/$phase/g" \
    "$TPL_DIR/domain-scaffold.md.tpl" > "$target"
  written=$((written + 1))
done
[[ $written -gt 0 ]] && echo "OK: seeded $written domain scaffold file(s) under .ori/domain/"
[[ $skipped -gt 0 ]] && echo "INFO: skipped $skipped existing scaffold file(s) (use --force to overwrite)."

# Initialize bd (beads) workspace so /ori-flow and other skills that treat
# beads as their SSoT can run immediately. Best-effort: missing `bd` is a
# warning (some users don't use beads), existing `.beads/` is honored
# (idempotent), and any bd-side failure does not propagate to exit code.
init_bd_workspace() {
  if ! command -v bd >/dev/null 2>&1; then
    echo "NOTE: 'bd' not found on PATH — skipping beads workspace init." >&2
    echo "      Install beads (https://github.com/steveyegge/beads) to enable /ori-flow." >&2
    return 0
  fi
  if [[ -d "$DEST/.beads" ]]; then
    echo "NOTE: $DEST/.beads/ already exists — skipping bd init (idempotent)."
    return 0
  fi

  local prefix="${ORI_BD_PREFIX:-ori}"
  [[ "$prefix" != *- ]] && prefix="${prefix}-"

  echo "Running: bd init -p $prefix --non-interactive  (in $DEST)"
  if ! ( cd "$DEST" && bd init -p "$prefix" --non-interactive --quiet ); then
    echo "WARN: 'bd init' failed; continuing without beads workspace." >&2
    echo "      Re-run manually: bd init -p $prefix" >&2
    return 0
  fi
}

init_bd_workspace
