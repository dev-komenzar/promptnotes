---
coherence:
  source: human
  last_validated: 2026-06-20
  upstream:
    - aggregates.md#note-aggregate
    - domain-events.md#note-restored-from-trash
    - validation.md#s5-delete-undo-in-window
    - validation.md#s7-undo-after-toast
---

# restore-deleted-note {#restore-deleted-note}

トースト表示中の Undo ボタン押下で `DeletedNote` を OS ゴミ箱から復帰する。

## Input {#input}

```rust
struct RestoreDeletedNoteCommand {
  // 引数なし。UndoSlot の現在値を使う
}
```

## Output {#output}

- `Note`（復帰した Note の再読み込み結果）
- domain event: [NoteRestoredFromTrash](../domain-events.md#note-restored-from-trash)

## Errors {#errors}

- `NoUndoAvailable` — UndoSlot が空（トースト消失後、S7）
- `TrashRestoreError { path: PathBuf, cause: TrashErrorKind }`
  - ゴミ箱からの取り出し失敗
- `ReadError { path: PathBuf, cause: io::Error }`
  - 復帰後の `.md` 読み込み失敗

## Steps {#steps}

1. `peekUndoSlot: () → Result<DeletedNote, NoUndoAvailable>`
   - UndoSlot が `None` なら `NoUndoAvailable`（S7: UI 層で reject 済みのはずだが二重ガード）
2. `restoreFromTrash: PathBuf → Result<(), TrashRestoreError>`
   - OS ゴミ箱から原 path に復帰
3. `reloadNote: PathBuf → Result<Note, ReadError>`
   - 復帰した `.md` をパースして Note を再構築
4. `clearUndoSlot: () → ()`
   - 復元成功後に UndoSlot を None にリセット
5. `buildEvent: (NoteId, Timestamp) → NoteRestoredFromTrash`
6. `emit: NoteRestoredFromTrash`

## Dependencies {#dependencies}

- `TrashService`
- `NoteRepository`（read 用）
- `UndoSlot`
- `Clock`
- `EventBus`

## Notes {#notes}

- **トースト UI 側の guard**: トースト消失と同時に Undo ボタンを disable し、
  workflow 呼び出しを未然に防ぐ。本 workflow の `NoUndoAvailable` は二重防御
- 復元後の Note は元と同じ `NoteId` / `body` / `tags` / `createdAt`。
  `updatedAt` は OS ゴミ箱の API 仕様に依存（変更しない方針）
- restored note は NoteFeed に再登場する（購読側の責務）
