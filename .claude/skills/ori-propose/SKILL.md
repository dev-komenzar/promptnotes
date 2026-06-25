---
name: ori-propose
description: 派生文書側の発見をドメイン上流への提案として明示的に作成する
---

slice 実装中にドメイン仕様の矛盾や欠落を発見した場合、人間の判断を経るための proposal markdown を `.ori/proposals/` 配下に作成します。**ドメイン文書は勝手に書き換えません**（accept は `/ori-review-proposals` の責務）。

## 役割

- **発見の記録係**：派生側（slice / spec）で気付いた上流（domain）の問題を proposal ファイル化
- **痕跡係**：当該 slice の `notes.md` に proposal 作成を記録（後で `/ori-review-proposals` から逆引き可能）
- **案内係**：人間レビューが必要なことをユーザに通知

## 入力 / 出力

- 入力（ユーザ or 上流スキルから渡される）：
  - `target`：影響を受けるドメイン section（例：`domain/aggregates.md#note-aggregate`）
  - `by`：提案元の slice（例：`slices/capture-auto-save`）
  - `reason`：1〜2 行の発見サマリ（なぜこの提案を作るのか）
  - `detail`（任意）：現状仕様と発見の差分、提案する変更案、影響範囲など長文
- 出力：
  - `.ori/proposals/<YYYY-MM-DD>-<slice-id>-<target-slug>.md`
  - `.ori/slices/<slice-id>/notes.md` に追記（proposal 作成済みマーク）

## ファイル名規約

`.ori/proposals/<YYYY-MM-DD>-<slice-id>-<target-slug>.md`

- `<YYYY-MM-DD>`：作成日（UTC）
- `<slice-id>`：`by: slices/<id>` から抽出した id
- `<target-slug>`：`target` から以下の変換で生成
  - 先頭の `domain/` を除去
  - 拡張子 `.md` を除去
  - `/` と `#` を `-` に置換
  - 例：`domain/aggregates.md#note-aggregate` → `aggregates-note-aggregate`
  - 例：`domain/workflows/capture-auto-save.md` → `workflows-capture-auto-save`

衝突時（同日に同一 slice × 同一 target で複数 propose）は末尾に `-2`, `-3`, ... を付与。

## 出力テンプレート

```markdown
---
target: domain/aggregates.md#note-aggregate
by: slices/capture-auto-save
reason: <1〜2 行の発見サマリ>
created: 2026-06-25
status: pending
---

# Proposal: <target を 1 行で要約>

## 発見の経緯 {#context}

- 検出元：`slices/capture-auto-save` の <phase>（例：impl-green / review）
- 何を試みていたか
- 何が想定と違ったか

## 現状仕様 {#current}

> domain/aggregates.md#note-aggregate より：
>
> <該当箇所の引用 — 短く>

## 矛盾／欠落 {#gap}

- 派生側（spec.md or impl）で必要としている条件
- 現状ドメインで満たせない理由

## 提案する変更 {#proposal}

- 追加 / 修正したい不変条件・規則
- 影響範囲（推定で OK、他の slice に波及するかなど）

## 代替案 {#alternatives}

（任意）拒否される場合に派生側で吸収可能な代替策。なければ「なし」と明記。
```

## 手順

1. **発見の整理**（ユーザ or 呼び出し元から取得）：
   - `target` / `by` / `reason` を確定
   - 必要に応じて `detail` をヒアリングして補強
2. **既存 proposal の確認**：
   ```bash
   ls .ori/proposals/ 2>/dev/null | grep "<slice-id>" | grep "<target-slug>"
   ```
   - 同 slice × 同 target の pending が既にあれば、上書きせず追記検討（ユーザに「既存に追記 / 別案として新規 / 中止」を確認）
3. **ファイル名の決定**：上記「ファイル名規約」に従って `<YYYY-MM-DD>-<slice-id>-<target-slug>.md` を組み立てる
4. **proposal ファイルの作成**：Write ツールで `.ori/proposals/<filename>.md` を作成
   - 上記テンプレートに沿って frontmatter と本文を埋める
   - `detail` が渡されていなければ最低限 `context` / `current` / `gap` / `proposal` の 4 セクションを埋める（`alternatives` は省略可）
   - 「現状仕様」セクションは推測で書かず、必ず `target` を Read して引用する
5. **slice notes.md への痕跡**：`.ori/slices/<slice-id>/notes.md` に以下を追記（ファイルが無ければ作成）：
   ```markdown
   ## <YYYY-MM-DD> proposal 作成

   - target: domain/aggregates.md#note-aggregate
   - file: .ori/proposals/<filename>.md
   - reason: <reason>
   ```
6. **beads への記録**（任意、緊急度が高い場合）：
   ```bash
   bd create --title="proposal: <target> from <slice-id>" --type=task --priority=2 \
     --description="$(cat .ori/proposals/<filename>.md)" \
     --labels=proposal
   ```
7. **ユーザ通知**：作成パスと次手（`/ori-review-proposals`）を提示

## 注意

- **proposal はあくまで提案**：このスキルから `.ori/domain/` を書き換えない。accept 判断は `/ori-review-proposals` で人間が行う
- **複数 proposal の併存は許容**：同じ section に対し別 slice 由来の proposal が並列していても OK（`/ori-review-proposals` 側で merge 判定）
- **推測でドメインを引用しない**：`current` セクションは必ず `target` ファイルを Read してから引用する。引用元と一致しない記述は人間の判断を誤らせる
- **slice の作業は止めない**：proposal を出しても当該 slice の他 phase（test-red / impl-green 等）は継続可能。最終的な domain 反映後に再 derive が必要なら finalize が dirty を残す

## 次のアクション

proposal 作成後、ユーザに以下を提示：

- **メインパス**：`/ori-review-proposals` — 人間と共に accept/reject/merge を判断
- **作業継続パス**：proposal は upstream 反映待ち。当該 slice の他 phase が進行可能なら継続。最終的に domain 反映後の再 derive が必要
- **複数 proposal が溜まったらまとめてレビューパス**：slice を複数同時進行している場合、ある程度溜まってから `/ori-review-proposals` でバッチ処理が効率的
- **緊急度が高い場合の通知パス**：手順 6 の `bd create --labels=proposal` で issue 化し、`bd human` で人間判断を促す
