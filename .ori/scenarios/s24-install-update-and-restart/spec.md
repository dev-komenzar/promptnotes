---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s24-install-update-and-restart
      hash: a47549dec6ff
    - path: domain/workflows/install-update.md#install-update
      hash: 4b291a6c25a9
    - path: domain/workflows/flush-note.md#flush-note
      hash: 06ace0dff2ff
    - path: domain/aggregates.md#update-channel-aggregate
      hash: 0549c2d38dd2
    - path: domain/domain-events.md#note-body-edited
      hash: 34746af8d1fa
    - path: domain/ui-fields/screen-3.md#screen-3
      hash: ddf71dd05ef4
    - path: domain/ui-fields/page-groups.md#widget-update-toast
      hash: f77c8a834634
---

# s24-install-update-and-restart — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s24-install-update-and-restart phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

更新通知（Toast）の「Update & restart」で、最新リリースを取得し直して download + 署名検証 + install し（I-U5〜I-U7）、
**編集中の Note を Flush して永続化を待ってから**、インストールしたバージョンでアプリを再起動する（I-U8）ことを
E2E で検証する scenario。

- 対応 workflow: `domain/workflows/install-update.md#install-update`（step 3 `fetchLatestUpdate`: 通知時の情報を使い回さず
  取得し直す / step 4 `ensureNewer` / step 5 `downloadAndInstall`: 署名検証は updater plugin に委譲 / 再起動は workflow の外で
  UI 層が「全 pending Note の Flush を await → `restartApp`」の順に行う）
- 対応 workflow: `domain/workflows/flush-note.md#flush-note`（再起動前の Flush は quit 時の S13 と同じ経路）
- 対応 aggregate: `domain/aggregates.md#update-channel-aggregate`（I-U5 / I-U6 / I-U7 / I-U8、`UpdateChannel::install()`）
- 対応 event: `domain/domain-events.md#note-body-edited`（Flush で Note A の編集を永続化する）。install 自体は event を発行しない
- 対応 UI: `domain/ui-fields/screen-3.md#screen-3`（`#cross-install-update`: 押すと「Updating…」、install 中はボタンと dismiss を無効化。
  成功時は page-main が全 pending Note を Flush してから再起動）
- 対応 page: `page-main`（`WidgetUpdateToast` の composition root。再起動 orchestration を持つ。page-groups `#widget-update-toast`）

> domain/validation.md#s24-install-update-and-restart より:

- Given: current_version = `0.2.4`、`InstallationKind::Bundled(AppImage)` / 起動時の確認で `0.2.5` を検出し、Toast を表示中 /
  Note A が EDITING 状態（AutoSave debounce timer 中）
- When: ユーザが Toast の「Update & restart」を押す
- Then: `UpdateChannel::install()` が最新リリースを取得し直し `0.2.5 > 0.2.4` を確認（I-U7）/ updater plugin が download + 署名検証 +
  install を行い `0.2.5` を返す / 処理中は Toast が「Updating…」を表示しボタンは押せない / インストール成功後、Note A を Flush
  （NoteBodyEdited）し、永続化を待ってからアプリを再起動する（I-U8、S13 と同じ順序保証）

## シナリオステップ {#scenario-steps}

```gherkin
Scenario: S24 更新をインストールし、編集中の Note を Flush してから再起動
  Given current_version = 0.2.4、InstallationKind::Bundled(AppImage)
  And 起動時の確認で 0.2.5 を検出し、Toast を表示中
  And Note A が EDITING 状態（AutoSave debounce timer 中）
  When ユーザが Toast の「Update & restart」を押す
  Then UpdateChannel::install() が最新リリースを取得し直し、0.2.5 > 0.2.4 を確認する（I-U7）
  Then updater plugin が download + 署名検証 + install を行い、0.2.5 を返す
  Then 処理中は Toast が「Updating…」を表示し、ボタンは押せない
  Then インストール成功後、Note A を Flush（NoteBodyEdited）し、永続化を待ってからアプリを再起動する（I-U8）
```

