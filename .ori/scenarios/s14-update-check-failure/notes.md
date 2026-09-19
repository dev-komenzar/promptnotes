# s14-update-check-failure — Scenario implementation notes

## 概要 {#overview}

アプリ起動時の更新チェックが **HTTP 失敗しても silent に握り潰され**（S14 / C-CFU1 / C-CFU3）、
`NewVersionDetected` を発行しない（I-U3）ため更新通知 Toast が mount されず、UI 通知なし
（ログ出力のみ）でユーザの作業を妨げない、ことを E2E で検証する。

## テスト方式: debug endpoint seam + 実 HTTP 失敗 {#test-approach}

- **失敗再現の課題**: 実 GitHub Releases endpoint への到達を E2E で断つのは不安定（実ネットワーク
  依存）。かといって「fake の error を直接 inject」すると reqwest の接続失敗経路を通らない。
- **採用した方法**: **debug build 限定の endpoint override seam** を `TauriUpdaterPort` に追加し、
  E2E は実 HTTP 交換を local server に向ける。
  - seam: 環境変数 `TAURI_TEST_UPDATER_ENDPOINT`（`apps/promptnotes/src-tauri/src/update_distribution/
    slices/check_for_updates/infrastructure.rs`）。`#[cfg(debug_assertions)]` ガードにより
    **release build では分岐ごと compile out** され、本番 endpoint（`tauri.conf.json`）は不変。
  - `wdio.conf.ts` の `onPrepare` が `127.0.0.1` に小さな `node:http` server を起動し、
    `TAURI_TEST_UPDATER_ENDPOINT=http://127.0.0.1:<port>/latest.json` を app に渡す。
  - server は **mode ファイル**（`fail` / `release`）を request 毎に読み、応答を切り替える:
    - `fail`（既定）: `res.socket.destroy()` で接続を切断 = ネットワーク断相当 →
      `tauri_plugin_updater::Error::Reqwest` → `UpdateError::NetworkError` → `execute` が
      silent に `latest_release: None` へ降格
    - `release`: 有効な `latest.json`（`version: 9.9.9`, `platforms["linux-x86_64"]`）を返す →
      `NewVersionDetected` 発行 → `widget-update-toast` が mount（step 4 の陽性対照）
  - テストは `TAURI_TEST_S14_UPDATER_MODE_FILE` のパスへ mode を書いて切替える。
- **Tauri 境界 invoke**: `browser.tauri.execute(({ core }) => core.invoke('check_for_updates'))`
  （`@wdio/tauri-service` v1.4.0。`tauri-plugin-wdio` は debug build で有効 — `VITE_WDIO_TEST=1`
  必須。test build は `bun run build:test`）。
- **陽性対照 (step 4)**: 失敗 inject が本当に失敗を引き起こしていること、および step 2 の
  「Toast 不在」が空振りでないことを示す。成功時は実際に Toast が mount されることを確認する。

## テストカバレッジ {#coverage}

| 検証項目 | 検証方法 |
|---|---|
| 更新チェック失敗は reject せず `latest_release=null`（核 / C-CFU1 / S14） | E2E step 1（mode=fail → invoke → `current_version=0.2.1`, `latest_release=null`） |
| `NewVersionDetected` 非発行 → UI 通知なし（I-U3 / screen-3 / page-groups failure mode） | E2E step 2（`[data-testid="widget-update-toast"]` が `isExisting()==false`） |
| ユーザの作業を妨げない | E2E step 3（seed Note がフィードに残存・永続化ファイル無傷） |
| 陽性対照（非空虚性）: 成功時は event → Toast mount | E2E step 4（mode=release → `latest_release.version=9.9.9` → Toast + `screen-3-latest-version=9.9.9`） |
| Rust 層 silent failure（C-CFU1..C-CFU6 / I-U2） | `check-for-updates` slice unit test（`tests.rs` TP-S14-1..5, TP-I3, TP-T1）— 20 passed |

## RED → GREEN 検証 {#red-green}

E2E が空振りでないことを示すため、production を一時変異させて RED を実測した:

- **変異 A（`check_for_updates/application.rs` の `Err` 分岐を非 silent 化）**: 失敗時に
  fabricated `NewVersionDetected` を publish し `UpdateChannel::with_release` を返す →
  `bun run build:test` → E2E **2 failing / 2 passing**:
  - step 1: `Expected: null` に対し `Received: {"version":"0.2.1", ...}`（失敗が silent でない）
  - step 2: `widget-update-toast` `Expected: false` に対し `Received: true`（event が発行された）
  - step 3 / step 4 は pass（失敗 inject と成功経路は別軸）
- **復元後** → 再ビルド → E2E **4 passing (~1.8s)** で GREEN。

> 注意: E2E は必ず `bun run build:test`（`VITE_WDIO_TEST=1 bun run tauri build --debug --no-bundle`）
> で作った binary に対して実行すること。`cargo build` 単体では frontend の test setup
> （`@wdio/tauri-plugin` import）が含まれず `browser.tauri.execute` が機能しない。

production への正味の変更は **1 file**:
`apps/promptnotes/src-tauri/src/update_distribution/slices/check_for_updates/infrastructure.rs`
（debug seam のみ。release build は compile out されるため本番挙動に差なし）。

回帰確認:

- frontend unit test: **143 passed / 15 files**
- Rust `update_distribution` unit: **20 passed**
- E2E 回帰: s13-quit-flush **3 passing**、s12-startup-state **1 passing**
- Rust 全体 (`cargo test`): 328 passed / 2 failed。失敗 2 件
  (`note_feed::slices::list_feed::tests::tp_f5_last_7_days` / `tp_f6_and_composition`) は
  **fixture が 2026-06 固定・`now` が実時計依存** の date-range テストで、本変更を stash した
  クリーン HEAD でも同様に fail する **既存の time-bomb**（本 scenario とは無関係）。

## 既知の未対応 / 補足 {#known-issues}

- **起動時 invoke wiring は未実装**: production の frontend は現状 `check_for_updates` を起動時に
  invoke していない（command は `invoke_handler` 登録済み）。本 scenario は validation の When
  「`UpdateChannel::check_at_startup()` 実行」を Tauri 境界 invoke で再現する。起動時 wiring は
  本 scenario のスコープ外（follow-up 候補）。
- **platform key は build target 依存**: 陽性対照の `latest.json` は linux x86_64 の
  `platforms["linux-x86_64"]` を使う。別 OS/arch で走らせる場合は key を合わせる必要がある
  （`bundle_type()` が None の debug no-bundle build では installer suffix なしの key が使われる）。
- **`signature` は check では検証されない**: Tauri updater は署名を install 時に検証するため、
  `check()` 経路では dummy signature で問題ない（本 scenario は install しない）。
- **storage_dir 隔離**: `TAURI_TEST_STORAGE_DIR` override + `XDG_CONFIG_HOME`/`XDG_DATA_HOME` で
  実 user 環境を汚さない。`onComplete` で updater server を close し temp dir を `rmSync`。
