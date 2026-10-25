---
coherence:
  source: derived
  last_derived: 2026-10-25
  hash:
    domain/workflows/check-for-updates.md#.*: f08ad6cf0874
    domain/aggregates.md#.*: e3aba26a3001
    domain/bounded-contexts.md#.*: 957e7f4a4930
    domain/domain-events.md#.*: cc1d928f09e4
    domain/validation.md#.*: a12e27d425ae
ori:
  schema:
    propagation_level: file
---

# check-for-updates spec {#check-for-updates-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive check-for-updates`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

アプリ起動時に 1 回、以降は **前回確認から 24 時間経過するごとに** GitHub Releases の最新版を確認し、
現在版より新しく、かつ **同一プロセス内でまだ通知していない** バージョンであれば `NewVersionDetected` を
発行する command slice。失敗は silent。Update Distribution BC の最初の slice で、`UpdateChannel`
aggregate root と `Version` / `Release` VO を定義する。

> domain/workflows/check-for-updates.md#check-for-updates より：「アプリ起動時、および以降 24 時間ごとに
> GitHub Releases へ HTTP リクエストを送り、同一プロセス内で未通知の新バージョンがあれば
> `NewVersionDetected` を発行する（I-U3 / I-U4）。失敗は silent。」
>
> domain/bounded-contexts.md#update-distribution より：本 BC は **generic subdomain**。Tauri v2 updater
> plugin + GitHub Releases に外注し、PromptNotes 側は「確認する」「通知する」のみ。

2026-10-24 の I-U3 改訂（起動時 1 回のみ → 起動時 + 24 時間ごと）と I-U4 新設（同一バージョンの再通知なし）
に伴う再 derive（proposal: `.ori/proposals/accepted/2026-10-24-check-for-updates-aggregates-update-channel-aggregate-invariants.md`、
bd: ori-m2ra.4）。production wiring（`TauriUpdaterPort` / `TauriEventBus` / Tauri command / page-main の
起動時 invoke）は ori-2lm.8 / ori-m2ra.1 で完了済み。

## 入出力 {#io}

### Input {#io-input}

> domain/workflows/check-for-updates.md#input より：

```rust
struct CheckForUpdatesCommand {
  current_version: Version,    // ビルド時に埋め込まれた app version
}
```

依存（外部から注入される port）:

- `UpdaterPort` — `fetch_latest_release(&self) -> Result<RawRelease, UpdateError>`（Tauri v2 updater plugin の薄いラッパー、production impl は `TauriUpdaterPort`）
- `EventBus` — `NewVersionDetected` の publish 先（同期 in-process、`domain-events.md#notes-sync-rationale`。production impl は `TauriEventBus` = Tauri event `new_version_detected`）
- `NotifiedVersionLedger` — 同一プロセス内で通知済みのバージョンを保持する揮発 state（I-U4、本改訂で新設）

`current_version` はビルド時定数（`shared::build_info::build_version()` = `env!("CARGO_PKG_VERSION")`）から composition root で構築する。

### Output {#io-output}

戻り値: `UpdateChannel`（1 回分の確認結果、move semantics）

> domain/workflows/check-for-updates.md#output より：
> - `UpdateChannel`（チェック結果）
> - domain event: `NewVersionDetected`（新バージョン検出時のみ）

- 成功 + 新版あり + 未通知: `UpdateChannel { current_version, latest_release: Some(Release) }` を返し、`NewVersionDetected` を 1 件 publish、そのバージョンを通知済みとして記録
- 成功 + 新版あり + 通知済み（通知済みバージョン以下）: `latest_release: Some(Release)` を返すが event は **発行しない**（I-U4）
- 成功 + 同一 / 古いバージョン: `latest_release: None` を返し、event は **発行しない**（I-U2）
- 失敗（network / parse / rate limit）: `latest_release: None` を返し、event は **発行しない**（S14 silent failure）

> domain/validation.md#s23-update-same-version-once より：「When 1: `latest_release = Some(0.2.3)` だが通知済みのため
> event **NewVersionDetected** は **発行されない**（I-U4）」

