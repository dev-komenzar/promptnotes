---
coherence:
  source: derived
  last_derived: 2026-06-26
  upstream:
    - domain/workflows/change-sort-order.md#change-sort-order
    - domain/aggregates.md#note-feed-aggregate
    - domain/aggregates.md#settings-aggregate
    - domain/aggregates.md#notes-sort-side-effect
    - domain/domain-events.md#sort-preference-changed
  hash:
    domain/workflows/change-sort-order.md#.*: 137e642d209c
    domain/aggregates.md#.*: 9f9048f5816b
    domain/domain-events.md#.*: 8abdfac78084
ori:
  schema:
    propagation_level: file
---

# change-sort-order spec {#change-sort-order-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive change-sort-order`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

ツールバーのソートトグル（または設定モーダル）から呼び出され、`NoteFeed.sort` と `Settings.sort_preference` を **1 トランザクションで更新** する command slice。13 workflow 中で **NoteFeed BC と User Preferences BC を同時に touch する唯一の slice** であり、aggregates.md `#notes-sort-side-effect` で警告された「Customer-Supplier の唯一の逆流」（NoteFeed → Settings 書き込み）の実装責任を持つ。

> domain/workflows/change-sort-order.md より：「ツールバーのソートトグルで NoteFeed の sort を変更し、同時に Settings に永続化する。aggregates.md で警告した『NoteFeed → Settings の唯一の逆流』」
>
> domain/aggregates.md#notes-sort-side-effect より：「NoteFeed::change_sort は Settings.sort_preference を更新する副作用を持つ。これは Customer-Supplier の唯一の逆流（NoteFeed → Settings の書き込み）。application service 層で『NoteFeed の状態変更』と『Settings の永続化』を 1 トランザクションで扱う」

### 既存資産の再利用 {#overview-reuse}

