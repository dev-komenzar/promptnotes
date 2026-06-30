---
ori:
  node_id: event:collection
  type: event
  depends_on:
    - aggregate:collection
    - event-storming:timeline
    - context-map:map
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
- **UI 層**: 「元に戻す」トーストを **画面下部の縦パイル** に追加表示（仮 5 秒、各 Toast 個別、Q5 改訂）
- **Application service**: `DeletedNote` を Undo スタック (`Vec<DeletedNote>`) に **push**

#### Timing {#note-deleted-to-trash-timing}

同期。複数削除が連続しても各 `DeletedNote` は独立して保持され、対応する Toast の
有効期間中はそれぞれ Undo 可能（Phase 11a UI 設計改訂による）。

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

## External File Change Events {#external-file-change-events}

外部プログラム（vim, VSCode, Syncthing 等）による `storage_dir/*.md` の変更を
OS レベルのファイルウォッチャーが検知した際に発行される。
発行元は infrastructure 層（ファイルウォッチャー）であり、特定の aggregate command
からは発行されない。購読者は NoteFeed（表示更新）と Note Capture（競合検出）。

### NoteFileCreatedExternally {#note-file-created-externally}

#### Trigger {#note-file-created-externally-trigger}

ファイルウォッチャーが `storage_dir/` 配下に新規 `.md` ファイルの作成を検知した時。
ファイル内容の parse（frontmatter 抽出、Note 構築）は application service 層で行い、
成功した場合のみこの event を発行する。parse 失敗時は silent に無視（malformed ファイル）。

#### Payload {#note-file-created-externally-payload}

```rust
struct NoteFileCreatedExternally {
  note_id: NoteId,
  note: Note,              // parse 済みの完全な Note
  file_path: PathBuf,
  detected_at: Timestamp,
}
```

#### Subscribers {#note-file-created-externally-subscribers}

- **Note Feed**: `upsert_note(note)` で source に追加（I-F8）
- **UI 層**: 通知不要（フィードの自然な更新で十分）

#### Timing {#note-file-created-externally-timing}

同期。ファイルウォッチャーの OS イベント受信から parse 完了までを含む。
debounce は infrastructure 層で適用済み。

### NoteFileModifiedExternally {#note-file-modified-externally}

#### Trigger {#note-file-modified-externally-trigger}

ファイルウォッチャーが `storage_dir/` 配下の既存 `.md` ファイルの変更を検知した時。
`Note::list_all()` と同様の parse 経路でディスクから再読み込みし、
構築に成功した場合のみ発行。

#### Payload {#note-file-modified-externally-payload}

```rust
struct NoteFileModifiedExternally {
  note_id: NoteId,
  disk_body_hash: BodyHash,  // ディスクから読み込んだ body の SHA-256（I-N9 競合検出用）
  note: Note,                 // parse 済みの完全な Note
  file_path: PathBuf,
  detected_at: Timestamp,
}
```

#### Subscribers {#note-file-modified-externally-subscribers}

- **Note Feed**: `upsert_note(note)` で source 内の該当 Note を差し替え（I-F8）
- **Note Capture (application service)**: 当該 `note_id` が現在編集中の場合、
  `Note::is_stale(&disk_body_hash)` で競合を判定し、ユーザに選択肢を提示する
  （「外部変更を適用」「編集中を保持」）

#### Timing {#note-file-modified-externally-timing}

同期。競合検出と UI への通知は application service 層が subscriber として
同期的に処理する。

### NoteFileDeletedExternally {#note-file-deleted-externally}

#### Trigger {#note-file-deleted-externally-trigger}

ファイルウォッチャーが `storage_dir/` 配下の `.md` ファイル削除を検知した時。
ファイル名から `NoteId` を解決できる場合のみ発行（非 `.md` ファイルの削除は無視）。

#### Payload {#note-file-deleted-externally-payload}

```rust
struct NoteFileDeletedExternally {
  note_id: NoteId,
  file_path: PathBuf,
  detected_at: Timestamp,
}
```

#### Subscribers {#note-file-deleted-externally-subscribers}

- **Note Feed**: `remove_note(&note_id)` で source から除外（I-F8）
- **UI 層**: 通知不要（フィードの自然な更新で十分）。
  編集中に削除された場合は application service が別途判断

#### Timing {#note-file-deleted-externally-timing}

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
- **Infrastructure 層（ファイルウォッチャー）**: 監視対象ディレクトリを
  `new_dir` に切り替え。旧ディレクトリの監視は停止（Phase 9 workflow で詳細設計）

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
- **NoteFileRenamedExternally**:
  ファイル名変更は NoteId の変更を意味し、事実上の「削除＋新規作成」として扱う。
  ファイルウォッチャーは rename を delete＋create の 2 イベントとして報告するため、
  `NoteFileDeletedExternally` + `NoteFileCreatedExternally` の連続発行で十分

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
- **外部変更イベントの ordering**: Syncthing 等による一括同期時、短時間に多数の
  `NoteFileModifiedExternally` が発行される可能性がある。
  購読側は各イベントを独立に処理できる（`upsert_note` / `remove_note` は冪等）。
  debounce は infrastructure 層（ファイルウォッチャー）で適用する

## Open Questions {#open-questions}

Phase 6 改訂（外部ファイル変更イベントの追加に伴う）:

- ファイルウォッチャーの debounce 戦略は `detect-external-changes` workflow に記述済み
  （Rust `notify` crate、debounce 500ms）
- `NoteFileModifiedExternally` の競合検出: 編集中判定はフロントエンドの
  Block ステートマシン（IDLE/FOCUSED/EDITING）で行う。
  EDITING 状態かつ `is_stale()` = true の場合に競合ダイアログを表示する方針を確定
  → Phase 7 (validation) S19/S20 でシナリオ化済み
- Phase 11a (UI fields) で競合通知ダイアログの UI 設計を確定する
