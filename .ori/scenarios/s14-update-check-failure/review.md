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
