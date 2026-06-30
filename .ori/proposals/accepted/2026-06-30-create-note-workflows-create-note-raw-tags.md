---
target: domain/workflows/create-note.md#input
by: (直接ドメイン編集)
reason: raw_tags のコメントを「将来拡張」から「現行機能」に変更。バックエンドは既に対応済みで制約は UI レイヤーのみだったため
created: 2026-06-30
status: accepted
accepted_at: 2026-06-30
accepted_by: human (takuya.kometan@gmail.com)
applied_to: domain/workflows/create-note.md#input (raw_tags コメント変更: 「spec では Draft からタグ付与しないが将来拡張」 → 「Draft 入力時に指定可能」)
---

# Proposal: create-note ワークフローの raw_tags を将来拡張から現行機能に変更

## 発見の経緯 {#context}

- 検出元：Draft 入力欄でのタグ設定を可能にする機能要望
- 試みていたこと：Draft モードでタグを入力し、Note 作成時に一緒に保存する
- 想定との差：create-note workflow の Input に `raw_tags: Vec<String>` が既に定義されていたが、コメントで「将来拡張」扱いだった。バックエンド (`create_note` Tauri command) は既に `raw_tags` を受け入れており、制約は UI レイヤーのみ

## 現状仕様 {#current}

> domain/workflows/create-note.md#input より：

```rust
struct CreateNoteCommand {
  raw_body: String,
  raw_tags: Vec<String>,   // optional: 初期タグ（spec では Draft からタグ付与しないが将来拡張）
}
```

## 矛盾／欠落 {#gap}

- `CreateNoteCommand.raw_tags` は型定義・Steps ともに既に存在し、バックエンド実装も受け入れ済み
- UI レイヤー（DraftRegion.svelte）が `create(body, [])` と空配列を渡していたため、事実上タグ付与が不可能だった
- コメントが「将来拡張」となっているため、実装済みのバックエンド機能との不整合が生じていた

## 提案する変更 {#proposal}

コメントを「将来拡張」から「現行機能」に変更：

```rust
struct CreateNoteCommand {
  raw_body: String,
  raw_tags: Vec<String>,   // 初期タグ（Draft 入力時に指定可能）
}
```

## 影響範囲 {#impact}

- domain/workflows/create-note.md#input（1 行のコメント変更のみ）
- slices/create-note/spec.md（派生文書、同様のコメント変更が必要だが再 derive で自動反映可）
- 実装: draft.svelte.ts / DraftRegion.svelte（UI レイヤー、既に対応済み）
- Note Aggregate や他のワークフローへの影響なし
