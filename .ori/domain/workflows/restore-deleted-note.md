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

Toast 表示中の「元に戻す」ボタン押下で **特定の** `DeletedNote` を
OS ゴミ箱から復帰する。Toast は複数並列表示されるため、どの DeletedNote を
復元するかを `NoteId` で指定する。

## Input {#input}

```rust
struct RestoreDeletedNoteCommand {
  note_id: NoteId,    // どの Toast の Undo を実行するか
}
```

## Output {#output}

- `Note`（復帰した Note の再読み込み結果）
- domain event: [NoteRestoredFromTrash](../domain-events.md#note-restored-from-trash)

## Errors {#errors}

- `NoUndoAvailable` — 指定 `NoteId` の `DeletedNote` が Undo スタックに存在しない
  （Toast 消失後、または別 workflow で既に Undo 済み、S7）
- `TrashRestoreError { path: PathBuf, cause: TrashErrorKind }`
  - ゴミ箱からの取り出し失敗
- `ReadError { path: PathBuf, cause: io::Error }`
  - 復帰後の `.md` 読み込み失敗

## Steps {#steps}

1. `findInUndoStack: NoteId → Result<DeletedNote, NoUndoAvailable>`
   - Undo スタックから `NoteId` 一致の要素を検索
   - 見つからなければ `NoUndoAvailable`
     （S7: UI 層で Toast の有効期間外は disable しているはずだが二重ガード）
2. `restoreFromTrash: PathBuf → Result<(), TrashRestoreError>`
   - OS ゴミ箱から原 path に復帰
3. `reloadNote: PathBuf → Result<Note, ReadError>`
   - 復帰した `.md` をパースして Note を再構築
4. `removeFromUndoStack: NoteId → ()`
   - 復元成功後、当該 DeletedNote のみをスタックから除去
     （他の Toast / DeletedNote は影響を受けない）
   - 対応する Toast UI のクローズを通知
5. `buildEvent: (NoteId, Timestamp) → NoteRestoredFromTrash`
6. `emit: NoteRestoredFromTrash`

## Dependencies {#dependencies}

- `TrashService`
- `NoteRepository`（read 用）
- `UndoStack`
- `Clock`
- `EventBus`

## Notes {#notes}

- **Toast UI 側の guard**: 各 Toast は対応する DeletedNote の TTL に同期して
  Undo ボタンを disable する。本 workflow の `NoUndoAvailable` は二重防御
- 復元後の Note は元と同じ `NoteId` / `body` / `tags` / `createdAt`。
  `updatedAt` は OS ゴミ箱の API 仕様に依存（変更しない方針）
- restored note は NoteFeed に再登場する（購読側の責務）
- **per-toast 独立性**: 1 つの Toast の Undo 実行は他の Toast / DeletedNote に
  影響を与えない（S6 改訂の核となる性質）
