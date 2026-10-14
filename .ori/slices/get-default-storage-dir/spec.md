---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  last_derived: 2026-10-14
  derives_from:
    - domain/workflows/get-default-storage-dir.md#get-default-storage-dir
    - domain/aggregates.md#settings-aggregate
    - domain/bounded-contexts.md#user-preferences
  hash:
    domain/workflows/get-default-storage-dir.md#.*: 84a2bc66ea78
    domain/aggregates.md#.*: c2eaa083eaf0
    domain/bounded-contexts.md#.*: 957e7f4a4930
---

# get-default-storage-dir — Specification {#get-default-storage-dir-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive get-default-storage-dir`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

設定モーダル（widget-settings-modal）の「デフォルトに戻す」のために、OS 慣習の
**default `storage_dir`** を返す読み取り専用 query slice。User Preferences BC に属する。

> domain/workflows/get-default-storage-dir.md#get-default-storage-dir より:
> 設定モーダルの「デフォルトに戻す」のために、OS 慣習に基づく default `storage_dir`（I-S3）を返す
> 読み取り専用 query。`settings.json` は読まない / 書かない。

## 入出力 {#io}

### Input {#io-input}

```rust
struct GetDefaultStorageDirQuery;    // 入力なし
```

### Output {#io-output}

- Rust: `StorageDir`（`#[serde(transparent)]` で絶対パス文字列として serialize）
- Tauri command 戻り値: `StorageDir`（`Result` で包まない）
- TS 側: `Promise<string>`
- domain event: なし

### Errors {#io-errors}

> domain/workflows/get-default-storage-dir.md#errors より:
> なし（OS dir の解決に失敗した場合は load-settings と同じ fallback で一時ディレクトリ配下を返す）

## 不変条件 {#invariants}

### Settings Aggregate 由来 {#invariants-settings-aggregate}

> domain/aggregates.md#settings-aggregate-invariants より:
> **I-S1**: `storage_dir` は絶対パス / **I-S3**: 不在時のデフォルト `storage_dir` は OS 慣習パス

- 戻り値は `StorageDir` VO のため I-S1 を型で満たす
- I-S2 は `OsDirs` port の契約で満たす（本 slice は defensive re-check しない）

### slice 固有制約 {#invariants-slice-specific}

- **C-GDSD1**: 戻り型に `Result` を露出しない（Errors なし）
- **C-GDSD2**: 副作用なし。`settings.json` の読み書き・`ensure_dir`・event publish を行わない
- **C-GDSD3**: load-settings が不在時に使う default と **同じ情報源** を使う
  （composition root で共通の `user_preferences::shared::default_storage_dir::resolve_default_storage_dir` を呼ぶ）

## 境界契約 {#boundary-contract}

- boundary kind: `tauri_command`
- Rust 側 contact point:
  `apps/promptnotes/src-tauri/src/user_preferences/slices/get_default_storage_dir/commands.rs`
  - `#[tauri::command] pub fn get_default_storage_dir<R: Runtime>(app: AppHandle<R>) -> StorageDir`
  - `apps/promptnotes/src-tauri/src/lib.rs` の `invoke_handler!` に配線する
- TS 側 contact point:
  `apps/promptnotes/src/lib/user-preferences/slices/get-default-storage-dir/index.ts`
  - `getDefaultStorageDir(): Promise<string>` — `invoke('get_default_storage_dir')` の薄いラッパー
- 禁止される import 経路:
  - UI が `@tauri-apps/api/path` の `appDataDir()` 等で default を独自に計算すること（情報源の二重化は C-GDSD3 違反）

## テスト観点 {#test-points}

### OS default を返す {#tp-returns-os-default}

- **TP-D1**: use case は `OsDirs::default_storage_dir` の値をそのまま返す
- **TP-D2**: 1 回の execute で port を 1 回だけ参照し、繰り返し呼んでも同じ値（C-GDSD2）

### 共通 resolve 関数 {#tp-shared-resolve}

- **TP-R1**: `app_data_dir` が解決できれば `<app_data_dir>/notes`
- **TP-R2**: 解決失敗時は `<temp_dir>/promptnotes/notes`
- **TP-R3**: 相対パスが来ても `temp_dir` に fallback し、panic しない（I-S1）

### boundary test {#tp-boundary}

- **TP-B1**: TS — `getDefaultStorageDir()` が `invoke('get_default_storage_dir')` を引数なしで 1 回呼び、
  戻り値をそのまま resolve する（invoke は adapter 境界のため mock 可）

### 型レベル {#tp-type-level}

- **TP-T1**: command のシグネチャが `fn(AppHandle<R>) -> StorageDir`（`Result` を含まない、C-GDSD1）

## 実装ノート {#impl-notes}

### アーキ層への落とし込み {#impl-layers}

```
apps/promptnotes/src-tauri/src/user_preferences/
├── shared/default_storage_dir.rs   # resolve_default_storage_dir (各 command で共有) + TP-R*
└── slices/get_default_storage_dir/
    ├── mod.rs
    ├── domain.rs                   # GetDefaultStorageDirQuery
    ├── application.rs              # GetDefaultStorageDirUseCase<O: OsDirs>
    ├── commands.rs                 # #[tauri::command] get_default_storage_dir
    └── tests.rs                    # TP-D* / TP-T1

apps/promptnotes/src/lib/user-preferences/slices/get-default-storage-dir/
├── index.ts                        # getDefaultStorageDir(): Promise<string>
└── tests/get-default-storage-dir.test.ts   # TP-B1
```

- `OsDirs` port と `FixedOsDirs` は load-settings と共有する（新しい port は作らない）
- `resolve_default_storage_dir` はこれまで 5 つの command module に同じ実装が重複していたため、
  `user_preferences/shared/` に集約し、各 command はそれを呼ぶ（振る舞いは変えない）
- tauri-specta 未採用のため bindings 再生成は無い（DoD rule 4 対象外）
