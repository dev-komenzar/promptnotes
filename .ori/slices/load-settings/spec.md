---
coherence:
  source: derived
  last_derived: 2026-06-30
  hash:
    domain/workflows/load-settings.md#.*: 0c241b32210d
    domain/aggregates.md#.*: 82947dbfd3f6
    domain/bounded-contexts.md#.*: 7ebfcda8743b
    domain/validation.md#.*: 31244b277867
ori:
  schema:
    propagation_level: file
---

# load-settings spec {#load-settings-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive load-settings`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

アプリ起動時に `app_config_dir/settings.json` を読み込み、`Settings` Aggregate を **always-Ok** で復元する slice。User Preferences BC の唯一の bootstrap 経路で、settings.json が不在 / parse 失敗 / 欠損フィールドのいずれの場合も I-S3 のデフォルト値で fallback する（**no-error path**）。

> domain/workflows/load-settings.md#load-settings より：「アプリ起動時に `app_config_dir/settings.json` を読み込み、不在 / 不正時はデフォルト値で初期化する」
>
> domain/bounded-contexts.md#user-preferences-purpose より：本 BC は **supporting subdomain**。Note の lifecycle とは独立に変更され、Note とは別の永続化先（OS 慣習パス）に住む。
>
> domain/bounded-contexts.md#user-preferences-subdomain-type より：「`storageDir` の変更は Note Capture の挙動を切り替えるため軽量に扱えない」— 本 slice は bootstrap、変更は別 slice (`update-settings`) の責務。

