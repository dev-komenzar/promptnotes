---
ori:
  node_id: ui-field:screen-4
  type: ui-field
  depends_on:
    - type-definitions:index
    - workflow:detect-external-changes
    - aggregate:NoteFeed
---

# Screen 4: External Change Conflict Dialog {#screen-4}

外部プログラム（vim, VSCode, Syncthing 等）が `.md` ファイルを変更したことを
`NoteFileModifiedExternally` event 経由で検知し、ユーザが現在編集中の Note と
競合した場合に表示する競合解決ダイアログ。

aggregates.md I-N9（`BodyHash` による競合検出）と I-F8（`NoteFeed` の差分更新）
を UI 層で受ける入口。

## Purpose {#purpose}

`detect-external-changes` workflow が発行する `NoteFileModifiedExternally` を
購読し、`Note::is_stale(&disk_body_hash)` が `true` を返した場合に表示。
ユーザに「外部変更を適用するか、編集中の内容を保持するか」の選択肢を提示する。

表示条件:
- ユーザが当該 `note_id` を EDITING 状態で開いている
- ディスク上の `body` ハッシュ (`BodyHash`) がメモリ上のハッシュと異なる（`is_stale`）

## Fields {#fields}

| id | label | 型 | 必須 | VO | UI | 備考 |
|----|------|----|-----|----|----|----|
| {#screen-4-note-title} | 競合ノート | `NoteId` (readonly) | ✓ | NoteId | readonly label | ファイル名 (`YYYYMMDDhhmmss.md`) 表示 |
| {#screen-4-body-local} | 編集中の内容 | `NoteBody` | — | NoteBody | readonly textarea (disabled) | 現在の編集内容（変更前の snapshot） |
| {#screen-4-body-external} | 外部変更の内容 | `NoteBody` | — | NoteBody | readonly textarea (disabled) | ディスクから読み込んだ内容（diff 表示推奨）|
| {#screen-4-hash-local} | ローカルハッシュ | `BodyHash` | — | BodyHash | readonly label | SHA-256 hex（デバッグ用、常時非表示）|
| {#screen-4-hash-external} | ディスクハッシュ | `BodyHash` | — | BodyHash | readonly label | `NoteFileModifiedExternally.disk_body_hash`（デバッグ用） |
| {#screen-4-resolution-choice} | 解決方法 | `ConflictResolution` | ✓ | — | radio button group (2 択) | 後述の Cross-Field Rules 参照 |
| {#screen-4-confirm} | 確定 | — | ✓ | — | primary button | 選択内容を確定（後述） |
| {#screen-4-cancel} | キャンセル | — | — | — | secondary button | ダイアログを閉じる（編集中を保持 = KeepEditing と等価） |

### ConflictResolution enum {#screen-4-conflict-resolution-enum}

```rust
enum ConflictResolution {
    ApplyExternal,  // ディスクの内容で上書き（is_stale == false になる）
    KeepEditing,    // 現在の編集内容を維持（再保存時に上書き）
}
```

## Cross-Field Rules {#cross-field-rules}

- `{ApplyExternal}` 選択 → 確定ボタンで `NoteFeed::upsert_note(external_note)` を呼び、
  Editor の内容を外部バージョンに差し替える。`body_hash` も更新され `is_stale` が `false` になる。
- `{KeepEditing}` 選択 → ダイアログを閉じ、現在の編集内容を保持。
  次回の AutoSave / Flush でディスクを上書き（競合は解消される）。
- キャンセル (`Esc` / ×ボタン) → `KeepEditing` と等価。
- ダイアログ表示中は Editor のキー入力をブロックする（編集操作が競合解決を複雑化させるのを防ぐ）。

## Depended By {#depended-by}

- `screen-1`（メインウィンドウ）の toast / modal overlay 領域に表示
- `detect-external-changes` workflow からの event 購読で発火

## Display States {#display-states}

### compare-layout {#display-states-compare}

```text
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

### resolution-complete {#display-states-resolved}

選択後は `NoteFeed::upsert_note` で source を更新し、Feed の表示が自動で反映される。
ダイアログは即座に閉じる。成功/失敗の追加 toast は不要（Feed の状態変化で十分伝わる）。

## Notes {#notes}

### トリガーとライフサイクル {#notes-lifecycle}

- トリガー: `NoteFileModifiedExternally` event（application service 層で `is_stale` チェック後）
- ライフサイクル: modal overlay（parent = page-main）
- 解除条件: Apply / Cancel / Esc / ×ボタン
- 同一 note_id の重複発火: 最初のダイアログが開いている間は後続 event を無視（debounce）
- `NoteFileCreatedExternally` / `NoteFileDeletedExternally` はダイアログ不要（Feed が自動更新）

### 型レベル検証 {#notes-type-verification}

- `BodyHash` の比較は `==` でバイト等価（決定論的）
- `ConflictResolution` は 2 variant enum で網羅性保証（match 漏れなし）
- `is_stale` のシグネチャ: `fn is_stale(&self, disk_hash: &BodyHash) -> bool`
