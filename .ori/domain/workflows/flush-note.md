---
coherence:
  source: human
  last_validated: 2026-06-20
  upstream:
    - aggregates.md#note-aggregate
    - domain-events.md#note-body-edited
    - validation.md#s3-flush-on-blur
    - validation.md#s13-quit-flush
---

# flush-note {#flush-note}

debounce timer を待たず即時永続化を行う。トリガーは Q4 決定の 3 種：
(1) ブロック focus 喪失、(2) ウィンドウ blur、(3) アプリ quit。

## Input {#input}

```rust
struct FlushNoteCommand {
  note_id: NoteId,
  pending_body: String,   // debounce 中だった編集中 body
  trigger: FlushTrigger,  // BlockBlur | WindowBlur | AppQuit
}
```

## Output {#output}

- `Note`（更新後） または `Note`（変化なし時もそのまま返す）
- domain event: [NoteBodyEdited](../domain-events.md#note-body-edited)（body 変化時のみ）

## Errors {#errors}

- `NoteNotFound { id: NoteId }`
- `PersistError { path: PathBuf, cause: io::Error }`

## Steps {#steps}

1. `cancelDebounce: NoteId → ()`
   - DebounceTimer を即時キャンセル（重複 AutoSave を防ぐ）
2. `loadNote: NoteId → Result<Note, NoteNotFound>`
3. `parseBody: String → NoteBody`
4. `compareBody: (Note, NoteBody) → BodyDiff`
5. `branchOnDiff:`
   - `Unchanged` → 早期 return（event 非発行）
   - `Changed(new_body)` → step 6 へ
6. `updateBody: (Note, NoteBody, Timestamp) → Note`
7. `persist: Note → Result<(), PersistError>`
8. `emit: Note → NoteBodyEdited`

## Dependencies {#dependencies}

- `NoteRepository`
- `Clock`
- `EventBus`
- `DebounceTimer`（キャンセル用 handle）

## Notes {#notes}

- **AppQuit trigger** はすべての EDITING Note に対して flush-note を発行する
  （S13: 連続 Flush）。順序は処理順（並列性は持たない）
- 永続化が完了するまで quit を待つ。最大欠損 500ms を許容（Q4 補足）
- 冪等ガード（compareBody）は auto-save-note と同じロジック