### Errors {#io-errors}

> domain/workflows/check-for-updates.md#errors より：

```rust
enum UpdateError {
  NetworkError,
  ParseError,
  RateLimited,
}
```

**戻り型に `Result` を露出しない**（C-CFU1）。`UpdateError` は application service の outer layer で握り潰し、
`latest_release: None` の `UpdateChannel` に正規化する（S14）。ログ出力のみ、UI 通知なし。

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

> domain/domain-events.md#new-version-detected-trigger より：「`UpdateChannel::check()` 成功 かつ
> `latest_release: Some(_)` で、そのバージョンが同一プロセス内で未通知の場合。起動時および 24 時間ごとの
> 定期確認で発行されうる（I-U3）が、同一バージョン（またはそれ以下）について 2 回目以降は発行しない（I-U4）。」

## 不変条件 {#invariants}

slice 完了時に成立すべき条件。括弧内は domain での出典。

### UpdateChannel Aggregate 由来 {#invariants-update-channel-aggregate}

> domain/aggregates.md#update-channel-aggregate-invariants より引用：

- **I-U1**: `current_version` は immutable（ビルド時定数）。本 slice は input の `current_version` をそのまま結果に保持する
- **I-U2**: `latest_release` が `Some` のとき `latest_release.version > current_version`。同一 / 古いバージョンは `None` に正規化（`compare` が単一エントリポイント）
- **I-U3**: 確認は **アプリ起動時に 1 回**、以降は **前回確認から 24 時間経過するごとに** 行う。間隔は定数（設定 UI なし）。前回確認の時刻は揮発（プロセス内のみ、永続化しない）。失敗時の即時リトライはせず、次の確認は通常の 24 時間周期を待つ
- **I-U4**: 同一プロセス内で一度通知した `Release::version` **以下** のバージョンについては `NewVersionDetected` を再発行しない。通知済みバージョンは揮発（再起動後は再び通知される）。抑止は application service が揮発 state で担い、aggregate は 1 回分の確認結果を返すだけ

### slice 固有制約 {#invariants-slice-specific}

- **C-CFU1**: 戻り型に `Result` を露出しない。use case シグネチャは `execute(cmd) -> UpdateChannel`（Tauri command も `UpdateChannelResponse` を直接返す）
- **C-CFU2**: `compare` の判定: `latest > current` → `NewVersion(Release)`、`==` → `UpToDate`、`<` → `OlderVersion`（I-U2）
- **C-CFU3**: `UpdateError` は use case 内で握り潰し UI 層へ伝搬しない（S14）。ログ出力のみ
- **C-CFU4**: 1 回の確認（`execute` 1 呼出）で `UpdaterPort::fetch_latest_release` を呼ぶのは **ちょうど 1 回**。失敗時も同一確認内で再試行しない（I-U3「即時リトライなし」）
- **C-CFU5**: 1 回の確認で publish する `NewVersionDetected` は **高々 1 件**。payload は `current_version` / `latest_version` / `release_url` / `release_notes`
- **C-CFU6**: 失敗時 / UpToDate 時 / OlderVersion 時 / 通知済み時 のいずれでも `EventBus::publish` は呼ばれない（S14 + I-U2 + I-U4）
- **C-CFU7**: 副作用（network I/O / event publish / 揮発 state）は port 越しのみ。application layer は port を介してのみ外界に触れる
- **C-CFU8（I-U4 の判定）**: `latest` を通知すべきなのは「通知済みバージョンが無い」または「`latest > 通知済みバージョン`」のときのみ。通知した場合は通知済みバージョンを `latest` に更新する。判定と記録は 1 操作で行い（`NotifiedVersionLedger::claim`）、同時に走った 2 つの確認が同じバージョンを二重通知しない
- **C-CFU9（I-U4 の記録対象）**: 通知済みとして記録するのは event を publish したバージョンのみ。失敗 / UpToDate / OlderVersion の確認は通知済み state を変更しない
- **C-CFU10（I-U3 の周期判定）**: 確認が「due」になるのは、前回確認時刻（未確認ならプロセス起動時刻）から **24 時間以上** 経過したとき。前回確認時刻は確認の **開始時** に更新する（成否を問わない。失敗しても次は 24 時間後）
- **C-CFU11（休止の吸収）**: 経過時間は **wall clock**（`SystemTime`）で測り、定期確認の判定は短い間隔（1 時間）の tick ごとに行う。sleep / 休止中も wall clock は進むため、復帰後最初の tick で due になる。wall clock が前回確認時刻より過去に戻った場合は due とみなす（1 回確認して前回確認時刻を張り直す）
- **C-CFU12（起動時確認の経路）**: 起動時の確認は frontend（page-main）が `new_version_detected` の listen 確立後に `check_for_updates` を invoke する既存経路（ori-m2ra.1、page-main I-PM3）を維持する。定期確認は Rust 側の背景 task が同じ確認処理（前回確認時刻の更新 + use case 実行）を呼ぶ。どちらの経路の確認も同じ揮発 state（前回確認時刻 / 通知済みバージョン）を共有する

