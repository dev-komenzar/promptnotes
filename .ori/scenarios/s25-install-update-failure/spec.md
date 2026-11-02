---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s25-install-update-failure
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

# s25-install-update-failure — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s25-install-update-failure phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

更新のインストールが失敗したとき、失敗を **ユーザに伝え**（S14 の silent 方針は適用しない）、アプリは再起動せずに継続し、
「Update & restart」を再び押せる状態に戻す（I-U7）ことを E2E で検証する scenario。自動リトライはしない。

- 対応 workflow: `domain/workflows/install-update.md#install-update`（`#errors`: `DownloadFailed` / `InstallFailed`（署名検証失敗を含む）。
  `#error-handling`: すべてのエラーを UI に返し、要求元はエラーを表示して再度押せる状態に戻す。失敗時は再起動しない。
  View release は引き続き使える）
- 対応 aggregate: `domain/aggregates.md#update-channel-aggregate`（I-U7: 署名検証失敗はインストール失敗。失敗時はアプリを継続し
  失敗をユーザに伝える / I-U10: 失敗したら再び要求を受け付ける）
- 対応 UI: `domain/ui-fields/screen-3.md#screen-3`（`#cross-install-update`: 失敗時は `screen-3-install-status = Failed` で
  失敗メッセージを表示し、ボタンを再び押せるようにする。View release は使える）
- 対応 page: `page-main`（page-groups `#widget-update-toast` の failure mode: install-update 失敗時は toast 内に失敗を表示して継続）

> domain/validation.md#s25-install-update-failure より:

- Given: current_version = `0.2.4`、`InstallationKind::Bundled(Deb)` / Toast に `0.2.5` を表示中
- When: ユーザが「Update & restart」を押し、download がネットワークエラーで失敗する（または deb の権限昇格をユーザが拒否して
  install が失敗する）
- Then: `InstallUpdateError::DownloadFailed`（または `InstallFailed`）が UI に返る / アプリは再起動しない。Note の Flush も行わない /
  Toast は失敗メッセージを表示し、「Update & restart」を再度押せる状態に戻る（I-U7）/ 「View release」は引き続き使える。自動リトライはしない

## シナリオステップ {#scenario-steps}

```gherkin
Scenario: S25 更新インストールの失敗はユーザに伝え、アプリは継続
  Given current_version = 0.2.4、InstallationKind::Bundled(Deb)
  And Toast に 0.2.5 を表示中
  When ユーザが「Update & restart」を押し、download がネットワークエラーで失敗する（または install が失敗する）
  Then InstallUpdateError::DownloadFailed（または InstallFailed）が UI に返る
  Then アプリは再起動しない。Note の Flush も行わない
  Then Toast は失敗メッセージを表示し、「Update & restart」を再度押せる状態に戻る（I-U7）
  Then 「View release」は引き続き使える。自動リトライはしない
```

