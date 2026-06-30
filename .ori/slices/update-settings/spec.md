---
coherence:
  source: derived
  last_derived: 2026-06-30
  hash:
    domain/workflows/update-settings.md#.*: f420da94bd93
    domain/aggregates.md#.*: 82947dbfd3f6
    domain/bounded-contexts.md#.*: 7ebfcda8743b
    domain/domain-events.md#.*: 8abdfac78084
    domain/validation.md#.*: 31244b277867
ori:
  schema:
    propagation_level: file
---

# update-settings spec {#update-settings-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive update-settings`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

設定モーダルからの保存操作で `Settings` Aggregate の `storage_dir` / `theme` を更新する command slice。差分があれば `app_config_dir/settings.json` に永続化し、変更があったフィールドだけ対応する domain event（`StorageDirChanged` / `ThemeChanged`）を 0〜2 件発行する。`sort_preference` は別 slice (`change-sort-order`) の担当のため本 slice は触らない。

> domain/workflows/update-settings.md#update-settings より：「設定モーダルからの保存操作で Settings を更新する。`storage_dir` / `theme` のいずれか（または両方）の変更を扱う」
>
> domain/bounded-contexts.md#user-preferences より：本 BC は supporting subdomain。ただし「`storageDir` の変更は Note Capture の挙動を切り替えるため軽量に扱えない」ため、I-S4 に従い即時マイグレーションは行わず再起動を要求する設計。

`load-settings` slice が **bootstrap (read + mkdir)** を担うのに対し、本 slice は **write 経路** を担う。両 slice は同じ `Settings` Aggregate / `app_config_dir/settings.json` を share するが、起動シーケンス上は独立。

## 入出力 {#io}

### Input {#io-input}

> domain/workflows/update-settings.md#input より：

```rust
struct UpdateSettingsCommand {
  new_storage_dir: Option<PathBuf>,
  new_theme: Option<Theme>,
  // sort_preference は change-sort-order workflow が担当
}
```

依存（外部から注入される interface）:

- `SettingsRepository` — 現在の `Settings` の読み込み (`load`) と永続化 (`save`)
- `EventBus` — 発行する domain event の publish 先（同期 in-process、`domain-events.md#notes-sync-rationale`）

`config_path` 自体の解決は composition root の責務。本 slice は注入された repository / bus をそのまま使う。

### Output {#io-output}

戻り値: `Result<Settings, UpdateSettingsError>`

> domain/workflows/update-settings.md#output より：
> - `Settings`（更新後）
> - domain events (差分に応じて 0〜2 件): `StorageDirChanged`, `ThemeChanged`

- 成功時: 更新後の `Settings` を返す。差分があったフィールドに対応する event のみ `EventBus` に publish する（最大 2 件、順序は **storage_dir → theme**）
- 差分なし (両 `Option` が `None` または現在値と同一) の場合: 現在値の `Settings` を返し event は **発行しない**（C-US5）

### Errors {#io-errors}

> domain/workflows/update-settings.md#errors より：

- `InvalidPath { path: PathBuf, reason: PathError }` — `new_storage_dir` が絶対パス検証 (I-S1) に失敗
- `PersistError { path: PathBuf, cause: io::Error }` — `settings.json` 書き出し失敗

エラー時は **`Settings` を変更しない** + **event を発行しない**（C-US6）。InvalidPath は `new_storage_dir` の検証段階で発生するため persist 前に止まる。PersistError は `save` 失敗時に発生し、in-memory の `Settings` は呼出側で破棄される（Settings は immutable な `mut self` consume style）。

#### StorageDirChanged Payload {#io-storage-dir-changed}

> domain/domain-events.md#storage-dir-changed-payload より：

```rust
struct StorageDirChanged {
  old_dir: PathBuf,
  new_dir: PathBuf,
}
```

#### ThemeChanged Payload {#io-theme-changed}