## 境界契約 {#boundary-contract}

- boundary kind: `tauri_command`
- Rust 側 source / contact point:
  `apps/promptnotes/src-tauri/src/update_distribution/slices/check_for_updates/commands.rs`
  - `#[tauri::command] pub async fn check_for_updates(app_handle, state) -> UpdateChannelResponse`（`Result` なし、C-CFU1）
  - `apps/promptnotes/src-tauri/src/lib.rs` の `invoke_handler!` に配線済み。揮発 state（`InMemoryUpdateCheckState`）は `lib.rs` で `.manage(...)`、定期確認 task は `.setup(...)` で起動する
- TS 側 contact point:
  `apps/promptnotes/src/lib/update-distribution/slices/check-for-updates/index.ts`
  - `checkForUpdates(): Promise<UpdateChannelResponse>` — `invoke('check_for_updates')` の薄いラッパー
  - 本プロジェクトは tauri-specta を採用していないため手書き invoke wrapper を contact point とする
- public_entry:
  - Rust: `update_distribution/slices/check_for_updates/mod.rs`（`pub use` で `CheckForUpdatesUseCase` / `check_for_updates` / `CheckForUpdatesCommand` を公開）
  - TS: 上記 `index.ts`
- 禁止される import 経路:
  - UI 層が Tauri updater plugin（`@tauri-apps/plugin-updater`）を直接呼んで確認すること（I-U4 の通知済み state を迂回する）
  - 定期確認 task が `CheckForUpdatesUseCase` を前回確認時刻の更新なしに直接呼ぶこと（C-CFU10 違反）

## テスト観点 {#test-points}

phase 3 で failing test に展開する観点。Rust 側は `check_for_updates/tests.rs`（`#[cfg(test)]`）、
`UpdaterPort` / `EventBus` は adapter 境界の test double（`FakeUpdater` / `FakeBus`）、
`NotifiedVersionLedger` は production の `InMemoryUpdateCheckState` を実物で使う。

### happy path: 新版検出 {#tp-new-version}

- **TP-N1**: `current = 0.3.1`、`UpdaterPort` が `"0.4.0"` を返す → `latest_release == Some(0.4.0)`
- **TP-N2**: TP-N1 と同条件で `NewVersionDetected` が **1 件** publish される
- **TP-N3**: TP-N2 の payload が `{ current_version: 0.3.1, latest_version: 0.4.0, release_url, release_notes }` と一致

### UpToDate / OlderVersion: event 非発行 {#tp-no-event}

- **TP-U1**: 同一バージョン（`"0.3.1"`）→ `latest_release == None`
- **TP-U2**: TP-U1 で event 非発行（I-U2 / C-CFU6）
- **TP-O1**: 古いバージョン（`"0.2.0"`）→ `latest_release == None`
- **TP-O2**: TP-O1 で event 非発行

### S14 シナリオ: silent failure {#tp-s14}

