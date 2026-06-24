---
ori:
  node_id: workflow:assign-tag
  type: workflow
  depends_on:
    - aggregate:Note
    - event:NoteTagsChanged
    - scenario:s4-tag-assign-normalize
    - scenario:s10-tag-invalid-char
---

# assign-tag {#assign-tag}

Note にタグを付与する。正規化後の重複は no-op、禁止文字は reject。

## Input {#input}

```rust
struct AssignTagCommand {
  note_id: NoteId,
  raw_tag: String,    // ユーザ入力（未正規化）
}
```

## Output {#output}

- `Note`（TagSet 更新後 または変化なし）
- domain event: [NoteTagsChanged](../domain-events.md#note-tags-changed)（TagSet が変化した時のみ）

## Errors {#errors}

- `NoteNotFound { id: NoteId }`
- `InvalidTag { name: String, reason: TagError }`
  - `TagError = EmptyAfterTrim | InvalidChar(char)`
- `PersistError { path: PathBuf, cause: io::Error }`

## Steps {#steps}

1. `parseTag: String → Result<Tag, InvalidTag>`
   - lowercase + trim 適用 → 結果が空文字なら `EmptyAfterTrim`
   - 禁止文字 (` `, `\t`, `\n`, `,`, `[`, `]`) を含めば `InvalidChar`
2. `loadNote: NoteId → Result<Note, NoteNotFound>`
3. `applyAssign: (Note, Tag) → (Note, TagDiff)`
   - `Note::assign_tag(tag)` で TagSet 更新
   - `TagDiff = Unchanged | Added(Tag)`（既存ならば Unchanged、S4）
4. `branchOnDiff:`
   - `Unchanged` → 早期 return（event 非発行）
   - `Added(_)` → step 5 へ
5. `persist: Note → Result<(), PersistError>`
6. `emit: Note → NoteTagsChanged`

## Dependencies {#dependencies}

- `NoteRepository`
- `Clock`（updatedAt 更新のため）
- `EventBus`

## Notes {#notes}

- `parseTag` 単独で reject 可能（Note を load する前にバリデーション）
- 同一タグの再 assign は noop（I-N5 = TagSet の重複排除）
