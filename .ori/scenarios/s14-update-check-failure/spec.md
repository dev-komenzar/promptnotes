---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s14-update-check-failure
      hash: a12e27d425ae
    - path: domain/workflows/check-for-updates.md#check-for-updates
      hash: f08ad6cf0874
    - path: domain/aggregates.md#update-channel-aggregate
      hash: e3aba26a3001
    - path: domain/domain-events.md#new-version-detected
      hash: cc1d928f09e4
    - path: domain/ui-fields/screen-3.md#screen-3
      hash: 8da9c543d8da
    - path: domain/ui-fields/page-groups.md#widget-update-toast
      hash: a9fb317174a9
---

# s14-update-check-failure — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s14-update-check-failure phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

更新チェック（`check-for-updates` workflow）が **HTTP 失敗したときに silent に握り潰され、
ユーザの作業を妨げない** ことを E2E で検証する scenario。確認はアプリ起動時と、以降 24 時間ごとの
定期確認で走る（I-U3）。どちらの確認でも失敗時の扱いは同じ。

ネットワーク断の状態で確認が走ったケースを想定する。`TauriUpdaterPort` が `UpdateError` を返しても、
application service の outer layer がそれを握り潰し、`latest_release: None` の `UpdateChannel` を返す
(S14 / I-U2 / C-CFU1 / C-CFU3)。`NewVersionDetected` は発行されないため更新通知 Toast は mount されず
(S14 / screen-3 / page-groups `widget-update-toast` failure mode)、UI 通知は一切出ない（ログ出力のみ）。
フィード表示などの通常操作は継続できる。失敗しても即時リトライはせず、次の確認は通常の 24 時間周期を待つ（I-U3）。

- 対応 workflow: `domain/workflows/check-for-updates.md#check-for-updates`
  （起動時 + 24 時間ごと。失敗は silent。即時リトライなし = I-U3）
- 対応 aggregate: `domain/aggregates.md#update-channel-aggregate`
  （`UpdateChannel::check()` は async network 呼び出し。失敗は application service 内部で
  silent に握り潰し `latest_release: None` を返す。外部 API は `Result` を露出しない。I-U1 / I-U2）
- 対応 event: `domain/domain-events.md#new-version-detected`
  （Trigger は「成功 かつ latest_release: Some(_) かつ未通知」の場合のみ。失敗時は非発行）
- 対応 UI: `domain/ui-fields/screen-3.md#screen-3`（NewVersionDetected 受信時のみ Toast 表示。
  失敗時は silent）/ `domain/ui-fields/page-groups.md#widget-update-toast`
  （`NewVersionDetected` 未発行時は mount されない）
- 対応 page: `page-main`（`WidgetUpdateToast` の composition root。listen 確立後に起動時の
  `check_for_updates` を invoke する）

> domain/validation.md#s14-update-check-failure より:

- Given: アプリ起動時、または前回確認から 24 時間経過した定期確認時（I-U3）/ ネットワーク断
- When: `UpdateChannel::check()` 実行
- Then:
  - HTTP 失敗 → `UpdateError` を application service が握り潰す
  - event **NewVersionDetected** は **発行されない**（S14 / domain-events.md 参照）
  - UI 通知なし、ログ出力のみ
  - ユーザの作業を妨げない

> domain/workflows/check-for-updates.md#error-handling より:
> 「すべての `UpdateError` を application service の outer layer で握り潰す」「ログは出すが
> UI 通知はしない（S14: silent）」「失敗時の即時リトライは行わない。次の確認は通常の 24 時間周期を待つ（I-U3）」

> domain/aggregates.md#update-channel-aggregate-operations より:
> 「失敗は **application service 内部で silent に握り潰し**、`latest_release: None` の
> `UpdateChannel` を返す」「内部実装は `Result<UpdateChannel, UpdateError>` を持つ private fn を
> 経由してよいが、外部 API は **`Result` を露出しない**」

> domain/ui-fields/page-groups.md#widget-update-toast より:
> 「**failure mode**: `NewVersionDetected` 未発行時は **mount されない** (silent, S14)」

## シナリオステップ {#scenario-steps}

```gherkin
Scenario: S14 更新チェック失敗は silent
  Given アプリ起動時、または前回確認から 24 時間経過した定期確認時
  And ネットワーク断
  When UpdateChannel::check() を実行する
  Then HTTP 失敗の UpdateError を application service が握り潰す
  Then event NewVersionDetected は発行されない
  Then UI 通知なし、ログ出力のみ
  Then ユーザの作業を妨げない
```

E2E での具体化:

### Given {#given}

