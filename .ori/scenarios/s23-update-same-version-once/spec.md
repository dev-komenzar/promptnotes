---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s23-update-same-version-once
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
      hash: f5d0be02c55e
---

# s23-update-same-version-once — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s23-update-same-version-once phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

更新確認は起動時と、以降 24 時間ごとの定期確認で走る（I-U3）。同じ新バージョンが確認のたびに
見つかっても、**同一プロセス内では 1 回しか通知しない**（I-U4）ことを E2E で検証する scenario。
通知済みより新しいバージョンが見つかれば改めて通知する。

- 対応 workflow: `domain/workflows/check-for-updates.md#check-for-updates`
  （step 6 `suppressNotified`: 同一プロセス内の通知済みバージョン以下なら event 非発行で終了。
  step 7 `emit`: 発行したバージョンを通知済みとして記録する（揮発））
- 対応 aggregate: `domain/aggregates.md#update-channel-aggregate`（I-U3 / I-U4。抑止は application service が
  揮発 state で担い、aggregate は 1 回分の確認結果を返すだけ）
- 対応 event: `domain/domain-events.md#new-version-detected`（同一バージョン（またはそれ以下）について
  2 回目以降は発行しない）
- 対応 UI: `domain/ui-fields/screen-3.md#screen-3`（`#cross-display-timing`: 同一バージョンの Toast は
  1 プロセスにつき 1 回のみ / `#cross-dismiss-view`: dismiss 後は次回起動時またはより新しいバージョン検出時に再表示）
- 対応 page: `page-main`（`WidgetUpdateToast` の composition root）

> domain/validation.md#s23-update-same-version-once より:

- Given: current_version = `0.2.2` / 起動時の確認で `0.2.3` を検出し `NewVersionDetected { latest_version: 0.2.3 }` を
  発行済み / ユーザは Toast を dismiss 済み（アプリは終了していない）
- When: 1. 24 時間後の定期確認で再び `0.2.3` を検出 2. さらに 24 時間後の定期確認で `0.2.4` を検出
- Then: When 1 は通知済みのため event 非発行（UI 通知なし）/ When 2 は `0.2.4` を通知し Toast 表示 /
  再起動した場合は通知済み記録が消えるため起動時の確認で再び通知される

## シナリオステップ {#scenario-steps}

```gherkin
Scenario: S23 通知済みバージョンは定期確認で再通知しない
  Given current_version = 0.2.2
  And 起動時の確認で 0.2.3 を検出し、NewVersionDetected { latest_version: 0.2.3 } を発行済み
  And ユーザは Toast を dismiss 済み（アプリは終了していない）
  When 前回確認から 24 時間経過し、定期確認で UpdateChannel::check() を実行して再び 0.2.3 を検出する
  Then latest_release = Some(0.2.3) だが通知済みのため NewVersionDetected は発行されず、UI 通知なし
  When さらに 24 時間後の定期確認で 0.2.4 を検出する
  Then 0.2.4 は未通知のため NewVersionDetected { latest_version: 0.2.4 } を発行し、Toast を表示する
  Then アプリを再起動した場合は通知済み記録が消えるため、起動時の確認で再び通知される
```

