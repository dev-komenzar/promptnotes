---
coherence:
  source: human
  last_validated: 2026-06-20
  upstream:
    - aggregates.md
    - event-storming.md
    - context-map.md
---

# Domain Events {#domain-events}

PromptNotes は単一プロセス + Shared Kernel 構成のため、event は **同期 in-process**
で発行される（message bus は採用しない）。それでも「概念としての domain event」を
正式化することで、責務の言語化と将来の外部連携余地を残す。

実装上は単純な関数呼び出し（または observer pattern）で十分。
ただし event 単位の発行・購読は **テスト境界** として有用なので、各 event は
発行点を明確にする。

## Note Aggregate Events {#note-aggregate-events}

### NoteCreated {#note-created}

#### Trigger {#note-created-trigger}

`Note::create(body, tags, now)` の永続化（`.md` ファイル書き出し）成功時。
Cmd+Enter による Draft 確定が唯一の発行経路。

#### Payload {#note-created-payload}

```rust
struct NoteCreated {
  note_id: NoteId,
  created_at: Timestamp,
  initial_tags: TagSet,
}
```

#### Subscribers {#note-created-subscribers}

- **Note Feed (Shared Kernel)**: フィード最上部に新規ブロックを差し込む
- **UI 層**: Draft 入力欄をクリア + 新規ブロックへフォーカス遷移

#### Timing {#note-created-timing}

同期。ordering 要件なし（個別 user 操作なので並行性なし）。

### NoteBodyEdited {#note-body-edited}

#### Trigger {#note-body-edited-trigger}

`Note::edit_body(new_body, now)` の永続化成功時。
発行経路は **AutoSave (debounce)** または **Flush (focus 喪失 / blur / quit)**。
キー入力ごとには発行しない（永続化が完了して初めて event）。

#### Payload {#note-body-edited-payload}

```rust
struct NoteBodyEdited {
  note_id: NoteId,
  updated_at: Timestamp,
}
```

`body` 本文は載せない（Shared Kernel 経由で `&Note` を参照すれば取得可能）。

#### Subscribers {#note-body-edited-subscribers}

- **Note Feed**: `updatedAt` ソートのとき表示順を再計算

#### Timing {#note-body-edited-timing}

同期。同一 Note への連続発行は `updated_at` 秒精度で de-duplicate される。

### NoteTagsChanged {#note-tags-changed}

#### Trigger {#note-tags-changed-trigger}

`Note::assign_tag(tag)` または `Note::remove_tag(tag_name)` の永続化成功時。
個別の付与 / 削除を区別せず、結果としての TagSet 全体を運ぶ。

#### Payload {#note-tags-changed-payload}

```rust
struct NoteTagsChanged {
  note_id: NoteId,
  tags: TagSet,
  updated_at: Timestamp,
}
```

#### Subscribers {#note-tags-changed-subscribers}

- **Note Feed**: TagFilter が active のとき表示集合を再計算

#### Timing {#note-tags-changed-timing}

同期。

### NoteDeletedToTrash {#note-deleted-to-trash}

#### Trigger {#note-deleted-to-trash-trigger}

`Note::delete_to_trash()` の OS ゴミ箱移動成功時。

#### Payload {#note-deleted-to-trash-payload}

```rust
struct NoteDeletedToTrash {
  note_id: NoteId,
  original_path: PathBuf,
  deleted_at: Timestamp,
}
```

#### Subscribers {#note-deleted-to-trash-subscribers}

- **Note Feed**: 表示集合から除外
- **UI 層**: 「元に戻す」トーストを表示（仮 5 秒、Q5 決定）
- **Application service**: 直前の `DeletedNote` を 1 件保持（in-memory single path）

#### Timing {#note-deleted-to-trash-timing}

同期。複数削除が連続したら直前の `DeletedNote` は破棄される（古い削除の Undo は不可）。

### NoteRestoredFromTrash {#note-restored-from-trash}

#### Trigger {#note-restored-from-trash-trigger}

`DeletedNote::restore()` の OS ゴミ箱からの復帰成功時。
トースト消失後の呼び出しは UI 層で reject されるため、この event は
**トースト有効期間中の Undo 操作のみ** で発行される。

#### Payload {#note-restored-from-trash-payload}

```rust
struct NoteRestoredFromTrash {
  note_id: NoteId,
  restored_at: Timestamp,
}
```

#### Subscribers {#note-restored-from-trash-subscribers}

- **Note Feed**: 表示集合に再登場
- **UI 層**: トーストを閉じる

#### Timing {#note-restored-from-trash-timing}

同期。

## Settings Aggregate Events {#settings-aggregate-events}

### StorageDirChanged {#storage-dir-changed}

#### Trigger {#storage-dir-changed-trigger}

`Settings::change_storage_dir(new_dir)` の永続化（`settings.json` 書き出し）成功時。

