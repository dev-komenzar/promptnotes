#!/usr/bin/env bash
# ori-doctor: check ui-fields contract → scenario test-points traceability (ori-7jm.9, R9 future 枠の最小実装)
# .ori/domain/ui-fields/ の field contract (screen-*.md の field id {#screen-N-xxx} と、*.md 全体 (index/README 含む) のショートカット Cmd/Ctrl+Key) が
# いずれかの scenario / page / slice の spec.md#test-points に写っていなければ「未写像」として WARN。
# Cmd+N のような cross-screen shortcut が未実装・未検証のまま GREEN になる再発を防ぐ (P-C)。WARN only。
#
# 前提・限界:
#  - 写像判定は単語境界付きの文字列一致 (大小無視)。field は field id そのもの or purpose 部分 (screen-N- prefix 除去、'-' と空白は同一視) が出現すれば写像済み
#  - ショートカットは修飾キー表記ゆれ (Cmd/Command/Ctrl/Control/⌘、'+' 前後の空白、⌘S の '+' 省略) を同一視し、key は複数文字 (Enter 等) で右境界つきで比較する
#  - どの spec に写すべきかは判定しない (scenarios/pages/slices の test-points を 1 つの corpus として扱う)
#  - test-points が 1 つも無いときは skip (未 scaffold は check-scenario-coverage.sh が扱う)
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

shopt -s nullglob
contracts=(.ori/domain/ui-fields/*.md)
screens=(.ori/domain/ui-fields/screen-*.md)
if [[ ${#contracts[@]} -eq 0 ]]; then
  echo "  ui-fields: no .ori/domain/ui-fields/*.md (skipped)"
  exit 0
fi

# 全 scenario / page / slice の spec.md#test-points 節を 1 つの corpus に連結 (fenced code 内は除外、sub-bullet は含める)
corpus=""
for spec in .ori/scenarios/*/spec.md .ori/pages/*/spec.md .ori/slices/*/spec.md; do
  corpus+="$(awk '/^```/{c=!c} /^## .*\{#test-points\}/{f=1;next} /^## /{f=0} f && !c' "$spec")"$'\n'
done
if [[ -z "${corpus//[[:space:]]/}" ]]; then
  echo "  ui-fields: no spec.md#test-points (skipped)"
  exit 0
fi

# ショートカット正規化: 小文字化 → 修飾キー(+ 必須。⌘ のみ + 省略可)を mod+ に統一 → '+' 前後の空白除去
norm_sc() {
  tr '[:upper:]' '[:lower:]' <<<"$1" | sed -E \
    's/(cmd|command|ctrl|control|meta)[[:space:]]*\+[[:space:]]*/mod+/g; s/⌘[[:space:]]*\+?[[:space:]]*/mod+/g; s/[[:space:]]*\+[[:space:]]*/+/g'
}
corpus_lc="$(tr '[:upper:]' '[:lower:]' <<<"$corpus")"
corpus_sc="$(norm_sc "$corpus")"

# 正規表現 meta 文字のエスケープ (field id / key は英数と - + のみ想定だが念のため)
re_escape() { sed -E 's/[][\.^$*+?(){}|/]/\\&/g' <<<"$1"; }

# field contract (screen-*.md の field id): id 自体 or purpose 部分が単語境界つきで出現するか
for screen in "${screens[@]}"; do
  sid=$(basename "$screen" .md)
  while IFS= read -r fid; do
    [[ -n "$fid" ]] || continue
    purpose="${fid#"$sid"-}"
    id_re="$(re_escape "$fid")"
    pu_re="$(re_escape "$purpose" | sed 's/-/[- ]/g')"
    b_l='(^|[^a-z0-9-])'; b_r='([^a-z0-9-]|$)'
    if ! grep -qiE -- "${b_l}${id_re}${b_r}" <<<"$corpus_lc" && ! grep -qiE -- "${b_l}${pu_re}${b_r}" <<<"$corpus_lc"; then
      echo "  WARN  ui-fields/$sid#$fid: test-points に未写像 (field contract が scenario で検証されていない)"
      echo "        fix: 該当 spec.md#test-points に項目を追加し /ori-flow で再生成 (purpose: $purpose)"
      ((ISSUES++)) || true
    fi
  done < <(grep -oE "\{#${sid}-[A-Za-z0-9-]+\}" "$screen" | sed -E 's/^\{#//; s/\}$//' | sort -u)
done

# shortcut contract (ui-fields/*.md 全体。cross-screen shortcut は index/README にある): Cmd/Ctrl/⌘ + key
mods='(shift|alt|option)[[:space:]]*\+[[:space:]]*'
for doc in "${contracts[@]}"; do
  did=$(basename "$doc" .md)
  while IFS= read -r sc; do
    [[ -n "$sc" ]] || continue
    nsc="$(norm_sc "$sc")"
    if ! grep -qE -- "(^|[^a-z0-9+])$(re_escape "$nsc")([^a-z0-9]|$)" <<<"$corpus_sc"; then
      echo "  WARN  ui-fields/$did: shortcut '$sc' が test-points に未写像 (cross-screen shortcut の未実装・未検証リスク)"
      echo "        fix: 該当 spec.md#test-points に '$sc' の検証項目を追加し /ori-flow で再生成"
      ((ISSUES++)) || true
    fi
  done < <(grep -oiE "((cmd|command|ctrl|control)[[:space:]]*\+|⌘[[:space:]]*\+?)[[:space:]]*(${mods})*[A-Za-z0-9]+" "$doc" | sort -u)
done

echo "  ui-fields traceability: $ISSUES issue(s)"
exit $ISSUES
