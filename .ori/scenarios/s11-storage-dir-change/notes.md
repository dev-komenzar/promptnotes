# s11-storage-dir-change — Scenario implementation notes

## 概要 {#overview}

`storage_dir` 変更は **即時マイグレーションせず再起動を要求するのみ**（I-S4）ことを検証する。
`update-settings` workflow が `settings.json` に永続化し `StorageDirChanged` を発行、
UI 層（page-main）がそれを購読して再起動モーダルを表示し、Note Feed は再起動まで旧ディレクトリを
見続ける。再起動後に new dir のスキャン結果が表示される。

## テスト方式: config dir 隔離 + 境界 invoke + modal save {#test-approach}

- **config dir 隔離**: wdio `onPrepare` が `XDG_CONFIG_HOME` / `XDG_DATA_HOME` を temp dir に設定する。
  Tauri の `app_config_dir()` は `$XDG_CONFIG_HOME/com.komenzar.promptnotes` に解決されるため、
  そこへ `settings.json`（`storage_dir = old dir`）を seed して Given の Settings を再現する。
  `TAURI_TEST_STORAGE_DIR` override は使わない（`storage_dir` が固定され、再起動後の new dir 反映を
  検証できなくなるため）。
- **Note seed**: old dir に 3 件（alpha / beta / gamma）、new dir に別の 1 件（delta-from-new-dir）を
  `onPrepare` が書き込む。
- **storage_dir 変更**: 保存ディレクトリは read-only + OS native folder picker のため E2E から
  dialog を駆動できない。そこで `update_settings { input: { storage_dir } }` を Tauri 境界で直接
  invoke する（同期 `browser.execute` で kick-off → window 上の結果を poll。`@wdio/tauri-service`
  (driverProvider=external) の patchedExecute は `browser.executeAsync` を扱えないため。s10 と同方式）。
- **modal Save 経路 + Feed 据え置き guard の検証**: `handleSettingsSaved` は modal の Save 時のみ呼ばれる。
  storage_dir 差分ありの Save を成立させるため、**modal を開いた状態**で境界 invoke により
  `storage_dir` を変更し、その後 modal から theme を変更して Save する。このとき `next.storage_dir`
  （new）と frontend `currentSettings.storage_dir`（old）が異なるため、guard が効かない実装では
  Feed が new dir へ再 hydrate され step 2 が RED になる。
- **再起動**: `restart-prompt-restart`（今すぐ再起動）→ `window.location.reload()` で remount し、
  settings.json を再読込して new dir の Note が現れることを確認する（S22 のウォッチャー再起動は範囲外）。

## テストカバレッジ {#coverage}

| 検証項目 | 検証方法 |
|---|---|
| 初期状態（old dir 3 件表示 / settings.json storage_dir=old） | E2E `before`（`[data-block-id]` 集合 + `readFileSync`） |
| 設定モーダルが old dir を表示（UI 統合） | E2E step1（`screen-1-toolbar-settings-button` → `screen-2-storage-dir` の value） |
| 対照: theme のみの UI save は再起動モーダル非表示 | E2E step1（`screen-2-theme-Dark` → `screen-2-save` → `restart-prompt` 不在 + theme 永続化） |
| `Settings::change_storage_dir` 実行 + settings.json 永続化 | E2E step2（境界 invoke → `settings.json` storage_dir=new） |
| event `StorageDirChanged` 発行 | E2E step2（`settings:storage_dir_changed` subscriber → `restart-prompt` 出現） |
| 再起動を促すモーダル表示（I-S4） | E2E step2（`[data-testid="restart-prompt"]`） |
| フィードは旧ディレクトリのまま（I-S4 / guard） | E2E step2（storage_dir 差分あり Save 後も old 3 件のまま / new dir の Note 不在） |
| 再起動後に `/new/path` のスキャン結果表示 | E2E step3（`window.location.reload()` → new dir の Note 1 件） |
| `update_settings` の domain 挙動（persist / event / I-S4 非移行） | slice `update-settings` の Rust unit test（TP-S11-*）が担保 |

## RED → GREEN 検証 {#red-green}

production 側には S11 の前提のうち **UI 層の 2 点が欠落**していた:

1. `StorageDirChanged` を購読して再起動モーダルを表示する実装が無い（`widget-settings-modal` spec でも
   OQ-SM2 として先送りされていた）
2. `PageMain.handleSettingsSaved` が保存のたびに Feed を再 hydrate しており、`storage_dir` 変更時に
   新ディレクトリへ即座に切り替わってしまう（I-S4 違反）

テストが空振りでないことを示すため、production を一時変異させて RED を実測した:

- **変異 A（PageMain を HEAD に戻す = subscriber / guard なし）**: `bun run build:test` → E2E
  **1 passing / 2 failing**。step 2・3 が `restart prompt did not appear` で RED（step 1 のみ PASS）。
- **変異 B（guard のみ無効化 = 常に再 hydrate、subscriber は残す）**: 再ビルド → E2E
  **2 passing / 1 failing**。step 2 が Feed 期待値で RED:
  `Expected 3 件` に対し `Received ["20260701100000"]`（new dir の Note が現れた）。
- **復元後**: `bun run build:test` → E2E **3 passing (~3.6s)** で GREEN。

したがって本 scenario のテストは「再起動モーダル欠落」と「storage_dir 変更時の Feed 据え置き guard
欠落」の双方を実際に検出する。

## production 変更 {#production-change}

**`apps/promptnotes/src/ui-page/page-main/PageMain.svelte` のみ**（+84 / -10）:

1. `settings:storage_dir_changed` を購読する `$effect` を追加し、受信時に `restartPromptOpen = true`。
2. `restartPromptOpen` の `{#if}` ブロックで再起動モーダル（`role="alertdialog"`）を追加。
   `restart-prompt-restart` = `window.location.reload()`、`restart-prompt-later` = 閉じるのみ。
3. `handleSettingsSaved` に `storage_dir` 非変更時のみ Feed を再 hydrate する guard を追加
   （変更時は Feed を旧ディレクトリのまま維持 = I-S4）。
4. `page-main` に `data-restart-prompt-open` 属性を追加（観測点）。

Rust 側 / 設定モーダル / UI fields の変更は無し（`update_settings` command と `TauriEventBus` の
`settings:storage_dir_changed` emit は既存実装をそのまま利用）。

回帰確認:

- frontend unit test: **143 passed / 15 files**
- svelte-check: **0 errors / 0 warnings**
- E2E 回帰: s10-tag-invalid-char **3 passing**、s4-tag-assign-normalize **2 passing**
- Rust: 変更なし

## 既知の未対応 / 補足 {#known-issues}

- **event `StorageDirChanged` の Infrastructure 層 subscriber（ファイルウォッチャーの監視対象切替）**は
  S22 (`domain/validation.md#s22-storage-dir-change-watcher-restart`) の範囲であり、本 scenario では
  実装・検証しない。
- **既存 lint 失敗（本 scenario の変更対象外）**: `bun run lint` は以下 2 件で失敗するが、いずれも
  HEAD 時点で既に失敗している既存問題。
  - `src/ui-page/page-main/stores/toasts.test.ts`: prettier 非準拠（HEAD でも同じ）
  - `src/ui-page/page-main/PageMain.svelte`: `editingNote` / `EditingNoteState` の未使用 import
    （line 14、HEAD でも同じ）+ ファイル全体の prettier 非準拠（HEAD でも同じ。本変更では
    既存インデントを保ち差分を最小化したため、prettier の再整形は行っていない）
