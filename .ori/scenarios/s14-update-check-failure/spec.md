---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s14-update-check-failure
      hash: 4a02c17cc025
    - path: domain/workflows/check-for-updates.md#check-for-updates
      hash: e2935ac617af
    - path: domain/aggregates.md#update-channel-aggregate
      hash: 56f7a54a8ab2
    - path: domain/domain-events.md#new-version-detected
      hash: 71db66eafe03
    - path: domain/ui-fields/screen-3.md#screen-3
      hash: 147c00e5bcc8
    - path: domain/ui-fields/page-groups.md#widget-update-toast
      hash: 8986615249ac
---

# s14-update-check-failure — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s14-update-check-failure phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

アプリ起動時に走る更新チェック（`check-for-updates` workflow）が **HTTP 失敗したときに
silent に握り潰され、ユーザの作業を妨げない** ことを E2E で検証する scenario。

ユーザがネットワーク断の状態でアプリを起動したケースを想定する。`TauriUpdaterPort` が
`UpdateError` を返しても、application service の outer layer がそれを握り潰し、
`latest_release: None` の `UpdateChannel` を返す (S14 / I-U2 / C-CFU1 / C-CFU3)。
`NewVersionDetected` event は発行されないため更新通知 Toast は mount されず (I-U3 /
screen-3 / page-groups `widget-update-toast` failure mode)、UI 通知は一切出ない（ログ出力のみ）。
結果としてフィード表示などの通常操作が継続できる。

- 対応 workflow: `domain/workflows/check-for-updates.md#check-for-updates`
  （起動時 1 回のみ。失敗は silent failure。リトライ / 常駐 polling なし = I-U3）
- 対応 aggregate: `domain/aggregates.md#update-channel-aggregate`
  （`UpdateChannel::check_at_startup()` は async network 呼び出し。失敗は application service 内部で
  silent に握り潰し `latest_release: None` を返す。外部 API は `Result` を露出しない。I-U1 / I-U2）
- 対応 event: `domain/domain-events.md#new-version-detected`
  （Trigger は「成功 かつ latest_release: Some(_)」の場合のみ。失敗時は非発行）
- 対応 UI: `domain/ui-fields/screen-3.md#screen-3`（NewVersionDetected 受信時のみ Toast 表示。
  失敗時は silent）/ `domain/ui-fields/page-groups.md#widget-update-toast`
  （`NewVersionDetected` 未発行時は mount されない）
- 対応 page: `page-main`（`WidgetUpdateToast` の composition root。event 購読のみを担う）

> domain/validation.md#s14-update-check-failure より:

- Given: アプリ起動直後 / ネットワーク断
- When: `UpdateChannel::check_at_startup()` 実行
- Then:
  - HTTP 失敗 → `UpdateError` を application service が握り潰す
  - event **NewVersionDetected** は **発行されない**（I-U3）
  - UI 通知なし、ログ出力のみ
  - ユーザの作業を妨げない

> domain/workflows/check-for-updates.md#error-handling より:
> 「すべての `UpdateError` を application service の outer layer で握り潰す」「ログは出すが
> UI 通知はしない（S14: silent）」「リトライ・常駐 polling は行わない（I-U3）」

> domain/aggregates.md#update-channel-aggregate-operations より:
> 「失敗は **application service 内部で silent に握り潰し**、`latest_release: None` の
> `UpdateChannel` を返す」「内部実装は `Result<UpdateChannel, UpdateError>` を持つ private fn を
> 経由してよいが、外部 API は **`Result` を露出しない**」

> domain/ui-fields/page-groups.md#widget-update-toast より:
> 「**failure mode**: `NewVersionDetected` 未発行時は **mount されない** (silent, S14)」

## シナリオステップ {#scenario-steps}

### Given {#given}

