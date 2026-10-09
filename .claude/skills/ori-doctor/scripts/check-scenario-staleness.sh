#!/usr/bin/env bash
# ori-doctor: check scenario review.md staleness (ori-7jm.3)
# review.md の mtime が scenario の入力成果物 (spec.md / manifest.yaml / validation.md / runner config / docker-compose.yml / tests/**) より古ければ「陳腐化 review」として WARN。
# spec/tests/config を再生成・編集したのに旧 review.md が残っている状態を検出し、/ori-flow 再走を促す。WARN only。
#
# 前提・限界:
#  - 判定は mtime のみ。git checkout/clone は mtime を揃えるため false negative になりうる（同秒は stale 扱いにしない）
#  - review.md が無い scenario は対象外 (未 review は check-scenario-schema.sh が扱う)
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

# epoch 秒の mtime (GNU stat → BSD stat の順でフォールバック。秒単位比較なので同秒は stale にしない)
mtime() { stat -c %Y "$1" 2>/dev/null || stat -f %m "$1"; }

for dir in .ori/scenarios/*/; do
  [[ -d "$dir" ]] || continue
  dir="${dir%/}"
  id=$(basename "$dir")
  review="$dir/review.md"
  [[ -f "$review" ]] || continue

  rev_m=$(mtime "$review")
  newer=()
  # review の入力となる成果物 (scenario 直下の生成物 + tests/ 配下) のうち review.md より新しいもの。
  # status.yaml と review.md 自身は除外
  while IFS= read -r f; do
    [[ -n "$f" ]] || continue
    if [[ $(mtime "$f") -gt $rev_m ]]; then newer+=("${f#"$dir"/}"); fi
  done < <({ for g in spec.md manifest.yaml validation.md playwright.config.ts wdio.conf.ts teardown.mjs tsconfig.json docker-compose.yml; do [[ -f "$dir/$g" ]] && echo "$dir/$g"; done; [[ -d "$dir/tests" ]] && find "$dir/tests" -type f -not -path '*/node_modules/*'; } 2>/dev/null)

  if [[ ${#newer[@]} -gt 0 ]]; then
    shown=$(printf '%s, ' "${newer[@]:0:3}"); shown="${shown%, }"
    extra=""; [[ ${#newer[@]} -gt 3 ]] && extra=" 他 $(( ${#newer[@]} - 3 )) 件"
    echo "  WARN  scenarios/$id: review.md が陳腐化 (review.md より新しい: ${shown}${extra})"
    echo "        fix: /ori-flow $id を再走して再レビュー (review→finalize を再実行)"
    ((ISSUES++)) || true
  fi
done

echo "  scenarios staleness: $ISSUES issue(s)"
exit $ISSUES
