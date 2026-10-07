---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  last_derived: 2026-10-07
  derives_from:
    - domain/workflows/get-app-version.md#get-app-version
    - domain/aggregates.md#update-channel-aggregate
    - domain/bounded-contexts.md#update-distribution
  hash:
    domain/workflows/get-app-version.md#.*: c3a5c313f0f5
    domain/aggregates.md#.*: c2eaa083eaf0
    domain/bounded-contexts.md#.*: 957e7f4a4930
---

# get-app-version — Specification {#get-app-version-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive get-app-version`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

設定モーダル（widget-settings-modal）に表示するための、**現在のアプリバージョン** を返す
読み取り専用 query slice。Update Distribution BC に属する 2 本目の slice。

> domain/workflows/get-app-version.md#get-app-version より：
> 設定モーダル表示時に、ビルド時に埋め込まれた現在のアプリバージョン
> （`UpdateChannel.current_version`, I-U1）を返す読み取り専用 query。
> ネットワークアクセスは行わない。

> domain/bounded-contexts.md#update-distribution より：
> PromptNotes 側のロジックは「起動時に確認する」「通知する」「現在のバージョンを提示する」のみ。

## 入出力 {#io}

### Input {#io-input}

> domain/workflows/get-app-version.md#input より：

```rust
struct GetAppVersionQuery;    // 入力なし（ビルド時定数を読むだけ）
```

ビルド時定数の情報源は `check-for-updates` と同一（`env!("CARGO_PKG_VERSION")`）。
composition root（`commands.rs`）で読み、domain の純粋関数へ raw 文字列として渡す。

### Output {#io-output}

> domain/workflows/get-app-version.md#output より：
> - `AppVersion` — 表示用のバージョン文字列
>   - 通常: `Version` の Display（例: `0.2.2`）
>   - fallback: ビルド時定数が `Version` VO として parse できない場合は **raw 文字列をそのまま** 返す
> - domain event: **なし**

- Rust domain: `AppVersion`（`String` の newtype、表示用 DTO）
- Tauri command 戻り値: `String`（`AppVersion` の中身。`Result` で包まない）
- TS 側: `Promise<string>`

### Errors {#io-errors}

> domain/workflows/get-app-version.md#errors より：
> なし（parse 失敗は raw 文字列 fallback で吸収する。表示専用のため reject しない）

## 不変条件 {#invariants}

### UpdateChannel Aggregate 由来 {#invariants-update-channel-aggregate}

> domain/aggregates.md#update-channel-aggregate-invariants より：
> **I-U1**: `current_version` は immutable（ビルド時定数）

- 本 slice は I-U1 の定数を **読むだけ** で、書き換え・キャッシュ無効化等は行わない
- I-U2 / I-U3 は本 slice の対象外（GitHub Releases に問い合わせない）

### slice 固有制約 {#invariants-slice-specific}

- **C-GAV1**: 戻り型に `Result` を露出しない（Errors なし）。domain 関数・command ともに `AppVersion` / `String` を直接返す
- **C-GAV2**: raw 文字列が `Version::from_str` で parse 可能なら、戻り値は `Version` の Display と一致する（正規化）
- **C-GAV3**: parse 不能（pre-release `0.3.0-rc.1` / build metadata `+sha` / 空文字 等）なら、戻り値は raw 文字列と **完全一致**（fallback）
- **C-GAV4**: 副作用なし。ネットワーク I/O・fs・event publish を一切行わない（domain event なし）
- **C-GAV5**: 同一ビルドで `check-for-updates` が返す `current_version` と **同じ値** を返す（情報源の一致、workflow Notes）
- **C-GAV6**: 純粋関数であり、同じ入力に対して常に同じ出力（冪等）

## 境界契約 {#boundary-contract}

- boundary kind: `tauri_command`
- Rust 側 source / contact point:
  `apps/promptnotes/src-tauri/src/update_distribution/slices/get_app_version/commands.rs`
  - `#[tauri::command] pub fn get_app_version() -> String`
  - `apps/promptnotes/src-tauri/src/lib.rs` の `invoke_handler!` に配線する
- TS 側 contact point:
  `apps/promptnotes/src/lib/update-distribution/slices/get-app-version/index.ts`
  - `getAppVersion(): Promise<string>` — `invoke('get_app_version')` の薄いラッパー
  - 本プロジェクトは tauri-specta を採用していないため、既存 slice（`load-settings` 等）と同じく手書き invoke wrapper を contact point とする
- public_entry:
  - Rust: `update_distribution/slices/get_app_version/mod.rs`（`pub use commands::get_app_version`）
  - TS: 上記 `index.ts`
