---
ori:
  node_id: workflow:create-note
  type: workflow
  depends_on:
    - aggregate:Note
    - event:NoteCreated
    - scenario:s1-note-created-happy
---

# create-note {#create-note}

Draft 入力欄に入力された本文を Cmd+Enter で確定し、新規 Note として永続化する。

## Input {#input}

```rust
struct CreateNoteCommand {
  raw_body: String,        // Draft 入力欄の生テキスト
  raw_tags: Vec<String>,   // optional: 初期タグ（spec では Draft からタグ付与しないが将来拡張）
}
```

## Output {#output}

- `Note`（生成された Note）
- domain event: [NoteCreated](../domain-events.md#note-created)

## Errors {#errors}

- `InvalidTag { name: String, reason: TagError }` — タグの正規化 / 禁止文字違反
- `PersistError { path: PathBuf, cause: io::Error }` — `.md` 書き出し失敗

## Steps {#steps}

1. `parseBody: String → NoteBody`
   - frontmatter 記号 `---` を含まない検証（spec の data 仕様準拠）
2. `parseTags: Vec<String> → Result<TagSet, InvalidTag>`
   - 各 raw_tag に `Tag::new` を適用し正規化・reject
3. `assignId: Clock → NoteId`
   - 現在時刻を `YYYYMMDDhhmmss` に format
4. `build: (NoteId, NoteBody, TagSet, Timestamp) → Note`
   - `Note::create(body, tags, now)`
5. `persist: Note → Result<(), PersistError>`
   - `NoteRepository::write(&note)`（frontmatter + body を `.md` に書く）
6. `emit: Note → NoteCreated`

## Dependencies {#dependencies}

- `NoteRepository` — `.md` ファイルの書き出し
- `Clock` — `now()` 取得（テスト時は injectable）
- `EventBus` — domain event の同期発行（in-process）

## Notes {#notes}

- Draft → Note の確定経路は **これが唯一**。AutoSave 経路では新規作成しない
- 空 `raw_body` でも作成を許容（spec では明示禁止していない、I-N1 〜 I-N7 に違反しない）
- 作成成功後の Draft 入力欄クリアは UI 層の責務（event を購読して実行）
