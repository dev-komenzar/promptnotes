---
name: ori-review-proposals
description: 溜まった proposal を人間と一緒にレビューし、accept/reject の判断を反映する
---

`.ori/proposals/` 配下の pending proposal を順次レビューします。

## スクリプト — 一覧表示

```bash
node .apm/skills/ori-review-proposals/scripts/list.js [--check]
```

- `--check` — pending proposal が 1 件以上あれば非ゼロ終了（CI モード）

## 手順

1. **一覧表示**：
   ```bash
   node .apm/skills/ori-review-proposals/scripts/list.js
   ```
2. **各 proposal について**：
   - 内容を読み上げ（target、by、reason、diff）
   - ユーザに **accept / reject / merge** を問う
3. **accept**：対象ドメイン section を更新 → `/ori-sync` で順伝播 → 該当 slice の dirty 解除
4. **reject**：proposal を `.ori/proposals/rejected/` へ移動 → 派生文書側の変更を破棄（`git checkout` で再 derive 前の状態に戻す）
5. **merge**：複数 proposal を結合する場合、AI が統合案を作成 → ユーザ確認 → ドメイン更新

## 注意

- ユーザの判断なしに勝手に accept しない
- proposal の git history は残す（rejected 含む。失敗した提案も学びになる）

## 次のアクション

レビュー完了後、状況に応じて以下を提示：

- **accept した proposal が ≥ 1 の場合**：`/ori-sync` が dirty slice 群を検出 → `/ori-flow <id>` で順次再走（最も影響が大きい順）
- **reject のみだった場合**：当該 slice の作業は元のまま続行可能。`/ori-flow <slice-id>` で残り phase を続ける
- **merge を行った場合**：統合後の domain 文書を再検証 → 影響 slice を `/ori-flow` で再走
- **pending が残った場合**：休んで OK。次セッションで再度 `/ori-review-proposals` を呼ぶ
- **session 締めパス**：CLAUDE.md の Session Completion（`bd dolt push` / `git push`）を実行して状態保存