> domain/validation.md#s14-update-check-failure を walkthrough（Given: 起動時または 24 時間後の定期確認時 / ネットワーク断）：

- **TP-S14-1**: `UpdaterPort` が `Err(NetworkError)` → `latest_release == None`、`current_version` 保持
- **TP-S14-2**: TP-S14-1 で event 非発行
- **TP-S14-3**: 戻り型は `UpdateChannel`（`Result` でない、type-level）
- **TP-S14-4**: `ParseError` も silent
- **TP-S14-4b**: `Ok` で返った version 文字列が parse 不能（`"0.4.0-rc1"`）でも silent
- **TP-S14-5**: `RateLimited` も silent

### リトライなし {#tp-no-retry}

- **TP-R1**: 成功 path（NewVersion）で 1 回の確認あたり `fetch_latest_release` 呼出が **ちょうど 1 回**
- **TP-R1b**: UpToDate / OlderVersion path でも 1 回（C-CFU4 全 path）
- **TP-R2**: `NetworkError` 後に同一確認内で再試行しない（I-U3 / C-CFU4）

### I-U4 / S23: 通知済みバージョンの再通知抑止 {#tp-notified-once}

> domain/validation.md#s23-update-same-version-once を walkthrough（current 0.2.2、起動時に 0.2.3 を通知済み）：

- **TP-D1**: 同じ ledger で 2 回確認し、2 回とも `"0.2.3"` を受信 → 1 回目は event 1 件、2 回目は event **0 件**。2 回目の戻り値も `latest_release == Some(0.2.3)`（S23 When 1 / I-U4）
- **TP-D2**: TP-D1 の後、3 回目の確認で `"0.2.4"` を受信 → event 1 件（`latest_version == 0.2.4`）（S23 When 2）
- **TP-D3**: `"0.2.4"` を通知済みの状態で `"0.2.3"` を受信（current 0.2.2 より新しいが通知済み以下）→ event 0 件（I-U4「以下」）
- **TP-D4**: 新しい ledger（= 再起動相当）では、別 ledger で通知済みの `"0.2.3"` を再び通知する（S23 Then「再起動後は再び通知」/ I-U4 揮発）
- **TP-D5**: 失敗 / UpToDate / OlderVersion の確認の後でも、次の新版確認は通知される（C-CFU9: 通知しなかった確認は ledger を変えない）
- **TP-D6** (proptest): 任意のバージョン列を同じ ledger で順に確認したとき、publish されたバージョン列は **狭義単調増加** で、各要素は `current_version` より大きい（C-CFU8 / I-U2 / I-U4）
- **TP-D7**: `should_notify` 純粋関数 — `notified = None` なら true、`latest > notified` なら true、`latest <= notified` なら false

### I-U3: 24 時間周期の判定 {#tp-periodic}

純粋関数 `is_check_due(last_checked_at, now)` と揮発 state（`InMemoryUpdateCheckState`）を、時刻を引数注入して検証する（実時間で待たない）。

- **TP-P1**: 周期定数 `UPDATE_CHECK_INTERVAL == 24h`、tick 間隔 `PERIODIC_CHECK_TICK == 1h`（C-CFU11）
- **TP-P2**: 経過 `24h - 1s` → not due、`24h` ちょうど → due、`24h + 1s` → due（C-CFU10）
- **TP-P3**: `now` が `last_checked_at` より過去（wall clock 巻き戻り）→ due（C-CFU11）
- **TP-P4**: 未確認の state はプロセス起動時刻を基準にする — 起動直後は not due、起動から 24h 後は due
- **TP-P5**: state で確認を開始（`begin_check(now)`）すると前回確認時刻が `now` になり、`now + 24h - 1s` は not due、`now + 24h` は due（frontend 起動時 invoke も定期確認も同じ基準を更新する、C-CFU12）
- **TP-P6**: `begin_due_check(now)` は due のときだけ true を返して前回確認時刻を更新し、not due のときは false で state を変えない（定期確認 task の判定）
- **TP-P7** (proptest): 任意の `last` と `elapsed` について、`is_check_due(last, last + elapsed) == (elapsed >= 24h)`