1. 実 user 環境から隔離した temp dir に Note を seed 済み。Tauri App（test build）が起動済み
2. 更新チェックの HTTP 呼び出しが **失敗する** 状態（ネットワーク断相当）にある
   （[#impl-notes](#impl-notes) の test seam で再現。実 production の endpoint は変更しない）。
   起動時には page-main が listen 確立後に `check_for_updates` を invoke する（ori-m2ra.1）ため、
   起動時の確認もこの失敗状態で走る

### When {#when}

1. Tauri 境界（`check_for_updates` invoke）で更新チェックを実行する
   - 起動時の確認（page-main の自動 invoke）と、24 時間後の定期確認（Rust 背景 task が同じ確認処理を呼ぶ）を
     代表して、同じ確認処理を Tauri 境界 invoke で決定的に再実行する（実時間で 24 時間待たない。
     周期の判定は check-for-updates slice の unit test TP-P* が担保）

### Then {#then}

- invoke は **throw せず** `{ current_version: <ビルド時 version>, latest_release: null }` を返す
  （ビルド時 version は `get_app_version` を Tauri 境界 invoke して取得し、テストに version を直書きしない）
  （C-CFU1: `Result` を露出しない。silent degradation。I-U1: current_version はビルド時定数）
- `NewVersionDetected` は発行されない → 更新通知 Toast（`data-testid="widget.widget-update-toast.root"`）は
  **DOM に存在しない**（S14 / screen-3 / page-groups failure mode）。起動時の自動確認も失敗しているので
  起動直後から Toast は無い
- エラー UI 通知は出ない（ログ出力のみ）
- ユーザの作業は妨げられない: seed 済み Note がフィードに表示され続け、ウィンドウは生きたまま

## テスト観点 {#test-points}

- **TP1（核）— 更新チェック失敗は silent**: ネットワーク断相当の状態で `check_for_updates` を
  invoke すると、reject せず `latest_release: null` を返す（`UpdateChannel::without_release` への
  正規化）。`current_version` はビルド時 version（I-U1 immutable）
- **TP2 — NewVersionDetected 非発行 / UI 通知なし**: 起動時の自動確認と上記 invoke の失敗後、
  `widget.widget-update-toast.root` が DOM に存在しない（event 非発行 → 購読側 store の payload が `null` のまま）
- **TP3 — ユーザの作業を妨げない**: 失敗後もフィードに seed 済み Note が表示され、ウィンドウが
  存続する
- **TP4（陽性対照 / 非空虚性）— 成功時は通知される**: 同じ endpoint を「有効な release を返す」
  モードに切り替えて `check_for_updates` を invoke すると `latest_release` が `Some`（version > current）
  になり、`widget.widget-update-toast.root` が mount される。失敗した確認は通知済み state を変えないので
  （C-CFU9）、失敗の後でも初回の新版は通知される。これにより TP1/TP2 の inject が本当に失敗を
  引き起こしており、TP2 の「Toast 不在」が空振りでないことを示す
- **TP5 — 即時リトライなし / 24 時間周期**: 失敗後に同一確認内で再試行しないこと、次の確認が 24 時間後に
  なることは E2E では待てないため、check-for-updates slice の unit test（`tests.rs` TP-R2 / TP-P2 / TP-P5 / TP-P6）で担保する
- **TP6 — ログ出力のみ（S14 Then「UI 通知なし、ログ出力のみ」の「ログ出力」）**: E2E では app のログを読まず、
  check-for-updates slice の unit test（`tests.rs` TP-S14-6: 失敗時に `update check failed` の warn ログが 1 件、
  TP-S14-7: 成功時は出さない）で代替担保する。「UI 通知なし」は TP2 が E2E で検証する
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
- **起動時 invoke / 定期確認**: production の frontend（page-main）は listen 確立後に起動時の
  `check_for_updates` を自動で invoke する（ori-m2ra.1）。mode ファイルの既定は `fail` なので、起動時の
  自動確認も失敗する（Toast 無し）。24 時間ごとの定期確認は Rust 背景 task が同じ確認処理を呼ぶが、
  E2E では待たず Tauri 境界 invoke で確認処理を再実行する（周期は slice unit test が担保）
- **config dir / storage dir の隔離**: `wdio.conf.ts` の `onPrepare` が `XDG_CONFIG_HOME` /
  `XDG_DATA_HOME` と専用 `storage_dir` を temp dir に隔離する（実 user 環境を汚さない）
- **production 側の前提（本 scenario が満たすべき不変条件）**:
  1. `check_for_updates` Tauri command が `Result` を露出せず `UpdateChannelResponse` を返すこと
     （失敗時 `latest_release: null`）— `src-tauri/src/update_distribution/slices/check_for_updates/`
  2. `TauriUpdaterPort` が更新チェックを行い失敗を `UpdateError` に写像すること
  3. `TauriEventBus` は成功時のみ `new_version_detected` を emit すること
  4. frontend `WidgetUpdateToast` が `new_version_detected` を購読し payload 受信時のみ mount されること
  5. 失敗した確認が通知済み state（I-U4）を変えないこと（TP4 の前提、C-CFU9）
  これらが欠落すると S14 が RED になる
