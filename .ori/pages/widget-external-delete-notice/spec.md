---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  last_derived: 2026-10-15
  derives_from:
    - domain/ui-fields/screen-5.md#screen-5
    - domain/ui-fields/page-groups.md#widget-external-delete-notice
    - domain/workflows/detect-external-changes.md#detect-external-changes
    - domain/domain-events.md#note-file-deleted-externally
  hash:
    domain/ui-fields/screen-5.md#.*: 8ae9b5c9d594
    domain/ui-fields/page-groups.md#.*: f5d0be02c55e
    domain/workflows/detect-external-changes.md#.*: a66910b0d892
    domain/domain-events.md#.*: 86acb64f8149
---

# widget-external-delete-notice — Widget Specification {#widget-external-delete-notice-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive widget-external-delete-notice`. Use `/ori-propose` if the upstream needs to change.

## 概要 {#overview}

screen-5 を実体化する **event-driven modal overlay widget**。
[page-main](../page-main/spec.md) の modal overlay 領域に削除通知ダイアログとして表示し、
[detect-external-changes](../../domain/workflows/detect-external-changes.md#detect-external-changes) workflow
が発行する `NoteFileDeletedExternally` を起点に、ユーザが **EDITING 中**の Note が外部で削除された場合に限り表示される。

- **kind**: ui-widget
- **mount lifecycle**: page-main 起動時に subscribe 開始、EDITING 中の note の削除通知 payload 受信時のみ visible。破棄 / 新規ファイルとして保存で hidden に戻る
- **parent**: page-main（subscribe は page-main の lifecycle に従属）
- **subscribes**: 削除通知 payload（`NoteFileDeletedExternally` を起点に page-main の bridge が構成。[#event-exposure](#event-exposure)）
- **silent on absence**: payload 未受信 / 当該 note が EDITING 中でない場合は完全に表示なし（screen-5.md#notes-lifecycle、S18）

## 購読する payload {#subscribed-payload}

`NoteFileDeletedExternally` (`domain/domain-events.md#note-file-deleted-externally`) の
`note_id` / `file_path` / `detected_at` に、ローカルの編集中 snapshot を加えた payload:

| field | 型 | 用途 |
|--|--|--|
| `note_id` | `NoteId` | 判定キー。現在 EDITING 状態の note_id と比較 |
| `note_title` | `string` | ファイル名（`YYYYMMDDhhmmss.md`）。`widget.widget-external-delete-notice.note-title` / `.message` に表示 |
| `note_body` | `NoteBody` | 現在の編集中 body（ローカル snapshot）。`widget.widget-external-delete-notice.body-local` に表示 |
| `file_path` | `PathBuf` | 削除されたファイルのパス（UI 表示には使わない） |
| `detected_at` | `Timestamp` | 検知時刻（UI 表示には使わない） |

### event 露出の制約 {#event-exposure}

domain event `NoteFileDeletedExternally` の payload は frontend に露出しない
（Rust 側 subscriber は `remove_note` + payload なし Tauri event `notes-changed` の emit のみ。
screen-5.md#notes-event-exposure）。page-main の bridge が `notes-changed` 受信時に
「EDITING 中の note が disk snapshot に存在しない」ことを差分で判定し、上記 payload を構成して本 widget へ届ける。

### 表示判定フロー {#notice-detection-flow}

```
削除通知 payload 受信
  → note_id が現在 EDITING 状態か?
    → NO: 無視（Feed 自動更新のみ、S18）
    → YES: 通知が既に開いているか?
      → YES (同一 note_id): 無視（debounce）
      → NO: 通知を表示（本 widget の責務）
```

## レイアウト {#layout}

screen-5.md#display-states-notice の `notice-layout` を実装:

```
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

- `<dialog>` 要素で modal overlay 表示（`showModal()`）
- `Esc`（dialog の `cancel` event）は `preventDefault` で無効化し、2 択の明示選択のみ受け付ける
- 各要素の data-testid は `.ori/pages/widget-external-delete-notice/testids.yaml` の契約値を literal で付与する（ui-field id を写さない。詳細は [実装ノート](#impl-testid)）

### 表示状態 {#display-states}

| 状態 | 条件 | 表示 |
|--|--|--|
| `hidden` | payload 未受信 / 非 EDITING | DOM に何も出さない（`{#if store.payload}`） |
| `notice` | EDITING 中の note の削除を検知 | `notice-layout` を render |
| `resolved` | 破棄 / 新規ファイルとして保存 実行後 | ダイアログを閉じる（即座に hidden へ遷移） |

## 不変条件 {#invariants}

### Mount / lifecycle {#invariants-lifecycle}

- **I-DEL1（event-driven mount）**: page-main mount 時に subscribe を 1 回張り、unmount 時に解除。常駐 polling は行わない
- **I-DEL2（EDITING 中のみ表示）**: payload 未受信 / 当該 note が EDITING 中でない場合は DOM に何も出さない（silent on absence）。subscribe 失敗時も silent
- **I-DEL3（重複発火抑制）**: 同一 `note_id` の通知が開いている間は後続 payload を無視する（debounce。screen-5.md#notes-lifecycle）

### Resolution {#invariants-resolution}

- **I-DEL4（新規ファイルとして保存）**: 保存クリックで `onSaveAsNew` callback を payload を引数に 1 回呼び、hidden に戻る。page-main は編集中内容で `storage_dir/<note_id>.md` を再作成し、Feed に Note を保持したまま Block を IDLE へ遷移させる（screen-5.md#cross-field-rules）
- **I-DEL5（破棄）**: 破棄クリックで `onDiscard` callback を payload を引数に 1 回呼び、hidden に戻る。page-main は Feed から Note を除去しフォーカスを解除する（screen-5.md#cross-field-rules）
- **I-DEL6（暗黙クローズなし）**: `Esc` / ×ボタンでは閉じない（screen-5.md#cross-field-rules）

## テスト観点 (vitest) {#test-points}

### tp-del-no-mount-without-event: payload なしでは非表示 {#tp-del-no-mount-without-event}

store を `start()` した直後、payload を 1 件も emit せずに observe すると `store.payload === null`、`store.state === 'hidden'`（I-DEL2）。

### tp-del-event-mounts-notice: EDITING 中の削除で表示 {#tp-del-event-mounts-notice}

`currentNoteId()` と一致する `note_id` の payload を emit すると、`store.payload` が当該 payload を保持し `store.state === 'notice'` になる（I-DEL1, I-DEL2）。

### tp-del-not-editing-ignored: 非 EDITING の削除は無視 {#tp-del-not-editing-ignored}

`currentNoteId()` と異なる（または `null`）`note_id` の payload を emit しても `store.payload === null` のまま（I-DEL2）。

### tp-del-duplicate-ignored: 同一 note_id の重複を無視 {#tp-del-duplicate-ignored}

通知表示中に同一 `note_id` の 2 件目の payload を emit しても `store.payload` は最初の payload のまま（I-DEL3）。

### tp-del-save-as-new: 新規ファイルとして保存 {#tp-del-save-as-new}

`store.saveAsNew()` で `onSaveAsNew` が payload を引数に **1 回** 呼ばれ、`store.payload === null`、`store.state === 'hidden'` に戻る（I-DEL4）。

### tp-del-discard: 破棄 {#tp-del-discard}

`store.discard()` で `onDiscard` が payload を引数に **1 回** 呼ばれ、`store.payload === null`、`store.state === 'hidden'` に戻る（I-DEL5）。

### tp-del-stop-unsubscribes: stop で購読解除 {#tp-del-stop-unsubscribes}

`store.start()` 後の `store.stop()` で stub subscribeFn が返した unsubscribe function が **1 回** 呼ばれる（I-DEL1）。

### tp-del-subscribe-failure-silent: subscribe 失敗時 silent {#tp-del-subscribe-failure-silent}

stub subscribeFn が reject しても `store.start()` は throw せず、`store.payload === null` のまま（I-DEL2）。

## 実装ノート {#impl-notes}

### testid 契約 {#impl-testid}

- testid は `.ori/pages/widget-external-delete-notice/testids.yaml` の契約値（derived + extra）を literal で付与する。`testids.js check widget-external-delete-notice` が検査する
- root (`<dialog>`) は `widget.widget-external-delete-notice.root`

### store は createExternalDeleteNoticeStore で testable に {#impl-store}

- `createExternalDeleteNoticeStore(deps)` factory:
  - `deps.subscribeFn?`: `(handler: (payload) => void) => Promise<unsubscribe>`。default は no-op（production は `external-change-bridge.deleteDeps()` が注入）
  - `deps.currentNoteId?`: `() => NoteId | null`。現在 EDITING 状態の note_id
  - `deps.onSaveAsNew?` / `deps.onDiscard?`: 解決時の callback（page-main が実処理を注入）
- component (`*.svelte`) は store の薄い presentation layer
- vitest **server (node)** で全 deps を stub 注入して I-DEL1〜I-DEL5 を検証。I-DEL6 と E2E 経路は scenario s20 が検証

### ディレクトリ構成 {#impl-layout-dir}

```
apps/promptnotes/src/ui-widget/external-delete-notice/
├── WidgetExternalDeleteNotice.svelte    # presentation (modal dialog)
├── store.svelte.ts                      # createExternalDeleteNoticeStore factory
└── tests/
    └── store.test.ts                    # event-driven mount / resolution の unit test (server)
```

### page-main からの mount {#impl-mount-from-page-main}

- `PageMain.svelte` で **常に** `<WidgetExternalDeleteNotice deps={deleteNoticeDeps} />` を render
- `deleteNoticeDeps` は `external-change-bridge.deleteDeps(onSaveAsNew, onDiscard)` で構成
- 内部 store が `start()` で subscribe を張り、`{#if store.payload}` でのみダイアログを可視化（I-DEL2 silent）

### 非責務 {#impl-non-responsibility}

- **ファイル監視 / `remove_note`**: `detect-external-changes` slice（Rust）の責務
- **`.md` の再作成 / Feed 更新**: page-main が注入する callback の責務（本 widget は UI 表示とユーザ選択の取得に徹する）
- **`NoteFileModifiedExternally`**: [widget-external-change-conflict](../widget-external-change-conflict/spec.md) の責務
