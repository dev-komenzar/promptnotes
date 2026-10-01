#!/usr/bin/env sh
# 前回同期以降に更新された beads issue だけを GitHub に push する (差分 push)。
# pull は含めない (必要時に手動で `bd github sync --pull-only`)。
# 成功時のみ .github-sync-last を開始時刻で更新するので、失敗分は次回再試行される。
set -u
cd "$(dirname "$0")/.." || exit 1
state=.beads/.github-sync-last
started=$(date -u +%Y-%m-%dT%H:%M:%SZ)

[ -n "${GITHUB_TOKEN:-}" ] || GITHUB_TOKEN=$(gh auth token 2>/dev/null)
[ -n "$GITHUB_TOKEN" ] || { echo "github-sync: no GITHUB_TOKEN"; exit 1; }
export GITHUB_TOKEN

if [ ! -f "$state" ]; then
  echo "github-sync: first run, baseline=$started"
  echo "$started" >"$state"
  exit 0
fi
last=$(cat "$state")

ids=$(bd list --all --limit 0 --updated-after "$last" --json | jq -r '.[].id') || exit 1
if [ -z "$ids" ]; then
  echo "github-sync: no changes since $last"
  echo "$started" >"$state"
  exit 0
fi

echo "github-sync: pushing $(echo "$ids" | wc -l) issue(s) updated since $last"
# shellcheck disable=SC2086
if bd github push $ids; then
  echo "$started" >"$state"
else
  echo "github-sync: push failed; baseline kept at $last"
  exit 1
fi