> domain/domain-events.md#theme-changed-payload より：

```rust
struct ThemeChanged {
  new_theme: Theme,
}
```

## 不変条件 {#invariants}

slice 完了時に成立すべき条件。括弧内は domain での出典。

### Settings Aggregate 由来 {#invariants-settings-aggregate}

> domain/aggregates.md#settings-aggregate-invariants より引用：

- **I-S1**: 更新後の `storage_dir` は **絶対パス**。違反入力は `InvalidPath` で reject
- **I-S2**: 永続化先 `app_config_dir/settings.json` は `storage_dir` 配下にしない（循環参照回避）。判定方向は `config_path.starts_with(storage_dir)` を違反とみなす。**本 slice は新しい `storage_dir` を受理する前に I-S2 を再評価する**（C-US3）
- **I-S4**: `change_storage_dir` 操作は即時には Note の引っ越しを起こさない。本 slice は `Settings` の値更新 + event 発行のみで、Note migration は実施しない（再起動を促す UI 層が `StorageDirChanged` を購読する）

I-S3（デフォルト値）は本 slice の範囲外（read 経路は `load-settings` slice）。

### slice 固有制約 {#invariants-slice-specific}

- **C-US1**: 入力 `UpdateSettingsCommand` の両 field (`new_storage_dir`, `new_theme`) が `None` の場合は **no-op**（永続化も event 発行もせず現在値を返す）
- **C-US2**: 個別フィールドの新値が現在値と等しい場合、そのフィールドは「変更なし」として扱い対応する event を発行しない（差分検出は `SettingsDiff` レベルで行う、`workflows/update-settings.md#steps` の `applyChanges`）
- **C-US3**: `new_storage_dir` を受理する前に I-S1 (絶対パス) と I-S2 (`config_path` を子孫として含まない) を検証する。違反時は `InvalidPath` を返し、`Settings` も `theme` も更新しない（partial update しない）
- **C-US4**: `persist` 失敗時は `PersistError` を返し、`Settings` と event publish のいずれも行わない（write-then-emit の順序、`workflows/update-settings.md#steps`）
- **C-US5**: 両フィールド変更を含む command でも、差分のあるフィールドの event のみが発行される（0 / 1 / 2 件のいずれか）。順序は **`StorageDirChanged` → `ThemeChanged`**（複数 event の順次発行は workflow notes より）
- **C-US6**: エラー時は `Settings` を一切変更しない + event を発行しない（atomic な commit semantics）
- **C-US7**: 本 slice は **storage_dir 変更時に Note migration を行わない**（I-S4）。`StorageDirChanged` の購読者である UI 層が再起動モーダルを表示し、Infrastructure 層（ファイルウォッチャー）が監視対象ディレクトリを `new_dir` に切り替える（旧ディレクトリの監視は停止）。Note Capture / Note Feed は再起動まで旧 `storage_dir` を見続ける（S11 の Then 節、domain/domain-events.md#storage-dir-changed-subscribers より）
- **C-US8**: 本 slice は **冪等ではない**（command 発行ごとに persist と event が走る可能性がある）。ただし C-US2 により「同じ command を 2 回送る」場合は 2 回目以降は no-op として扱われる

## テスト観点 {#test-perspectives}

phase 3 で failing test に展開する観点を列挙。各観点は 1 つ以上の test に対応する想定。

### happy path: 単独フィールド更新 {#tp-happy}

- **TP-H1**: `UpdateSettingsCommand { new_storage_dir: Some("/new/abs"), new_theme: None }` で現在値の `storage_dir` を更新
  - 戻り値の `Settings.storage_dir == "/new/abs"`
  - `SettingsRepository::save` が 1 回呼ばれる（C-US4）
  - `StorageDirChanged { old_dir: "/old", new_dir: "/new/abs" }` が **1 件** publish される
  - `ThemeChanged` は publish **されない**