- 禁止される import 経路:
  - UI（widget-settings-modal）が `domain.rs` / `application.rs` を経由せずに `CARGO_PKG_VERSION` 相当を独自に取得すること（`@tauri-apps/api/app` の `getVersion()` 直呼び等）。情報源の二重化は C-GAV5 違反
  - boundary test が `application` 層を直 import して command を迂回すること

## テスト観点 {#test-points}

### 正規化 / fallback（domain 純粋関数）{#tp-normalize}

- **TP-N1**: raw `"0.2.2"` → `AppVersion("0.2.2")`（C-GAV2）
- **TP-N2**: raw `"1.10.0"` → `AppVersion("1.10.0")`（多桁）
- **TP-F1**: raw `"0.3.0-rc.1"` → `AppVersion("0.3.0-rc.1")`（pre-release fallback、C-GAV3）
- **TP-F2**: raw `"0.3.0+sha.abc"` → raw のまま（build metadata fallback）
- **TP-F3**: raw `""` → `""`（空文字でも panic しない）
- **TP-P1** (proptest): 任意の `(u32, u32, u32)` から作った `"{a}.{b}.{c}"` は `Version` の Display と一致（C-GAV2）
- **TP-P2** (proptest): 任意の文字列 `s` に対して、`Version::from_str(s)` が Err なら出力 == `s`（C-GAV3）、かつ panic しない
- **TP-P3** (proptest): 同じ入力で 2 回呼んで同じ出力（C-GAV6）

### boundary test（外部境界経由）{#tp-boundary}

- **TP-B1**: Rust — `commands::get_app_version()` を **public_entry 経由** で呼び、戻り値が `env!("CARGO_PKG_VERSION")` と一致する（現行の Cargo.toml version は parse 可能な semver のため正規化後も同値）
- **TP-B2**: Rust — `get_app_version()` の戻り値と `check_for_updates` が採用する `current_version` の情報源が同一であること（C-GAV5）。両 command が共通の build version 取得関数を使うことで担保し、test は共通関数の値と `get_app_version()` の一致を検証
- **TP-B3**: TS — `getAppVersion()` が `invoke('get_app_version')` を引数なしで 1 回呼び、その戻り値をそのまま resolve する（invoke は adapter 境界のため mock 可）

### 型レベル {#tp-type-level}

- **TP-T1**: `get_app_version` のシグネチャが `fn() -> String`（`Result` を含まない、C-GAV1）。fn pointer coercion で compile-time pin

### production fixture {#tp-production-fixture}

- 本 slice は状態・port を持たないため `setupProductionBuilder()` 相当の fixture は不要。
  boundary test は production の `commands::get_app_version` をそのまま呼ぶ（DoD rule 3 は「production wiring 以外で fixture を組まない」ことで満たす）

## 実装ノート {#impl-notes}

### アーキ層への落とし込み {#impl-layers}

```
apps/promptnotes/src-tauri/src/update_distribution/slices/get_app_version/
├── mod.rs              # pub mod ...; pub use commands::get_app_version
├── domain.rs           # GetAppVersionQuery, AppVersion, to_app_version(raw) (純粋関数)
├── application.rs      # GetAppVersionUseCase::execute(raw) -> AppVersion（domain 委譲のみ）
├── commands.rs         # #[tauri::command] get_app_version() -> String
└── tests.rs            # TP-* (#[cfg(test)])

apps/promptnotes/src/lib/update-distribution/slices/get-app-version/
├── index.ts            # getAppVersion(): Promise<string>
└── index.test.ts       # TP-B3
```

- sub_layers: `domain` / `application` / `presentation`(= commands.rs) / `tests` を埋める。
  `infrastructure` は port を持たないため **不要**（空ファイルを置かない）
- ビルド時定数の取得は `update_distribution/shared/` に共通関数（例: `build_version() -> &'static str`）として切り出し、
  `check_for_updates::commands` と本 slice の両方から使う（C-GAV5）。既存 `check_for_updates` の振る舞いは変えない
- `Version` VO（`shared/types/version.rs`）は pre-release を reject する現行仕様のまま流用する（拡張しない）
- RED 段階では `commands.rs` に stub（例: `String::new()` / `todo!()` ではなく固定の誤値）を置き、invoke_handler 配線まで先に済ませる
- tauri-specta 未採用のため cross_root 生成物（bindings 再生成）は無い（DoD rule 4 対象外）
- `domain` 側の fallback ではログを出さない（表示専用で、pre-release ビルドは正常系のため）

### 参照元 {#impl-references}

- `update_distribution/slices/check_for_updates/commands.rs` — 現行の `env!("CARGO_PKG_VERSION")` 読み取り箇所
- `apps/promptnotes/src/lib/user-preferences/slices/load-settings/index.ts` — TS invoke wrapper の書式
