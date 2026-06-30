---
ori:
  node_id: workflow:detect-external-changes
  type: workflow
  depends_on:
    - aggregate:NoteFeed
    - aggregate:Note
    - event:NoteFileCreatedExternally
    - event:NoteFileModifiedExternally
    - event:NoteFileDeletedExternally
    - event:StorageDirChanged
---

# detect-external-changes {#detect-external-changes}

`storage_dir` 配下の `.md` ファイルに対する外部プログラム（vim, VSCode, Syncthing 等）
からの変更を OS レベルのファイルウォッチャーで検知し、対応する domain event
（`NoteFileCreatedExternally` / `NoteFileModifiedExternally` / `NoteFileDeletedExternally`）
を発行する workflow。発行された event は NoteFeed（upsert/remove）と
Note Capture（競合検出）が購読する。

本 workflow は infrastructure 層（ファイルウォッチャー）から domain 層への
**橋渡し** を行う。検知機構そのものは infrastructure の責務であり、
本 workflow は raw OS イベントを domain event に変換する pipeline を定義する。

## Input {#input}

```rust
struct DetectExternalChangesCommand {
  storage_dir: StorageDir,    // 監視対象ディレクトリ（Settings から解決）
}
```

トリガー:

- **アプリ起動時**: `list-feed` による初回 hydration 完了後に 1 回呼ぶ（watcher 開始）
- **StorageDirChanged event 購読時**: 旧ディレクトリの監視を停止し、新ディレクトリで再開
- **アプリ quit 時**: watcher を停止（infrastructure 層が OS シグナルで処理）

## Output {#output}

- domain event: [NoteFileCreatedExternally](../domain-events.md#note-file-created-externally)
- domain event: [NoteFileModifiedExternally](../domain-events.md#note-file-modified-externally)
- domain event: [NoteFileDeletedExternally](../domain-events.md#note-file-deleted-externally)
- **なし**（検知したが parse 失敗 / 非 `.md` ファイル / debounce により集約された場合）

## Errors {#errors}

- **なし**（ファイルウォッチャーの障害は application 全体の可用性に影響しない）
  - OS レベルの監視失敗（permission 変更、ディレクトリ削除等）は
    infrastructure 層で silent に処理し、ログに残す。ユーザー通知は行わない
  - `.md` ファイルの parse 失敗は個別 skip（`list-feed` と同様の「読めるものだけ」原則）
  - watcher の再起動失敗は `StorageDirChanged` の subscriber として
    infrastructure 層が retry またはユーザーに再起動を促す

## Steps {#steps}

1. `resolveStorageDir: () → StorageDir`
   - Settings から現在の `storage_dir` を解決
   - `StorageDirChanged` 購読時は新しいディレクトリを直接受け取る

2. `startWatcher: StorageDir → WatcherHandle`
   - OS ネイティブのファイル監視 API を起動
   - 監視対象: `storage_dir/*.md`（サブディレクトリは監視しない、I-N2 に基づき flat 構造）
   - `WatcherHandle` は監視の停止に必要なハンドル（Drop 時に自動停止）
   - **debounce**: infrastructure 層で同一ファイルへの短時間の連続イベントを集約する
     （例: Syncthing の一時ファイル書き込み → rename パターン。間隔: 500ms）

3. `onFileCreated: PathBuf → Option<NoteFileCreatedExternally>`
   - ファイル拡張子が `.md` であることを確認（非 `.md` は無視）
   - `NoteRepository::load_by_id()` 相当の parse 経路で `Note` を構築
   - parse 成功 → `NoteFileCreatedExternally { note_id, note, file_path, detected_at }` を返す
   - parse 失敗 → `None`（skip）

4. `onFileModified: PathBuf → Option<NoteFileModifiedExternally>`
   - ファイル拡張子が `.md` であることを確認
   - `NoteRepository::load_by_id()` 相当の parse 経路で `Note` を構築
   - ディスクから読んだ body から `BodyHash` を計算（I-N9 競合検出用）
   - parse 成功 → `NoteFileModifiedExternally { note_id, disk_body_hash, note, file_path, detected_at }` を返す
   - parse 失敗 → `None`（skip）

5. `onFileDeleted: PathBuf → Option<NoteFileDeletedExternally>`
   - ファイル拡張子が `.md` であることを確認
   - ファイル名（basename、拡張子除く）から `NoteId` を解決（`^\d{14}$` に一致するか）
   - 解決成功 → `NoteFileDeletedExternally { note_id, file_path, detected_at }` を返す
   - 解決失敗（非 Note ファイル名）→ `None`（skip）

6. `emitEvent: DomainEvent → ()`
   - 生成された domain event を同期的に発行
   - 購読者（NoteFeed, Note Capture application service）が同期的に処理

## Dependencies {#dependencies}

- `NoteRepository` — `load_by_id()` でディスクから Note を再構築
- `Settings` — `storage_dir` 解決のため
- `Clock` — `detected_at` タイムスタンプ生成
- `EventBus` — domain event 発行（同期 in-process）
- **Infrastructure**: OS ファイル監視 API（Rust `notify` crate）

## Notes {#notes}

- **debounce 戦略**: Syncthing は一時ファイル（`.syncthing.xxx.tmp`）への書き込み →
  rename のパターンを使う。infrastructure 層の watcher は `.tmp` ファイルを無視し、
   rename 先が `.md` の場合のみ `Created` / `Modified` として扱う。
   debounce 窓: 500ms（同一ファイルへの連続イベントを 1 つに集約）
- **watcher の粒度**: サブディレクトリは監視しない（I-N2: flat 構造、NoteId が
  ファイル名と 1:1 で対応する前提）
- **watcher 再起動**: `StorageDirChanged` event 購読時、旧ディレクトリの watcher を
  停止し、新ディレクトリで新規に起動する。watcher の停止は `WatcherHandle` の Drop で保証。
  再起動失敗時はアプリ全体の再起動を促す（I-S4 と整合）
- **parse 失敗の扱い**: 外部プログラムが不正な frontmatter を持つ `.md` を書き込んだ場合、
  当該ファイルは skip される。ユーザー通知は行わない（「読めるものだけ読む」原則、
  `list-feed` と同様）
- **rename の扱い**: ファイル名変更（= NoteId 変更）はファイルウォッチャーが
  delete + create の 2 イベントとして報告するため、`NoteFileDeletedExternally` +
  `NoteFileCreatedExternally` の連続発行で自然に対応できる。
  `NoteFileRenamedExternally` は発行しない設計判断（domain-events.md#non-events）
- **パフォーマンス**: Note 数が 1k を超えても、差分更新（`upsert_note`）により
  変更されたファイルのみ処理するため、全体再 hydrate に比べて効率的
