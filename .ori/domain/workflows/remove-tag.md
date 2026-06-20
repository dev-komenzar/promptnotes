---
coherence:
  source: human
  last_validated: 2026-06-20
  upstream:
    - aggregates.md#note-aggregate
    - domain-events.md#note-tags-changed
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

- `NoteNotFound { id: NoteId }`
- `PersistError { path: PathBuf, cause: io::Error }`

## Steps {#steps}

1. `loadNote: NoteId → Result<Note, NoteNotFound>`
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
