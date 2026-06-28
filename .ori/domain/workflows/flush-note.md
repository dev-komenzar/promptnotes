---
ori:
  node_id: workflow:flush-note
  type: workflow
  depends_on:
    - aggregate:Note
    - event:NoteBodyEdited
    - scenario:s3-flush-on-blur
    - scenario:s13-quit-flush
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

- `NoteNotFound { id: NoteId }` — load_by_id が `Ok(None)` を返した場合
- `InvalidBody { source: NoteBodyError }` — `NoteBody::new(pending_body)` が失敗した場合（frontmatter delimiter line `---` を含む等、I-N8 違反）
- `LoadError { path: PathBuf, source: io::Error }` — load_by_id の read I/O 失敗 / 既存 `.md` ファイルの parse 失敗
- `PersistError { path: PathBuf, source: io::Error }` — `NoteRepository::write` の I/O 失敗 (write 経路専用)

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
- `NoteBody` 不変条件（frontmatter delimiter `---` を含まない、I-N8）は aggregate 由来。Flush は pending_body を受け取り構築するため、aggregate と同じ smart constructor を通る
- read 失敗 (`LoadError`) と write 失敗 (`PersistError`) は意味的に異なる経路として error variant を分離する（auto-save-note workflow と同形）
- 本 errors 形は `auto-save-note` workflow と shared な「Note::edit_body 経由の永続化契約」を反映している。両 workflow を改訂する際は同時に同形を保つこと
