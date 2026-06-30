#!/usr/bin/env bash
# ori-doctor: Slice DoD sweep.
#
# Walks .ori/slices/*/manifest.yaml, resolves each slice's source-tree location
# from .ori/architecture.md, and checks the 4 Slice DoD rules declared in
# .apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md ("Slice Definition of Done"):
#
#   rule:dod-1  sub_layers 全埋め違反 (declared layer dir が空 / placeholder のみ)
#   rule:dod-2  boundary 経由 test 違反 (tests が application/ 直 import / bindings 不使用)
#   rule:dod-3  production wiring 違反 (tests が setupProductionBuilder() を経由していない)
#   rule:dod-4  cross_root 同期切れ (commands.rs が bindings.ts より新しい = specta 再生成漏れ)
#
# Output:
#   stdout: human-readable report (1 finding per line)
#   exit: 0 = no violations, 1 = violations found
#
# When --emit-issues is set, each violation is filed as a bd issue with the
# label convention from .apm/instructions/task-management.instructions.md
# ("/ori-doctor violation issue の label convention"). Idempotency is
# enforced by querying `bd list --label=dod-violation --label=slice:<id>
# --label=rule:<rule-id>` before creating — same violation never re-files.
#
# Limitations (pragmatic heuristics):
#   - YAML parsing uses grep/sed, not a full parser. Manifests using exotic
#     YAML shapes may slip past. yq is preferred when present (auto-detected).
#   - rule:dod-3 is a textual check (greps for "setupProductionBuilder"); a
#     test that imports the helper but still constructs fakes inline will
#     pass this gate. The /ori-review step should catch the rest.
#   - rule:dod-4 compares mtimes, not content hashes — slow CI runs that
#     touch bindings.ts as a side effect can produce false negatives.
set -euo pipefail

EMIT_ISSUES=false
PROJECT_ROOT=""
RUN_TESTS=false

usage() {
  cat >&2 <<'EOF'
Usage: check-dod-sweep.sh [options]

Options:
  --emit-issues       Auto-file bd issues for each violation found
                      (idempotent: dedupes by slice + rule label set).
  --run-tests         Additionally run the slice's tests with the production
                      fixture and treat a failure as a rule:dod-3 violation.
                      OFF by default (heavy; CI-mode only).
  --project-root <d>  Project root containing .ori/. Default: auto-detect.
  -h, --help          Show this help and exit.

Exit codes:
  0  no violations
  1  one or more violations detected
  2  usage / config error
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --emit-issues) EMIT_ISSUES=true; shift ;;
    --run-tests)   RUN_TESTS=true; shift ;;
    --project-root) PROJECT_ROOT="${2:-}"; shift 2 ;;
    -h|--help)     usage; exit 0 ;;
    *) echo "ERROR: unknown argument: $1" >&2; usage; exit 2 ;;
  esac
done

