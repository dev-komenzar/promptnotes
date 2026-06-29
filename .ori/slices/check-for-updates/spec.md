---
coherence:
  source: derived
  last_derived: 2026-06-28
  upstream:
    - domain/workflows/check-for-updates.md#check-for-updates
    - domain/aggregates.md#update-channel-aggregate
    - domain/bounded-contexts.md#update-distribution
    - domain/domain-events.md#new-version-detected
    - domain/validation.md#s14-update-check-failure
  hash:
    domain/workflows/check-for-updates.md#.*: e2935ac617af
    domain/aggregates.md#.*: 43aabb6b099b
    domain/bounded-contexts.md#.*: 4d579125a513
    domain/domain-events.md#.*: 8abdfac78084
    domain/validation.md#.*: 5294b0c32f1b
ori:
  schema:
    propagation_level: file
---

# check-for-updates spec {#check-for-updates-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive check-for-updates`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

アプリ起動時に **1 回のみ** GitHub Releases から最新版を取得し、現在版より新しければ `NewVersionDetected` を発行する command slice。Update Distribution BC の最初の slice であり、本 slice で `UpdateChannel` aggregate root と `Version` / `Release` VO を新規定義する。

> domain/workflows/check-for-updates.md より：「アプリ起動時に GitHub Releases へ HTTP リクエストを送り、新バージョンがあれば NewVersionDetected を発行する。失敗は silent」
>
> domain/bounded-contexts.md#update-distribution より：本 BC は **generic subdomain**。Tauri v2 updater plugin + GitHub Releases に外注し、PromptNotes 側は「起動時に確認する」「通知する」のみ。

