---
name: ori-finalize
description: /ori-flow phase 7。当該 slice の dirty 解除・proposal の浮上・beads 後始末を行う。/ori-sync が全体伝播するのに対し、こちらは 1 slice を締める
---

ユーザが `/ori-finalize <slice-id>` を呼ぶ、または `/ori-flow` 内部から phase 7 として起動した際に、**当該 slice の状態を整理して "1 slice を完了させる"**。`/ori-sync` が全体に変更を fan-out するのに対し、`/ori-finalize` は **1 slice の内向きの締め**。

## 引数

- `slice-id`：対象 slice の id

## 役割

- **clean-up 担当**：当該 slice の `status.yaml.dirty[]` をクリア
- **proposal 浮上係**：phase 中に `--force` で生成された proposal を一覧化しユーザに通知
- **beads クローザー**：当該 slice の phase issue を全て close。epic の進捗を更新
- **次手案内**：次の slice 候補 or proposal review を提示

## /ori-sync との違い

| | /ori-sync | /ori-finalize |
|--|----------|---------------|
| スコープ | 全体（domain → 全 slice） | 1 slice 内 |
| 方向 | fan-out（伝播） | fan-in（締め） |
| トリガー | post-write hook / 手動 | phase 7 / 手動 |
| 主作用 | dirty マーク追加 | dirty マーク解除、issue close |

両者は補完関係。**`/ori-sync` で他の slice が dirty になっていてもこのスキルは関与しない**（次の `/ori-flow` で対応）。

## 入力 / 出力

- 入力：
  - `.ori/slices/<id>/status.yaml`
  - `.ori/slices/<id>/review.md`（phase 6 結果）
  - `.ori/proposals/` 配下で当該 slice 由来のもの（`by: slices/<id>`）
  - beads phase issues：`ori-{derive,plan,test-red,impl-green,refactor,review,finalize}-<id>`
- 出力：
  - `.ori/slices/<id>/status.yaml.dirty[]` がクリアされる
  - 当該 slice の beads phase issue 全 close
  - epic issue の進捗更新（CLI が自動）
  - proposal が存在すれば一覧表示

## 手順

1. **前提確認**：
   - phase 6（review）が close されているか（`bd show ori-review-<id>`）
   - `pnpm -F <slice-pkg> test` GREEN を最終確認
2. **slice の締め処理**：
   ```bash
   bash ./scripts/clear-dirty.sh <slice-id>
   bash ./scripts/update-hash.sh <slice-id>
   ```
   - `status.yaml.dirty[]` を空に
   - `spec.md` の `hash:` を最新に更新
   - 残り phase issue の close（既に close 済みのものは no-op）
3. **proposal の浮上**：
   ```bash
   ls .ori/proposals/ 2>/dev/null
   ```
   - 結果が空 → 通常終了
   - 1 件以上ある → ユーザに通知：
     ```
     この slice 由来で生成された proposal が <N> 件あります：

       - 2026-05-14-capture-auto-save-aggregates-note-aggregate.md
       - 2026-05-14-capture-auto-save-types-throttle-config.md

     /ori-review-proposals で確認してください。
     ```
4. **review.md 指摘の clean-up 確認**：
   - `review.md` の `Findings` が全て disposition 済みか確認
   - 未対応があれば停止し phase 6 へ差し戻し
5. **次手の提示**：
    - 他に dirty な slice が残っているなら `.ori/slices/*/status.yaml` の `dirty` 欄で一覧
    - 次の `/ori-flow <next-id>` 候補を提示
6. **完了**：
   ```bash
   bd close ori-finalize-<slice-id> --reason="slice complete; status cleared; <N> proposals surfaced"
   ```

## 注意

- **scope は 1 slice**：他の slice の dirty には触らない
- **proposal を勝手に accept しない**：人間判断のため `/ori-review-proposals` を案内
- **review 指摘の未対応で finalize しない**：sloppy finalize はバグの温床
- **スキルが決定的処理を担当**：このスキルはオーケストレーションと通知

## 次のアクション

phase 7 完了後、`/ori-flow` 全体が完了。ユーザに以下を提示：

- **次 slice パス**：`/ori-flow <next-slice-id>` — 次の dirty slice や未着手 slice
- **proposal review パス**：`/ori-review-proposals` — 浮上した proposal を人間と共にレビュー
- **idle パス**：dirty 残ゼロ・proposal ゼロなら一旦休む。`/ori-feature-status` で全体俯瞰
- **session 締めパス**：CLAUDE.md の Session Completion 手順（`bd dolt push` / `git push` 等）を実行
