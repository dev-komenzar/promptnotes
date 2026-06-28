---
target: domain/workflows/assign-tag.md#errors
by: slices/assign-tag
reason: workflow#errors の TagError variant 名 `EmptyAfterTrim | InvalidChar(char)` と impl 既存の `Empty | InvalidChar { raw: String }` でずれがある。Tag は既に複数 BC（Note Capture / Note Feed）で使われるため domain 側を impl に合わせる方が影響範囲が小さい
created: 2026-06-26
status: accepted
accepted_at: 2026-06-28
accepted_by: human (takuya.kometan@gmail.com)
applied_to: domain/workflows/assign-tag.md#errors (TagError = Empty | InvalidChar { raw: String } に統一) + #steps (parseTag step 文言更新) + domain/aggregates.md#note-aggregate-elements (Tag VO 説明を Empty / InvalidChar { raw } 二分岐に整合)
---

# Proposal: `TagError` variant 名を `Empty | InvalidChar { raw }` に統一する

## 発見の経緯 {#context}

- 検出元：`slices/assign-tag` の phase 4 (impl-green) + phase 6 (review MED-1)
- 試みていたこと：`AssignTagError::InvalidTag { name, reason: TagError }` の `reason` payload に `Tag::new` のエラーを載せる
- 想定との差:
  - workflow#errors の表記 `TagError = EmptyAfterTrim | InvalidChar(char)` と既存 impl の `TagError { Empty, InvalidChar { raw: String } }` で variant 名 + payload 形が両方ずれている
  - 既存 impl は `apps/promptnotes/src-tauri/src/note_capture/shared/types/tag.rs:9-14` で **Note Feed 等の他 BC からも参照される予定** の共有 VO。後から名前変更すると影響範囲が広がる

## 現状仕様 {#current}

> domain/workflows/assign-tag.md#errors より：

```
- `InvalidTag { name: String, reason: TagError }`
  - `TagError = EmptyAfterTrim | InvalidChar(char)`
```

> impl (apps/promptnotes/src-tauri/src/note_capture/shared/types/tag.rs:8-14) より：

```rust
pub enum TagError {
    #[error("tag '{raw}' contains invalid character")]
    InvalidChar { raw: String },
    #[error("tag must not be empty after normalization")]
    Empty,
}
```

## 矛盾／欠落 {#gap}

1. **variant 名**: `EmptyAfterTrim` (workflow) vs `Empty` (impl)。impl 側の方が短く、`Tag::new` の正規化 (trim → forbidden-char → lowercase) の文脈が doc-comment で十分伝わる
2. **InvalidChar の payload 形**: `InvalidChar(char)` (workflow) vs `InvalidChar { raw: String }` (impl)。impl は失敗時に **入力文字列全体** を持つ方を選択（UI でエラー表示する際に有用）。`char` 単独だとどの文字でエラーになったかは分かるが、ユーザに「`foo,bar` は使えません」と表示するための raw 文字列復元コストが上がる
3. **共有 VO であること**: `Tag` は `Note` (Note Capture) と `NoteFeed.filter` (Note Feed) の両方で使われる。impl 既存名のまま残すと将来 slice (`remove-tag`, `update-feed-filter` 等) も同じ variant 名を使う。今 doc 側を impl に合わせれば、後の slice からは proposal 不要

## 提案する変更 {#proposal}

### workflow#errors の `TagError` を impl 既存形に統一

```
- `InvalidTag { name: String, reason: TagError }`
  - `TagError = Empty | InvalidChar { raw: String }`
  - `Empty`: trim 後に空文字（正規化結果が空）
  - `InvalidChar { raw }`: 入力 raw 文字列が禁止文字 (` `, `\t`, `\n`, `,`, `[`, `]`) を含む
```

### aggregates.md#note-aggregate-elements の Tag 説明も整合させる

> 現状: 「construction 時に禁止文字を含む入力は `TagError::InvalidChar` で reject」

> 変更後: 「construction 時に **trim 後の空文字** は `TagError::Empty` で、**禁止文字を含む入力** は `TagError::InvalidChar { raw }` で reject」

## 影響範囲 {#impact}

- domain/workflows/assign-tag.md#errors
- domain/aggregates.md#note-aggregate-elements (Tag 説明)
- domain/workflows/remove-tag.md (将来 slice — 同じ variant 名で書き始められる)
- 既存 impl: `apps/promptnotes/src-tauri/src/note_capture/shared/types/tag.rs` (変更なし — doc を impl に合わせる方向)
- 既存 spec: `.ori/slices/assign-tag/spec.md#oq-tag-new-signature` (resolved 状態に遷移)

## トレース {#trace}

- 派生 spec: `.ori/slices/assign-tag/spec.md#oq-tag-new-signature`
- review.md: `.ori/slices/assign-tag/review.md` MED-1
- 既存 impl: `apps/promptnotes/src-tauri/src/note_capture/shared/types/tag.rs:8-14`
- impl 用例: `apps/promptnotes/src-tauri/src/note_capture/slices/assign_tag/tests.rs` (TP-IC1 / TP-IC4 が `Empty | InvalidChar` 前提)
