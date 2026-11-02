# s26-unbundled-notify-only — Scenario implementation notes

## 実行方法 {#how-to-run}

```bash
cd apps/promptnotes && bun run build:test
cd ../../.ori/scenarios/s26-unbundled-notify-only
PATH=$HOME/.cargo/bin:$PATH env -u LD_LIBRARY_PATH ../node_modules/.bin/wdio run wdio.conf.ts
```

## テスト方式 {#test-approach}

- 配布形式の seam は使わず、`--no-bundle` の debug build の既定（`Unbundled`）で実行する。取得先の seam
  （`TAURI_TEST_UPDATER_ENDPOINT`）のみ使う
- server は path ごとの request 数を記録し、install 要求の前後で増えないことを確認する

## RED → GREEN 検証 {#red-green}

- **変異 B（debug build の既定配布形式を `Bundled(AppImage)` にする）** → E2E **3 failing**:
  `is_update_installable` が `true`、「Update & restart」が表示され、`install_update` は `UnsupportedInstallation` ではなく
  download まで進んで `DownloadFailed`（server の `/download` は 500）
- **復元後** → 再ビルド → E2E **3 passing**
