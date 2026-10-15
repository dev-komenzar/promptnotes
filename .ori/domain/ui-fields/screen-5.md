---
ori:
  node_id: ui-field:screen-5
  type: ui-field
  depends_on:
    - type-definitions:index
    - workflow:detect-external-changes
    - aggregate:NoteFeed
---

# Screen 5: External Delete Notice {#screen-5}

外部プログラム（vim, VSCode, Syncthing 等）が `.md` ファイルを削除したことを
`NoteFileDeletedExternally` event 経由で検知し、ユーザが当該 Note を **EDITING 状態**で
開いている場合に表示する削除通知ダイアログ。

validation.md S20（EDITING 中の外部削除）を UI 層で受ける入口。
EDITING 中でなければ表示せず、Feed の自動更新（I-F8 `remove_note`）のみで完結する（S18）。

## Purpose {#purpose}

`detect-external-changes` workflow が発行する `NoteFileDeletedExternally` を受け、
当該 `note_id` が EDITING 中の場合に、編集中の内容を黙って失わないよう
「新規ファイルとして保存」か「破棄」かの選択肢を提示する。

表示条件:
- ユーザが当該 `note_id` を EDITING 状態で開いている
- ディスク上の `storage_dir/<note_id>.md` が外部で削除された

## Fields {#fields}

| id | label | 型 | 必須 | VO | UI | 備考 |
|----|------|----|-----|----|----|----|
| {#screen-5-note-title} | 削除されたノート | `NoteId` (readonly) | ✓ | NoteId | readonly label | ファイル名 (`YYYYMMDDhhmmss.md`) 表示 |
| {#screen-5-message} | 通知文 | — | ✓ | — | readonly text | 外部削除された旨と選択肢の説明 |
| {#screen-5-body-local} | 編集中の内容 | `NoteBody` | — | NoteBody | readonly textarea (disabled) | 現在の編集内容（ローカル snapshot） |
| {#screen-5-discard} | 破棄 | — | ✓ | — | secondary button | 編集中の内容を破棄し、Feed から除去 |
| {#screen-5-save-as-new} | 新規ファイルとして保存 | — | ✓ | — | primary button | 編集中の内容で `.md` を再作成 |

## Cross-Field Rules {#cross-field-rules}

- **新規ファイルとして保存** → 現在の編集中内容で `storage_dir/<note_id>.md` を再作成し、
  Feed は当該 Note を保持したまま Block を IDLE へ遷移させる。
- **破棄** → `NoteFeed::remove_note(&note_id)` 相当で Feed から除去し、編集中の内容を破棄する
  （フォーカスも解除）。
- 2 択の明示選択のみ。`Esc` / ×ボタンによる暗黙のクローズは提供しない
  （どちらとも決まらない状態で Block が取り残されるのを防ぐ）。

## Depended By {#depended-by}

- `screen-1`（メインウィンドウ）の modal overlay 領域に表示
- `detect-external-changes` workflow の `NoteFileDeletedExternally` を起点に発火

## Display States {#display-states}

### notice-layout {#display-states-notice}

```text
┌─ 20260620120000.md ─────────────────────────────┐
│                                                  │
│  編集中のノート「20260620120000.md」が外部で      │
│  削除されました。現在の内容で新規ファイルとして   │
│  保存するか、破棄するかを選んでください。         │
│                                                  │
│  ┌──────────────────────────────────────────┐   │
│  │ hello local                (textarea, r/o) │   │
│  └──────────────────────────────────────────┘   │
│                                                  │
│                [破棄]  [新規ファイルとして保存]   │
└──────────────────────────────────────────────────┘
```

### resolution-complete {#display-states-resolved}

選択後はダイアログを即座に閉じる。成功/失敗の追加 toast は不要
（Feed / Block の状態変化で十分伝わる）。

## Notes {#notes}

### トリガーとライフサイクル {#notes-lifecycle}

- トリガー: `NoteFileDeletedExternally` event（当該 note_id が EDITING 中の場合のみ）
- ライフサイクル: modal overlay（parent = page-main）
- 解除条件: 破棄 / 新規ファイルとして保存
- 同一 note_id の重複発火: 通知が開いている間は後続 event を無視（debounce）
- EDITING 中でない Note の削除では表示しない（Feed が自動更新）
- `NoteFileModifiedExternally` は対象外（[screen-4](screen-4.md) の競合ダイアログが担う）

### event 露出の制約 {#notes-event-exposure}

- domain event `NoteFileDeletedExternally` の payload は frontend に直接露出しない
  （Rust 側 subscriber は `remove_note` + payload なし Tauri event `notes-changed` の emit のみ）
- そのため frontend は `notes-changed` 受信時に、EDITING 中の note がディスクから消えたかを
  差分で判定して通知 payload を組み立てる（application service 層の判断、
  domain-events.md#note-file-deleted-externally-subscribers）
