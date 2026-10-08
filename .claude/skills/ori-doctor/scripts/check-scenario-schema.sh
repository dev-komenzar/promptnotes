#!/usr/bin/env bash
# ori-doctor: check scenario status.yaml ⟷ artifact consistency
# Validates: phases/completion の自己整合、および phase 状態と成果物 (tests/, review.md) の実在突合
# phases:{} なのに成果物が存在する drift (ori-7jm) を事後検出する。WARN only (in-progress を fail させない)
#
# 前提・判定根拠:
#  - status.yaml は scenario-status.js / new-scenario.js が出力する yaml.stringify の block style を前提とする
#    (flow style `completion: [a, b]` の手編集は解釈できず false positive になりうる)
#  - derive は spec.md の実在を根拠にしない (new-scenario.js が scaffold 時点で spec.md を作るため)
#  - finalize は固有の成果物が無い。代わりに「review=done なのに finalize 未記録 (review=done は PASS 時のみ記録される)」を drift とみなす
#    (進行中の scenario でも当たりうるので WARN 文言に「進行中なら無視可」を明記している)
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

ISSUES=0

if [[ ! -d .ori/scenarios ]]; then
  echo "  scenarios: no .ori/scenarios/ directory"
  exit 0
fi

# phase 名 → state / completion 所属を status.yaml (yaml.stringify 形式) から awk で抽出
# legacy の scalar 形式 (`  generate: completed`) は `legacy:<value>` を返す
phase_state() { # <file> <phase> → state (無ければ空)
  awk -v p="$2" '
    /^phases:/ { inph=1; next }
    inph && /^[^ ]/ { inph=0 }
    inph && $0 ~ "^  " p ": *[^ ]" { v=$0; sub("^  " p ": *", "", v); print "legacy:" v; exit }
    inph && $0 ~ "^  " p ":" { inp=1; next }
    inph && /^  [^ ]/ { inp=0 }
    inph && inp && /^    state:/ { print $2; exit }
  ' "$1"
}
in_completion() { # <file> <phase> → 0 if listed
  awk -v p="$2" '
    /^  completion:/ { inc=1; next }
    inc && /^  [^ -]/ { inc=0 }
    inc && /^[^ ]/ { inc=0 }
    inc && $0 ~ "^    - " p "$" { found=1 }
    END { exit found ? 0 : 1 }
  ' "$1"
}

# 成果物突合用: legacy vocab は done 相当 (done/closed/complete/completed) のみ done に正規化
effective_state() { # <state> → state
  case "$1" in
    legacy:done|legacy:closed|legacy:complete|legacy:completed) echo done ;;
    legacy:*) echo "" ;;
    *) echo "$1" ;;
  esac
}

has_tests() { [[ -d "$1/tests" ]] && [[ -n "$(find "$1/tests" -type f 2>/dev/null | head -1)" ]]; }

warn() { echo "  WARN  scenarios/$1: $2"; echo "        fix: $3"; ((ISSUES++)) || true; }

for dir in .ori/scenarios/*/; do
  [[ -d "$dir" ]] || continue
  dir="${dir%/}"
  # scenario は manifest.yaml を持つ dir に限る (wdio の node_modules symlink 等を除外)
  [[ -f "$dir/manifest.yaml" ]] || continue
  id=$(basename "$dir")
  status="$dir/status.yaml"

  if [[ ! -f "$status" ]]; then
    warn "$id" "no status.yaml (created outside new-scenario.js or legacy); phase 台帳が無い" \
         "restore .ori/scenarios/$id/status.yaml, then node <ori-flow>/scripts/scenario-status.js set $id <phase> done"
    continue
  fi

  fixcmd="node <ori-flow>/scripts/scenario-status.js set $id"

  for phase in derive generate review finalize; do
    st="$(phase_state "$status" "$phase")"
    comp=0; in_completion "$status" "$phase" && comp=1
    # legacy 形式の phase は legacy WARN のみ報告し completion 突合はしない
    # (scenario-status.js set で正規化した後の再実行で突合される。二重報告を避ける意図)
    if [[ "$st" == legacy:* ]]; then
      warn "$id" "phases.$phase が legacy 形式 (${st#legacy:}); {state: ...} 形式でない" "$fixcmd $phase <started|done|failed> (正規化して書き戻される)"
    elif [[ "$st" == "done" && $comp -eq 0 ]]; then
      warn "$id" "phases.$phase=done but beads.completion に $phase が無い" "$fixcmd $phase done"
    elif [[ "$st" != "done" && $comp -eq 1 ]]; then
      warn "$id" "beads.completion に $phase があるが phases.$phase.state=${st:-(未記録)}" "$fixcmd $phase done (または completion を修正)"
    fi
  done

  # 成果物実在 ⟷ phase 状態 (spec.md は scaffold が作るため根拠にしない)
  gen_st="$(effective_state "$(phase_state "$status" generate)")"
  rev_st="$(effective_state "$(phase_state "$status" review)")"

  if has_tests "$dir"; then
    [[ "$gen_st" == "done" ]] || warn "$id" "tests/ が存在するが phases.generate=${gen_st:-(未記録)} (台帳 drift)" "$fixcmd generate done"
  else
    [[ "$gen_st" != "done" ]] || warn "$id" "phases.generate=done だが tests/ に成果物が無い" "/ori-flow $id (generate をやり直す) または台帳を修正"
  fi

  if [[ -f "$dir/review.md" ]]; then
    [[ "$rev_st" == "done" ]] || warn "$id" "review.md が存在するが phases.review=${rev_st:-(未記録)} (台帳 drift)" "$fixcmd review done"
  else
    [[ "$rev_st" != "done" ]] || warn "$id" "phases.review=done だが review.md が無い" "/ori-review $id (review をやり直す) または台帳を修正"
  fi

  # review=done なのに finalize 未記録 (finalize に成果物が無いための代替検出)
  fin_st="$(effective_state "$(phase_state "$status" finalize)")"
  # phases.review=done は ori-review が PASS 時にのみ書く (決定的 writer) ので台帳だけで判定する。
  # review.md の verdict 行は書式揺れ・履歴混在があるため見ない
  if [[ "$rev_st" == "done" && "$fin_st" != "done" ]]; then
    warn "$id" "review=done だが phases.finalize=${fin_st:-(未記録)} (台帳 drift。進行中の scenario なら無視可)" \
         "/ori-finalize $id または $fixcmd finalize done"
  fi
done

echo "  scenarios schema: $ISSUES issue(s)"
exit $ISSUES
