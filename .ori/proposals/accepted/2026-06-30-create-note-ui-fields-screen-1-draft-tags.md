---
target: domain/ui-fields/screen-1.md#fields-draft
by: (直接ドメイン編集)
reason: Draft Input region にタグ入力・表示・削除の UI フィールドを追加。Block 領域の既存タグ UI パターンを踏襲
created: 2026-06-30
status: accepted
accepted_at: 2026-06-30
accepted_by: human (takuya.kometan@gmail.com)
applied_to: domain/ui-fields/screen-1.md#fields-draft (draft-tag-chip / draft-tag-input / draft-tag-remove の 3 フィールド追加) + #cross-draft-submit (タグクリア・引き渡し追記) + #cross-tag-error (draft-tag-input をエラー対象に追加)
---

# Proposal: Draft Input region にタグ入力 UI フィールドを追加

## 発見の経緯 {#context}

- 検出元：Draft 入力欄でのタグ設定を可能にする機能要望
- 試みていたこと：Draft モードでタグを入力し、Note 作成時に一緒に保存する
- 想定との差：screen-1.md の Draft Input region には本文入力 (`draft-body`) と提出ボタン (`draft-submit`) のみが定義されており、タグ入力 UI が存在しなかった。Block 領域には既にタグ入力 UI が存在する

## 現状仕様 {#current}

> domain/ui-fields/screen-1.md#fields-draft より：

| id | label | 型 | 必須 | UI | 備考 |
|--|--|--|--|--|--|
| `screen-1-draft-body` | 本文 | `String → NoteBody` | - | CodeMirror 6 | フィード最上部に常時固定 |
| `screen-1-draft-submit` | ＋追加 | (action) | - | button | クリックで Cmd+Enter と同等 |

## 矛盾／欠落 {#gap}

- `create-note` ワークフローは `raw_tags` を受け入れるが、Draft 領域にタグ入力 UI がなく、ユーザは Note 作成後にしかタグを付与できなかった
- Block 領域のタグ UI (`screen-1-block-tag-chip` / `screen-1-block-tag-input` / `screen-1-block-tag-remove`) と同等の操作を Draft でも提供すべき

## 提案する変更 {#proposal}

Draft Input region に 3 つのタグ関連フィールドを追加：

| id | label | 型 |
|--|--|--|
| `{#screen-1-draft-tag-chip}` | 下書きタグ | `Tag` |
| `{#screen-1-draft-tag-input}` | 新規タグ入力 | `String → Tag` |
| `{#screen-1-draft-tag-remove}` | タグ削除 | (action) |

合わせて：
- `#cross-draft-submit` にタグクリア・`raw_tags` 引き渡しを追記
- `#cross-tag-error` に `draft-tag-input` をエラー表示対象として追加

## 影響範囲 {#impact}

- domain/ui-fields/screen-1.md（フィールド定義 + cross-field rules）
- pages/page-main/spec.md（派生文書、必要に応じて再 derive）
- 実装: DraftRegion.svelte（タグ UI 実装、既に対応済み）
- 他画面・他ワークフローへの影響なし
