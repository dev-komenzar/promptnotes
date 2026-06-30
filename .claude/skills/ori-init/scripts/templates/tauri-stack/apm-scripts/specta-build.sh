#!/usr/bin/env bash
# specta-build.sh — regenerate tauri-specta TS bindings.
#
# Called by the `phase_hooks.flow-impl-{red-pre,green-post}` defined in
# .ori/architecture.md (typescript-tauri stack). Wraps
#   cd <app>/src-tauri && cargo run --bin export-types
# with the workspace-relative path the hooks expect.
#
# Exit codes:
#   0  bindings regenerated (or no-op if cargo not installed)
#   1  cargo run failed
#   2  usage error / invalid --app-dir
set -euo pipefail

APP_DIR=""

usage() {
  cat >&2 <<'EOF'
Usage: specta-build.sh --app-dir <apps/<app-name>>

Regenerates the tauri-specta bindings under
  <app-dir>/src/<BC>/shared/ipc/bindings.ts
by invoking `cargo run --bin export-types` inside <app-dir>/src-tauri.

Options:
  --app-dir <dir>   The Tauri app directory (must contain src-tauri/).
  -h, --help        Show this help and exit.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --app-dir) APP_DIR="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "ERROR: unknown argument: $1" >&2; usage; exit 2 ;;
  esac
done

if [[ -z "$APP_DIR" ]]; then
  echo "ERROR: --app-dir is required" >&2
  usage
  exit 2
fi

if [[ ! -d "$APP_DIR/src-tauri" ]]; then
  echo "ERROR: $APP_DIR/src-tauri not found (run pnpm tauri init first)" >&2
  exit 2
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "NOTE: cargo not on PATH — skipping specta build." >&2
  exit 0
fi

echo "Running: cargo run --bin export-types  (in $APP_DIR/src-tauri)"
( cd "$APP_DIR/src-tauri" && cargo run --quiet --bin export-types )