### 不変条件チェック {#tp-invariants}

- **TP-I1**: 戻り値の `current_version == cmd.current_version`（I-U1）
- **TP-I2**: `latest_release.is_some()` なら `version > current_version`（I-U2）
- **TP-I3**: `latest_release.is_none()` ならば event 0 件（C-CFU6）

### 型レベル {#tp-type-level}

- **TP-T1**: `CheckForUpdatesUseCase::execute` のシグネチャに `Result` を含まない（C-CFU1）。fn pointer coercion で compile-time pin

### boundary test（外部境界経由）{#tp-boundary}

- **TP-B1**: TS — `checkForUpdates()` が `invoke('check_for_updates')` を引数なしで 1 回呼び、戻り値をそのまま resolve する（invoke は adapter 境界のため mock 可。既存 `tests/check-for-updates.test.ts`）
- **TP-B2**: Tauri 境界から UI までの縦断（invoke → event → Toast、2 回目同一バージョンで Toast 再表示なし）は scenario `s14-update-check-failure` / `s23-update-same-version-once`（wdio）で検証する

### production fixture {#tp-production-fixture}

- `NotifiedVersionLedger` は production の `InMemoryUpdateCheckState` をそのまま使う（揮発 in-memory で外部 I/O を持たないため mock しない）。`UpdaterPort` / `EventBus` のみ adapter 境界として test double に差し替える（DoD rule 3: production wiring 以外で domain / application を組み替えない）

## 実装ノート {#impl-notes}

### アーキ層への落とし込み {#impl-layers}

```
apps/promptnotes/src-tauri/src/update_distribution/
├── shared/
│   ├── types/               # Version / Release / UpdateChannel / UpdateError / NewVersionDetected (変更なし)
│   ├── build_info.rs        # build_version() (変更なし)
│   └── ports.rs             # UpdaterPort / EventBus + NotifiedVersionLedger (新設)
└── slices/check_for_updates/
    ├── mod.rs
    ├── domain.rs            # CheckForUpdatesCommand / Comparison
    │                        # + UPDATE_CHECK_INTERVAL / PERIODIC_CHECK_TICK / should_notify / is_check_due (純粋)
    ├── application.rs       # CheckForUpdatesUseCase<U, B, L>: fetch → parse → compare → claim → emit
    ├── infrastructure.rs    # TauriUpdaterPort / TauriEventBus (既存)
    │                        # + InMemoryUpdateCheckState (前回確認時刻 + 通知済みバージョン、Mutex)
    ├── commands.rs          # #[tauri::command] check_for_updates + execute_update_check (起動時 / 定期の共通入口)
    │                        # + spawn_periodic_update_check (tick ループ。composition 側に置く)
    └── tests.rs
```

