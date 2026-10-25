# Review: s14-update-check-failure {#review-s14-update-check-failure}

## Pass 1 {#pass-1}

> reviewer agent (`ori-reviewer`) は spawn せず、main session の objective 検証で代替
> （orchestrator 指示: reviewer spawn がハングする既知問題があるため。s1〜s13 と同方針）。

### Syntax checks {#pass-1-syntax}

- tsc (`tsc --noEmit -p tsconfig.json`): PASS（テスト spec は 0 error。既知の
  `wdio.conf.ts TS2353 'tauri:options'` のみ — s1〜s13 と同一の WDIO v9 型定義起因、許容）
- docker-compose: SKIP — compose-service 参加者ゼロ (app `promptnotes` is mode=local)
- E2E: PASS (`wdio run wdio.conf.ts` → **4 passing**, ~1.8s)

### Mode checklist (local tauri) {#mode-checklist}

| item | status | note |
|---|---|---|
| spec `runner: wdio` ↔ test 実行 ↔ wdio.conf.ts | ✅ PASS | 一致（derive chain step 2: 参加 local app の runtime.runner） |
| テストコード内に service lifecycle が無い | ✅ PASS | seed / updater server は onPrepare、テストは assert + invoke のみ |
| local app が docker-compose に含まれない | ✅ PASS | compose 不在が正しい |
| wdio `tauri:options.application` == runtime `binary` | ✅ PASS | `target/debug/app` |
| `@wdio/tauri-service` が services に含まれる | ✅ PASS | `driverProvider: 'external'` |

### Semantic findings (main session objective verification) {#pass-1-findings}

spec ↔ テストコード ↔ domain 整合性:

- **PASS**: **step 1（本 scenario の核）** — ネットワーク断相当（local server が socket を destroy）で
  `check_for_updates` を Tauri 境界 invoke → reject せず
  `{ current_version: "0.2.1", latest_release: null }` を返す。`Result` 非露出 = C-CFU1、
  `without_release` への silent 降格 = S14 / I-U2 を実測 ✓
- **PASS**: **step 2（NewVersionDetected 非発行 / UI 通知なし）** — 失敗後
  `[data-testid="widget-update-toast"]` が `isExisting()==false`。event 非発行 → 購読 store の
  payload が `null` のまま（I-U3 / screen-3 / page-groups `widget-update-toast` failure mode）✓
- **PASS**: **step 3（ユーザの作業を妨げない）** — 失敗後も seed Note がフィードに残存し、
  永続化 `.md` も無傷。フィード操作が継続できる ✓
- **PASS**: **step 4（陽性対照 / 非空虚性）** — endpoint を release モードに切替 → 同じ invoke で
  `latest_release.version == "9.9.9"`（Some）になり `widget-update-toast` が mount、
  `screen-3-latest-version == "9.9.9"`。これにより step 1-2 の失敗 inject が実際に失敗を
  引き起こしており、step 2 の「Toast 不在」が空振りでないことを保証（event → Toast 購読は
  `store.test.ts` の unit test とも整合）✓
- **PASS**: **失敗再現が real HTTP 経路** — fake error の直接 inject ではなく、local server の
  接続断を reqwest が踏むため `tauri_plugin_updater::Error::Reqwest` →
  `UpdateError::NetworkError` の写像まで通る ✓
- **PASS**: **production 影響なし** — seam は `#[cfg(debug_assertions)]` ガードで release build は
  compile out。本番 endpoint (`tauri.conf.json`) は不変 ✓
- **PASS**: spec.md frontmatter の `coherence.source: derived` + upstream 6 件（hash 付き:
  validation / check-for-updates / update-channel-aggregate / new-version-detected / screen-3 /
  widget-update-toast）✓
- **PASS**: テストデータ独立性 — `mkdtempSync` の temp dir に config/data/storage を隔離し
  `onComplete` で updater server close + `rmSync`。`TAURI_TEST_STORAGE_DIR` override により実 user
  環境を汚さない ✓
