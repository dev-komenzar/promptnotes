---
ori:
  node_id: workflow:auto-save-note
  type: workflow
  depends_on:
    - aggregate:Note
    - event:NoteBodyEdited
    - scenario:s2-autosave-debounce
    - scenario:s9-idempotent-autosave
---

# auto-save-note {#auto-save-note}

EDITING ブロックでのキー入力後 500ms debounce が成立した時に、Note 本文を永続化する。

## Input {#input}

```rust
struct AutoSaveNoteCommand {
  note_id: NoteId,
  new_body: String,        // 編集中の現在 body
}
```

## Output {#output}

- `Note`（更新後）
- domain event: [NoteBodyEdited](../domain-events.md#note-body-edited)

または **何もしない**（冪等性により event 非発行、S9 検証済み）。

## Errors {#errors}

- `NoteNotFound { id: NoteId }` — `NoteRepository::load_by_id` が `Ok(None)` を返した場合
- `InvalidBody { source: NoteBodyError }` — `NoteBody::new(new_body)` が失敗した場合
  （aggregates.md#note-aggregate-invariants の I-N8 違反、典型的には `---` 行を含む body）
- `LoadError { path: PathBuf, source: io::Error }` — `load_by_id` の read I/O 失敗、
  または既存 `.md` ファイルの parse 失敗 (`io::ErrorKind::InvalidData`)
- `PersistError { path: PathBuf, source: io::Error }` — `NoteRepository::write` の I/O 失敗
  （write 経路専用に意味を絞る）

## Steps {#steps}

1. `loadNote: NoteId → Result<Note, NoteNotFound>`
2. `parseBody: String → NoteBody`
3. `compareBody: (Note, NoteBody) → BodyDiff`
   - `BodyDiff = Unchanged | Changed(NoteBody)`
   - **S9 の application service レベル冪等性ガード**
4. `branchOnDiff:`
   - `Unchanged` → 早期 return（event 非発行）
   - `Changed(new_body)` → step 5 へ
5. `updateBody: (Note, NoteBody, Timestamp) → Note`
   - `Note::edit_body(new_body, now)`
6. `persist: Note → Result<(), PersistError>`
   - `NoteRepository::write(&note)`
7. `emit: Note → NoteBodyEdited`

## Dependencies {#dependencies}

- `NoteRepository`
- `Clock`
- `EventBus`
- `DebounceTimer` (application 層、UI からのキー入力を 500ms 集約する責務)

## Notes {#notes}

- 同一秒内の連続編集は `updated_at` が変わらない（I-N4 補足、S15）。
  ただし永続化と event 発行は実行される（body は変わっているため）
- `DebounceTimer` の cancel は flush-note workflow（focus 喪失等）が責務
- `NoteBody` 不変条件（I-N8: frontmatter delimiter `---` を含まない）は aggregate 由来。
  AutoSave は新規 body を受け取り `NoteBody::new` で構築するため、aggregate と同じ
  smart constructor を通る。違反時は `InvalidBody` で表面化
- read 失敗（`LoadError`）と write 失敗（`PersistError`）は意味的に異なる経路として
  error variant を分離する。前者は UI 側「ノートが壊れている」フィードバック、
  後者は「保存できない」フィードバックを別に提示できる
- 同じ error 分類は `flush-note` workflow にも同形で適用すべき（accept 時の確認事項として
  記録、`flush-note` 派生時に再確認）