E2E での具体化（バージョン番号は test build の current_version に合わせて読み替える。[#impl-notes](#impl-notes) 参照）:

### Given {#given}

1. 実 user 環境から隔離した temp dir で Tauri App（test build）が起動済み。current_version はビルド時 version
2. 配布形式は `Bundled(AppImage)`（debug seam `TAURI_TEST_INSTALLATION_KIND=appimage`）。実行中アプリの AppImage
   （`APPIMAGE`）は temp の偽 AppImage
3. 更新配信は local server。latest.json は current より新しい `9.9.8`（domain 例の `0.2.5` 相当）と、実行時に生成した鍵で
   署名したテストパッケージを指す。起動時の確認で `9.9.8` の Toast が「Update & restart」付きで表示される
4. seed した Note A を EDITING にし、本文末尾に編集を入力する（debounce 中のため未永続化）

### When {#when}

1. ユーザが Toast の「Update & restart」（`widget.widget-update-toast.install-update`）をクリックする

### Then {#then}

- 処理中は `widget.widget-update-toast.install-status` が「Updating…」、install ボタンと dismiss が disabled
- install は latest.json を取得し直し（起動時の確認より request が増える）、パッケージを 1 回 download する
- 偽 AppImage の中身が配信したテストパッケージと一致する（署名検証を通ったパッケージが install された）
- 再起動でインストールしたパッケージが起動される（パッケージが marker に `9.9.8` を書く）
- 再起動の時点で Note A の編集（install 完了直前の未保存分を含む）がファイルに永続化済み。その永続化は最後の打鍵の
  AutoSave debounce（500ms）成立前に起きている（= 再起動前の Flush による）

## テスト観点 {#test-points}

- **TP1 — Given の成立（インストール可能な配布形式で通知される）**: 起動時の確認で Toast が `9.9.8` で表示され、
  `Bundled(AppImage)` のため「Update & restart」が表示・押下可能（I-U6）。Note A は EDITING で編集が未永続化
- **TP2 — 処理中表示**: 押下後、download 完了まで「Updating…」を表示し、install ボタンと dismiss は押せない（screen-3 `#cross-install-update`）
- **TP3 — 最新リリースを取得し直す**: install は通知時の情報を使い回さず latest.json を再取得する（install-update step 3）
- **TP4 — download + 署名検証 + install**: 実署名のパッケージを 1 回 download し、updater plugin の検証を通って AppImage が置き換わる（I-U5 / I-U7）
- **TP5（核）— Flush してから再起動**: install 完了直前（debounce 成立前）の Note A の編集が、再起動の時点で永続化済みである（I-U8）
- **TP6 — インストールしたバージョンで再起動**: 再起動で `APPIMAGE`（= インストール済みパッケージ）が起動される
- **TP7 — `0.2.5 > 0.2.4` の比較（I-U7）と `0.2.5` を返すこと**: 比較で新しくなければ install しないことと、成功時にインストールした
  バージョンを返すことは install-update slice unit test で担保する（E2E は UI 経由で戻り値を観測しない）
- **TP8 — Flush → 再起動の呼出順**: page-main が install 成功後に `flushAll` を await してから `restart_app` を呼ぶ順序は
  page-main component test で担保する（E2E は TP5 で結果としての永続化を観測する）

## 実装ノート {#impl-notes}

- **runner**: `wdio`（優先チェーン step 2 — manifest に `runner:` 明示なし。参加 app `promptnotes` は `local` 系で `runtime.runner = wdio`）
- **infrastructure**: `promptnotes` のみ（`mode: local`）。compose-service 系 app が参加しないため docker-compose は不要
- **runner config**: `wdio.conf.ts`。`tauri:options.application` は runtime block の `binary` =
  `apps/promptnotes/src-tauri/target/debug/app`（build-then-test）
- **バージョンの読み替え**: domain 例は current `0.2.4` / 新版 `0.2.5`。test build の current_version はビルド時 version なので、
  E2E は current より確実に新しい `9.9.8` を配信する。関係（current < 新版）は同じ
- **test seam（debug build 限定。release build では compile out）**:
  - `TAURI_TEST_UPDATER_ENDPOINT`（`update_distribution/shared/updater.rs` の `with_test_endpoint`）: 更新確認と install の
    両方の取得先を local server に向ける（s14 / s23 と共通）
  - `TAURI_TEST_UPDATER_PUBKEY`（`install_update/infrastructure.rs`）: install の署名検証に使う公開鍵を差し替える。
    wdio `onPrepare` が `cargo tauri signer generate` で鍵を実行時に生成し、`cargo tauri signer sign` でテストパッケージに
    署名する（鍵・署名済みパッケージはコミットしない）
  - `TAURI_TEST_INSTALLATION_KIND`（`install_update/infrastructure.rs`）: debug build の配布形式（既定 `Unbundled`）を
    `appimage` / `deb` / `rpm` / `unbundled` で上書きする
- **install / 再起動の観測（production 変更なし）**: updater plugin の Linux install は `APPIMAGE` が指すファイルを新パッケージで
  置き換え、`AppHandle::restart` は `APPIMAGE` を起動する（いずれも tauri / plugin の既存挙動）。`onPrepare` が `APPIMAGE` を temp の
  偽 AppImage に向けるので `target/debug/app` は上書きされない。テストパッケージは shell script で、起動されると marker と
  「その時点の Note A ファイル」の snapshot を書く。再起動でアプリ process は終了し WebDriver session は失われるため、
  再起動後の検証は fs のみで行う
- **Flush の観測**: 「Update & restart」のクリックで editor は blur し（S3 flush-on-blur）、AutoSave debounce（500ms）も install 中に
  成立しうるため、押す前の編集だけでは再起動前の Flush を外しても永続化されてしまう。server は `/download` の応答を
  `/release-download` が叩かれるまで保留し、テストは install 中に Note A へ追加の編集を入力して未永続化を確認した直後に
  download を解放する。この編集は debounce 成立前に install 完了 → page-main の Flush → 再起動の経路でしか永続化されない。
  さらに、Note A ファイルの最終更新時刻が最後の打鍵から 500ms（AutoSave debounce）未満であることを assert し、
  負荷で debounce が先に成立した場合の偽 GREEN を排除する（debounce 成立後の書き込みは同一 body のため起きない、S9）
- **キー入力の注意**: WebKitWebDriver の key action は連続する同一キーの 2 打目を落とすことがあるため、入力文字列に同じ文字を
  連続させない。入力後は editor の DOM に反映されたことを待ってから次へ進む
- **前提条件（test readiness）**:
  - **build 契約**: `bun run build:test`（`VITE_WDIO_TEST=1 bun run tauri build --debug --no-bundle`）で `runtime.binary` を作る。
    `cargo build` 単体は devUrl 参照の dev binary になるため不可
  - **plugin 前提**: `tauri-plugin-wdio` の app 側配線（Cargo dep / capabilities `wdio:default` / `lib.rs` の `#[cfg(debug_assertions)]`
    登録 / frontend の `VITE_WDIO_TEST` 動的 import）は既存
  - **storage 隔離**: `wdio.conf.ts` の `onPrepare` が per-run temp root を採番し `XDG_CONFIG_HOME` / `XDG_DATA_HOME` /
    `XDG_CACHE_HOME` / `XDG_STATE_HOME` を root 配下に向ける（`ORI_SCENARIO_TMP` で公開。Linux のみ）。
    `app_config_dir/settings.json` に temp の `storage_dir` を seed する
  - **node_modules 解決**: `.ori/scenarios/node_modules` → `apps/promptnotes/node_modules` の symlink
  - **fixture seed**: Note A（`20260620120000.md`、frontmatter `createdAt` / `updatedAt` / `tags`）を `onPrepare` で seed する
  - **署名ツール**: `cargo tauri signer`（nix devShell の cargo-tauri）が PATH にあること
- **platform key**: `latest.json` の `platforms` は linux x86_64（`linux-x86_64`）。debug の `--no-bundle` build は bundle 種別を
  持たないため、updater は installer 付きの key ではなく `linux-x86_64` を読む
