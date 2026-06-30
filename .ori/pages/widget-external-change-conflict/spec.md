---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  last_derived: 2026-06-30
  derives_from:
    - domain/ui-fields/screen-4.md#screen-4
    - domain/ui-fields/page-groups.md#widget-external-change-conflict
    - domain/workflows/detect-external-changes.md#detect-external-changes
    - domain/domain-events.md#note-file-modified-externally
---

# widget-external-change-conflict — Widget Specification {#widget-external-change-conflict-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive widget-external-change-conflict`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

screen-4 を実体化する **event-driven modal overlay widget**。
[page-main](../page-main/spec.md) の modal overlay 領域に競合解決ダイアログとして表示し、
[detect-external-changes](../../domain/workflows/detect-external-changes.md#detect-external-changes) workflow
が発行する `NoteFileModifiedExternally` event を購読する。
ユーザが現在編集中の Note とディスク上の外部変更が競合した場合に限り表示される。

- **kind**: ui-widget
- **mount lifecycle**: page-main 起動時に subscribe 開始、`NoteFileModifiedExternally` 受信 + `Note::is_stale()` が `true` の場合のみ visible。Apply / Cancel / Esc / ×ボタンで hidden に戻る
- **parent**: page-main（subscribe は page-main の lifecycle に従属）
- **subscribes**: `NoteFileModifiedExternally` (in-process synchronous event bus)
- **silent on absence**: event 未受信 / `is_stale` が `false` / `NoteFileCreatedExternally` / `NoteFileDeletedExternally` の場合は完全に表示なし（screen-4.md#notes-lifecycle）

## 購読する event {#subscribed-event}

`NoteFileModifiedExternally` payload (`domain/domain-events.md#note-file-modified-externally`):

| field | 型 | 用途 |
|--|--|--|
| `note_id` | `NoteId` | 競合判定のキー。現在 EDITING 状態の note_id と比較 |
| `disk_body_hash` | `BodyHash` | `Note::is_stale(&disk_body_hash)` で競合判定。`screen-4-hash-external` にデバッグ表示 |
| `note` | `Note` | ディスクから parse された完全な Note。ApplyExternal 選択時に Editor へ差し替え |
| `file_path` | `PathBuf` | ファイル名抽出用（`screen-4-note-title` に `YYYYMMDDhhmmss.md` 表示） |
| `detected_at` | `Timestamp` | 検知時刻（UI 表示には使わない） |

### 競合判定フロー {#conflict-detection-flow}

```
NoteFileModifiedExternally 受信
  → note_id が現在 EDITING 状態か?
    → NO: 無視（他の Note への変更）
    → YES: Note::is_stale(&disk_body_hash) が true?
      → YES: ダイアログ表示（本 widget の責務）
      → NO: 無視（同一内容 = 競合なし）
```

同一 `note_id` のダイアログが既に開いている間は後続 event を無視（debounce、screen-4.md#notes-lifecycle）。

## レイアウト {#layout}

screen-4.md#display-states-compare の `compare-layout` を実装:

```
┌─ External Change Detected ──────────────────────┐
│                                                  │
│  "20260630120000.md" が外部で変更されました。     │
│  編集中の内容と競合しています。                   │
│                                                  │
│  ┌─ Your Version ───┐ ┌─ External Version ───┐  │
│  │ hello world       │ │ hello world!!!       │  │
│  │ (textarea, r/o)   │ │ (textarea, r/o)      │  │
│  └───────────────────┘ └──────────────────────┘  │
│                                                  │
│  ○ Apply external changes                        │
│  ● Keep my edits                                 │
│                                                  │
│  [Apply]  [Cancel]                               │
└──────────────────────────────────────────────────┘
```

- `<dialog>` 要素で modal overlay 表示（`showModal()`）
- 左右 2 カラムでローカル版 / 外部版の body を並列表示
- `ConflictResolution` radio button: `ApplyExternal` / `KeepEditing`
- Apply ボタンで選択内容を確定、Cancel / Esc / ×ボタンで KeepEditing と等価に閉じる

### 表示状態 {#display-states}

| 状態 | 条件 | 表示 |
|--|--|--|
| `hidden` | event 未受信 / 競合なし | DOM に何も出さない（`{#if store.conflictPayload}`） |
| `compare` | 競合検出 → ダイアログ表示 | `compare-layout` を render |
| `resolved` | Apply / Cancel 実行後 | ダイアログを閉じる（即座に hidden へ遷移） |

## 不変条件 {#invariants}

### Mount / lifecycle {#invariants-lifecycle}

- **I-WC1（event-driven mount）**: page-main mount 時に subscribe 開始、unmount 時に解除。常駐 polling は行わない
- **I-WC2（競合時のみ表示）**: `NoteFileModifiedExternally` 未受信 / `is_stale` が `false` の場合は DOM に何も出さない（silent on absence）
- **I-WC3（重複発火抑制）**: 同一 `note_id` のダイアログが既に開いている間は後続 event を無視する（debounce。screen-4.md#notes-lifecycle）
- **I-WC4（Create/Delete は無視）**: `NoteFileCreatedExternally` / `NoteFileDeletedExternally` では mount しない（screen-4.md#notes-lifecycle）

### Conflict Resolution {#invariants-resolution}

- **I-WC5（ApplyExternal で上書き）**: `ConflictResolution::ApplyExternal` 選択 + Apply クリックで Editor の内容を外部バージョン（`event.note`）で差し替え、`body_hash` を再計算。結果として `is_stale` が `false` になる
- **I-WC6（KeepEditing で保持）**: `ConflictResolution::KeepEditing` 選択 + Apply クリック / Cancel / Esc / ×ボタンでダイアログを閉じ、現在の編集内容を保持。次回 AutoSave / Flush でディスクを上書き
- **I-WC7（編集中ブロック）**: ダイアログ表示中は Editor のキー入力をブロックする（編集操作が競合解決を複雑化させるのを防ぐ。screen-4.md#cross-field-rules）

### 経路境界 {#invariants-boundary}

- **I-WC8（event listen 経由）**: `NoteFileModifiedExternally` の購読は in-process event bus を経由する。直接ファイル監視は行わない
- **I-WC9（Editor 更新は inject 可能）**: `ApplyExternal` 実行時の Editor 差し替えロジックは injectable に保ち、テストでは stub を注入可能にする

## テスト観点 (vitest) {#test-points}

### tp-wc-no-mount-without-event: event なしでは非表示 {#tp-wc-no-mount-without-event}

store を `start()` した直後、`NoteFileModifiedExternally` を 1 件も emit せずに observe すると
`store.conflictPayload === null`。UI 側は `{#if store.conflictPayload}` で nothing を render（I-WC2）。

### tp-wc-event-mounts-dialog: 競合検出で payload set {#tp-wc-event-mounts-dialog}

stub event bus に `NoteFileModifiedExternally` payload（EDITING 中の note_id と異なる `disk_body_hash` を持つ）を emit させると、`store.conflictPayload` が当該 payload の全 field を保持し、`store.state === 'compare'` になる（I-WC1, I-WC2）。

### tp-wc-no-conflict-no-mount: 競合なしでは非表示 {#tp-wc-no-conflict-no-mount}

stub event bus に `NoteFileModifiedExternally` を emit するが、`disk_body_hash` が現在編集中の Note の `body_hash` と一致する（`is_stale` が `false`）場合、`store.conflictPayload === null` のまま（I-WC2）。

### tp-wc-apply-external: ApplyExternal で解決 {#tp-wc-apply-external}

`store.selectResolution('ApplyExternal')` → `store.apply()` を呼ぶと、`store.onApplyExternal` callback が event payload の `note` を引数に **1 回** 呼ばれ、`store.conflictPayload === null`、`store.state === 'hidden'` に戻る（I-WC5）。

### tp-wc-keep-editing: KeepEditing で解決 {#tp-wc-keep-editing}

`store.selectResolution('KeepEditing')` → `store.apply()` を呼ぶと、`store.onApplyExternal` callback は呼ばれず、`store.conflictPayload === null`、`store.state === 'hidden'` に戻る（I-WC6）。

### tp-wc-cancel-equals-keep: Cancel は KeepEditing と等価 {#tp-wc-cancel-equals-keep}

`store.cancel()` を呼ぶと、resolution 選択に関わらず `store.conflictPayload === null`、`store.state === 'hidden'` に戻る。`store.onApplyExternal` は呼ばれない（I-WC6）。

### tp-wc-duplicate-ignored: 同一 note_id の重複発火を無視 {#tp-wc-duplicate-ignored}

`store.conflictPayload !== null`（ダイアログ表示中）の状態で、同一 `note_id` の 2 件目の event を emit しても `store.conflictPayload` は最初の payload のまま変わらない（I-WC3）。

### tp-wc-stop-unsubscribes: stop で購読解除 {#tp-wc-stop-unsubscribes}

`store.start()` 後の `store.stop()` で stub subscribeFn が返した unsubscribe function が **1 回** 呼ばれる（I-WC1）。

### tp-wc-subscribe-failure-silent: subscribe 失敗時 silent {#tp-wc-subscribe-failure-silent}

stub subscribeFn が reject しても `store.start()` は throw せず、`store.conflictPayload === null` のまま（I-WC2: silent failure）。

## 実装ノート {#impl-notes}

### ConflictResolution store は createConflictDialogStore で testable に {#impl-store}

- `createConflictDialogStore(deps)` factory:
  - `deps.subscribeFn?`: `(handler: (event: NoteFileModifiedExternallyPayload) => void) => Promise<unsubscribe>`。default は in-process event bus（Rust → TS bridge 経由。実装時点では stub）
  - `deps.isStaleFn?`: `(localBodyHash: string, diskBodyHash: string) => boolean`。default は単純な文字列比較（hash 比較は Rust 側で行う想定だが、テスト容易性のため injectable）
  - `deps.onApplyExternal?`: `(note: NoteDto) => void`。ApplyExternal 選択時の Editor 差し替え callback
  - `deps.currentNoteId?`: `() => NoteId | null`。現在 EDITING 状態の note_id 取得
  - `deps.currentBodyHash?`: `() => BodyHash | null`。現在編集中の Note の `body_hash`
- component (`*.svelte`) は store の薄い presentation layer
- vitest **server (node)** で全 deps を stub 注入して I-WC1〜I-WC9 を検証

### ディレクトリ構成 {#impl-layout-dir}

```
apps/promptnotes/src/ui-widget/external-change-conflict/
├── WidgetExternalChangeConflict.svelte    # presentation (modal dialog)
├── store.svelte.ts                        # createConflictDialogStore factory
└── tests/
    └── store.test.ts                      # event-driven mount / resolution の unit test (server)
```

### page-main からの mount {#impl-mount-from-page-main}

- `PageMain.svelte` の末尾で **常に** `<WidgetExternalChangeConflict />` を render
- 内部 store が `start()` で subscribe を張り、競合検出時のみ `{#if store.conflictPayload}` でダイアログを可視化（I-WC2 silent）
- `page-main` の unmount 時に store の `stop()` で unsubscribe

### 非責務 {#impl-non-responsibility}

- **ファイル監視**: 本 widget は event 購読のみ。ファイル監視自体は `detect-external-changes` slice（Rust）の責務
- **NoteFeed の更新**: `ApplyExternal` 選択時の `NoteFeed::upsert_note` 呼出は Rust 側（event subscriber）で行うか、フロントエンドの application service 層に委譲。本 widget は UI 表示とユーザ選択の取得に徹する
- **`NoteFileCreatedExternally` / `NoteFileDeletedExternally`**: これらの event は本 widget では購読しない（screen-4.md#notes-lifecycle）

## Open Questions {#open-questions}

- **OQ-WC1（event bridge）**: `NoteFileModifiedExternally` を Rust から TS へ bridge する経路が未確定（tauri-specta bindings 未生成）。本 widget の subscribe は stub でテストし、実 bridge は `detect-external-changes` slice の実装完了後に follow-up で繋ぐ
- **OQ-WC2（ConflictResolution type の所在）**: `ConflictResolution` enum は Rust 側（`note_capture::shared::types` または `note_feed::shared::types`）で定義し、tauri-specta 経由で TS に export するか、TS 側で独立定義するか未確定。widget 実装時は TS 側で `type ConflictResolution = 'ApplyExternal' | 'KeepEditing'` を仮定義する
- **OQ-WC3（ApplyExternal 後の Editor 差し替え）**: `ApplyExternal` 選択時の Editor 更新ロジック（`Note::edit_body` の呼出経路）は `note-capture` BC の application service に委譲するか、page-main の store が直接ハンドルするか未確定。本 widget は `onApplyExternal` callback を inject して分離
