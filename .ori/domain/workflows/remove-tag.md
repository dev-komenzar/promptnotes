---
ori:
  node_id: workflow:remove-tag
  type: workflow
  depends_on:
    - aggregate:Note
    - event:NoteTagsChanged
---

# remove-tag {#remove-tag}

Note から指定されたタグを削除する。存在しないタグの削除は no-op。

## Input {#input}

```rust
struct RemoveTagCommand {
  note_id: NoteId,
  tag_name: String,    // 正規化済みの想定（UI 側でタグチップから取得）
}
```

## Output {#output}

- `Note`（TagSet 更新後 または変化なし）
- domain event: [NoteTagsChanged](../domain-events.md#note-tags-changed)（変化時のみ）

## Errors {#errors}

- `NoteNotFound { id: NoteId }` — load_by_id が `Ok(None)` を返した場合
- `LoadError { path: PathBuf, source: io::Error }` — load_by_id の read I/O 失敗 / 既存 `.md` ファイルの parse 失敗
- `PersistError { path: PathBuf, source: io::Error }` — `NoteRepository::write` の I/O 失敗 (write 経路専用)

## Steps {#steps}

1. `loadNote: NoteId → Result<Note, NoteNotFound | LoadError>`
2. `applyRemove: (Note, &str) → (Note, TagDiff)`
   - `Note::remove_tag(tag_name)` で TagSet 更新
   - `TagDiff = Unchanged | Removed(Tag)`
3. `branchOnDiff:`
   - `Unchanged` → 早期 return（event 非発行）
   - `Removed(_)` → step 4 へ
4. `persist: Note → Result<(), PersistError>`
5. `emit: Note → NoteTagsChanged`

## Dependencies {#dependencies}

- `NoteRepository`
- `Clock`
- `EventBus`

## Notes {#notes}

- `tag_name` は UI（タグチップの × ボタン）から既に正規化済みで来る前提
- 不正な tag_name が来ても「存在しないので no-op」となり安全
- read 失敗 (`LoadError`) と write 失敗 (`PersistError`) は意味的に異なる経路として error variant を分離する (auto-save-note / assign-tag workflow と同形)
- 本 errors 形は Note Capture BC の write-side 3 slice (auto-save-note / assign-tag / remove-tag) で共有される「read/write I/O 意味分離」契約を反映している