restore された `Settings.sort_preference` は後段で NoteFeed 初期化に渡る（S12 シナリオ）が、本 slice の責務は **Settings の復元 + storage_dir の物理確保まで**。NoteFeed の filter リセット / sort 適用は本 slice の out of scope（[#out-of-scope](#out-of-scope)）。

## 入出力 {#io}

### Input {#io-input}

> domain/workflows/load-settings.md#input より：

```rust
struct LoadSettingsCommand {
  config_path: PathBuf,    // app_config_dir/settings.json
}
```

依存（外部から注入される interface）:

- `FileSystem` — `tryRead`（読み取り）と `ensureDir`（不在ディレクトリの作成）を提供
- `OsDirs` — OS 慣習のデフォルト `storage_dir` 取得（macOS / Linux / Windows）

`config_path` 自体の解決（Tauri `app.path().app_config_dir()` 等）は composition root の責務。本 slice は注入された `PathBuf` をそのまま使う。

### Output {#io-output}

戻り値: `Settings`（**no Result wrap**）

> domain/workflows/load-settings.md#output より：「`Settings` / domain event: なし / Errors: なし（不在 / parse 失敗はデフォルトに fallback）」

- 復元成功時: settings.json の内容 + 欠損フィールドにデフォルト補完した `Settings`
- 不在 / parse 失敗時: 全フィールドデフォルトの `Settings`
- 副作用: `Settings.storage_dir` が物理的に存在しない場合のみ `mkdir -p`（initial bootstrap）

### Errors {#io-errors}

**なし。** 本 slice は workflow 定義上「失敗しない」契約。下位 layer の I/O 失敗（read / mkdir）は以下のように扱う：

- **read 失敗**: `tryRead` が `None` を返す（IO error も「ファイル不在と同じ」扱い、保守的）
- **parse 失敗**: `tryParse` が `None` を返す
- **mkdir 失敗**: panic せず、`Settings.storage_dir` はそのまま返す。Note Capture 側の `NoteRepository::write` で初めて `PersistError` として観測される（C-LS6 参照）

これは workflow 定義「Errors: なし（不在 / parse 失敗はデフォルトに fallback）」と整合する。**実装は `Result` 型を露出しない**。

## 不変条件 {#invariants}

slice 完了時に成立すべき条件。括弧内は domain での出典。

### Settings Aggregate 由来 {#invariants-settings-aggregate}

> domain/aggregates.md#settings-aggregate-invariants より引用：

- **I-S1**: 戻り値の `storage_dir` は **絶対パス**（settings.json から読んだ場合も、デフォルト fallback の場合も）
- **I-S2**: 戻り値の `storage_dir` は `config_path` を子孫として含まない（Q6: 循環参照回避）。判定方向は `config_path.starts_with(storage_dir)` を違反とみなす ([#impl-i-s2-direction](#impl-i-s2-direction))。sibling layout (`Application Support/promptnotes/{settings.json, notes/}`) は許容。
  - **port-level 契約** (`aggregates.md#settings-aggregate-invariants` より): `OsDirs::default_storage_dir` は返す `StorageDir` が妥当な `config_path` に対し I-S2 を満たすことを契約として保証する。slice 側で `default_storage_dir()` の戻り値に対し I-S2 を defensive re-check **しない**
- **I-S3**: 不在時のデフォルトは以下：
  - `storage_dir`: OS 慣習パス（macOS `~/Library/Application Support/promptnotes/notes/`, Linux `~/.local/share/promptnotes/notes/`, Windows `%APPDATA%\promptnotes\notes\`）
  - `theme`: `System`
  - `sort_preference`: `{ field: createdAt, direction: desc }`

I-S4（`change_storage_dir` の再起動要求）は本 slice の範囲外（write 経路は `update-settings` slice）。

### slice 固有制約 {#invariants-slice-specific}

- **C-LS1**: 戻り値は常に有効な `Settings`。**`Result` / `Option` 型は API 表面に露出しない**（domain workflow output 契約より）
- **C-LS2**: settings.json **不在**時は I-S3 の全デフォルトを使う
- **C-LS3**: settings.json の top-level が **JSON Object でない** 場合（parse 失敗 / `null` / array / scalar）は「全フィールドが欠損した Object」とみなし、C-LS4 と同じ field-level fallback 経路を通って結果的に全フィールド I-S3 になる
  > domain/workflows/load-settings.md#notes より：「JSON が Object でない場合は『全フィールド欠損 Object』として同経路を通る (degenerate case)」
- **C-LS4**: settings.json が **valid JSON Object** だが一部フィールド欠損 / 不正な型の場合は、**該当フィールドのみ** I-S3 で補完（フィールド単位 fallback）。他フィールドは保持
  > domain/workflows/load-settings.md#notes より：「JSON 構造として valid なら、欠損 / 不正な型のフィールドのみ I-S3 で補完。UX 上 `theme` typo で `storage_dir` まで巻き戻すのは避ける」
- **C-LS5**: `Settings.storage_dir` 解決後、**ディレクトリが存在しなければ作成**（`mkdir -p` 相当）。これは初回起動の前提条件
- **C-LS6**: `mkdir` 失敗時は **silent**（panic せず Settings をそのまま返す）。理由：load-settings は no-error 契約。実 I/O 失敗は Note Capture 側で `PersistError` として観測される
- **C-LS7**: 本 slice は domain event を **発行しない**（domain/workflows/load-settings.md#output より）
- **C-LS8**: 本 slice は冪等。**ただし冪等性は `FileSystem::ensure_dir` (mkdir -p 相当) 側に委譲**する。use case は stateless で、`ensure_dir` は呼ばれるたびに実行される（2 回目以降は FS impl が no-op を保証）。同じ `config_path` + 同じファイル内容での 2 回呼び出しは同じ `Settings` を返す
  > domain/workflows/load-settings.md#notes より：「冪等性は `FileSystem::ensure_dir` の `mkdir -p` 相当契約に委譲する」
- **C-LS9**: NoteFeed 初期化（filter リセット / sort 適用）は本 slice の **out of scope**。S12 シナリオの「NoteFeed 初期化」項は呼び出し側（composition root）の責務

## テスト観点 {#test-perspectives}

phase 3 で failing test に展開する観点を列挙。各観点は 1 つ以上の test に対応する想定。

### happy path: 完全な settings.json {#tp-happy}

- **TP-H1**: `config_path` に valid な settings.json (`storage_dir`, `theme`, `sort_preference` 全フィールド) が存在 → 中身がそのまま `Settings` に反映される
  - 例: `{ "storage_dir": "/abs/path", "theme": "Dark", "sort_preference": { "field": "updatedAt", "direction": "asc" } }`
  - 戻り値 `Settings { storage_dir: "/abs/path", theme: Dark, sort_preference: { updatedAt, asc } }`
- **TP-H2**: `FileSystem::ensure_dir(storage_dir)` が 1 回呼ばれる（C-LS5）
- **TP-H3**: domain event は **発行されない**（C-LS7）

### settings.json 不在 {#tp-absent}

- **TP-A1**: `config_path` が不在 → I-S3 の全デフォルトが返る（C-LS2）
  - `Settings.storage_dir == OsDirs::default_storage_dir()`（mock で injected）
  - `Settings.theme == System`
  - `Settings.sort_preference == { createdAt, desc }`
- **TP-A2**: `FileSystem::try_read` が `None` を返したケースで `OsDirs::default_storage_dir` が呼ばれる（依存解決の経路確認）
- **TP-A3**: `FileSystem::ensure_dir(default_storage_dir)` が呼ばれる（初回起動の自動作成、C-LS5）

### settings.json parse 失敗 {#tp-parse-fail}

- **TP-P1**: `config_path` 内容が `"not a json {"` → I-S3 全デフォルトが返る（C-LS3）
- **TP-P2**: top-level が array (`"[]"`) → I-S3 全デフォルト（オブジェクトを期待する型と不整合、C-LS3）
- **TP-P3**: 内容が空文字列 → I-S3 全デフォルト（C-LS3）
- **TP-P4**: 内容が `"null"` (valid JSON だが Settings 構造ではない) → I-S3 全デフォルト（C-LS3）

### フィールド単位 fallback {#tp-partial}

- **TP-PT1**: `{ "theme": "Dark" }` のみ → `Settings.theme = Dark`、他は I-S3 デフォルト（C-LS4）
- **TP-PT2**: `{ "storage_dir": "/abs/x", "sort_preference": { "field": "updatedAt", "direction": "asc" } }` → 該当フィールドはそのまま、`theme` のみ I-S3 デフォルト
- **TP-PT3**: `{ "theme": "Invalid" }` (enum 値不正) → **`theme` のみ I-S3 デフォルト** (`System`)、他フィールドは保持。`domain/workflows/load-settings.md#notes` の field-level fallback 規定に従う（C-LS4）
- **TP-PT4**: `{ "sort_preference": { "field": "updatedAt" } }` (nested 欠損) → `direction` だけ I-S3 デフォルト `desc` で補完

### storage_dir mkdir {#tp-mkdir}

- **TP-M1**: storage_dir が既存ディレクトリ → `ensure_dir` は呼ばれるが no-op (mock で記録)
- **TP-M2**: storage_dir が不在 → `ensure_dir` が成功し、mkdir が記録される（C-LS5）
- **TP-M3**: `ensure_dir` が `io::Error` を返す → **panic せず** `Settings` をそのまま返す（C-LS6、silent failure）
- **TP-M4**: TP-M3 の場合、戻り値 `Settings` は他 TP と同等の構造（mkdir 失敗は API 表面に出ない、no-error 契約）

### 不変条件チェック {#tp-invariants}

- **TP-I1**: 任意の入力に対して `Settings.storage_dir.is_absolute()` が真（I-S1）
- **TP-I2**: settings.json で `"storage_dir": "relative/path"` を渡したケース → **`storage_dir` のみ** `OsDirs::default_storage_dir()` で fallback、他フィールドは保持（C-LS4、field-level fallback）
- **TP-I3**: 任意の `config_path` に対して、戻り値の `Settings.storage_dir` が `config_path.parent()` 以下を **含まない** path であること（I-S2 の弱い形：path 関係チェック）
- **TP-I4**: 同じ `config_path` + 同じファイル内容で 2 回呼出 → 同じ `Settings` が返る。**use case は stateless** なため `ensure_dir` は毎回呼ばれてよい（2 回目以降の no-op 保証は `FileSystem` impl の責務、C-LS8）

### no-error API 表面 {#tp-api-shape}

- **TP-AS1**: `LoadSettingsUseCase::execute` のシグネチャに `Result` / `Option` を持たない（型レベル test、compile-time assertion）。**panic-free は型レベルでは検証不能**のため本 TP のスコープ外。Tauri-boundary (`commands.rs`) は infrastructure 層で `env::temp_dir()` fallback により best-effort panic-free を維持する ([#impl-tauri](#impl-tauri))。
- **TP-AS2**: `tryRead` / `tryParse` / `ensure_dir` がそれぞれ任意の失敗を返しても、`LoadSettingsUseCase::execute` の戻り値は **常に有効な Settings**（C-LS1 の network test）

## 実装ノート {#impl-notes}

### アーキ層への落とし込み {#impl-layers}

DDD-VSA-Hex / typescript-tauri の階層に従い、本 slice は Rust 側で実装する（`implementation.language: rust`）。

```
apps/promptnotes/src-tauri/src/user_preferences/
├── mod.rs                  # pub use slices::*; shared::*
├── shared/
│   ├── mod.rs
│   ├── types/
│   │   ├── mod.rs
│   │   ├── settings.rs     # Settings Aggregate root
│   │   ├── storage_dir.rs  # StorageDir VO (I-S1 検証)
│   │   ├── theme.rs        # Theme enum
│   │   └── sort_order.rs   # SortOrder VO (NoteFeed と shared kernel)
│   └── ports.rs            # FileSystem / OsDirs trait
└── slices/
    ├── mod.rs
    └── load_settings/
        ├── mod.rs          # pub use commands::*
        ├── domain.rs       # LoadSettingsCommand
        ├── application.rs  # LoadSettingsUseCase: try_read → try_parse → apply_defaults → ensure_dir
        ├── infrastructure.rs  # FileSystem / OsDirs impl
        ├── commands.rs     # #[tauri::command] load_settings → tauri-specta surface
        └── tests.rs        # unit tests for TP-* (in-memory FileSystem mock)
```

### 依存 interface {#impl-deps}

- `FileSystem::try_read(&Path) -> Option<String>` — 読み取り失敗 / 不在は `None`
- `FileSystem::ensure_dir(&Path) -> io::Result<()>` — `mkdir -p` 相当 (**冪等**: 2 回目以降は no-op を返す)。slice 側は結果を握り潰す（C-LS6）が、impl の冪等保証 (C-LS8) はここに住む
- `OsDirs::default_storage_dir() -> StorageDir` — OS ごとの慣習パス取得。返す `StorageDir` は I-S1 (絶対パス) と **I-S2 (`config_path` を子孫として含まない)** の両方を契約として満たす責務を持つ (`aggregates.md#settings-aggregate-invariants`)

infrastructure 層では `std::fs` + `dirs` crate（または Tauri `app.path()`）でこれらを実装。

### Domain 内の VO 構築 {#impl-vos}

- `StorageDir::new(PathBuf) -> Result<StorageDir, InvalidPath>` — I-S1 (絶対パス) 検証。**ただし load-settings の context では Result を外に出さず、失敗時はデフォルト fallback**（C-LS1）
- `Theme::deserialize` — serde の derive で enum unit variant をそのまま受ける
- `SortOrder::deserialize` — `{ field, direction }` 構造体として nested deserialize

### Pipeline ステップ {#impl-pipeline}

> domain/workflows/load-settings.md#steps の DMMF pipeline を採用。impl では `SettingsRaw` DTO を介さず、`serde_json::Value` (具体的には `Map<String, Value>`) から直接 per-field decode する経路を取る (field-level fallback を素直に表現するため):

1. `try_read: PathBuf → Option<String>` — `FileSystem::try_read`
2. `parse_top_level_object: &str → Option<JsonObject>` — `serde_json::from_str::<Value>` を実行し、結果が `Value::Object(m)` の場合のみ `Some(m)` を返す。それ以外 (parse err / `null` / array / scalar) は `None`
3. `apply_defaults: Option<JsonObject> → Settings`:
   - 各フィールドについて `pick_or_default::<T>(obj, key)` を呼び、欠損 / 不正な型は `T::default()` (I-S3) で埋める
   - `Option<JsonObject>` が `None` の場合は全フィールド欠損として degenerate に処理 (C-LS3)
   - `storage_dir` は `pick_or_default` ではなく専用 `resolve_storage_dir` で扱う (I-S2 検査 + `StorageDir::try_from` smart constructor → 失敗時 `OsDirs::default_storage_dir()` fallback)
4. `ensure_storage_dir: StorageDir → ()` — `FileSystem::ensure_dir` 呼出 (C-LS5、Result は `let _` で破棄 → C-LS6)

ステップ 1-3 は副作用なし。ステップ 4 で初めて I/O が走る。

### serde の戦略 {#impl-serde}

- 中間 DTO (`SettingsRaw` 等) は **使わない**。`serde_json::Map<String, Value>` を field-level fallback の data carrier として直接扱う
- `pick_or_default::<T: DeserializeOwned + Default>(obj, key)`:
  - `obj.get(key)` が `None` / `null` / 型不整合のいずれでも `serde_json::from_value::<T>` が `Err` を返し、`.ok().unwrap_or_default()` で `T::default()` (I-S3) に降格
  - `Theme` / `SortField` / `SortDirection` の enum 値不正もこの経路で自動的にデフォルトに巻き戻る (C-LS4 を「for free」で実現)
- 入れ子オブジェクト (`sort_preference: { field, direction }`) は `Value::as_object` で sub-Map を取り出してから同じ pattern を適用 (`resolve_sort_preference`)
- top-level の JSON 構文エラー / 型不整合は `serde_json::from_str` が `Err` を返し、step 2 (`parse_top_level_object`) で `None` に降格 (C-LS3)

### Tauri command surface {#impl-tauri}

- `#[tauri::command] async fn load_settings(app: AppHandle) -> Settings`
- 引数の `config_path` は composition root で `app.path().app_config_dir()` から解決。失敗時は `env::temp_dir().join("promptnotes/settings.json")` に fallback（panic-free を best-effort で維持）
- `default_storage_dir` も同様: `app.path().app_data_dir()` → 失敗時は `env::temp_dir().join("promptnotes/notes")`。最終的な `StorageDir::try_from` は `env::temp_dir()` を sentinel として使う（OS 契約上絶対パス）
- 戻り値は `Settings`（Result wrap なし、C-LS1）
- tauri-specta で TS bindings 生成（将来実装）

### TP-I3 / I-S2 の方向 {#impl-i-s2-direction}

I-S2 (循環参照回避) のチェックは `config_path.starts_with(storage_dir)` 方向。すなわち：
- **違反**: `config_path` が `storage_dir` の子孫である（settings.json が storage_dir 内にネスト）
- **許容**: sibling layout（`Application Support/promptnotes/{settings.json, notes/}` のように同じ親）

`violates_i_s2` は `config_path.starts_with(storage_dir.as_path())` で判定する。違反時は I-S3 デフォルトに降格（C-LS4 と同経路）。

### Out of scope {#out-of-scope}

本 slice は **query (read side) + bootstrap mkdir** のみを扱う。以下は別 slice / layer の責務：

- Settings の **書き込み**（`update-settings` slice の責務、I-S4 に従い再起動要求）
- NoteFeed の初期化（filter リセット / sort 適用） — S12 シナリオの後段、composition root が `Settings.sort_preference` を NoteFeed bootstrap に渡す
- `settings.json` の **マイグレーション**（将来 schema 変更時の責務）
- `OsDirs` の crate 選択（infrastructure テストで固定）
- UpdateChannel の起動時チェック（S14 シナリオ、別 BC）

## Open Questions {#open-questions}

### TP-PT3 / TP-I2: enum 値不正 / 相対パスのフィールド単位 fallback {#oq-field-level-fallback}

- **status**: **RESOLVED (2026-06-25)**。`domain/workflows/load-settings.md#notes` を更新し、field-level fallback を「規定」として格上げ。「部分的に壊れた settings.json は フィールド単位で I-S3 fallback、JSON が Object でない場合は全フィールド欠損 degenerate case」が公式仕様
- **解決方法**: domain 修正 → C-LS3 / C-LS4 を本 spec で整理 (C-LS3 は C-LS4 の degenerate case として再定義)、TP-PT3 / TP-I2 を「該当フィールドのみ I-S3 fallback」に確定
- **影響**: tp_pt3_* / tp_i2_* / tp_i3_* 系の test 群はそのまま domain と consistent

### TP-AS1: no-error API 表面の型レベル保証 {#oq-no-result-typelevel}

- **status**: **RESOLVED (2026-06-25, Pass 2 で部分対応 + 本 triage で確定)**。scope を **use case 層のみ** と確定。Tauri-boundary (`commands.rs`) の panic-free は infrastructure 層で `env::temp_dir()` fallback を採用する best-effort で別管理 ([#impl-tauri](#impl-tauri))
- **解決方法**: `LoadSettingsUseCase::execute(&self, _) -> Settings` のシグネチャを fn-pointer bind で type-level pin (TP-AS1)。「panic-free」は型レベル検証不能のため TP-AS1 のスコープ外と spec.md#tp-api-shape に明記済
- **影響**: 既存 test 構造に変更なし
