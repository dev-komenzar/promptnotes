---
coherence:
  source: human
  last_validated: 2026-06-20
  upstream:
    - aggregates.md#note-aggregate
    - domain-events.md#note-body-edited
    - validation.md#s2-autosave-debounce
    - validation.md#s9-idempotent-autosave
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

- `NoteNotFound { id: NoteId }`
- `PersistError { path: PathBuf, cause: io::Error }`

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