# Auto-detect project root (PWD-first; SCRIPT_DIR is last-resort fallback).
# Why PWD-first: when ori bundle is installed inside a user project, SCRIPT_DIR
# points into the ori repo, so `git -C "$SCRIPT_DIR" rev-parse` resolves to the
# ori repo root and the user's .ori/ becomes invisible (ori-fzr.15).
if [[ -z "$PROJECT_ROOT" ]]; then
  PWD_DIR="$(pwd)"
  PROJECT_ROOT="$(git -C "$PWD_DIR" rev-parse --show-toplevel 2>/dev/null || true)"
  if [[ -n "$PROJECT_ROOT" && ! -d "$PROJECT_ROOT/.ori" ]]; then PROJECT_ROOT=""; fi
  if [[ -z "$PROJECT_ROOT" ]]; then
    d="$PWD_DIR"
    while [[ "$d" != "/" ]]; do
      [[ -d "$d/.ori" ]] && { PROJECT_ROOT="$d"; break; }
      d="$(dirname "$d")"
    done
  fi
  if [[ -z "$PROJECT_ROOT" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
    PROJECT_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null || true)"
  fi
fi
[[ -n "$PROJECT_ROOT" && -d "$PROJECT_ROOT/.ori" ]] || { echo "ERROR: .ori/ not found under ${PROJECT_ROOT:-<unresolved>}" >&2; exit 2; }
cd "$PROJECT_ROOT"

ARCH="$PROJECT_ROOT/.ori/architecture.md"
[[ -f "$ARCH" ]] || { echo "INFO: no .ori/architecture.md — skipping DoD sweep"; exit 0; }

# Pull a flat top-level scalar from a YAML file. Greps the first occurrence
# of `<key>: <value>` ignoring nested indentation. Good enough for
# config.yaml / manifest.yaml top-level entries.
yaml_get() {
  local file="$1" key="$2"
  grep -E "^${key}:" "$file" 2>/dev/null | head -n1 | sed -E "s/^${key}:[[:space:]]*//; s/[[:space:]]*#.*$//; s/^['\"]//; s/['\"]$//"
}

# Resolve <app_name> from .ori/config.yaml workspace.apps[0].name.
APP_NAME="$(grep -E '^\s*-\s*name:' "$PROJECT_ROOT/.ori/config.yaml" 2>/dev/null | head -n1 | sed -E 's/^\s*-\s*name:\s*//' || true)"
[[ -n "$APP_NAME" ]] || APP_NAME=""

# Decide whether the project uses a Tauri stack (presence of `cross_root:`
# in architecture.md is the signal; the typescript-tauri stack template
# declares it, the typescript stack template does not).
TAURI_STACK=false
grep -qE '^cross_root:' "$ARCH" && TAURI_STACK=true

VIOLATIONS=0

# Walk every slice declared under .ori/slices/.
slices=()
while IFS= read -r -d '' f; do slices+=("$f"); done < <(find "$PROJECT_ROOT/.ori/slices" -mindepth 2 -maxdepth 2 -name manifest.yaml -print0 2>/dev/null)

if [[ ${#slices[@]} -eq 0 ]]; then
  echo "INFO: no slices found under .ori/slices/ — skipping DoD sweep"
  exit 0
fi

# Emit a violation line to stdout; if --emit-issues, also file a bd issue
# (idempotent — existing open issue with the same slice+rule label set
# short-circuits).
emit_violation() {
  local slice_id="$1" rule_id="$2" summary="$3" detail="$4"
  VIOLATIONS=$((VIOLATIONS + 1))
  printf '✗ [%s] %s — %s\n    detail: %s\n' "$rule_id" "$slice_id" "$summary" "$detail"

  if [[ "$EMIT_ISSUES" != true ]]; then return 0; fi
  if ! command -v bd >/dev/null 2>&1; then
    echo "    WARN: bd not on PATH; cannot auto-file issue" >&2
    return 0
  fi

  # Idempotency: dedupe by slice + rule. The label convention is the SSoT
  # (.apm/instructions/task-management.instructions.md).
  local existing
  existing="$(bd list --label=dod-violation --label="slice:${slice_id}" --label="rule:${rule_id}" --status=open 2>/dev/null | grep -E '^○|^◐' | head -n1 || true)"
  if [[ -n "$existing" ]]; then
    echo "    INFO: existing open issue found — skipping re-file (idempotent)"
    return 0
  fi

  local title="[dod:${rule_id}] ${slice_id}: ${summary}"
  local desc="${detail}

Reference:
- Slice DoD rule definitions: .apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md (\"Slice Definition of Done\")
- Label convention: .apm/instructions/task-management.instructions.md (\"/ori-doctor violation issue の label convention\")
- Auto-filed by: .apm/skills/ori-doctor/scripts/check-dod-sweep.sh"

  bd create \
    --title="$title" \
    --description="$desc" \
    --type=bug \
    --priority=2 \
    --labels="dod-violation,slice:${slice_id},rule:${rule_id}" >/dev/null \
    && echo "    ✓ filed bd issue (dod-violation, slice:${slice_id}, rule:${rule_id})" \
    || echo "    WARN: bd create failed — issue not filed" >&2
}

# Per-slice DoD checks.
for manifest in "${slices[@]}"; do
  slice_dir="$(dirname "$manifest")"
  slice_id="$(basename "$slice_dir")"
  bc_kebab="$(yaml_get "$manifest" "bc")"
  bc_snake="${bc_kebab//-/_}"
  slice_snake="${slice_id//-/_}"

  # Resolve TS / Rust source-tree paths for this slice.
  ts_slice="apps/${APP_NAME}/src/${bc_kebab}/slices/${slice_id}"
  rs_slice="apps/${APP_NAME}/src-tauri/src/${bc_snake}/slices/${slice_snake}"

  # --- rule:dod-1 — sub_layers の全埋め -----------------------------------
  # manifest.expected_deliverables.sub_layers の値を行頭スペース付きで grep。
  # 宣言が存在する layer はディレクトリ実体に non-empty file が要る。
  sub_layers="$(awk '/^expected_deliverables:/{flag=1; next} flag && /^[^ ]/{flag=0} flag && /^[[:space:]]+sub_layers:/{getline; while($0 ~ /^[[:space:]]+-/){gsub(/^[[:space:]]+-[[:space:]]*/, ""); print; getline}}' "$manifest" 2>/dev/null || true)"
  while IFS= read -r layer; do
    [[ -z "$layer" ]] && continue
    ts_layer_dir="$ts_slice/$layer"
    if [[ -d "$ts_layer_dir" ]]; then
      # Non-empty if any non-.gitkeep file exists in the layer.
      payload="$(find "$ts_layer_dir" -type f ! -name '.gitkeep' 2>/dev/null | head -n1)"
      [[ -n "$payload" ]] && continue
    fi
    # Rust side fallback (Tauri stack only; <layer>.rs single-file convention).
    if [[ "$TAURI_STACK" == true && -f "$rs_slice/${layer}.rs" ]]; then
      [[ -s "$rs_slice/${layer}.rs" ]] && continue
    fi
    emit_violation "$slice_id" "rule:dod-1" \
      "sub_layer '${layer}' が空または不在" \
      "expected_deliverables.sub_layers で宣言された '${layer}' が ${ts_layer_dir} (TS) または ${rs_slice}/${layer}.rs (Rust) のいずれにも実体を持たない"
  done <<< "$sub_layers"

  # --- rule:dod-2 — boundary 経由 test (Tauri stack のみ) -----------------
  # tests/ が bindings 経由で呼んでいるか + application/ 直 import が無いか。
  tests_dir="$ts_slice/tests"
  if [[ "$TAURI_STACK" == true && -d "$tests_dir" ]]; then
    # NG: application/handle_* 直 import
    if grep -RInE '(import|use)[^;\n]+application/' "$tests_dir" 2>/dev/null | head -n1 >/dev/null; then
      offending="$(grep -RInE '(import|use)[^;\n]+application/' "$tests_dir" 2>/dev/null | head -n1)"
      emit_violation "$slice_id" "rule:dod-2" \
        "tests が application/ を直 import" \
        "${offending}"
    fi
    # OK 必須: bindings 経由 import
    if ! grep -RInE 'shared/ipc/bindings' "$tests_dir" 2>/dev/null | head -n1 >/dev/null; then
      emit_violation "$slice_id" "rule:dod-2" \
        "tests が bindings 経由で invoke していない" \
        "${tests_dir} 配下の test file が '${bc_kebab}/shared/ipc/bindings' を import しない (DoD rule 2)"
    fi
  fi

  # --- rule:dod-3 — production wiring -------------------------------------
  if [[ "$TAURI_STACK" == true && -d "$tests_dir" ]]; then
    if ! grep -RInE 'setupProductionBuilder' "$tests_dir" 2>/dev/null | head -n1 >/dev/null; then
      emit_violation "$slice_id" "rule:dod-3" \
        "tests が production fixture (setupProductionBuilder) を経由していない" \
        "${tests_dir} 配下に 'setupProductionBuilder' 参照が無い。inline fake/mock で構築している疑い (DoD rule 3)"
    fi

    if [[ "$RUN_TESTS" == true && -n "$APP_NAME" ]] && command -v pnpm >/dev/null 2>&1; then
      if ! pnpm -F "$APP_NAME" test "$tests_dir" >/tmp/dod-sweep-test-$$.log 2>&1; then
        emit_violation "$slice_id" "rule:dod-3" \
          "production fixture 経由の test が green でない" \
          "pnpm -F ${APP_NAME} test ${tests_dir} が失敗 — log: /tmp/dod-sweep-test-$$.log"
      fi
    fi
  fi

  # --- rule:dod-4 — cross_root 同期 (commands.rs mtime vs bindings.ts) ----
  if [[ "$TAURI_STACK" == true ]]; then
    cmd_rs="$rs_slice/commands.rs"
    bindings_ts="apps/${APP_NAME}/src/${bc_kebab}/shared/ipc/bindings.ts"
    if [[ -f "$cmd_rs" && -f "$bindings_ts" ]]; then
      # bindings.ts older than commands.rs → specta 再生成漏れの強い疑い
      if [[ "$cmd_rs" -nt "$bindings_ts" ]]; then
        emit_violation "$slice_id" "rule:dod-4" \
          "commands.rs が bindings.ts より新しい (specta 再生成漏れ)" \
          "bash apm-scripts/specta-build.sh --app-dir apps/${APP_NAME} を実行して同期し、再 sweep してください"
      fi
    fi
  fi
done

echo
echo "=== DoD sweep summary: ${VIOLATIONS} violation(s) across ${#slices[@]} slice(s) ==="
exit $(( VIOLATIONS > 0 ? 1 : 0 ))
