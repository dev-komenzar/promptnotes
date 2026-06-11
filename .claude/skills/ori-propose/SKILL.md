---
name: ori-propose
description: 派生文書側の発見をドメイン上流への提案として明示的に作成する
---

slice 実装中にドメイン仕様の矛盾を発見した場合、人間の判断を経るための proposal を作成します。

## 手順

1. **発見の整理**：
   - 何が問題か（現状仕様と発見の差分）
   - どの section に影響するか（`path#section-id`）
   - 提案する変更内容
2. **Bash で proposal 作成**:
   ```
   ori propose \
     --target domain/aggregates.md#note-aggregate \
     --by slices/<slice-id> \
     --reason "..."
   ```
   CLI が `.ori/proposals/<date>-<slice>-<target>.md` を生成
3. **notes.md に痕跡を残す**：slice の `notes.md` に「proposal 作成済み」を記録
4. **ユーザに通知**：`/ori-review-proposals` でレビュー可能と伝える

## 注意

- proposal はあくまで提案。**ドメイン文書を勝手に書き換えない**
- 同じ section への複数 proposal は許容（並列して別 slice 由来でも問題なし）

## 次のアクション

proposal 作成後、ユーザに以下を提示：

- **メインパス**：`/ori-review-proposals` — 人間と共に accept/reject/merge を判断
- **作業継続パス**：proposal は upstream 反映待ちなので、当該 slice の他 phase（test-red / impl-green 等）が進行可能なら継続。最終的に domain 反映後の再 derive が必要
- **複数 proposal が溜まったらまとめてレビューパス**：slice を複数同時進行している場合、ある程度溜まってから `/ori-review-proposals` でバッチ処理が効率的
- **緊急度が高い場合の通知パス**：`bd human ori-propose-<slice>` で flag を立て、人間に即時判断を促す
