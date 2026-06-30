---
coherence:
  source: derived
  last_derived: 2026-06-30
  upstream:
    - domain/workflows/detect-external-changes.md#detect-external-changes
    - domain/aggregates.md#note-feed-aggregate
    - domain/bounded-contexts.md#note-feed
    - domain/domain-events.md#note-file-created-externally
    - domain/domain-events.md#note-file-modified-externally
    - domain/domain-events.md#note-file-deleted-externally
    - domain/domain-events.md#storage-dir-changed
  hash:
    domain/workflows/detect-external-changes.md#.*: 43039669e809
    domain/aggregates.md#.*: 36f5d6dd006c
    domain/bounded-contexts.md#.*: ea610e21effd
    domain/domain-events.md#.*: 5914d20573c9
ori:
  schema:
    propagation_level: file
---

# detect-external-changes spec {#detect-external-changes-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive detect-external-changes`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

`storage_dir` 配下の `.md` ファイルに対する外部プログラム（vim, VSCode, Syncthing 等）からの変更を OS レベルのファイルウォッチャーで検知し、対応する domain event を発行する slice。発行された event は NoteFeed（upsert/remove）と Note Capture（競合検出）が購読する。

> domain/workflows/detect-external-changes.md#detect-external-changes より：「`storage_dir` 配下の `.md` ファイルに対する外部プログラムからの変更を OS レベルのファイルウォッチャーで検知し、対応する domain event を発行する workflow」
>
> domain/bounded-contexts.md#note-feed-purpose より：本 BC は Note 集合の read side を司る。外部変更検知は Note Feed BC の infrastructure 層（ファイルウォッチャー）から domain 層への**橋渡し**を行う

本 slice は infrastructure 層（notify crate によるファイル監視）から domain 層への bridge を実装する。検知機構そのものは infrastructure の責務であり、use case は raw OS イベントを domain event に変換する pipeline を定義する。

トリガーは 3 種類（workflows/detect-external-changes.md より）:
1. **アプリ起動時**: list-feed による初回 hydration 完了後に watcher 開始
2. **StorageDirChanged event 購読時**: 旧ディレクトリの監視を停止し、新ディレクトリで再開
3. **アプリ quit 時**: watcher を停止（infrastructure 層が OS シグナルで処理）

debounce 戦略: Syncthing 等の一時ファイル（`.syncthing.xxx.tmp`）書き込み → rename パターンに対応するため、同一ファイルへの連続イベントを **500ms** の窓で集約する。`.tmp` ファイルは無視し、rename 先が `.md` の場合のみ Created/Modified として扱う。

## 入出力 {#io}

### Input {#io-input}

> domain/workflows/detect-external-changes.md#input より：

```rust
struct DetectExternalChangesCommand {
  storage_dir: StorageDir,    // 監視対象ディレクトリ（Settings から解決）
}
```

依存（外部から注入される interface）:

- `NoteRepository` — `load_by_id()` でディスクから Note を再構築
- `Settings` — `storage_dir` 解決のため
- `Clock` — `detected_at` タイムスタンプ生成
- `EventBus` — domain event 発行（同期 in-process）
- **Infrastructure**: `notify` crate（OS ファイル監視 API）

### Output {#io-output}

戻り値: `Result<WatcherHandle, DetectExternalChangesError>`

