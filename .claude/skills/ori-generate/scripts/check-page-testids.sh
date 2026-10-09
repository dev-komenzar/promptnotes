#!/usr/bin/env bash
# page / widget の testid 契約 (.ori/pages/<id>/testids.yaml) の検査と bd 起票 (ori-oan.7 / ori-oan.13)
# 本体は同 bundle の testids.js check。契約 ⊆ 実装・derived の stale・extra 形式・実装 testid lint を検出する。
# scenario-first で後から extra に増えた行 (source: scenario:<id>) の実装追従漏れもここで拾う。
# packages/skills-shared/src から ori-doctor / ori-generate の scripts/ に複製配置される (build-skills.mjs)。
#
# Usage: check-page-testids.sh [--emit-issues] [--implemented-only] [<page-id>...]
#   <page-id>...        指定 page だけを検査 (省略時は全 page = --all。page に寄せられない実装違反は全 page 時のみ)
#   --emit-issues       page ごとに bd issue を起票 (label: testid-violation + page:<id>。open 重複は re-file しない)
#   --implemented-only  実装の無い page を飛ばす (/ori-generate 用。scenario-first の未実装 page は起票しない)
set -euo pipefail

EMIT_ISSUES=false
CHECK_FLAGS=()
PAGES=()
for arg in "$@"; do
  case "$arg" in
    --emit-issues) EMIT_ISSUES=true ;;
    --implemented-only) CHECK_FLAGS+=(--implemented-only) ;;
    --*) echo "ERROR: unknown option: $arg" >&2; exit 2 ;;
    *) PAGES+=("$arg") ;;
  esac
done
[[ ${#PAGES[@]} -eq 0 ]] && PAGES=(--all)

# Auto-detect project root (PWD-first; SCRIPT_DIR fallback last).
# Why: when ori is installed inside a user project, SCRIPT_DIR resolves to the
# ori repo so git toplevel misses the user's .ori/ (ori-fzr.15).
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
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
  PROJECT_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null || true)"
fi
if [ -z "$PROJECT_ROOT" ] || [ ! -d "$PROJECT_ROOT/.ori" ]; then echo "ERROR: cannot find project root (.ori/ not found)" >&2; exit 1; fi
cd "$PROJECT_ROOT"

has_page=false
for m in .ori/pages/*/manifest.yaml; do [[ -f "$m" ]] && has_page=true && break; done
if [[ "$has_page" != true ]]; then
  echo "  page testids: no .ori/pages/*/manifest.yaml"
  exit 0
fi
if ! command -v node >/dev/null 2>&1; then
  echo "  WARN  page testids: node が無いため skip"
  exit 0
fi
# 指定 page id の typo を「page 未 scaffold」の違反として起票しない
if [[ "${PAGES[0]}" != --all ]]; then
  for p in "${PAGES[@]}"; do
    if [[ ! -f ".ori/pages/$p/manifest.yaml" ]]; then
      echo "ERROR: page が見つかりません: .ori/pages/$p/manifest.yaml" >&2
      exit 2
    fi
  done
fi

rc=0
out="$(node "$SCRIPT_DIR/testids.js" check "${PAGES[@]}" ${CHECK_FLAGS[@]+"${CHECK_FLAGS[@]}"} --root "$PROJECT_ROOT" 2>&1)" || rc=$?
violations="$(grep '^VIOLATION ' <<<"$out" || true)"
ISSUES=0
[[ -n "$violations" ]] && ISSUES=$(wc -l <<<"$violations")
# testids.js が検査を完走しなかった (crash / 入力 YAML 破損 / 想定外 exit) 場合は green にしない
if [[ $rc -gt 1 ]] || ! grep -qE '^(OK: testid 契約違反なし|testid 契約違反: [0-9]+ 件)$' <<<"$out"; then
  echo "  ERROR page testids: testids.js check が完走しませんでした (exit $rc)"
  grep -v '^VIOLATION ' <<<"$out" | head -n 5 | sed 's/^/        /'
  ISSUES=$((ISSUES + 1))
fi

while IFS= read -r line; do
  [[ -n "$line" ]] || continue
  echo "  WARN  ${line#VIOLATION }"
done <<<"$violations"
# 検査しなかった page (--implemented-only) と、件数に含めない帰属不能の実装違反 (page 指定時) も見せる
while IFS= read -r line; do
  [[ -n "$line" ]] || continue
  case "$line" in
    skip:*) echo "  SKIP  ${line#skip: }" ;;
    NOTE*) echo "  NOTE  ${line#NOTE }" ;;
  esac
done < <(grep -E '^(skip: |NOTE )' <<<"$out" || true)

if [[ $ISSUES -gt 0 ]]; then
  echo "        fix: /ori-bug <issue-id> で移行手順へ (置換対応表は testids.js migrate-map <page-id>) / derived stale は testids.js sync <page-id>"
fi

if [[ "$EMIT_ISSUES" == true && $ISSUES -gt 0 ]]; then
  if ! command -v bd >/dev/null 2>&1; then
    echo "    WARN: bd not on PATH; cannot auto-file issue" >&2
  else
    # page 単位に集約して起票 (impl lint は page に紐付かないため page:_impl)
    while IFS= read -r key; do
      [[ -n "$key" ]] || continue
      page="$key"; [[ "$key" == impl ]] && page="_impl"
      # 作業中 (in_progress) / 依存で blocked の issue も既存として扱う。表示記号に依存しないよう JSON で id を取る
      existing="$(bd list --label=testid-violation --label="page:${page}" --status=open,in_progress,blocked --json 2>/dev/null \
        | node -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>{try{const a=JSON.parse(s);if(Array.isArray(a)&&a[0]&&a[0].id)process.stdout.write(String(a[0].id))}catch{}})' || true)"
      if [[ -n "$existing" ]]; then
        echo "    INFO: page:${page} の open issue ${existing} あり — re-file しない (idempotent)"
        continue
      fi
      detail="$(awk -v p="VIOLATION ${key}: " 'index($0, p) == 1 { sub(/^VIOLATION /, "- "); print }' <<<"$violations")"
      if [[ "$page" == _impl ]]; then
        howto="page に帰属できない実装 testid の違反。各 page の移行後も残るものを個別に直す (testids.js check --all で確認)"
      else
        howto="実装 testid を契約へ移行する: /ori-bug <この issue の id> で移行手順へ案内される。置換対応表は testids.js migrate-map ${page}。完了条件は testids.js check ${page} が exit 0"
      fi
      if id="$(bd create \
        --title="[testid] ${page}: page testid 契約違反" \
        --description="${detail}

How to fix:
- ${howto}

Reference:
- 契約: .ori/pages/${page}/testids.yaml / 規範: ddd-vsa-hex/pattern.md \"page / widget の testid 契約\"
- Auto-filed by: scripts/check-page-testids.sh --emit-issues (/ori-doctor --testid-sweep または /ori-generate)" \
        --type=bug \
        --priority=2 \
        --labels="testid-violation,page:${page}" \
        --silent)"; then
        echo "    ✓ filed bd issue ${id} (testid-violation, page:${page})"
      else
        echo "    WARN: bd create failed — issue not filed" >&2
      fi
    done < <(sed -nE 's/^VIOLATION ([^:]+):.*/\1/p' <<<"$violations" | sort -u)
  fi
fi

echo "  page testids: $ISSUES issue(s)"
# exit code は件数 (run-checks.sh が合算)。256 で 0 に巻き戻らないよう 255 で頭打ち
exit $(( ISSUES > 255 ? 255 : ISSUES ))
