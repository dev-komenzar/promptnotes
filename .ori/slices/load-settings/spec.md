---
coherence:
  source: derived
  last_derived: 2026-06-24
  upstream:
    - domain/workflows/load-settings.md#load-settings
    - domain/aggregates.md#settings-aggregate
    - domain/bounded-contexts.md#user-preferences
    - domain/validation.md#s12-startup-state
  hash:
    domain/workflows/load-settings.md#.*: fe592ba3c569
    domain/aggregates.md#.*: 37fa7433eab4
    domain/bounded-contexts.md#.*: 4d579125a513
    domain/validation.md#.*: 5294b0c32f1b
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
- **I-S2**: 戻り値の `storage_dir` は `config_path.parent()` 配下に **ない**（Q6: 循環参照回避）。デフォルト fallback では OS 慣習により自動的にこの制約を満たす（macOS の場合 `Application Support/promptnotes/notes` vs `Application Support/promptnotes/settings.json` は同じ親だが、ファイル `settings.json` 自身は対象ではなく `config_path.parent() == storage_dir.parent()` でも I-S2 違反ではない。**チェック対象は `storage_dir` がディレクトリとして `config_path.parent()` を含まないこと**）
- **I-S3**: 不在時のデフォルトは以下：
  - `storage_dir`: OS 慣習パス（macOS `~/Library/Application Support/promptnotes/notes/`, Linux `~/.local/share/promptnotes/notes/`, Windows `%APPDATA%\promptnotes\notes\`）
  - `theme`: `System`
  - `sort_preference`: `{ field: createdAt, direction: desc }`

I-S4（`change_storage_dir` の再起動要求）は本 slice の範囲外（write 経路は `update-settings` slice）。

### slice 固有制約 {#invariants-slice-specific}

- **C-LS1**: 戻り値は常に有効な `Settings`。**`Result` / `Option` 型は API 表面に露出しない**（domain workflow output 契約より）
- **C-LS2**: settings.json **不在**時は I-S3 の全デフォルトを使う
- **C-LS3**: settings.json **parse 失敗**（JSON 構文エラー / 型不整合等）時は I-S3 の全デフォルトを使う（**フィールド単位の部分復元はしない**、保守的）
  > domain/workflows/load-settings.md#notes より：「部分的に壊れた `settings.json` の扱いは『全フィールドデフォルト』を採用（保守的）」
- **C-LS4**: settings.json が **valid JSON だが一部フィールド欠損**の場合は、**欠損フィールドのみ** I-S3 で補完（フィールド単位の merge）。これは C-LS3（全 parse 失敗）と区別される。**判定基準**: `serde_json` が JSON 構造として valid と判定し、`#[serde(default)]` 等で欠損フィールドを許容できるならフィールド単位 fallback、それ以外（top-level が array 等）は C-LS3 経由で全デフォルト
- **C-LS5**: `Settings.storage_dir` 解決後、**ディレクトリが存在しなければ作成**（`mkdir -p` 相当）。これは初回起動の前提条件
- **C-LS6**: `mkdir` 失敗時は **silent**（panic せず Settings をそのまま返す）。理由：load-settings は no-error 契約。実 I/O 失敗は Note Capture 側で `PersistError` として観測される
- **C-LS7**: 本 slice は domain event を **発行しない**（domain/workflows/load-settings.md#output より）
- **C-LS8**: 本 slice は冪等。同じ `config_path` + 同じファイル内容で何度呼んでも同じ `Settings` を返し、`mkdir` は 2 回目以降 no-op
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
- **TP-PT3**: `{ "theme": "Invalid" }` (enum 値不正) → **slice 設計判断**: 該当フィールドだけデフォルト fallback とする（C-LS4 の延長）か、全フィールドデフォルト（C-LS3）か。**現 spec では「該当フィールドだけデフォルト」を採用**（保守的だが UX 上「theme 以外は壊さない」が望ましい）。これは [#open-questions](#open-questions) で domain への確認候補
- **TP-PT4**: `{ "sort_preference": { "field": "updatedAt" } }` (nested 欠損) → `direction` だけ I-S3 デフォルト `desc` で補完

### storage_dir mkdir {#tp-mkdir}

- **TP-M1**: storage_dir が既存ディレクトリ → `ensure_dir` は呼ばれるが no-op (mock で記録)
- **TP-M2**: storage_dir が不在 → `ensure_dir` が成功し、mkdir が記録される（C-LS5）
- **TP-M3**: `ensure_dir` が `io::Error` を返す → **panic せず** `Settings` をそのまま返す（C-LS6、silent failure）
- **TP-M4**: TP-M3 の場合、戻り値 `Settings` は他 TP と同等の構造（mkdir 失敗は API 表面に出ない、no-error 契約）

### 不変条件チェック {#tp-invariants}

- **TP-I1**: 任意の入力に対して `Settings.storage_dir.is_absolute()` が真（I-S1）
- **TP-I2**: settings.json で `"storage_dir": "relative/path"` を渡したケース → **slice 設計判断**: 相対パスは I-S1 違反として該当フィールドだけ I-S3 デフォルト fallback とする（C-LS4 の延長、TP-PT3 と同じ方針）
- **TP-I3**: 任意の `config_path` に対して、戻り値の `Settings.storage_dir` が `config_path.parent()` 以下を **含まない** path であること（I-S2 の弱い形：path 関係チェック）
- **TP-I4**: 同じ `config_path` + 同じファイル内容で 2 回呼出 → 同じ `Settings` が返り、2 回目の `ensure_dir` は no-op（C-LS8、冪等）

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
- `FileSystem::ensure_dir(&Path) -> ()` — `mkdir -p` 相当。失敗時も silent（C-LS6）
- `OsDirs::default_storage_dir() -> StorageDir` — OS ごとの慣習パス取得

infrastructure 層では `std::fs` + `dirs` crate（または Tauri `app.path()`）でこれらを実装。

### Domain 内の VO 構築 {#impl-vos}

- `StorageDir::new(PathBuf) -> Result<StorageDir, InvalidPath>` — I-S1 (絶対パス) 検証。**ただし load-settings の context では Result を外に出さず、失敗時はデフォルト fallback**（C-LS1）
- `Theme::deserialize` — serde の derive で enum unit variant をそのまま受ける
- `SortOrder::deserialize` — `{ field, direction }` 構造体として nested deserialize

### Pipeline ステップ {#impl-pipeline}

> domain/workflows/load-settings.md#steps の DMMF pipeline をそのまま採用：

1. `tryRead: PathBuf → Option<String>`
2. `tryParse: Option<String> → Option<SettingsRaw>` — `SettingsRaw` は **`Option` で全フィールドを wrap した DTO**（serde で部分復元を可能にする）
3. `applyDefaults: Option<SettingsRaw> → Settings` — `None` または各 `Option<Field>` が `None` の箇所を I-S3 で補完
4. `ensureStorageDir: StorageDir → ()` — `FileSystem::ensure_dir` 呼出（C-LS5、失敗 silent）

ステップ 1-2 は副作用なし。ステップ 3 もメモリ内変換のみ。ステップ 4 で初めて I/O が走る。

### serde の戦略 {#impl-serde}

- `SettingsRaw { storage_dir: Option<PathBuf>, theme: Option<Theme>, sort_preference: Option<SortOrder> }` で受ける
- `#[serde(default)]` を全フィールドに付与し、欠損は `None` になる
- `theme` の enum 値不正は serde が error を返す → catch して該当フィールドだけ `None` 扱いにするには **field-level error handling** が必要。**現案**: `#[serde(deserialize_with = ...)]` で「parse 失敗 → None」変換を被せる
- top-level の JSON 構文エラー / 型不整合は `serde_json::from_str` が `Err` を返し、step 2 で `Option::None` に降格（C-LS3）

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

- **状況**: domain/workflows/load-settings.md#notes は「部分的に壊れた `settings.json` の扱いは『全フィールドデフォルト』を採用（保守的）」と明示する一方、`#notes` 末尾は「将来『フィールド単位でデフォルト補完』に変えてもよい」と含みを残す
- **slice 側決定（暫定）**: C-LS4 で「JSON 構造として valid なら欠損 / 不正フィールドは個別に I-S3 fallback」を採用（TP-PT3 / TP-I2）。理由：UX 上「theme 文字を typo しただけで storage_dir まで巻き戻る」のは避けたい
- **upstream への提案候補**: domain/workflows/load-settings.md#notes を更新し、フィールド単位 fallback を **TBD ではなく規定** に格上げ。proposal を `/ori-propose` で起票する候補
- **status**: 本 spec では暫定採用。phase 4 (impl-green) 着手前に user 確認 → 必要なら `/ori-propose` で domain 修正

### TP-AS1: no-error API 表面の型レベル保証 {#oq-no-result-typelevel}

- **状況**: domain workflow が「Errors: なし」と契約しているが、Rust で type-level に保証するには関数シグネチャから `Result` / panic を除く必要がある
- **暫定**: `LoadSettingsUseCase::execute(&self) -> Settings` のシグネチャをテストで `compile_fail` doctest 等で検査する案、または signature assertion test (`fn _assert_sig() { let _: fn(&_) -> Settings = LoadSettingsUseCase::execute; }`)
- **status**: phase 3 (test-red) で TP-AS1 をどう書き起こすか確定する。**TBD**