- `Ok(handle)` — watcher 起動成功。`WatcherHandle` は Drop 時に自動停止する RAII ガード
- `Err(_)` — watcher 起動失敗（[#io-errors](#io-errors) 参照）

成功時の watcher は非同期に以下の domain event を発行する:

> domain/domain-events.md#external-file-change-events より：

- **`NoteFileCreatedExternally`**: 新規 `.md` ファイル検知（parse 成功時のみ）

```rust
struct NoteFileCreatedExternally {
  note_id: NoteId,
  note: Note,              // parse 済みの完全な Note
  file_path: PathBuf,
  detected_at: Timestamp,
}
```

- **`NoteFileModifiedExternally`**: 既存 `.md` ファイル変更検知（parse 成功時のみ）

```rust
struct NoteFileModifiedExternally {
  note_id: NoteId,
  disk_body_hash: BodyHash,  // ディスクから読み込んだ body の SHA-256（I-N9 競合検出用）
  note: Note,                 // parse 済みの完全な Note
  file_path: PathBuf,
  detected_at: Timestamp,
}
```

- **`NoteFileDeletedExternally`**: `.md` ファイル削除検知（NoteId 解決成功時のみ）

```rust
struct NoteFileDeletedExternally {
  note_id: NoteId,
  file_path: PathBuf,
  detected_at: Timestamp,
}
```

発行なし（skip ケース）:
- parse 失敗（malformed `.md`）
- 非 `.md` ファイルの変更（`.txt`, `.tmp` 等）
- debounce により集約された中間イベント

### Errors {#io-errors}

> domain/workflows/detect-external-changes.md#errors より：「**なし**（ファイルウォッチャーの障害は application 全体の可用性に影響しない）」

watcher 起動失敗時のみ `DetectExternalChangesError` を返す:

```rust
enum DetectExternalChangesError {
  WatcherStartFailed { path: PathBuf, source: io::Error },
}
```

起動後の実行時イベント（permission 変更、ディレクトリ削除等）は infrastructure 層で silent に処理し、ログに残すのみ。ユーザー通知は行わない。

## 不変条件 {#invariants}

### NoteFeed Aggregate 由来 {#invariants-note-feed-aggregate}

> domain/aggregates.md#note-feed-aggregate-invariants より引用：

- **I-F8**: NoteFeed は外部ファイル変更の検知を契機とした差分更新を受け付ける。`upsert_note` / `remove_note` 操作により部分更新可能。本 slice は検知機構そのものを提供し、発行した domain event 経由で NoteFeed の差分更新をトリガーする

### Note Aggregate 由来 {#invariants-note-aggregate}

> domain/aggregates.md#note-aggregate-invariants より引用：

- **I-N9**: `body_hash` は `body` から決定論的に導出される。外部プログラムが `.md` ファイルを変更した場合、application service 層がディスクから読み込んだ `body` のハッシュとメモリ上の `body_hash` を比較して競合を検出できる。本 slice は `NoteFileModifiedExternally` の payload に `disk_body_hash` を含め、競合検出の判断は application service 層に委ねる

### slice 固有制約 {#invariants-slice-specific}

- **C-DEC1**: watcher は `storage_dir/` 直下のファイルのみを監視する（サブディレクトリは監視しない。I-N2: flat 構造）
- **C-DEC2**: `.md` 拡張子のファイルのみを対象とする。非 `.md` ファイルの変更イベントは無視する
- **C-DEC3**: debounce 間隔は **500ms**。同一ファイルへの連続イベントを 1 つに集約する
- **C-DEC4**: `.tmp` ファイル（例: `.syncthing.xxx.tmp`）へのイベントは無視する
- **C-DEC5**: `onFileCreated` / `onFileModified` では `NoteRepository::load_by_id()` 相当の parse 経路で `Note` を構築する。parse 失敗時は silent skip
- **C-DEC6**: `onFileDeleted` ではファイル名（basename、拡張子除く）から `NoteId` を解決（`^\d{14}$` に一致するか）。解決失敗時は silent skip
- **C-DEC7**: `WatcherHandle` は Drop 時に自動停止する RAII ガードとして実装。`StorageDirChanged` 時の旧 watcher 停止は `WatcherHandle` の drop で保証
- **C-DEC8**: domain event は `EventBus` を介して**同期的に**発行される（in-process、message bus 不要）
- **C-DEC9**: watcher 起動失敗 (`WatcherStartFailed`) を除き、実行時エラーは error を返さない（log + silent skip）

### 経路境界 {#invariants-boundary}

- **C-DEC10**: 本 slice は infrastructure 層のファイルウォッチャー起動・停止と、raw OS イベント → domain event 変換 pipeline を実装する。NoteFeed の `upsert_note` / `remove_note` 呼び出しは **domain event の購読側**（Note Feed application service）の責務であり、本 slice の scope 外
- **C-DEC11**: 本 slice は `StorageDirChanged` event を**購読**し、watcher の再起動を行う。`StorageDirChanged` の**発行**は `update-settings` slice の責務

## 境界契約 {#boundary-contract}

- **kind**: `command`（ファイルウォッチャー起動・停止 + domain event 発行。side effect あり）
- **contact_point**: `#[tauri::command] pub async fn start_file_watcher()` / `stop_file_watcher()` in `apps/promptnotes/src-tauri/src/note_feed/slices/detect_external_changes/commands.rs`
- **cross_root**: Rust → TypeScript via **tauri-specta**（bindings 生成）
- **public_entry**: `apps/promptnotes/src-tauri/src/note_feed/slices/detect_external_changes/mod.rs` (Rust)
- **production_fixture**: `apps/promptnotes/src-tauri/src/note_feed/shared/test-fixtures/`（未設置なら追加）
- **forbidden_imports**: 他 slice の直接 import 禁止。cross-slice は `note_feed::shared` / `note_capture::shared` 経由のみ

## テスト観点 {#test-perspectives}

phase 3 で failing test に展開する観点を列挙。

### watcher 起動・停止 {#tp-watcher-lifecycle}

- **TP-WL1**: 有効な `storage_dir` で `start_watcher` → `Ok(WatcherHandle)`、watcher が起動する
- **TP-WL2**: 存在しない `storage_dir` で `start_watcher` → `Err(DetectExternalChangesError::WatcherStartFailed { path, source })`
- **TP-WL3**: `WatcherHandle` を drop → watcher が停止する（後続のファイル変更が検知されない）
- **TP-WL4**: `StorageDirChanged` 購読時: 旧 watcher 停止 → 新 watcher 起動の順序が保証される

### .md ファイル作成検知 {#tp-file-created}

- **TP-CR1**: `storage_dir/` に新規 `.md` ファイル（valid frontmatter）を作成 → `NoteFileCreatedExternally` が 1 回発行される
- **TP-CR2**: TP-CR1 で payload の `note_id`、`note.body`、`file_path`、`detected_at` が正しい
- **TP-CR3**: malformed `.md` ファイル（不正な frontmatter）を作成 → event は発行されない（silent skip、C-DEC5）
- **TP-CR4**: `.txt` ファイルを作成 → event は発行されない（非 `.md` 無視、C-DEC2）
- **TP-CR5**: `.tmp` ファイルを作成 → event は発行されない（C-DEC4）

### .md ファイル変更検知 {#tp-file-modified}

- **TP-MD1**: 既存 `.md` ファイルを外部変更 → `NoteFileModifiedExternally` が 1 回発行される
- **TP-MD2**: TP-MD1 で payload の `disk_body_hash` がディスク上の body の SHA-256 と一致する（I-N9）
- **TP-MD3**: `.md` ファイル変更後、`detected_at` が `Clock::now()` の値と一致する
- **TP-MD4**: `.md` ファイルを malformed な内容に変更 → event は発行されない（parse 失敗 skip）

### .md ファイル削除検知 {#tp-file-deleted}

- **TP-DL1**: `storage_dir/` の `.md` ファイル（NoteId 解決可能なファイル名）を削除 → `NoteFileDeletedExternally` が 1 回発行される
- **TP-DL2**: TP-DL1 で payload の `note_id` がファイル名から解決した NoteId と一致する
- **TP-DL3**: 非 Note ファイル名（`^\d{14}$` に一致しない）の `.md` ファイルを削除 → event は発行されない（C-DEC6）

### debounce 集約 {#tp-debounce}

- **TP-DB1**: 同一 `.md` ファイルに対し 100ms 間隔で 2 回の変更 → 500ms 窓で集約され event は **1 回のみ**発行（C-DEC3）
- **TP-DB2**: 異なる 2 ファイルに対し 100ms 間隔で変更 → 各ファイルにつき 1 回ずつ event が発行される（ファイル単位の debounce）
- **TP-DB3**: 同一ファイルに対し 600ms 間隔で変更 → 各変更につき event が発行される（debounce 窓を超えるため）

### サブディレクトリ無視 {#tp-flat}

- **TP-FL1**: `storage_dir/subdir/` に `.md` ファイルを作成 → event は発行されない（C-DEC1: flat 構造のみ）

### rename の扱い {#tp-rename}

- **TP-RN1**: `storage_dir/` 内で `.md` ファイルの名前を変更 → `NoteFileDeletedExternally`（旧名）+ `NoteFileCreatedExternally`（新名）の 2 イベントが連続発行される

### Boundary test {#tp-boundary}

- **TP-B1**: tauri-specta bindings 経由で `startFileWatcher()` を invoke → watcher 起動成功（DoD rule 2）
- **TP-B2**: production fixture 経由で watcher 起動 → ファイル操作 → domain event 発行 → `EventBus` に publish されたことを assert（DoD rule 3）

### no-op / 冪等 {#tp-idempotent}

- **TP-ID1**: 既に watcher 起動済みの状態で `start_watcher` を再呼出 → 既存 watcher を停止し新規起動（または `AlreadyRunning` を返す。設計判断は #oq-watcher-idempotent 参照）

## 実装ノート {#impl-notes}

### アーキ層 {#impl-layers}

DDD-VSA-Hex / typescript-tauri に従い Rust 側で実装する。全 sub_layers (`domain` / `application` / `infrastructure` / `presentation` / `tests`) を埋め込む（DoD rule 1）。

Note Feed BC の slice ディレクトリに追加：

```
apps/promptnotes/src-tauri/src/note_feed/
├── shared/                      # 既存：types/, events.rs, ports.rs
└── slices/
    └── detect_external_changes/ # 新規
        ├── mod.rs               # pub use commands::*; pub use application::*;
        ├── domain.rs            # DetectExternalChangesCommand, WatcherHandle, event payload types
        ├── application.rs       # DetectExternalChangesUseCase: start_watcher / stop_watcher / event handler
        ├── infrastructure.rs    # FsWatcher: notify crate wrapper, debounce logic, event transform
        ├── commands.rs          # #[tauri::command] start_file_watcher / stop_file_watcher
        └── tests/               # unit tests + integration tests
            ├── mod.rs
            ├── domain_tests.rs
            ├── application_tests.rs
            └── infrastructure_tests.rs
```

> **RED state b3 (DoD rule 2)**: 実装着手時は `commands.rs` を `Err("pending")` 返す stub として先に配置し、tauri-specta bindings を再生成してから boundary test を書き起こす。

### notify crate 統合 {#impl-notify}

> domain/workflows/detect-external-changes.md#notes より：debounce 戦略は Rust `notify` crate、debounce 500ms

```toml
# Cargo.toml 依存追加
[dependencies]
notify = { version = "6", default-features = false, features = ["macos_kqueue"] }
```

- `notify::recommended_watcher` で OS ネイティブのファイル監視を起動
- `notify::Config` で監視対象を `storage_dir` に設定
- `notify_debouncer_full` または自前の debounce ロジックで 500ms 集約
- `.tmp` 拡張子のファイルに対するイベントをフィルタリング（C-DEC4）
- watcher の event loop は別スレッドで実行し、`WatcherHandle` でライフサイクル管理

### Pipeline ステップ {#impl-pipeline}

> domain/workflows/detect-external-changes.md#steps の DMMF pipeline を採用：

1. `resolve_storage_dir: () -> StorageDir` — Settings から現在の `storage_dir` を解決
2. `start_watcher: StorageDir -> WatcherHandle` — notify crate で OS ファイル監視を起動、debounce 500ms 設定
3. `on_file_created: PathBuf -> Option<NoteFileCreatedExternally>` — `.md` 検証 → `NoteRepository::load_by_id()` で parse → event 構築
4. `on_file_modified: PathBuf -> Option<NoteFileModifiedExternally>` — `.md` 検証 → parse + `BodyHash` 計算 → event 構築
5. `on_file_deleted: PathBuf -> Option<NoteFileDeletedExternally>` — `.md` 検証 → `NoteId` 解決 → event 構築
6. `emit_event: DomainEvent -> ()` — `EventBus::publish()` で同期的に発行

### WatcherHandle の設計 {#impl-watcher-handle}

```rust
pub struct WatcherHandle {
    tx: Sender<()>,           // 停止シグナル送信用 channel
    handle: Option<JoinHandle<()>>,  // watcher event loop thread
}

impl Drop for WatcherHandle {
    fn drop(&mut self) {
        let _ = self.tx.send(());  // watcher thread に停止を通知
        if let Some(h) = self.handle.take() {
            let _ = h.join();       // thread 終了を待つ
        }
    }
}
```

- `WatcherHandle` は `Send + Sync` を実装し、process-local state で保持可能にする
- application service 層が `WatcherHandle` を保持し、`StorageDirChanged` 購読時に drop → 新規作成

### Tauri command surface {#impl-tauri}

- `#[tauri::command] async fn start_file_watcher(state: State<AppState>) -> Result<(), String>` — watcher 起動。`storage_dir` は Settings から解決
- `#[tauri::command] async fn stop_file_watcher(state: State<AppState>) -> Result<(), String>` — watcher 停止（アプリ quit 時）
- tauri-specta で TS bindings 生成

### StorageDirChanged 購読 {#impl-storage-dir-changed}

- `EventBus::subscribe::<StorageDirChanged>(handler)` で購読を登録
- handler 内で旧 `WatcherHandle` を drop → 新 `storage_dir` で watcher 再起動
- 再起動失敗時は silent log（workflows/detect-external-changes.md#errors: 「watcher の再起動失敗は infrastructure 層が retry またはユーザーに再起動を促す」— MVP では silent retry で実装）

### Out of scope {#out-of-scope}

- `NoteFeed::upsert_note` / `remove_note` の実装（domain event の購読側、Note Feed application service の責務）
- `NoteFileModifiedExternally` の競合検出・解決 UI（Note Capture application service + UI 層の責務）
- `StorageDirChanged` event の**発行**（`update-settings` slice の責務）
- フロントエンドからの watcher 状態表示 UI
- watcher 再起動失敗時のユーザー通知 UX

## Open Questions {#open-questions}

### 既に watcher 起動済みの状態での再呼出し {#oq-watcher-idempotent}

- **status**: open
- **問題**: `start_file_watcher` が既に watcher 起動済みの状態で呼ばれた場合の振る舞い。`StorageDirChanged` イベントハンドラは旧 watcher を drop してから新規起動するため、Tauri command 経由で直接 call された場合に限る
- **trade-off**:
  - 既存 watcher を停止して再起動: 冪等だが in-flight イベントが失われる可能性
  - `AlreadyRunning` error を返す: caller 側の責務明確化、シンプル
- **暫定**: `AlreadyRunning` をエラーとして返す（application service 層での二重起動防止は `WatcherState` で guard）
- **影響**: `DetectExternalChangesError` variant 追加の要否

### notify crate の debounce 戦略 {#oq-notify-debounce}

- **status**: resolved
- **決定**: `notify` crate v6 の `recommended_watcher` + 自前の debounce（`tokio::time::interval` + ファイル単位の最後のイベント時刻を `HashMap<PathBuf, Instant>` で追跡）で実装。`notify-debouncer-full` は追加 dependency が多く、単純なファイル単位 debounce で十分と判断
- **影響**: infrastructure.rs の実装方針