E2E での具体化（バージョン番号は test build の current_version に合わせて読み替える。
[#impl-notes](#impl-notes) 参照）:

### Given {#given}

1. 実 user 環境から隔離した temp dir で Tauri App（test build）が起動済み。current_version は
   ビルド時 version（`CARGO_PKG_VERSION`）
2. 更新確認 endpoint は local server に向いており、current より新しい `9.9.8`（domain 例の `0.2.3` 相当）を返す
3. 起動時に page-main が listen 確立後 `check_for_updates` を自動 invoke → `9.9.8` の Toast が表示される
   （= `NewVersionDetected { latest_version: 9.9.8 }` 発行済み）
4. ユーザが Toast を dismiss する（`widget.widget-update-toast.dismiss` クリック）

### When {#when}

1. 定期確認相当: endpoint が引き続き `9.9.8` を返す状態で、確認処理（`check_for_updates`）を
   Tauri 境界 invoke で実行する
2. 定期確認相当: endpoint を `9.9.9`（domain 例の `0.2.4` 相当）に切り替え、確認処理を Tauri 境界 invoke で実行する

### Then {#then}

- When 1: invoke は `latest_release.version == "9.9.8"` を返す（I-U2 上は Some のまま）が、Toast
  （`widget.widget-update-toast.root`）は再表示されない（I-U4: 通知済み）
- When 2: invoke は `latest_release.version == "9.9.9"` を返し、Toast が `9.9.9` で表示される
- 再起動後の再通知: E2E ではプロセス再起動を扱わず、slice unit test で代替担保する（[#test-points](#test-points)）

## テスト観点 {#test-points}

- **TP1 — 起動時の確認で通知される（Given の成立）**: endpoint が `9.9.8` を返す状態で起動すると、page-main の
  自動 invoke により Toast が `9.9.8` で表示される。dismiss で Toast は消える（screen-3 `#cross-dismiss-view`）
- **TP2（核）— 通知済みバージョンは再通知しない**: dismiss 後に同じ `9.9.8` を返す確認を実行すると、
  invoke 結果は `latest_release.version == "9.9.8"` だが、Toast は DOM に再出現しない（I-U4 / C-CFU8）
- **TP3 — より新しいバージョンは通知する**: endpoint を `9.9.9` に切り替えて確認すると、Toast が `9.9.9` で
  表示される（S23 When 2 / I-UT4 再表示）。TP2 の「Toast 不在」が空振り（event 経路の故障）でないことの陽性対照も兼ねる
- **TP4 — 再起動後は再び通知される（I-U4 揮発）**: E2E では扱わない。check-for-updates slice unit test
  `tests.rs` TP-D4（新しい state では同じバージョンを再通知）で代替担保する
- **TP5 — 24 時間周期で確認が走る（I-U3）**: E2E では 24 時間待てないため、確認処理を Tauri 境界 invoke で
  再実行して代替する。周期判定は slice unit test `tests.rs` TP-P1〜TP-P7 で代替担保する

## 実装ノート {#impl-notes}

- **runner**: `wdio`（優先チェーン step 2 — manifest に `runner:` 明示なし。参加 app `promptnotes` は
  `local` 系で `runtime.runner = wdio`）
- **infrastructure**: `promptnotes` のみ（`mode: local`）。compose-service 系 app が参加しないため docker-compose は不要
- **runner config**: `wdio.conf.ts`。`tauri:options.application` は runtime block の `binary` =
  `apps/promptnotes/src-tauri/target/debug/app`（build-then-test）
- **バージョンの読み替え**: domain 例は current `0.2.2` / 通知済み `0.2.3` / 新版 `0.2.4`。test build の
  current_version はビルド時 version（現行 `0.2.3`）なので、E2E は current より確実に新しい `9.9.8` / `9.9.9` を使う。
  関係（current < 通知済み < 新版）は同じ
- **endpoint の差し替え（test seam）**: s14 と同じ debug build 限定 seam `TAURI_TEST_UPDATER_ENDPOINT`
  （`check_for_updates/infrastructure.rs`、release build では compile out）。wdio `onPrepare` が 127.0.0.1 に
  `node:http` server を立て、version ファイルの内容（`9.9.8` / `9.9.9`）を version に持つ有効な `latest.json` を返す。
  テストは version ファイルを書き換えて応答を切り替える
- **定期確認の代替**: 24 時間ごとの定期確認は Rust 背景 task（`spawn_periodic_update_check`）が
  前回確認時刻の記録 + 確認処理（`execute_update_check`）を呼ぶ。E2E は同じ確認処理を
  `browser.tauri.execute(({ core }) => core.invoke('check_for_updates'))` で実行する。通知済み state は
  command 経路と定期 task 経路で共有される（C-CFU12）ので、I-U4 の検証として等価
- **Toast 不在の判定**: event は invoke 中に同期 emit される。invoke の resolve 後に短い待機（500ms）を置いてから
  `isExisting()==false` を確認する（emit → listen → render の遅延を吸収）
- **前提条件（test readiness）**:
  - **build 契約**: `bun run build:test`（`VITE_WDIO_TEST=1 bun run tauri build --debug --no-bundle`）で
    `runtime.binary` を作る。`cargo build` 単体は devUrl 参照の dev binary になるため不可
  - **plugin 前提**: `tauri-plugin-wdio` の app 側配線（Cargo dep / capabilities `wdio:default` /
    `lib.rs` の `#[cfg(debug_assertions)]` 登録 / frontend の `VITE_WDIO_TEST` 動的 import）は既存
  - **storage 隔離**: `wdio.conf.ts` の `onPrepare` が per-run temp root を採番し `XDG_CONFIG_HOME` /
    `XDG_DATA_HOME` / `XDG_CACHE_HOME` / `XDG_STATE_HOME` を root 配下に向ける（`ORI_SCENARIO_TMP` で公開。Linux のみ）。
    `TAURI_TEST_STORAGE_DIR` は使わず、`app_config_dir/settings.json` に temp の `storage_dir` を seed する
  - **node_modules 解決**: `.ori/scenarios/node_modules` → `apps/promptnotes/node_modules` の symlink
  - **fixture seed**: Note の seed は不要（更新通知のみを検証する）
- **platform key**: `latest.json` の `platforms` は linux x86_64（`linux-x86_64`）。別 OS/arch では key を合わせる
- **production 側の前提**: `check_for_updates` が `Result` を露出せず `UpdateChannelResponse` を返すこと /
  通知済み以下のバージョンでは `latest_release` を返しつつ event を emit しないこと（check-for-updates C-CFU8）/
  `WidgetUpdateToast` が `new_version_detected` 受信のたびに payload を上書きして表示すること（I-UT3 / I-UT4）
