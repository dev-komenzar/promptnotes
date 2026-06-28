---
target: domain/aggregates.md#note-aggregate-commands
by: slices/assign-tag
reason: Note::assign_tag は updatedAt を更新する義務があるため `now: Timestamp` 引数注入が必要だが、aggregates.md は `Note::assign_tag(self, tag: Tag) -> Note` のままで signature と意味（"updatedAt は更新する"）が impl と乖離する
created: 2026-06-26
status: accepted
accepted_at: 2026-06-28
accepted_by: human (takuya.kometan@gmail.com)
applied_to: domain/aggregates.md#note-aggregate-commands (assign_tag/remove_tag signature に now: Timestamp 注入 + no-op 時 updatedAt 据え置きへの仕様明確化) + domain/workflows/assign-tag.md#steps (applyAssign signature 更新)
---

# Proposal: `Note::assign_tag` signature を `(self, tag: Tag, now: Timestamp) -> Note` に拡張する

## 発見の経緯 {#context}

- 検出元：`slices/assign-tag` の phase 4 (impl-green) + phase 6 (review MED-2)
- 試みていたこと：`Note::assign_tag` で TagSet に新規 Tag を追加し、I-N4 / I-N3 を守るため `updated_at` を更新する
- 想定との差:
  - aggregates.md の `Note::assign_tag(self, tag: Tag) -> Note` signature は `now: Timestamp` を受け取らない
  - aggregates.md 説明文は「`updatedAt` は **更新する**（tags も frontmatter 経由で永続化されるため）」と書くが、time injection 手段が signature 上に無いため pure aggregate としては実現不能
  - 同様の問題は既に解決済の `Note::edit_body(self, new_body: NoteBody, now: Timestamp) -> Note` で対処済。assign_tag / remove_tag だけが旧 signature のまま残っている

## 現状仕様 {#current}

> domain/aggregates.md#note-aggregate-commands より：

```
- `Note::assign_tag(self, tag: Tag) -> Note`
  - TagSet に追加（既存なら no-op、I-N5）。`updatedAt` は **更新する**
    （tags も frontmatter 経由で永続化されるため）
- `Note::remove_tag(self, tag_name: &str) -> Note`
  - TagSet から削除。`updatedAt` を更新
```

## 矛盾／欠落 {#gap}

派生側 (`slices/assign-tag/spec.md#oq-assign-tag-now-injection`, `apps/promptnotes/src-tauri/src/note_capture/shared/types/note.rs`) が必要とする条件:

1. **`updated_at` を更新するために `now: Timestamp` の注入手段が必要**：aggregate は Clock 依存を持つべきでない（pure）。よって signature 経由で受け取るのが王道。Phase 4 impl-green では `Note::assign_tag(self, tag: Tag, now: Timestamp) -> Note` で実装している（auto-save-note の `edit_body` と同形）
2. **`remove_tag` も同じ問題を抱える**：将来の `remove-tag` slice で同じ proposal を再度上げないよう、本 proposal で両方の signature を揃える

## 提案する変更 {#proposal}

### Note Aggregate Commands の signature 更新

```
- `Note::assign_tag(self, tag: Tag, now: Timestamp) -> Note`
  - TagSet に追加（既存なら no-op、I-N5）。新規追加時は `updatedAt = now`（同一 `name` の no-op 経路では updatedAt も据え置き — 永続化の必要が無いため）
- `Note::remove_tag(self, tag_name: &str, now: Timestamp) -> Note`
  - TagSet から削除。削除があった場合は `updatedAt = now`
```

### 補足 (no-op 時の updatedAt 扱い)

現行 aggregates.md 文言（「既存なら no-op、I-N5。`updatedAt` は **更新する**」）は no-op 時にも updatedAt を更新するように読めるが、永続化に書き戻す情報が無いため updatedAt を bump する実用上の意味が無い。本 proposal では「変更があった時のみ updatedAt を bump」に統一する（impl と整合）。

## 影響範囲 {#impact}

- domain/aggregates.md#note-aggregate-commands
- domain/workflows/assign-tag.md#steps の文言（`Note::assign_tag(tag)` → `Note::assign_tag(tag, now)`）
- 将来 slice: `remove-tag`（signature 一致化）
- 既存 impl: `apps/promptnotes/src-tauri/src/note_capture/shared/types/note.rs` (assign_tag は既に新 signature で実装済、remove_tag は未実装)

## トレース {#trace}

- 派生 spec: `.ori/slices/assign-tag/spec.md#oq-assign-tag-now-injection`
- review.md: `.ori/slices/assign-tag/review.md` MED-1 / MED-2
- 既存 impl: `apps/promptnotes/src-tauri/src/note_capture/shared/types/note.rs:57-72`
- 同形先例: `Note::edit_body(self, new_body: NoteBody, now: Timestamp) -> Note` (auto-save-note proposal 経由で domain 反映済)