- **MEDIUM**: **起動時 invoke wiring 未実装** — production frontend は `check_for_updates` を起動時に
  invoke していない（command は `invoke_handler` 登録済み）。validation の Given「アプリ起動直後」/
  When「`check_at_startup()` 実行」は Tauri 境界 invoke で再現している。起動時 wiring は本 scenario
  のスコープ外。spec `#impl-notes` / notes.md#known-issues に明記済み ✓
- **LOW**: 陽性対照の `latest.json` は build target (linux-x86_64) 依存。別 OS/arch では key 調整が
  必要（notes.md#known-issues に明記）✓
- **LOW**: `signature` は `check()` では未検証（install 時検証）。本 scenario は install しないため
  dummy signature で問題なし（notes.md#known-issues）✓

### Production fix discovery {#pass-1-production-fix}

production 側は S14 の silent failure を**既に満たしていた**（`check_for_updates` command /
`CheckForUpdatesUseCase::execute` / `TauriUpdaterPort` / `TauriEventBus` / `WidgetUpdateToast`）。
E2E を決定論的にするため、**debug build 限定の endpoint override seam** のみを追加した:

- `apps/promptnotes/src-tauri/src/update_distribution/slices/check_for_updates/infrastructure.rs`:
  `TAURI_TEST_UPDATER_ENDPOINT`（`#[cfg(debug_assertions)]`）で updater endpoint を差替え可能に。
  release build はこの分岐を compile out するため本番挙動は不変。

RED 実測（テストが空振りでないことの確認）:

- **変異 A（`check_for_updates/application.rs` の `Err` 分岐を非 silent 化）**: 失敗時に
  fabricated `NewVersionDetected` を publish + `UpdateChannel::with_release` を返す
  → `bun run build:test` → E2E **2 failing / 2 passing**:
  - step 1: `Expected: null` に対し `Received: {"version":"0.2.1", ...}`
  - step 2: `widget-update-toast` `Expected: false` に対し `Received: true`
  - step 3 / step 4 は pass
- **復元後** → 再ビルド → E2E **4 passing (~1.8s)** で GREEN。

production への正味の変更は **1 file**（debug seam のみ）。

回帰確認:

- frontend unit test: **143 passed / 15 files**
- Rust `update_distribution` unit: **20 passed**
- E2E 回帰: s13-quit-flush **3 passing**、s12-startup-state **1 passing**
- Rust 全体 (`cargo test`): 328 passed / 2 failed。失敗 2 件は fixture が 2026-06 固定で `now` が
  実時計依存の date-range テスト（`list_feed::tp_f5_last_7_days` / `tp_f6_and_composition`）。
  変更を stash したクリーン HEAD でも同様に fail する **既存の time-bomb**（本変更とは無関係）。

### Disposition {#pass-1-disposition}

- 指摘は MEDIUM/LOW のみで、いずれも起動時 wiring のスコープ外注記と build target 由来の注意。
  spec / notes に明記済みで、E2E の観測点（silent 降格 / event 非発行 / UI 通知なし / 非妨害 /
  陽性対照）は満たす。
- production 変更は debug seam 1 file のみ。RED 検出力は変異 A の実測で確認済み。
- Verdict: **PASS**

## Pass 2 (re-derive 2026-10-25) {#pass-2}

> reviewer: ori-reviewer (fresh context)。対象は re-derive 後の spec.md#scenario-steps と tests/*.spec.ts、wdio.conf.ts。E2E 実測 (main session): 4 passing。Pass 1 は旧 testid / 旧 version を参照する stale な記述のため参照しない。

### Then ↔ assertion coverage {#pass-2-coverage}

Then 件数: spec.md#scenario-steps に 4 件 (main session 申告と一致)。

