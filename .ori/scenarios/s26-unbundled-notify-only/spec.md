---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s26-unbundled-notify-only
      hash: a47549dec6ff
    - path: domain/workflows/install-update.md#install-update
      hash: 4b291a6c25a9
    - path: domain/aggregates.md#update-channel-aggregate
      hash: 0549c2d38dd2
    - path: domain/ui-fields/screen-3.md#screen-3
      hash: ddf71dd05ef4
    - path: domain/ui-fields/page-groups.md#widget-update-toast
      hash: f77c8a834634
---

# s26-unbundled-notify-only — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s26-unbundled-notify-only phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

updater が対応しない配布形式（`InstallationKind::Unbundled`: nix package 等）では、新バージョンを **通知するだけ** で
インストール操作を提示せず、仮に install が要求されても `UnsupportedInstallation` で拒否してネットワークに触れない（I-U6）ことを
E2E で検証する scenario。

- 対応 workflow: `domain/workflows/install-update.md#install-update`（step 1 `ensureInstallable`: `Unbundled` は
  `UnsupportedInstallation` で終了し、ネットワークには触れない / query `isUpdateInstallable`）
- 対応 aggregate: `domain/aggregates.md#update-channel-aggregate`（I-U6、`InstallationKind::is_installable()`）
- 対応 UI: `domain/ui-fields/screen-3.md#screen-3`（`screen-3-install-update` は `is_installable()` のときだけ表示。
  `#cross-install-update`: false のときは View release / dismiss のみ）
- 対応 page: `page-main`（`WidgetUpdateToast` の composition root）

> domain/validation.md#s26-unbundled-notify-only より:

- Given: `InstallationKind::Unbundled`（nix package 等、bundle 種別を持たないビルド）/ 起動時の確認で新バージョンを検出
- When: Toast が表示される
- Then: `InstallationKind::is_installable() == false` のため、Toast に「Update & restart」は表示されない（View release と dismiss のみ、I-U6）/
  仮に install が要求されても `UnsupportedInstallation` で拒否し、ネットワークには触れない

## シナリオステップ {#scenario-steps}

```gherkin
Scenario: S26 非対応の配布形式では通知のみ
  Given InstallationKind::Unbundled（nix package 等、bundle 種別を持たないビルド）
  And 起動時の確認で新バージョンを検出
  When Toast が表示される
  Then InstallationKind::is_installable() == false のため、Toast に「Update & restart」は表示されない（View release と dismiss のみ、I-U6）
  Then 仮に install が要求されても UnsupportedInstallation で拒否し、ネットワークには触れない
```

E2E での具体化（[#impl-notes](#impl-notes) 参照）:

### Given {#given}

1. 実 user 環境から隔離した temp dir で Tauri App（`--no-bundle` の test build）が起動済み。配布形式は `Unbundled`
   （debug build の既定。配布形式の seam は使わない）
2. 更新確認は local server に向いており、current より新しい `9.9.8` を返す。起動時の確認で Toast が表示される

### When {#when}

1. Toast が表示される
2. （UI からは要求できないため）install を Tauri 境界で `install_update` invoke として要求する

### Then {#then}

- When 1: `is_update_installable` は `false`。Toast に `widget.widget-update-toast.install-update` と `install-status` は存在せず、
  `view-release` と `dismiss` は表示・押下可能
- When 2: `install_update` は `kind == "UnsupportedInstallation"` で reject する。その前後で local server への request
  （latest.json / download）は増えない。偽 AppImage は置き換わらない。Toast は通知のまま残る

## テスト観点 {#test-points}

- **TP1 — Given の成立**: `--no-bundle` の test build は `Unbundled`（`is_update_installable == false`）で、起動時の確認で Toast が表示される
- **TP2（核）— 通知のみ**: Toast に「Update & restart」が表示されず、View release と dismiss のみ使える（I-U6、screen-3 `#cross-install-update`）
- **TP3 — install 要求の拒否**: install が要求されても `UnsupportedInstallation` で拒否する（install-update step 1）
- **TP4 — ネットワークに触れない**: 拒否の前後で更新配信 server への request が増えない（最新リリースの取得し直しも download もしない）
- **TP5 — release build（nix package）の配布形式判定**: bundle 種別を持たない release build が `Unbundled` になる写像
  （`bundle_type() == None` → `Unbundled`）は install-update slice unit test で担保する（E2E は debug build で実行するため）

## 実装ノート {#impl-notes}

- **runner**: `wdio`（優先チェーン step 2 — manifest に `runner:` 明示なし。参加 app `promptnotes` は `local` 系で `runtime.runner = wdio`）
- **infrastructure**: `promptnotes` のみ（`mode: local`）。compose-service 系 app が参加しないため docker-compose は不要
- **runner config**: `wdio.conf.ts`。`tauri:options.application` は runtime block の `binary` =
  `apps/promptnotes/src-tauri/target/debug/app`（build-then-test）
- **配布形式**: debug build は `Unbundled` が既定（macOS の dev build でボタンが出ないようにするため）。S26 は配布形式の seam
  （`TAURI_TEST_INSTALLATION_KIND`）を使わず、`onPrepare` で明示的に unset する
- **test seam（debug build 限定。release build では compile out）**: `TAURI_TEST_UPDATER_ENDPOINT`（更新確認の取得先、s14 / s23 と共通）のみ
- **ネットワーク不接触の観測**: server は path ごとの request 数を JSON ファイルに記録する。`/download` は呼ばれたら HTTP 500 を返す
- **安全策**: 万一 install まで進んでも `target/debug/app` を上書きしないよう、`onPrepare` が `APPIMAGE` を temp の偽 AppImage に向け、
  テストはその不変を確認する
- **前提条件（test readiness）**:
  - **build 契約**: `bun run build:test`（`VITE_WDIO_TEST=1 bun run tauri build --debug --no-bundle`）で `runtime.binary` を作る。
    `cargo build` 単体は devUrl 参照の dev binary になるため不可
  - **plugin 前提**: `tauri-plugin-wdio` の app 側配線（Cargo dep / capabilities `wdio:default` / `lib.rs` の `#[cfg(debug_assertions)]`
    登録 / frontend の `VITE_WDIO_TEST` 動的 import）は既存
  - **storage 隔離**: `wdio.conf.ts` の `onPrepare` が per-run temp root を採番し `XDG_CONFIG_HOME` / `XDG_DATA_HOME` /
    `XDG_CACHE_HOME` / `XDG_STATE_HOME` を root 配下に向ける（`ORI_SCENARIO_TMP` で公開。Linux のみ）。
    `app_config_dir/settings.json` に temp の `storage_dir` を seed する
  - **node_modules 解決**: `.ori/scenarios/node_modules` → `apps/promptnotes/node_modules` の symlink
  - **fixture seed**: Note の seed は不要（更新通知のみを検証する）
- **platform key**: `latest.json` の `platforms` は linux x86_64（`linux-x86_64`）
