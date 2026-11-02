# s25-install-update-failure — Scenario implementation notes

## 実行方法 {#how-to-run}

```bash
cd apps/promptnotes && bun run build:test
cd ../../.ori/scenarios/s25-install-update-failure
PATH=$HOME/.cargo/bin:$PATH env -u LD_LIBRARY_PATH ../node_modules/.bin/wdio run wdio.conf.ts
```

## テスト方式 {#test-approach}

- s24 と同じ 3 つの debug seam を使う（配布形式は `deb`）。server は mode ファイルで `download-fail`（`/download` が HTTP 500）と
  `bad-signature`（latest.json の署名が別ファイルのもの）を切り替える
- 「再起動しない」は、Given でテストが window に目印 (`__s25Sentinel`) を置き、失敗後・待機後も残っていることで
  同じ webview / process の継続として観測する（app は目印を読まない。テスト専用の値）
- エラー種別は UI に出ないため、UI 操作の検証後に同じ `install_update` を Tauri 境界 invoke して `kind` を確認する

## RED → GREEN 検証 {#red-green}

- **変異 C（`install_update/infrastructure.rs` の `map_download_error` で署名エラーを `DownloadFailed` に写す）** →
  E2E **2 passing / 1 failing**: step 2 で `Expected: "InstallFailed"` に対し `Received: "DownloadFailed"`
- **復元後** → 再ビルド → E2E **3 passing**