E2E での具体化（バージョン番号は test build の current_version に合わせて読み替える。[#impl-notes](#impl-notes) 参照）:

### Given {#given}

1. 実 user 環境から隔離した temp dir で Tauri App（test build）が起動済み。配布形式は `Bundled(Deb)`
   （debug seam `TAURI_TEST_INSTALLATION_KIND=deb`）
2. 更新配信は local server。起動時の確認で current より新しい `9.9.8`（domain 例の `0.2.5` 相当）の Toast が
   「Update & restart」付きで表示される

### When {#when}

1. download がネットワークエラー（`/download` が HTTP 500）になる状態で「Update & restart」をクリックする
2. 失敗表示から再び「Update & restart」をクリックし、今度は install が失敗する（latest.json の署名が配信パッケージと
   一致せず、updater plugin の署名検証で失敗する）

### Then {#then}

- 各 When の後: `widget.widget-update-toast.install-status` が「Updating…」を経て失敗メッセージ（`role="alert"`）になる。
  install ボタン・View release・dismiss は押せる状態に戻る
- 各 When の後: アプリは再起動していない（Given でテストが window に置いた目印が残っている = 同じ webview / process のまま。
  パッケージも起動されず marker が無い）。偽 AppImage は置き換わっていない
- 各 When の後: 3 秒待っても latest.json / download への request は増えない（自動リトライしない）
- When 1 の失敗種別は `DownloadFailed`、When 2 は `InstallFailed`（UI には種別が出ないため、同じ `install_update` を Tauri 境界
  invoke して `InstallUpdateErrorResponse.kind` を確認する）

## テスト観点 {#test-points}

- **TP1 — Given の成立**: `Bundled(Deb)` のビルドで、起動時の確認で Toast が `9.9.8` で表示され、「Update & restart」が押下可能（I-U6）
- **TP2（核）— download 失敗を伝えて継続**: download 失敗で失敗メッセージを表示し、アプリは再起動せず、
  「Update & restart」を再度押せる状態に戻る。エラー種別は `DownloadFailed`
- **TP3 — install 失敗（署名検証失敗）を伝えて継続**: 失敗後の再押下で install が走り（再度押せることの確認を兼ねる）、署名検証失敗で
  失敗メッセージを表示して継続する。何もインストールされない。エラー種別は `InstallFailed`（I-U7）
- **TP4 — View release は引き続き使える**: 失敗表示中も View release が表示・押下可能
- **TP5 — 自動リトライしない**: 失敗後に待っても updater への request が増えない
- **TP6 — 失敗時は Note の Flush も行わない**: install 失敗時に page-main の Flush / 再起動 hook を呼ばないことは
  widget-update-toast store / page-main の unit test で担保する（E2E では「Update & restart」のクリック自体が editor の blur flush を
  起こすため、Flush の有無を区別できない）
- **TP7 — deb の権限昇格（pkexec）をユーザが拒否した場合の `InstallFailed`**: 実 deb パッケージと対話的な認証が要るため E2E では扱わない。
  install 失敗の写像（install 段の失敗は `InstallFailed`）は install-update slice unit test で担保する

## 実装ノート {#impl-notes}

- **runner**: `wdio`（優先チェーン step 2 — manifest に `runner:` 明示なし。参加 app `promptnotes` は `local` 系で `runtime.runner = wdio`）
- **infrastructure**: `promptnotes` のみ（`mode: local`）。compose-service 系 app が参加しないため docker-compose は不要
- **runner config**: `wdio.conf.ts`。`tauri:options.application` は runtime block の `binary` =
  `apps/promptnotes/src-tauri/target/debug/app`（build-then-test）
- **バージョンの読み替え**: domain 例の新版 `0.2.5` を、test build の current より確実に新しい `9.9.8` として配信する
- **test seam（debug build 限定。release build では compile out）**: s24 と同じ 3 つ。`TAURI_TEST_UPDATER_ENDPOINT`（取得先）/
  `TAURI_TEST_UPDATER_PUBKEY`（署名検証の公開鍵。wdio `onPrepare` が `cargo tauri signer` で実行時に生成）/
  `TAURI_TEST_INSTALLATION_KIND=deb`（配布形式）
- **失敗の再現**: server は mode ファイル（`download-fail` / `bad-signature`）で応答を切り替える。`download-fail` は `/download` を
  HTTP 500（updater の download 失敗 → `DownloadFailed`）、`bad-signature` は latest.json に別ファイルの署名を書く（updater の
  署名検証失敗 → `InstallFailed`）。download は 1 秒遅延させ、処理中表示を観測できるようにする
- **安全策**: updater plugin の Linux install は `APPIMAGE` が指すファイルを置き換える（debug の `--no-bundle` build は bundle 種別を
  持たないため deb ではなく AppImage 経路）。万一 install まで進んでも `target/debug/app` を上書きしないよう、`onPrepare` が
  `APPIMAGE` を temp の偽 AppImage に向け、テストはその不変を確認する
- **前提条件（test readiness）**:
  - **build 契約**: `bun run build:test`（`VITE_WDIO_TEST=1 bun run tauri build --debug --no-bundle`）で `runtime.binary` を作る。
    `cargo build` 単体は devUrl 参照の dev binary になるため不可
  - **plugin 前提**: `tauri-plugin-wdio` の app 側配線（Cargo dep / capabilities `wdio:default` / `lib.rs` の `#[cfg(debug_assertions)]`
    登録 / frontend の `VITE_WDIO_TEST` 動的 import）は既存
  - **storage 隔離**: `wdio.conf.ts` の `onPrepare` が per-run temp root を採番し `XDG_CONFIG_HOME` / `XDG_DATA_HOME` /
    `XDG_CACHE_HOME` / `XDG_STATE_HOME` を root 配下に向ける（`ORI_SCENARIO_TMP` で公開。Linux のみ）。
    `app_config_dir/settings.json` に temp の `storage_dir` を seed する
  - **node_modules 解決**: `.ori/scenarios/node_modules` → `apps/promptnotes/node_modules` の symlink
  - **fixture seed**: Note の seed は不要（更新のインストール失敗のみを検証する）
  - **署名ツール**: `cargo tauri signer`（nix devShell の cargo-tauri）が PATH にあること
- **platform key**: `latest.json` の `platforms` は linux x86_64（`linux-x86_64`）