- **TP-H2**: `UpdateSettingsCommand { new_storage_dir: None, new_theme: Some(Dark) }` で `theme` を更新
  - 戻り値の `Settings.theme == Dark`
  - `ThemeChanged { new_theme: Dark }` が **1 件** publish される
  - `StorageDirChanged` は publish **されない**

### happy path: 両フィールド同時更新 {#tp-both}

- **TP-B1**: `UpdateSettingsCommand { new_storage_dir: Some("/new"), new_theme: Some(Light) }` の両方変更
  - 戻り値の `Settings` 両フィールドが反映
  - `save` が 1 回（差分まとめて 1 回の write）
  - event は **2 件**、順序は `StorageDirChanged → ThemeChanged`（C-US5）

### no-op: 差分なし {#tp-noop}

- **TP-N1**: 両 field が `None` → 現在値 `Settings` をそのまま返す、`save` 呼ばれない、event 0 件（C-US1）
- **TP-N2**: `new_storage_dir == current.storage_dir` の場合、そのフィールド分の event は発行されない（C-US2）。`new_theme` も同様にチェック
- **TP-N3**: 両 field が同値 (no-op 等価) の場合、`save` 呼ばれない・event 0 件

### S11 シナリオ: storage_dir 変更は再起動要求のみ {#tp-s11}

> domain/validation.md#s11-storage-dir-change を walkthrough：

- **TP-S11-1**: Given `Settings { storage_dir: "/old/path" }`、When `UpdateSettingsCommand { new_storage_dir: Some("/new/path") }`、Then
  - `change_storage_dir` 相当の更新が走り `Settings.storage_dir == "/new/path"`
  - `settings.json` への persist が成功
  - `StorageDirChanged { old_dir: "/old/path", new_dir: "/new/path" }` が発行
- **TP-S11-2**: 本 slice は **Note migration を呼ばない**（I-S4 / C-US7）。NoteRepository に対する write call が **発生しない** ことを mock で assert
- **TP-S11-3**: 再起動モーダル表示は UI 層の責務（本 slice ではなく `StorageDirChanged` 購読者の関心事）。本 slice は event を発行するだけで UI を直接触らない

### エラー: InvalidPath {#tp-invalid-path}

- **TP-E1**: `new_storage_dir: Some("relative/path")` (絶対パスでない) → `InvalidPath` を返す（I-S1, C-US3）
  - `save` 呼ばれない、event 0 件
  - 元の `Settings` は変更されない（C-US6）
- **TP-E2**: `new_storage_dir` が `config_path` を子孫として含む (`storage_dir.starts_with(config_path.parent())` 違反 = I-S2) → `InvalidPath` を返す
  - **判定方向**: `config_path.starts_with(new_storage_dir)` が true なら違反
- **TP-E3**: 同じ command に `new_theme: Some(Dark)` も含まれていた場合でも、storage_dir の検証失敗が先行して reject される（partial update しない、C-US3）

### エラー: PersistError {#tp-persist-error}

- **TP-E4**: `SettingsRepository::save` が `io::Error` を返す → `PersistError` を返す
  - event 0 件（C-US4: write-then-emit）
  - 戻り値は `Err`、in-memory の更新済み `Settings` は呼出側に渡らない
- **TP-E5**: persist 失敗時、`EventBus::publish` が **一度も呼ばれない**（mock で記録）

### 不変条件チェック {#tp-invariants}

- **TP-I1**: 任意の成功 path で `result.storage_dir.is_absolute()` が真（I-S1）
- **TP-I2**: 任意の成功 path で `config_path.starts_with(result.storage_dir) == false`（I-S2）
- **TP-I3**: 任意の成功 path で **Note の物理移動が発生しない**（I-S4 / C-US7、NoteRepository への呼出が 0 回）

### event 発行順序 {#tp-event-order}

- **TP-O1**: 両フィールド変更時、`EventBus::publish` に対する呼出順が `StorageDirChanged` → `ThemeChanged` の順序であることを mock で assert（C-US5）