- sub_layers: `domain` / `application` / `infrastructure` / `presentation`(= commands.rs) / `tests` を埋める
- tauri-specta 未採用のため cross_root 生成物（bindings 再生成）は無い（DoD rule 4 対象外）。`check_for_updates` の TS 側シグネチャ（引数なし、`UpdateChannelResponse`）は変えない
- production fixture は `InMemoryUpdateCheckState`（[#tp-production-fixture](#tp-production-fixture)）

### Pipeline ステップ {#impl-pipeline}

> domain/workflows/check-for-updates.md#steps の DMMF pipeline を採用：

1. `fetch_latest_release` — `UpdaterPort::fetch_latest_release`
2. `parse_version` — `Version::from_str`
3. `compare_versions` — `NewVersion(Release) | UpToDate | OlderVersion`
4. `branch_on_comparison` — `UpToDate | OlderVersion` は `latest_release: None` で早期 return
5. `build_update_channel` — `UpdateChannel::with_release`
6. `suppress_notified` — `NotifiedVersionLedger::claim(&version)` が false なら event 非発行で channel を返す（I-U4）
7. `emit` — `EventBus::publish(NewVersionDetected)`（claim が true のときのみ。claim が通知済みの記録を兼ねる）

step 1 / 2 の `Err` は outer の `execute` で握り潰し `UpdateChannel::without_release` を返す（C-CFU3）。`log::warn!` でログのみ。

### NotifiedVersionLedger port {#impl-ledger}

```rust
pub trait NotifiedVersionLedger {
    /// `should_notify(version, 通知済み)` が true なら通知済みを `version` に更新して true。
    /// false なら state を変えずに false。判定と記録は 1 操作 (C-CFU8)。
    fn claim(&self, version: &Version) -> bool;
}
```

- `Rc<T>` / `Arc<T>` / `&T` 越しでも使えるよう blanket impl を置く（既存 `UpdaterPort` / `EventBus` と同じ流儀）
- 判定ロジックは domain の純粋関数 `should_notify(latest: &Version, notified: Option<&Version>) -> bool` に置き、ledger 実装はそれを呼ぶだけ

### 定期確認の runtime {#impl-periodic}

- `InMemoryUpdateCheckState { last_checked_at: Mutex<SystemTime>, notified: Mutex<Option<Version>> }`。`new(started_at)` でプロセス起動時刻を前回確認時刻の初期値にする（TP-P4）
  - `begin_check(now)`: 前回確認時刻を `now` に更新（frontend 起動時 invoke 経路）
  - `begin_due_check(now) -> bool`: `is_check_due(last, now)` なら更新して true（定期 task 経路）
- `commands::execute_update_check(app_handle, state)`: Tauri adapter で use case を組み立てて 1 回確認し `UpdateChannelResponse` を返す。Tauri command と定期 task の共通入口（C-CFU12）。前回確認時刻の記録は呼出側（command は `begin_check(SystemTime::now())`、定期 task は `begin_due_check` が true のときのみ確認）
- `commands::spawn_periodic_update_check(app_handle)`（Tauri adapter で use case を組み立てる composition 側に置き、infrastructure から presentation への逆依存を作らない）: `tauri::async_runtime::spawn` で `loop { sleep(PERIODIC_CHECK_TICK).await; if state.begin_due_check(SystemTime::now()) { 確認 } }`。`lib.rs` の `.setup` で 1 回だけ起動する。tokio の `time` feature を Cargo.toml で明示する（lock には影響しない）
- 起動時の確認は従来どおり page-main が listen 確立後に invoke する（listen 前の emit 取りこぼし防止、ori-m2ra.1）。定期 task は起動から 24h 以上後にしか確認しないため、listen 確立前に emit する経路は生まれない
- `TauriUpdaterPort` は `tokio::task::block_in_place` で同期化しているため、tauri の multi-thread runtime 上で動く `spawn` task からも呼べる

### release infra / 既存 wiring {#impl-existing-wiring}

- `TauriUpdaterPort` / `TauriEventBus` / `release_page_url` / debug seam（`TAURI_TEST_UPDATER_ENDPOINT`）は ori-2lm.8 / ori-m2ra.2 で実装済み。本改訂では変更しない
- `commands.rs` の module doc / TS wrapper の doc にある「起動時に 1 回だけ。polling しない」は I-U3 改訂に合わせて書き換える

### Out of scope {#out-of-scope}

- 前回確認時刻 / 通知済みバージョンの永続化（I-U3 / I-U4 は揮発と規定。永続 skip は screen-3 `#cross-suppression` で MVP 範囲外）
- 確認間隔の設定 UI（I-U3: 定数）
- 手動チェックボタン（workflow Notes: MVP 範囲外）
- ウィンドウ focus 復帰時の確認（C-CFU11 の wall clock + tick で休止を吸収するため不要）
- 通知 UI（widget-update-toast の責務）

## Open Questions {#open-questions}

### oq-version-pre-release {#oq-version-pre-release}

- `Version` は ori-2lm.9 で strict semver 対応済み（変更なし）。本改訂の論点ではない

### oq-log-coupling {#oq-log-coupling}

- `log::warn!` を infrastructure / commands で使用（ori-2lm.10 で決着済み）。本改訂の論点ではない