1. 実 user 環境から隔離した temp dir に Note を seed 済み。Tauri App（test build）が起動済み
2. 更新チェックの HTTP 呼び出しが **失敗する** 状態（ネットワーク断相当）にある
   （[#impl-notes](#impl-notes) の test seam で再現。実 production の endpoint は変更しない）

### When {#when}

1. Tauri 境界（`check_for_updates` invoke）で更新チェックを 1 回実行する
   - production では frontend が起動時に `check_for_updates` を invoke する経路が想定されるが、
     現状 frontend からの起動時 invoke wiring は未実装のため、scenario は Tauri 境界 invoke で
     起動直後の実行を再現する（[#impl-notes](#impl-notes) の制約参照）

### Then {#then}

- invoke は **throw せず** `{ current_version: "0.2.1", latest_release: null }` を返す
  （C-CFU1: `Result` を露出しない。silent degradation）
- `NewVersionDetected` は発行されない → 更新通知 Toast（`data-testid="widget-update-toast"`）は
  **DOM に存在しない**（I-U3 / screen-3 / page-groups failure mode）
- エラー UI 通知は出ない（ログ出力のみ）
- ユーザの作業は妨げられない: seed 済み Note がフィードに表示され続け、ウィンドウは生きたまま

## テスト観点 {#test-points}

- **TP1（核）— 更新チェック失敗は silent**: ネットワーク断相当の状態で `check_for_updates` を
  invoke すると、reject せず `latest_release: null` を返す（`UpdateChannel::without_release` への
  正規化）。`current_version` は `0.2.1`（I-U1 immutable）
- **TP2 — NewVersionDetected 非発行 / UI 通知なし**: 上記失敗後、`widget-update-toast` が DOM に
  存在しない（event 非発行 → 購読側 store の payload が `null` のまま）
- **TP3 — ユーザの作業を妨げない**: 失敗後もフィードに seed 済み Note が表示され、ウィンドウが
  存続する
- **TP4（陽性対照 / 非空虚性）— 成功時は通知される**: 同じ endpoint を「有効な release を返す」
  モードに切り替えて `check_for_updates` を invoke すると `latest_release` が `Some`（version > current）
  になり、`widget-update-toast` が mount される。これにより TP1/TP2 の inject が本当に失敗を
  引き起こしており、TP2 の「Toast 不在」が空振りでないことを示す
  （event → Toast の購読は `store.test.ts` の unit test とも整合）
- **check-for-updates slice unit test との分担**: 本 scenario は Update Distribution BC の 1 use case
  のみを対象とする。`check-for-updates` slice の unit test
  （`src-tauri/.../check_for_updates/tests.rs` の TP-S14-1..5, TP-I3, TP-T1）が Rust 層の silent
  failure を既に担保しており、本 scenario は **Tauri 境界 + UI までの縦断** を検証する

## 実装ノート {#impl-notes}

- **runner**: `wdio`（優先チェーン step 2 — manifest に `runner:` 明示なし。参加 app `promptnotes` は
  `local` 系で `runtime.runner = wdio`。`.ori/architecture.md` の `scenario_test_runner: wdio` とも一致）
- **infrastructure**: `promptnotes` のみ（`mode: local`）。compose-service 系 app が参加しないため
  docker-compose は不要
- **runner config**: `wdio.conf.ts` が `/ori-generate` により生成される。`tauri:options.application` は
  runtime block の `binary` = `apps/promptnotes/src-tauri/target/debug/app`（build-then-test）
- **ネットワーク失敗の再現（test seam）**: 実 GitHub Releases への到達を断つのは E2E では不安定なため、
  **debug build 限定の endpoint override** を `TauriUpdaterPort` に追加する
  （`TAURI_TEST_UPDATER_ENDPOINT`）。wdio `onPrepare` が 127.0.0.1 上の小さな HTTP server を立て、
  mode ファイル (`fail` / `release`) で応答を切り替える:
  - `fail`: socket を切断（ネットワーク断相当）→ `tauri_plugin_updater::Error::Reqwest` →
    `UpdateError::NetworkError` → `execute` が silent に `latest_release: None` へ降格
  - `release`: 有効な `latest.json`（current より新しい version）を返す → `NewVersionDetected` 発行 →
    Toast mount（TP4 陽性対照）
  - production (release build) では `#[cfg(debug_assertions)]` により env を一切読まない = 本番挙動は不変
  この方法は notes.md#test-approach にも明記する
- **Tauri 境界 invoke**: `browser.tauri.execute(({ core }) => core.invoke('check_for_updates'))`
  （`@wdio/tauri-service` v1.4.0。`tauri-plugin-wdio` は debug build で有効）
- **起動時 invoke wiring の制約**: production の frontend は現状 `check_for_updates` を起動時に
  invoke していない（command は `invoke_handler` 登録済み）。本 scenario は validation の When
  「`UpdateChannel::check_at_startup()` 実行」を Tauri 境界 invoke で再現する。起動時 wiring は
  本 scenario のスコープ外（必要なら follow-up）
- **config dir / storage dir の隔離**: `wdio.conf.ts` の `onPrepare` が `XDG_CONFIG_HOME` /
  `XDG_DATA_HOME` と専用 `storage_dir` を temp dir に隔離する（実 user 環境を汚さない）
- **production 側の前提（本 scenario が満たすべき不変条件）**:
  1. `check_for_updates` Tauri command が `Result` を露出せず `UpdateChannelResponse` を返すこと
     （失敗時 `latest_release: null`）— `src-tauri/src/update_distribution/slices/check_for_updates/`
  2. `TauriUpdaterPort` が更新チェックを行い失敗を `UpdateError` に写像すること
  3. `TauriEventBus` は成功時のみ `new_version_detected` を emit すること
  4. frontend `WidgetUpdateToast` が `new_version_detected` を購読し payload 受信時のみ mount されること
  これらが欠落すると S14 が RED になる
