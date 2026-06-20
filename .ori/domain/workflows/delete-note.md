---
coherence:
  source: human
  last_validated: 2026-06-20
  upstream:
    - aggregates.md#note-aggregate
    - domain-events.md#note-deleted-to-trash
    - validation.md#s5-delete-undo-in-window
    - validation.md#s6-delete-replace
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
4. `replaceDeletedSlot: DeletedNote → ()`
   - application service の保持 slot を **上書き**（S6: 直前の DeletedNote は破棄）
   - 同時にトーストタイマーをリセット
5. `buildDeletedNote: (NoteId, PathBuf, Timestamp) → DeletedNote`
6. `emit: DeletedNote → NoteDeletedToTrash`

## Dependencies {#dependencies}

- `NoteRepository`
- `TrashService` — OS ゴミ箱 API の薄いラッパー
- `UndoSlot` — `Option<DeletedNote>` を保持する application service
- `Clock`
- `EventBus`

## Notes {#notes}

- **連続削除時の挙動**（S6）: 古い `DeletedNote` は復元不能（OS ゴミ箱からの手動復元のみ）
- トースト UI のタイマー管理は UI 層の責務だが、`UndoSlot` の TTL と同期する必要あり
  → Phase 11a (ui-fields) で具体化
- ファイル名は `id` から決定論的に導出可能なので Note 全体を保持しない設計