| Then (source#anchor) | 期待される assertion | テスト (file:line) | 状態 |
|---|---|---|---|
| spec.md#scenario-steps Then HTTP 失敗の UpdateError を application service が握り潰す | invoke が reject せず `latest_release == null`、`current_version` 保持 | tests/s14-update-check-failure.spec.ts:63-67。補強: apps/promptnotes/src-tauri/src/update_distribution/slices/check_for_updates/tests.rs:269 `tp_s14_1_network_error_returns_channel_with_none` | VERIFIED |
| spec.md#scenario-steps Then event NewVersionDetected は発行されない | Toast root が DOM 不在 (購読 store payload が null のまま) | tests/s14-update-check-failure.spec.ts:73 (陽性対照 :95-97)。補強: tests.rs:289 `tp_s14_2_network_error_publishes_no_event` (bus.count()==0) | VERIFIED |
| spec.md#scenario-steps Then UI 通知なし、ログ出力のみ (UI 通知なしの半分) | エラー UI / Toast が出ない | tests/s14-update-check-failure.spec.ts:73 (Toast 不在のみ。エラー toast 等の他 UI 要素は未検査だが、domain 上 UI は Toast のみ) | VERIFIED |
| spec.md#scenario-steps Then UI 通知なし、ログ出力のみ (「ログ出力」の半分) | 失敗時に log が出力される | E2E / unit のどちらにも assertion なし。実装は infrastructure.rs:87 `log::warn!("updater check failed")` に存在するが検証するテストが無い | **UNVERIFIED** |
| spec.md#scenario-steps Then ユーザの作業を妨げない | seed Note がフィードに残存、永続 .md が無傷、ウィンドウ生存 | tests/s14-update-check-failure.spec.ts:78-79 | VERIFIED |

### test-points 網羅 gate {#pass-2-test-points}

spec.md#test-points は TP1〜TP5。TP1 (:63-67) / TP2 (:73) / TP3 (:78-79) / TP4 (:95-98 陽性対照) は E2E で assert 済み。TP5 (即時リトライなし / 24h 周期) は tests.rs の TP-R2 (:397) / TP-P2 (:691) / TP-P5 (:717) / TP-P6 (:731) で代替担保され、実在を確認。UNCOVERED なし。ただし Then「ログ出力」に対応する test-point が spec 側に存在しない (下記 Finding 1)。

### Mode checklist (local tauri) {#pass-2-mode-checklist}

| item | status | note |
|---|---|---|
| spec `runner: wdio` ↔ test ↔ wdio.conf.ts | PASS | `browser.tauri.execute` / `$`、conf は `@wdio/tauri-service`、architecture.md と一致 |
| テストコード内に service lifecycle が無い | PASS | updater server / seed は `onPrepare`、`onComplete` で close + rmSync。spec は mode ファイル切替 + invoke + assert のみ |
| docker-compose 不在 | PASS | compose-service 参加者ゼロ |
| `tauri:options.application` == runtime binary | PASS | `target/debug/app` |

### Findings {#pass-2-findings}

- [1 Then ↔ assertion] **HIGH**: 「ログ出力のみ」のログ出力が UNVERIFIED。spec.md#test-points にも対応 TP が無く、E2E / unit いずれも log を assert していない。実装は infrastructure.rs:87 の `log::warn!` で存在するが、application.rs:41 のコメント (`実 wiring (ori-6l4) 後は log::warn! を有効化`) が stale のまま、application service (握り潰す層) 自体は log を出さず infra 層任せ。未検証 Then の LOW 化は禁止のため HIGH。推奨修正 (`/ori-generate` へ差し戻し): (a) 失敗時 log を捕捉して assert する unit test を tests.rs (または infrastructure の adapter test) に追加し、spec.md#test-points に TP を追加して N/A(代替担保) として紐付ける、または (b) E2E で app の log ファイル / stderr を読み `updater check failed` の出現を assert する。(a) が低コスト。application.rs:41 の stale コメントも更新すること。
- [5 edge case] LOW: spec.ts:66 は `current_version` を `'0.2.3'` に、:98 も同様にハードコード。Cargo.toml version bump (リリース時に必ず変わる) で E2E が壊れる。推奨: Cargo.toml / tauri.conf から読む、または `typeof string` と非空の assert + 動的取得に。non-blocking だが保守負債。
- [5 edge case] LOW: wdio.conf.ts は Linux ガード (非 linux で SevereServiceError) と tauri-driver 残留 port 検査を持たず、同系の s23 conf と不整合。XDG のみで隔離するため非 Linux では実環境を汚染しうる。推奨: s23 と同じガードを追加。
- [5 edge case] LOW: Then「ユーザの作業を妨げない」の assert (:78-79) は seed Note の表示とファイル内容のみで、ウィンドウ生存・入力継続は間接的。許容範囲。
- [6 トレース] PASS: it 名に `validation#s14-update-check-failure` を引用、最外殻 `describe('scenario:s14-update-check-failure')`。
- [3 DDD / 4 副作用境界] PASS: seam は debug build 限定で本番挙動不変 (Pass 1 記載の通り)。

## 総合判定 (Pass 2)

**NEEDS_FIX**

理由:
1. Then「UI 通知なし、ログ出力のみ」のうち「ログ出力」を assert するテストが E2E / unit のどちらにも存在せず UNVERIFIED。代替担保も無いため HIGH (規則により LOW 不可)。
2. 他の Then 3 件は VERIFIED。test-points TP1〜TP5 に UNCOVERED なし、mode checklist も PASS。
3. 差し戻し先は `/ori-generate` (log 出力の assertion 追加と spec.md#test-points への TP 追記)。

verdict: NEEDS_FIX

## Pass 3 (re-review 2026-10-25) {#pass-3}

> reviewer: ori-reviewer (fresh context, single-pass 最終回)。対象は `git diff 4c4f9bd..64c75fc` の対応差分。E2E 実測 (main session): 4 passing。Rust `tp_s14_6` は application.rs の `log::warn!` を消す mutation で FAILED を確認済みとの申告 (再実行はしていない。コード読みで検出力を確認)。

### Then ↔ assertion coverage {#pass-3-coverage}

Then 件数: spec.md#scenario-steps に 4 件 (main session 申告と一致)。

| Then (source#anchor) | 期待される assertion | テスト (file:line) | 状態 |
|---|---|---|---|
| spec.md#scenario-steps Then HTTP 失敗の UpdateError を application service が握り潰す | invoke が reject せず `latest_release == null`、`current_version` == build 時 version | tests/s14-update-check-failure.spec.ts:71-75。補強: tests.rs:269 `tp_s14_1_network_error_returns_channel_with_none` | VERIFIED |
| spec.md#scenario-steps Then event NewVersionDetected は発行されない | Toast root が DOM 不在 (陽性対照で非空虚) | tests/s14-update-check-failure.spec.ts:81 (陽性対照 :100-108)。補強: tests.rs:289 `tp_s14_2_network_error_publishes_no_event` | VERIFIED |
| spec.md#scenario-steps Then UI 通知なし、ログ出力のみ (UI 通知なし) | エラー UI / Toast が出ない | tests/s14-update-check-failure.spec.ts:81 | VERIFIED |
| spec.md#scenario-steps Then UI 通知なし、ログ出力のみ (ログ出力) | 失敗時に `update check failed` の warn ログが 1 件 | E2E 不可 (app ログを読まない)。代替担保: apps/promptnotes/src-tauri/src/update_distribution/slices/check_for_updates/tests.rs:805 `tp_s14_6_failure_is_logged_as_warning` (Network/Parse/RateLimited/parse 不能の 4 経路で warn がちょうど 1 件、event 0 件) + :839 `tp_s14_7_success_paths_do_not_log_failure` (陰性対照)。実装: application.rs:42 `log::warn!("update check failed: ...")` と文言一致、実在確認済み | N/A(代替担保) |
| spec.md#scenario-steps Then ユーザの作業を妨げない | seed Note がフィードに残存、永続 .md が無傷 | tests/s14-update-check-failure.spec.ts:86-87 | VERIFIED |

### test-points 網羅 gate {#pass-3-test-points}

spec.md#test-points は TP1〜TP6 + slice 分担注記。test-points-map.md と突合した結果、全項目が表に存在し不一致なし。

- TP1 (:69 step 1) / TP2 (:78 step 2) / TP3 (:84 step 3) / TP4 (:90 step 4): COVERED。いずれも該当 assertion を確認 (map の行番号はテスト関数 `it` 開始行で整合)。
- TP5: tests.rs:397 / :691 / :717 / :731 の 4 関数が実在。
- TP6: tests.rs:805 / :839 が実在し、assert 内容が TP6 の記述 (失敗時 warn 1 件、成功時は出さない) と一致。thread-local capture logger のため並列テストと干渉しない。
- UNCOVERED なし、代替担保欠落の N/A なし。

### Mode checklist (local tauri) {#pass-3-mode-checklist}

| item | status | note |
|---|---|---|
| spec `runner: wdio` ↔ test ↔ wdio.conf.ts | PASS | `browser.tauri.execute` / `$`、conf は `@wdio/tauri-service`、architecture.md `scenario_test_runner: wdio` と一致 |
| テストコード内に service lifecycle が無い | PASS | updater server / seed / 隔離は `onPrepare`、後始末は `onComplete`。spec は mode ファイル切替 + invoke + assert のみ |
| docker-compose 不在 | PASS | compose-service 参加者ゼロ |
| `tauri:options.application` == runtime binary | PASS | `apps/promptnotes/src-tauri/target/debug/app` (architecture.md:12 と一致) |

### Findings {#pass-3-findings}

- [Pass 2 Finding 1 解消] 「ログ出力」の UNVERIFIED は、spec.md への TP6 追加と tests.rs の TP-S14-6/7 により N/A(代替担保) へ解消。stale コメント (application.rs) も `log::warn!` 実装に置換済み。HIGH 指摘なし。
- [Pass 2 LOW 解消] current_version のハードコード (`'0.2.3'`) を `get_app_version` の Tauri 境界 invoke に置換済み。wdio.conf.ts に Linux ガードと tauri-driver 残留 port 検査 (SevereServiceError) を追加済み。
- [5 edge case] LOW (non-blocking): TP-S14-6 が捕捉するのは application 層の warn のみ。infrastructure.rs:87 の `updater check failed` ログ (実 adapter 側) はこの unit test の対象外 (fake updater 使用)。Then の趣旨 (握り潰し層でログを出す) は満たすため許容。
- [5 edge case] LOW (non-blocking): `buildVersion()` と `check_for_updates` の current_version は同一のビルド時定数由来で、一致 assert は I-U1 の一貫性確認に留まる。独立した期待値が必要なら将来 Cargo.toml 読込を検討。
- [6 トレース] PASS: it 名に `validation#s14-update-check-failure`、最外殻 `describe('scenario:s14-update-check-failure')`。
- [3 DDD / 4 副作用境界 / 7 冗長性] PASS: seam は debug build 限定、新規抽象の過剰なし。

## 総合判定 (Pass 3)

**PASS**

理由:
1. 全 4 Then のうち 3 件が E2E で VERIFIED、「ログ出力」は E2E 原理的に不可のため unit test (tests.rs:805, :839) を代替担保として file:line 併記し N/A(代替担保)、実在と assertion 内容を確認。UNVERIFIED なし。
2. test-points TP1〜TP6 は test-points-map.md と完全一致、UNCOVERED / 代替なし N/A なし。
3. mode checklist 全 PASS。残るのは LOW 2 件のみ (non-blocking)。

verdict: PASS
