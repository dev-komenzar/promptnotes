---
ori:
  node_id: workflow:delete-note
  type: workflow
  depends_on:
    - aggregate:Note
    - event:NoteDeletedToTrash
    - scenario:s5-delete-undo-in-window
    - scenario:s6-delete-replace
---

# delete-note {#delete-note}

Note を OS のゴミ箱へ移動し、Undo 用 `DeletedNote` を application service に保持する。

## Input {#input}

```rust
struct DeleteNoteCommand {
  note_id: NoteId,
}
```

## Output {#output}

- `DeletedNote { id: NoteId, original_path: PathBuf }`
- domain event: [NoteDeletedToTrash](../domain-events.md#note-deleted-to-trash)

## Errors {#errors}

- `NoteNotFound { id: NoteId }`
- `TrashError { path: PathBuf, cause: TrashErrorKind }`
  - OS のゴミ箱 API が失敗した場合（権限・ファイルシステム制約）

## Steps {#steps}

1. `loadNote: NoteId → Result<Note, NoteNotFound>`
2. `resolvePath: Note → PathBuf`
   - `storage_dir / <note_id>.md`
3. `moveToTrash: PathBuf → Result<(), TrashError>`
   - OS 依存（macOS: `NSWorkspace`, Linux: XDG trash, Windows: SHFileOperation）
4. `buildDeletedNote: (NoteId, PathBuf, Timestamp) → DeletedNote`
5. `pushUndoStack: DeletedNote → ()`
   - application service の Undo スタック (`Vec<DeletedNote>`) に **push**
     （Phase 11a UI 設計改訂による Q5 改定: 既存の DeletedNote は破棄しない）
   - 新規 Toast を画面下部の縦パイル最上部に表示するよう UI 層に通知
   - 当該 DeletedNote 固有の TTL タイマー (仮 5 秒) を起動
6. `emit: DeletedNote → NoteDeletedToTrash`

## Dependencies {#dependencies}

- `NoteRepository`
- `TrashService` — OS ゴミ箱 API の薄いラッパー
- `UndoStack` — `Vec<DeletedNote>` を保持する application service
  （TTL 管理付き、各要素ごとに個別タイマー）
- `Clock`
- `EventBus`

## Notes {#notes}

- **連続削除時の挙動**（S6 改訂）: 各 `DeletedNote` は独立に保持される。
  対応する Toast が時間切れ / 明示クローズ / Undo 成功 のいずれかで該当要素が
  スタックから除去されるが、他の DeletedNote は影響を受けない
- Toast UI のタイマー管理は UI 層の責務だが、`UndoStack` の per-element TTL と
  同期する必要あり → Phase 11a (ui-fields) で確定済み（仮 5 秒、各 Toast 個別）
- ファイル名は `id` から決定論的に導出可能なので Note 全体を保持しない設計