#### Payload {#storage-dir-changed-payload}

```rust
struct StorageDirChanged {
  old_dir: PathBuf,
  new_dir: PathBuf,
}
```

#### Subscribers {#storage-dir-changed-subscribers}

- **UI 層**: 再起動を促すモーダルを表示（即時マイグレーションは行わない、I-S4）

#### Timing {#storage-dir-changed-timing}

同期。Note Capture / Note Feed は再起動まで反映しない。

### ThemeChanged {#theme-changed}

#### Trigger {#theme-changed-trigger}

`Settings::change_theme(new_theme)` の永続化成功時。

#### Payload {#theme-changed-payload}

```rust
struct ThemeChanged {
  new_theme: Theme,
}
```

#### Subscribers {#theme-changed-subscribers}

- **UI 層**: CodeMirror テーマと CSS 変数を即時切り替え

#### Timing {#theme-changed-timing}

同期。Note Capture / Note Feed は購読しない。

### SortPreferenceChanged {#sort-preference-changed}

#### Trigger {#sort-preference-changed-trigger}

`Settings::change_sort_preference(new_sort)` の永続化成功時。
発行経路は **設定モーダルからの変更** または **ツールバーのソートトグル**
（後者は NoteFeed.change_sort の副作用、aggregates.md の Notes 参照）。

#### Payload {#sort-preference-changed-payload}

```rust
struct SortPreferenceChanged {
  new_sort: SortOrder,
}
```

#### Subscribers {#sort-preference-changed-subscribers}

- **Note Feed**: 表示順を即時再計算
  - 注意: NoteFeed.change_sort 経路では NoteFeed 自身が既に更新済みなので
    重複適用にならないよう application service で冪等性を保つ

#### Timing {#sort-preference-changed-timing}

同期。

## UpdateChannel Aggregate Events {#update-channel-aggregate-events}

### NewVersionDetected {#new-version-detected}

#### Trigger {#new-version-detected-trigger}

`UpdateChannel::check_at_startup()` 成功 かつ `latest_release: Some(_)` の場合。
起動時 1 回のみ発行される（I-U3）。

#### Payload {#new-version-detected-payload}

```rust
struct NewVersionDetected {
  current_version: Version,
  latest_version: Version,
  release_url: Url,
  release_notes: String,
}
```

#### Subscribers {#new-version-detected-subscribers}

- **UI 層**: 起動時通知（Toast または Modal、Phase 11a UI 設計で確定）

#### Timing {#new-version-detected-timing}

非同期発行（network 呼び出しの結果）、購読は同期。
失敗時は event を発行しない（silent failure、I-U3 補足）。

## 発行しない event {#non-events}

Phase 2 / Phase 5 で候補に挙がったが、domain event として発行しないもの：

- **NoteBodyCopiedToClipboard**:
  副作用が「OS クリップボードへの書き込み」のみで domain state を変えない。
  UI レベルの操作として扱う
- **FeedFilterChanged / FeedSortToggled (NoteFeed 側)**:
  NoteFeed は read model のため、自身は event を発行しない。
  sort 変更の副作用は Settings 側の `SortPreferenceChanged` が代表する
- **NoteFocused / NoteEntered / NoteLeft**:
  ブロックのステートマシン (IDLE/FOCUSED/EDITING) 遷移は UI event であり
  ビジネスイベントではない（event-storming Notes で明示済み）

## Notes {#notes}

### in-process 同期発行の根拠 {#notes-sync-rationale}

- 全 BC が単一 Tauri プロセス内に同居
- Shared Kernel により Note の状態変化は参照経由で即座に見える
- 結果：event bus による配信は不要。**関数呼び出しに「発行点」のラベルを貼った形**
- 利点: テスト境界として有用（event が発行されたかを assert できる）
- 欠点: 将来 microservice 化や別プロセス分離をするなら event bus 化が必要

### payload の最小化方針 {#notes-payload-minimization}

- `body` 本文 / TagSet 以外の冗長データを載せない
- 購読側は `note_id` から Shared Kernel 経由で `&Note` を引ける
- 例外: `NoteTagsChanged` は TagSet 自体が「変化点」なので payload に含む

### event ordering {#notes-event-ordering}

- 単一 user による直列操作のみ想定 → ordering 要件なし
- 例外: アプリ quit 時の Flush は複数 Note の `NoteBodyEdited` を連続発行する可能性。
  購読側 (NoteFeed) は 1 個ずつ処理しても結果が同じ（冪等）であるべき

## Open Questions {#open-questions}

Phase 6 時点で未決事項はない。

- Phase 7 (validation) で「event 発行は永続化成功後」を invariant として明示する
  （現状は trigger 説明文に書いている）
- Phase 11a (UI fields) で `NewVersionDetected` の通知 UI（Toast vs Modal）を確定する