- `Settings` aggregate / `SortOrder` VO / `SettingsRepository` / `EventBus` は `update-settings` slice (PR #9 main 到達済) で実装済
- `NoteFeed` aggregate / `FeedFilter` 等は `update-feed-filter` slice (PR #11 main 到達済) で実装済（ただし `sort` field は notes.md で明示的に **drop** されている）
- **本 slice で新規追加が必要**: `NoteFeed.sort: SortOrder` field、`NoteFeed::change_sort(self, SortOrder) -> NoteFeed`、`Settings::change_sort_preference(self, SortOrder) -> Settings`、`SettingsEvent::SortPreferenceChanged { new_sort }` variant
- **PersistError の再利用**: `UpdateSettingsError::PersistError { path, cause }` を slice 跨ぎで共用（手動依存追加。新規 PersistError を作らない）

### NoteFeed BC への sort field 復活 {#overview-noterev}

`update-feed-filter` slice の notes.md は `sort` / `source` を「後続 slice 着手まで凍結」と decision していた。本 slice がその後続にあたるため、`NoteFeed { filter, sort }` 構造へ拡張する。`source` は依然 out of scope（`list-feed` slice で確定）。

## 入出力 {#io}

### Input {#io-input}

> domain/workflows/change-sort-order.md#input より：

```rust
struct ChangeSortOrderCommand {
  new_sort: SortOrder,    // { field: createdAt|updatedAt, direction: asc|desc }
}
```

`SortOrder` は `user_preferences::shared::types::SortOrder` を借りる（Customer-Supplier 規約。`update-feed-filter` slice の follow-up `ori-64x.9` で「shared か独立か」が議論されていたが、本 slice は「shared user_preferences::SortOrder を直接使う」方針を採用する。理由は aggregates.md で SortOrder が両 BC 共通 VO として宣言されており、「1 つの型」を守る方が事故が少ないため）。

依存（外部から注入される port）:

- `SettingsRepository` — `update-settings` slice の `user_preferences::shared::ports::SettingsRepository` を再利用
- `EventBus` — 同上、`user_preferences::shared::ports::EventBus`（`SettingsEvent` を publish する）

### Output {#io-output}

戻り値: `Result<NoteFeed, ChangeSortOrderError>`

> domain/workflows/change-sort-order.md#output より：
> - `NoteFeed`（sort 更新後）
> - domain event: `SortPreferenceChanged`

- 成功時: sort 更新後の `NoteFeed` を返し、`SettingsEvent::SortPreferenceChanged { new_sort }` を 1 件 publish
- 差分なし（new_sort == current.sort_preference）: 現在の `NoteFeed` を返し、persist / publish **共に skip**（C-CSO1 冪等性）
- persist 失敗: `Err(ChangeSortOrderError::PersistError)`、NoteFeed は in-memory では更新済だが呼出側で破棄される

### Errors {#io-errors}

> domain/workflows/change-sort-order.md#errors より：

```rust
enum ChangeSortOrderError {
  PersistError { path: PathBuf, cause: io::Error },
}
```

**実装は `UpdateSettingsError::PersistError` を直接再利用する**（型エイリアス or re-export）。新しい PersistError variant を定義しない。これにより `update-settings` と本 slice で「Settings 永続化失敗」のエラー型が **1 つに統一** され、UI 層が両 slice からの error を同一 handler で扱える。

### SortPreferenceChanged Payload {#io-sort-preference-changed}

> domain/domain-events.md#sort-preference-changed-payload より：

```rust
struct SortPreferenceChanged {
  new_sort: SortOrder,
}
```

`SettingsEvent::SortPreferenceChanged { new_sort: SortOrder }` を新規 variant として追加する。

## 不変条件 {#invariants}

slice 完了時に成立すべき条件。括弧内は domain での出典。

### NoteFeed / Settings Aggregate 由来 {#invariants-aggregates}

- **I-F2 / I-F3 (NoteFeed)**: 本 slice の責務範囲外（`visible_notes` / sort 適用ロジックは未実装。本 slice は **`sort` field の保持と書換え** のみ）
- **I-S1 / I-S2 (Settings)**: `storage_dir` は触らないため自動的に保持される
- **新規**: `NoteFeed.sort == Settings.sort_preference` を **本 slice 経由のみ** で同期する（spec 内不変条件、`#notes-sort-side-effect` を impl level に落としたもの）

### slice 固有制約 {#invariants-slice-specific}

- **C-CSO1**: `new_sort == current_settings.sort_preference` の場合は **no-op**（persist 呼ばれない、event 発行されない、NoteFeed は同値を返す）。冪等性 (`workflows/change-sort-order.md#notes`)
- **C-CSO2**: 成功時 `NoteFeed.sort` と `Settings.sort_preference` が **同一値** になる（atomic transaction、`#notes-sort-side-effect`）
- **C-CSO3**: persist 失敗時、`SortPreferenceChanged` は **発行されない**（write-then-emit、update-settings の C-US4 と同方針）
- **C-CSO4**: 成功時、`SortPreferenceChanged { new_sort }` を **1 件だけ** publish する（複数発行しない）
- **C-CSO5**: NoteFeed の `filter` field は本 slice では **touch しない**（C-UF5 直交性の外延：sort 軸変更は filter 軸に影響しない）
- **C-CSO6**: PersistError は `UpdateSettingsError::PersistError` を再利用する（型 alias or `pub use` re-export）。slice ローカルな新 enum variant を作らない
- **C-CSO7**: pipeline 順序: `load Settings → diff check → apply to NoteFeed → apply to Settings → persist → emit`。**persist 前に NoteFeed in-memory 更新を完了させてよい**（in-memory side effect は revert 可能、Settings persist 失敗時は呼出側で NoteFeed を破棄する想定）

## テスト観点 {#test-perspectives}

phase 3 で failing test に展開する観点。各観点は 1 つ以上の test に対応する想定。

### happy path: sort 更新 {#tp-happy}

- **TP-H1**: 現在 `{ field: CreatedAt, direction: Desc }`、`new_sort = { UpdatedAt, Asc }` → 戻り値 `NoteFeed.sort == { UpdatedAt, Asc }`
- **TP-H2**: TP-H1 で `SettingsRepository::save` が 1 回呼ばれ、保存内容の `Settings.sort_preference == { UpdatedAt, Asc }`
- **TP-H3**: TP-H1 で `SortPreferenceChanged { new_sort: { UpdatedAt, Asc } }` が 1 件 publish される
- **TP-H4**: 戻り値 `NoteFeed.filter` が変更前と同じ（C-CSO5 直交性）

### no-op (差分なし) {#tp-noop}

- **TP-N1**: 現在 `{ CreatedAt, Desc }`、`new_sort = { CreatedAt, Desc }` → save 呼ばれない、event 0 件 (C-CSO1)
- **TP-N2**: TP-N1 で戻り値の NoteFeed が **入力と同値**（sort 適用しても結果同じ）

### PersistError {#tp-persist-error}

- **TP-E1**: `SettingsRepository::save` が `io::Error` を返す → `Err(ChangeSortOrderError::PersistError)`
- **TP-E2**: TP-E1 で event 0 件（C-CSO3 write-then-emit）
- **TP-E3**: TP-E1 で in-memory の `SettingsRepository::current()` (mock) は元の値のまま（mock 実装が save 失敗時に内部 state を変更しない契約に依存。spec として「persist 失敗時は呼出側が NoteFeed を破棄する」を表現する間接的 test）
- **TP-E4**: TP-E1 の error 型が `UpdateSettingsError::PersistError` と **同一型**（C-CSO6、type-level assertion で fn pointer coercion）

### atomic transaction (NoteFeed ↔ Settings 同期) {#tp-atomic}

- **TP-A1**: 成功時 `result.sort == saved_settings.sort_preference`（C-CSO2）
- **TP-A2**: persist 後に発行された event の payload `new_sort` が saved Settings.sort_preference と等しい

### Customer-Supplier 逆流の type-level 表現 {#tp-cs-reverse}

- **TP-CS1**: `ChangeSortOrderUseCase::new` のシグネチャが `(SettingsRepository, EventBus, NoteFeed) -> Self` の形（NoteFeed を construction 時 inject）または `execute(NoteFeed, command) -> Result<NoteFeed, _>` のいずれかで、**NoteFeed と SettingsRepository を両方持つ** ことを type-level に固定する。これが「逆流が起きている唯一の場所」というドキュメンタリな保証になる
- **TP-CS2**: 戻り値の `NoteFeed.sort` 更新が `Settings.sort_preference` 更新と 1 対 1（mock で観測）

## 実装ノート {#impl-notes}

### アーキ層への落とし込み {#impl-layers}

本 slice は **Note Feed BC primary** で、User Preferences BC を cross-BC dependency として touch する。

```
apps/promptnotes/src-tauri/src/note_feed/
└── slices/
    └── change_sort_order/
        ├── mod.rs
        ├── domain.rs       # ChangeSortOrderCommand + ChangeSortOrderError (= UpdateSettingsError::PersistError re-export)
        ├── application.rs  # ChangeSortOrderUseCase: load → diff → apply both → persist → emit
        └── tests.rs        # TP-* を網羅
```

NoteFeed 側の修正:
- `note_feed/shared/types/note_feed.rs`: `sort: SortOrder` field 追加 + `change_sort(self, SortOrder) -> Self` method
- `note_feed/shared/types/mod.rs`: 必要なら re-export 調整

User Preferences 側の修正:
- `user_preferences/shared/types/settings.rs`: `change_sort_preference(self, SortOrder) -> Self` method 追加
- `user_preferences/shared/types/events.rs`: `SettingsEvent::SortPreferenceChanged { new_sort: SortOrder }` variant 追加

### Settings から SortOrder import の cross-BC 規約 {#impl-cross-bc-sort-order}

`note_feed::shared::types::NoteFeed` は `user_preferences::shared::types::SortOrder` を直接 import する。Note Feed BC は SortOrder を **構築せず保持するだけ** で、構築は `Settings` のみが行う（Customer-Supplier、Supplier = User Preferences）。`Tag` の Shared Kernel pattern と同じ判断。

`update-feed-filter` の follow-up `ori-64x.9`（「shared か独立か」）を本 slice で **shared 方針で確定**。

### Pipeline ステップ {#impl-pipeline}

> domain/workflows/change-sort-order.md#steps の DMMF pipeline を採用：

1. `load_settings: () → Settings` — `SettingsRepository::load`
2. `is_same_sort: (Settings, SortOrder) → bool` — 差分判定（C-CSO1）
   - true なら **早期 return**: `Ok(feed)` （no persist, no publish）
3. `apply_to_feed: (NoteFeed, SortOrder) → NoteFeed` — `NoteFeed::change_sort`
4. `apply_to_settings: (Settings, SortOrder) → Settings` — `Settings::change_sort_preference`
5. `persist: Settings → Result<(), PersistError>` — `SettingsRepository::save`
6. `emit: SortOrder → SortPreferenceChanged` — `EventBus::publish(SettingsEvent::SortPreferenceChanged { ... })`

step 5 が `Err` を返したら step 6 はスキップ（C-CSO3）。

### PersistError の再利用方法 {#impl-persist-error-reuse}

`note_feed::slices::change_sort_order::domain` に以下を書く:

```rust
pub use crate::user_preferences::slices::update_settings::domain::UpdateSettingsError as ChangeSortOrderError;
```

または `type ChangeSortOrderError = UpdateSettingsError;`。これにより:

- 本 slice の戻り型 `Result<NoteFeed, ChangeSortOrderError>` は実体として `Result<NoteFeed, UpdateSettingsError>`
- TP-E4 で `UpdateSettingsError::PersistError` variant に match できる
- 将来 UpdateSettingsError に新 variant が追加されると change-sort-order も影響を受けるが、それは domain semantics 上の正解（「Settings まわりのエラー」が両 slice で同じになる）

代替: `shared/types/persist_error.rs` に PersistError を抽出する大きな refactor も可能だが、本 slice の scope を超えるため follow-up とする。

### Out of scope {#out-of-scope}

本 slice は **sort 軸の更新 + 永続化 + event 発行** のみ。以下は別 slice / layer:

- `NoteFeed::visible_notes` の sort 適用ロジック（`list-feed` slice）
- `NoteFeed.source` の確定（`list-feed` slice）
- 設定モーダル経由の同じ workflow 再利用（UI 層 + 同じ use case の呼出経路）
- Tauri command surface (`#[tauri::command] change_sort_order`)（follow-up）

## Open Questions {#open-questions}

### oq-error-type-extraction {#oq-error-type-extraction}

- **問**: `UpdateSettingsError::PersistError` を再利用する pragmatism は理解できるが、`shared/types/persist_error.rs` に PersistError を抽出して両 slice が使う方が cleaner では
- **暫定方針**: 本 slice では `pub use` で済ませる。抽出 refactor は follow-up
- **解決方向**: `update-settings` と本 slice の error namespace が将来発散したくなったら抽出

### oq-cross-bc-import {#oq-cross-bc-import}

- **問**: `note_feed::slices::change_sort_order` が `user_preferences::slices::update_settings::domain` を直接 import するのは layer violation か（slice 間直接依存）
- **暫定方針**: 「PersistError を再利用するための pragmatic exception」として OK。長期的には shared error type が望ましい（[#oq-error-type-extraction](#oq-error-type-extraction)）
- **解決方向**: ori-conventions で「slice 間の error 型直接 import の方針」を確立する（cross-cutting concern）
