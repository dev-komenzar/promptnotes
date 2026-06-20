---
coherence:
  source: human
  last_validated: 2026-06-20
  upstream:
    - aggregates.md#note-aggregate
---

# copy-note-body {#copy-note-body}

ホバー時のコピーボタン押下で、Note の本文のみ（frontmatter / タグを除外）を
OS クリップボードへ書き出す。spec の **core 動作**。

## Input {#input}

```rust
struct CopyNoteBodyCommand {
  note_id: NoteId,
}
```

## Output {#output}

- `()` — 副作用としてクリップボードに本文が入る
- domain event: **なし**（domain state を変えないため）

## Errors {#errors}

- `NoteNotFound { id: NoteId }`
- `ClipboardError { cause: ClipboardErrorKind }` — OS クリップボード API 失敗

## Steps {#steps}

1. `loadNote: NoteId → Result<Note, NoteNotFound>`
2. `extractBody: Note → String`
   - `Note::body_for_clipboard()` を呼ぶ（frontmatter / タグ情報を除外）
3. `writeToClipboard: String → Result<(), ClipboardError>`

## Dependencies {#dependencies}

- `NoteRepository`（read のみ）
- `ClipboardService`

## Notes {#notes}

- domain event を発行しない理由: domain state（aggregate の中身）が変わらないため
- UI 層は成功時に「コピーしました」トーストを表示してよい（UI レベルの副作用）
- spec の core: 「本文のみ」をコピーすることがプロダクトの差別化点
