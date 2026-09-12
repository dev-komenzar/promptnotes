---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s4-tag-assign-normalize
      hash: 4a02c17cc025
    - path: domain/workflows/assign-tag.md
      hash: 4efe2dfe63c4
    - path: domain/aggregates.md#note-aggregate
      hash: 56f7a54a8ab2
    - path: domain/domain-events.md#note-tags-changed
      hash: 71db66eafe03
---

# s4-tag-assign-normalize — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s4-tag-assign-normalize phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

Note にタグを付与する操作において、以下を検証する：

1. **Tag 構築時の正規化**：ユーザ入力 `"  GPT  "`（前後空白 + 大文字）が `Tag::new` により
   lowercase + trim され、`Tag { name: "gpt" }` として構築される
2. **同一タグの no-op**：正規化後の Tag が Note の TagSet に既に存在する場合（I-N5）、
   `assign_tag` は TagSet を変更せず、永続化も event 発行も行わない
3. **変化時のみ NoteTagsChanged 発行**：assign_tag により TagSet が実際に変化した場合のみ、
   event **NoteTagsChanged** が発行される

> `> domain/validation.md#s4-tag-assign-normalize より:`
> Tag 構築時に正規化: `"gpt"` (lowercase + trim)
> TagSet に同一 `name` が既存（I-N5）→ assign は **no-op**
> event **NoteTagsChanged** は発行しない（変化がないため）

> `> domain/workflows/assign-tag.md より:`
> Step 3 `applyAssign`: `TagDiff = Unchanged | Added(Tag)`（既存ならば Unchanged、S4）
> Step 4 `branchOnDiff`: `Unchanged` → 早期 return（event 非発行）

## シナリオステップ {#scenario-steps}

### Step 1: 既存 Note に正規化後重複するタグを assign {#step-1-assign-duplicate}

**Given**:
- Note A が永続化済み（`id=<note_id>`, `tags=["gpt"]`, `body="hello"`）
- Note A のブロックは IDLE 状態
- `storage_dir/<note_id>.md` の frontmatter: `tags: ["gpt"]`

**When**:
1. ユーザが Note A のタグ編集 UI を開く（タグ入力欄表示）
2. `"  GPT  "`（前後空白 + 大文字）を入力し Enter
3. application service が `AssignTagCommand { note_id: <note_id>, raw_tag: "  GPT  " }` を処理

**Then**:
- `parseTag("  GPT  ")` が正規化: `trim("  GPT  ")` → `"  GPT  "` の trim 後 `"GPT"` → lowercase → `"gpt"`
  → `Tag { name: "gpt" }` を返却（禁止文字チェックも pass）
- `Note::assign_tag(Tag{name:"gpt"}, now)` の結果:
  - TagSet `["gpt"]` に `"gpt"` が既存 → `TagDiff::Unchanged`
  - `updatedAt` は据え置き
- application service は `TagDiff::Unchanged` を検知 → **永続化せず、event 非発行**
- event **NoteTagsChanged** は発行されない
- UI: タグチップ `gpt` の表示は変化なし（no-op 通知は不要）

### Step 2: 新規タグを assign した場合は NoteTagsChanged が発行される {#step-2-assign-new}

**Given**:
- Note A（`tags=["gpt"]`）
- Step 1 完了後

**When**:
1. ユーザが `"coding"` を入力し Enter
2. application service が `AssignTagCommand { note_id: <note_id>, raw_tag: "coding" }` を処理

**Then**:
- `parseTag("coding")` → `Tag { name: "coding" }`（正規化後も変化なし）
- `Note::assign_tag(Tag{name:"coding"}, now)`:
  - TagSet `["gpt", "coding"]` → `TagDiff::Added`
  - `updatedAt = now` に更新
- `.md` ファイル永続化: frontmatter の `tags: ["gpt", "coding"]` が書き出される
- event **NoteTagsChanged** `{ note_id, updated_at: now }` が発行される

## テスト観点 {#test-points}

- **Tag 正規化**: `Tag::new("  GPT  ")` → `Tag { name: "gpt" }`（trim + lowercase、禁止文字なし）
- **Tag 重複 no-op**: TagSet `["gpt"]` に `Tag{name:"gpt"}` を assign → TagSet 変化なし、
  event 非発行、永続化非実行
- **新規 Tag 追加**: TagSet `["gpt"]` に `Tag{name:"coding"}` を assign → TagSet 変化、
  event **NoteTagsChanged** 発行、永続化実行
- **禁止文字 reject**: `Tag::new("foo,bar")` → `TagError::InvalidChar`（S10 の責務、本 scenario
  では正常系の正規化と重複排除に限定）
- **フロントエンドの TagDiff 処理**: application service から `Unchanged` が返った場合、
  UI は再レンダリングせず、ユーザに no-op 通知も出さない（自然な挙動）
- **永続化後のファイル整合性**: `assign_tag` で TagSet が変化した場合、
  `.md` ファイルの frontmatter に正しい `tags:` 配列が書き込まれている

## 実装ノート {#impl-notes}

- **runner**: `wdio`（優先チェーン level 2 で解決 — 参加 app `promptnotes` の
  `runtime.runner: wdio`）
- **参加者**: `promptnotes`（Tauri desktop app、`local` mode、target: `host`）
- **run-mode**: `local` — E2E テストはビルド済み Tauri binary を WDIO 経由で起動する。
  テストコード内で `startApp()` / `stopApp()` 相当のライフサイクル管理が必要
- **前提データ準備**: テスト開始時に、`tags: ["gpt"]` を持つ Note A を
  `storage_dir/<id>.md` として配置する（fixture setup）
- **テスト構造**: WDIO + Tauri driver で、以下の検証を行う:
  - backend 経由で `assign-tag` コマンドを invoke（重複ケース → Unchanged）
  - ファイルシステムで `.md` の frontmatter が変化していないことを確認
  - event bus に NoteTagsChanged が発行されていないことを確認
  - 異なるタグで再度 invoke（新規ケース → Added）、event 発行と frontmatter 更新を確認
- **注意**: Step 1（重複 no-op）が S10（禁止文字）とは別 scenario であることを意識する。
  S10 は `parseTag` の reject ケース、本 scenario は accept 後の TagSet 重複排除と
  event 非発行が主題