実 wiring（Tauri v2 updater plugin の `endpoints` / `pubkey` / signing key）は release infrastructure 整備待ち（**[ori-6l4](#blocked-on-release-infra) で blocked**）。本 slice は **`UpdaterPort` trait + `FakeUpdater` (test impl)** で hex 境界を切り、production wiring は別 follow-up issue とする。これにより application layer の testability と domain invariants の検証を release infra から切り離す。

## 入出力 {#io}

### Input {#io-input}

> domain/workflows/check-for-updates.md#input より：

```rust
struct CheckForUpdatesCommand {
  current_version: Version,    // ビルド時に埋め込まれた app version
}
```

依存（外部から注入される port）:

- `UpdaterPort` — `fetch_latest_release(&self) -> Result<RawRelease, UpdateError>` を提供（Tauri v2 updater plugin の薄ラッパー想定、production impl は ori-6l4 完了後）
- `EventBus` — `NewVersionDetected` の publish 先（同期 in-process、`domain-events.md#notes-sync-rationale`）

`current_version` はビルド時に Cargo 経由で埋め込まれる前提（composition root で `env!("CARGO_PKG_VERSION")` 等から構築）。

### Output {#io-output}

戻り値: `UpdateChannel`（チェック結果、move semantics）

> domain/workflows/check-for-updates.md#output より：
> - `UpdateChannel`（チェック結果）
> - domain event: `NewVersionDetected`（新バージョン検出時のみ）

- 成功 + 新版あり: `UpdateChannel { current_version, latest_release: Some(Release) }` を返し、`NewVersionDetected` を 1 件 publish
- 成功 + 同一 / 古いバージョン: `UpdateChannel { current_version, latest_release: None }` を返し、event は **発行しない**（I-U2: 古いリリースは `None` に正規化）
- 失敗（network / parse / rate limit）: 同じく `latest_release: None` の `UpdateChannel` を返し、event は **発行しない**（S14 silent failure）

### Errors {#io-errors}

> domain/workflows/check-for-updates.md#errors より：

```rust
enum UpdateError {
  NetworkError,
  ParseError,
  RateLimited,
}
```

**戻り型に `Result` を露出しない**（C-CFU3）。`UpdateError` は application service の outer layer で握り潰し、`UpdateChannel { latest_release: None }` に正規化する（S14 silent failure）。ログ出力のみ、UI 通知なし。

### NewVersionDetected Payload {#io-new-version-detected}

> domain/domain-events.md#new-version-detected-payload より：

```rust
struct NewVersionDetected {
  current_version: Version,
  latest_version: Version,
  release_url: Url,
  release_notes: String,
}
```

## 不変条件 {#invariants}

slice 完了時に成立すべき条件。括弧内は domain での出典。

### UpdateChannel Aggregate 由来 {#invariants-update-channel-aggregate}

> domain/aggregates.md#update-channel-aggregate-invariants より引用：

- **I-U1**: `current_version` は immutable（ビルド時定数）。本 slice は `current_version` を input から受け取り、結果 `UpdateChannel` でもそのまま保持する（書換えしない）
- **I-U2**: `latest_release` が `Some` のとき、`latest_release.version > current_version` を満たす。同一 / 古いバージョンは `None` に **正規化**（comparator で `OlderVersion` / `UpToDate` 分岐に降格）。本 slice の `compare_versions` step が単一エントリポイントとして I-U2 を施行する
- **I-U3**: 確認は **アプリ起動時 1 回のみ**。常駐 polling なし、リトライなし。本 slice の use case は内部で retry loop を組まない（API 1 呼出 → 結果 1 個 → 終了）

### slice 固有制約 {#invariants-slice-specific}

- **C-CFU1**: 戻り型に `Result` を露出しない。use case シグネチャは `execute(cmd) -> UpdateChannel`（失敗時も `latest_release: None` の `UpdateChannel` を返す、S14 silent failure を type-level に固定）
- **C-CFU2**: `compare_versions` の判定:
  - `latest > current` → `Comparison::NewVersion(Release)` → `latest_release: Some(_)` + event 発行
  - `latest == current` → `Comparison::UpToDate` → `latest_release: None` + event 非発行
  - `latest < current` → `Comparison::OlderVersion` → `latest_release: None` + event 非発行
  （I-U2 を施行）
- **C-CFU3**: `UpdateError` は use case 内で握り潰し、UI 層には伝搬しない（S14）。ログ出力のみ（`log` crate 経由、ori-2lm.10 で確定）
- **C-CFU4**: 本 slice は **リトライしない**（I-U3）。`UpdaterPort::fetch_latest_release` を 1 回呼んで終わり。複数回呼出 (e.g. transient network failure 時のリカバリ) は scope 外
- **C-CFU5**: 新版検出時は **1 件だけ** `NewVersionDetected` を publish する（複数発行しない）。event の payload は `current_version` / `latest_version` / `release_url` / `release_notes`
- **C-CFU6**: 失敗時 / UpToDate 時 / OlderVersion 時 のいずれでも `EventBus::publish` は **一度も呼ばれない**（S14 + I-U2）
- **C-CFU7**: 本 slice は **副作用を持つ**（network I/O + event publish）。ただし `UpdaterPort` で I/O を抽象化し、application layer は port 越しのみで pure を保つ

## テスト観点 {#test-perspectives}

phase 3 で failing test に展開する観点を列挙。各観点は 1 つ以上の test に対応する想定。

### happy path: 新版検出 {#tp-new-version}

- **TP-N1**: `current = 0.3.1`、`UpdaterPort` が `RawRelease { version: "0.4.0", ... }` を返す → 戻り値 `UpdateChannel.latest_release == Some(Release { version: 0.4.0, ... })`
- **TP-N2**: TP-N1 と同条件で `NewVersionDetected` が **1 件** publish される
- **TP-N3**: TP-N2 の payload が `{ current_version: 0.3.1, latest_version: 0.4.0, release_url, release_notes }` と一致

### UpToDate / OlderVersion: event 非発行 {#tp-no-event}

- **TP-U1**: `current = 0.3.1`、`UpdaterPort` が `RawRelease { version: "0.3.1", ... }` を返す（同一バージョン）→ `UpdateChannel.latest_release == None`
- **TP-U2**: TP-U1 で `NewVersionDetected` が **発行されない**（I-U2 / C-CFU6）
- **TP-O1**: `current = 0.3.1`、`UpdaterPort` が `RawRelease { version: "0.2.0", ... }` を返す（古いバージョン）→ `UpdateChannel.latest_release == None`
- **TP-O2**: TP-O1 で event 非発行（I-U2）

### S14 シナリオ: silent failure {#tp-s14}

> domain/validation.md#s14-update-check-failure を walkthrough：

- **TP-S14-1**: Given ネットワーク断、When `UpdaterPort` が `Err(UpdateError::NetworkError)` を返す、Then `UpdateChannel.latest_release == None`
- **TP-S14-2**: TP-S14-1 で **event 非発行**（S14）
- **TP-S14-3**: TP-S14-1 で **`Result` を返さない**（戻り型は `UpdateChannel`、C-CFU1 を type-level で固定）
- **TP-S14-4**: `UpdateError::ParseError` も silent → `latest_release: None` + event 0 件
- **TP-S14-5**: `UpdateError::RateLimited` も silent → `latest_release: None` + event 0 件

### I-U3: リトライなし {#tp-no-retry}

- **TP-R1**: 任意の成功 / 失敗 path で `UpdaterPort::fetch_latest_release` の呼出が **ちょうど 1 回**（mock で count）
- **TP-R2**: `NetworkError` 後に `fetch_latest_release` を 2 回目呼ばない（リトライしない、I-U3 / C-CFU4）

### 不変条件チェック {#tp-invariants}

- **TP-I1**: 戻り値の `UpdateChannel.current_version == cmd.current_version`（I-U1 immutable）
- **TP-I2**: `latest_release.is_some()` なら `latest_release.version > current_version`（I-U2 正規化）
- **TP-I3**: `latest_release.is_none()` ならば event 0 件（C-CFU6）

### 型レベル {#tp-type-level}

- **TP-T1**: `CheckForUpdatesUseCase::execute` のシグネチャに `Result` を含まない（C-CFU1）。`fn (UseCase, cmd) -> UpdateChannel` で fn pointer coercion により compile-time pin

## 実装ノート {#impl-notes}

### アーキ層への落とし込み {#impl-layers}

DDD-VSA-Hex / typescript-tauri の階層に従い、本 slice は Rust 側で実装する（`implementation.language: rust`）。Update Distribution BC は本 slice で新規に切られる。

```
apps/promptnotes/src-tauri/src/update_distribution/
├── mod.rs                  # pub use slices::*; shared::*
├── shared/
│   ├── mod.rs
│   ├── types/
│   │   ├── mod.rs
│   │   ├── version.rs         # Version VO (semver 比較順)
│   │   ├── release.rs         # Release VO (version + url + notes)
│   │   ├── update_channel.rs  # UpdateChannel aggregate root
│   │   ├── update_error.rs    # UpdateError enum
│   │   └── events.rs          # NewVersionDetected event
│   └── ports.rs               # UpdaterPort, EventBus trait
└── slices/
    ├── mod.rs
    └── check_for_updates/
        ├── mod.rs
        ├── domain.rs          # CheckForUpdatesCommand, Comparison
        ├── application.rs     # CheckForUpdatesUseCase: load → compare → emitConditional
        └── tests.rs           # FakeUpdater で TP-* を網羅
```

### `Version` VO の実装 {#impl-version}

- `semver` crate の `semver::Version` を newtype で wrap (`Version(semver::Version)`)
- `Ord` / `PartialOrd` は inner に委譲 (`semver` crate 1.x 実装準拠: pre-release < release。build metadata は 2.0 仕様と異なり 1.x では `Ord` に影響する点に注意)
- `from_str(&str) -> Result<Version, UpdateError::ParseError>` smart constructor (`semver::Version::parse` へ委譲)
- ori-2lm.9 で strict semver 対応に拡張 (pre-release / build metadata を含む全 semver 文字列を parse 可能)

### `UpdaterPort` の境界 {#impl-updater-port}

```rust
pub trait UpdaterPort {
    fn fetch_latest_release(&self) -> Result<RawRelease, UpdateError>;
}

pub struct RawRelease {
    pub version_string: String,   // e.g. "0.4.0"
    pub url: String,              // e.g. "https://github.com/.../releases/tag/v0.4.0"
    pub notes: String,            // markdown body
}
```

- `RawRelease` は parse 前の生 payload。use case 内で `Version::from_str` する
- production impl は **本 slice の scope 外**: ori-6l4 (release infra 整備) 完了後に `TauriUpdaterPort` 実装を別 issue で追加
- test impl: `FakeUpdater` を `tests.rs` 内に書く（in-memory; `with_response(Ok(_))` / `with_error(_)` で挙動 inject）

### Pipeline ステップ {#impl-pipeline}

> domain/workflows/check-for-updates.md#steps の DMMF pipeline を採用：

1. `fetch_latest_release: () → Result<RawRelease, UpdateError>` — `UpdaterPort::fetch_latest_release`
2. `parse_version: RawRelease → Result<Version, UpdateError>` — `Version::from_str`
3. `compare_versions: (Version, Version) → Comparison` — `NewVersion(Release) | UpToDate | OlderVersion`
4. `branch_on_comparison`:
   - `NewVersion(release)` → step 5 へ
   - `UpToDate | OlderVersion` → 早期 return（`UpdateChannel { latest_release: None }` + event 非発行）
5. `build_update_channel: (Version, Release) → UpdateChannel`
6. `emit: UpdateChannel → NewVersionDetected`

step 1 / 2 のいずれかが `Err` を返した場合、application service の outer layer で握り潰し `UpdateChannel { latest_release: None }` を返す（C-CFU3、S14）。`log::warn!` でログ出力のみ。

### no-Result API 表面 {#impl-no-result}

`CheckForUpdatesUseCase::execute(&self, cmd) -> UpdateChannel` は **Result を返さない**:

```rust
pub fn execute(&self, cmd: CheckForUpdatesCommand) -> UpdateChannel {
    match self.try_execute(cmd.clone()) {
        Ok(uc) => uc,
        Err(e) => {
            log::warn!("update check failed: {:?}", e);
            UpdateChannel::without_release(cmd.current_version)
        }
    }
}
```

内部 `try_execute` が `Result` を返し、外部 `execute` が握り潰す。これは load-settings slice の no-error 契約と同じ pattern。

### release infra blocker {#blocked-on-release-infra}

- 本 slice の **production wiring**（`TauriUpdaterPort` 実装 / Cargo.toml への `tauri-plugin-updater` 有効化 / `tauri.conf.json` の `plugins.updater.endpoints` / `pubkey` 設定 / GitHub Releases 署名鍵設定）は **`ori-6l4` (Wire tauri-plugin-updater after release infra is ready) で blocked**
- 本 slice の scope: domain types + use case + UpdaterPort 境界 + `FakeUpdater` (test) のみ
- production impl 追加と invoke_handler 登録は ori-6l4 完了後の **別 follow-up issue** で対応

### Out of scope {#out-of-scope}

本 slice は **domain types + use case + test boundary** のみを扱う。以下は別 slice / layer の責務：

- `TauriUpdaterPort` の production impl（ori-6l4 待ち）
- `tauri-plugin-updater` の Cargo.toml 有効化 / `tauri.conf.json` 設定
- composition root から `app.path()` 経由で signing key を取り出す処理
- Tauri command surface（`#[tauri::command] check_for_updates`）と TS bindings
- 通知 UI（Toast vs Modal）— `NewVersionDetected` 購読側 (UI 層) の責務、Phase 11a UI 設計で確定
- 手動チェックボタン（MVP 範囲外、I-U3）

## Open Questions {#open-questions}

### oq-version-pre-release {#oq-version-pre-release}

- **問**: `Version` の比較で pre-release（`0.4.0-rc1`）/ build metadata（`0.4.0+sha`）をどう扱うか
- **暫定方針**: 本 slice では `major.minor.patch` の 3-tuple 比較のみ実装。pre-release / build metadata を含む文字列は `Version::from_str` で `ParseError` を返す（保守的 reject）
- **解決方向**: ori-6l4 wiring 時に GitHub Releases の tag 形式を確認 → 必要なら follow-up issue で `Version` を strict semver 対応に拡張
- **解決** (ori-2lm.9): `semver` crate ベースの strict semver 対応に拡張。`Version(semver::Version)` newtype として pre-release / build metadata を含む全 semver 文字列を parse・比較可能。GitHub Releases に tag が存在しない状態だが、standard semver tag 慣行 (`v0.4.0-rc1` 等) を想定し先行対応。注: `semver` crate 1.x では build metadata が `Ord` に影響する (2.0 仕様とは異なる)。PromptNotes のユースケース (起動時 1 回チェック) では build metadata 違いで判定が変わることは実運用上無害。

### oq-log-coupling {#oq-log-coupling}

- **問**: S14 silent failure で「ログは出すが UI 通知はしない」とあるが、本 slice の application layer に `log` crate 依存を入れるか
- **暫定方針**: `log::warn!` を application.rs で使用（既存 lib.rs に `tauri-plugin-log` があるため依存追加なし）
- **解決方向**: 副作用 (logging) を port で抽象化するかは follow-up で議論。テストでは log 出力を assert しない（実害なし）
- **解決** (ori-2lm.10): `log` crate 直接呼出 (port 抽象化なし)。Update Distribution BC は generic subdomain であり、logging は cross-cutting infrastructure → port 化の ROI なし。`tauri-plugin-log` が既に依存にあるため追加依存なし。テストでは log 出力を assert しない。