## 実装ノート {#impl-notes}

### アーキ層への落とし込み {#impl-layers}

DDD-VSA-Hex / typescript-tauri の階層に従い、本 slice は Rust 側で実装する（`implementation.language: rust`）。

```
apps/promptnotes/src-tauri/src/user_preferences/
├── shared/
│   ├── types/             # Settings / StorageDir / Theme (load-settings 同居)
│   └── ports.rs           # SettingsRepository, EventBus (load-settings と shared)
└── slices/
    ├── update_settings/
    │   ├── mod.rs         # pub use commands::*
    │   ├── domain.rs      # UpdateSettingsCommand, SettingsDiff, UpdateSettingsError
    │   ├── application.rs # UpdateSettingsUseCase: load → validate → apply → persist → emit
    │   ├── infrastructure.rs # （load-settings の SettingsRepository 実装を re-use）
    │   ├── commands.rs    # #[tauri::command] update_settings → tauri-specta surface (本 slice の scope 外でも可)
    │   └── tests.rs       # in-memory SettingsRepository + EventBus mock で TP-* 群を網羅
```

### 依存 interface {#impl-deps}

- `SettingsRepository::load(&self) -> Settings`
- `SettingsRepository::save(&self, settings: &Settings) -> Result<(), io::Error>`
- `EventBus::publish(&self, event: SettingsEvent)` — `SettingsEvent` は `StorageDirChanged | ThemeChanged` の sum type
- `config_path: &Path` — I-S2 判定用に composition root から渡される（または `SettingsRepository` の field として保持）

`load-settings` slice と `SettingsRepository` を共有する。`EventBus` は本 slice で初登場するため shared/ports.rs に追加する。

### Pipeline ステップ {#impl-pipeline}

> domain/workflows/update-settings.md#steps の DMMF pipeline を採用：

1. `loadCurrent: () → Settings` — `SettingsRepository::load`
2. `validateStorageDir: Option<PathBuf> → Result<Option<StorageDir>, InvalidPath>` — `None` は spread、`Some` は `StorageDir::try_from` + I-S2 再評価
3. `applyChanges: (Settings, Option<StorageDir>, Option<Theme>) → (Settings, SettingsDiff)` — 差分検出 (`storage_dir_changed: bool`, `theme_changed: bool`)
4. `persist: Settings → Result<(), PersistError>` — diff が全 false の場合 (no-op) は **persist 呼出をスキップ** する (C-US1 / C-US2)
5. `emitConditional: SettingsDiff → Vec<SettingsEvent>` — true の field 分だけ event を生成し、`StorageDirChanged` → `ThemeChanged` の順で publish

ステップ 2 までは副作用なし。ステップ 4 で I/O、ステップ 5 で event bus に副作用。

### no-op skip の根拠 {#impl-noop}

C-US1 / C-US2 を満たすには step 3 の `SettingsDiff` が `{ false, false }` の場合に step 4 / 5 を skip する。これは domain の「差分なし（変更指示が現在値と同一）の場合は event 非発行」（`workflows/update-settings.md#notes`）と一致。

### Out of scope {#out-of-scope}

本 slice は **command (write side) + event 発行** のみを扱う。以下は別 slice / layer の責務：

- `sort_preference` の更新 (`change-sort-order` slice、未着手)
- Settings の **読み込み**（`load-settings` slice）
- `StorageDirChanged` 購読側の **再起動モーダル表示** (UI 層)
- Note の物理マイグレーション（domain として「やらない」と決定済み、I-S4）
- `app_config_dir/settings.json` の path 解決（composition root）

## Open Questions {#open-questions}

Phase 1 (derive) 時点で未解決事項はない。phase 3 (test-red) で TP-* を test 化する際、`SettingsDiff` の表現や `SettingsEvent` sum type の具体形は phase 4 (impl-green) で確定する。
