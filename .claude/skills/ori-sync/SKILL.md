---
name: ori-sync
description: ドメイン文書 / slice 文書の変更を検知し、dirty マークを伝播する
---

`/ori-sync` を実行して変更を検知・伝播します。AI agent が文書編集後に呼ぶか、post-write hook が自動起動します。

## スクリプト

```bash
node ./scripts/sync.js [--file=<path>] [--since=<ref>] [--check] [--force]
```

- `--file=<path>` — 単一ファイルに限定して変更検知
- `--since=<ref>` — HEAD の代わりに指定 git ref と比較
- `--check` — dirty マーク残存時に非ゼロ終了（CI モード）
- `--force` — 派生文書の直接編集を許可、proposal を自動生成

## 手順

1. **変更検知**：
   ```bash
   node ./scripts/sync.js
   ```
   - `.ori/domain/` の変更ファイルと、影響を受ける slice を検出
2. 変更された domain section を `derives_from` に持つ slice/page の `status.yaml.dirty[]` に追加
3. dirty な slice ごとに：
   - 該当 phase の beads issue を reopen（手動 or `bd update --status=open`）
   - ユーザに「これらの slice の再 derive が必要です」と通知
4. proposal が生成されていれば（`--force` 経由）`/ori-review-proposals` の起動を案内

## --force 編集時

- 派生文書を直接編集する場合は `/ori-sync --force <path>` を使う
- proposal が `.ori/proposals/<date>-<slice>-<target>.md` として生成される
- proposal 自体の編集は人間に委ねる

## 次のアクション

`/ori-sync` 実行後、状況に応じて以下を提示：

- **dirty slice がある場合**：影響を受けた slice ごとに `/ori-flow <slice-id>` を提案（最も影響が大きい順に）
- **proposal が生成された場合**：`/ori-review-proposals` で人間判断を促す
- **dirty なし / proposal なし**：通常終了。次の作業（新 slice `/ori-flow` or DDD `/ori-distill`）へ
- **整合性エラー検出時**：`/ori-doctor` で詳細診断 → 修復方針をユーザと相談
- **scope 1 slice を締めたい場合**：`/ori-finalize <slice-id>` を呼ぶ（/ori-sync は全体伝播、/ori-finalize は単一 slice 終了）
