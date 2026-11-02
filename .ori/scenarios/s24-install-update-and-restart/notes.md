# s24-install-update-and-restart — Scenario implementation notes

## 実行方法 {#how-to-run}

```bash
cd apps/promptnotes && bun run build:test
cd ../../.ori/scenarios/s24-install-update-and-restart
PATH=$HOME/.cargo/bin:$PATH env -u LD_LIBRARY_PATH ../node_modules/.bin/wdio run wdio.conf.ts
```

`cargo tauri signer`（nix devShell）で鍵を実行時に生成するため、devShell 内で実行する。

## テスト方式 {#test-approach}

- **test seam（debug build 限定）**: `TAURI_TEST_UPDATER_ENDPOINT`（取得先、`shared/updater.rs` の `with_test_endpoint` を
  更新確認と install で共有）/ `TAURI_TEST_UPDATER_PUBKEY`（install の署名検証の公開鍵）/ `TAURI_TEST_INSTALLATION_KIND`
  （配布形式）。いずれも `#[cfg(debug_assertions)]` で release build から compile out される
- **install と再起動は production の経路そのもの**: updater plugin の Linux install は `APPIMAGE` が指すファイルを置き換え、
  `AppHandle::restart` は `APPIMAGE` を起動する。`onPrepare` が `APPIMAGE` を temp の偽 AppImage に向け、テストパッケージ
  （shell script）が起動時に marker と Note A の snapshot を書く。`target/debug/app` は上書きされない
- **Flush の観測**: download を `/release-download` まで保留し、install 中に入力した未保存の編集が再起動時点で永続化済みかを見る。
  クリック時の blur flush や AutoSave debounce では説明できない編集だけを判定に使う

## RED → GREEN 検証 {#red-green}

production を一時変異させて、E2E が空振りでないことを実測した（変異は復元済み）:

- **変異 A（`PageMain.svelte` の `restartAfterUpdate` から `flushAll` を削除）** → `bun run build:test` → E2E **2 passing / 1 failing**:
  step 2 で `Expected substring: "alpha body S24-pending-edit S24-typed-while-updating"` に対し
  snapshot は `alpha body S24-pending-edit`（install 中の編集が再起動時点で未永続化）
- **復元後** → 再ビルド → E2E **3 passing**（3 回連続で安定）
- **review Pass 1 の指摘対応**:
  - E2E: Note A の最終書き込みが最後の打鍵から 500ms 未満であることを assert（実測 26〜70ms）。debounce が先に成立する
    偽 GREEN を排除する
  - TP8 の代替担保: page-main component test に「flushAll を保留している間は `restart_app` を呼ばない」ケースを追加。
    `restartAfterUpdate` の `await` を外す変異で、このケースだけが RED になることを実測した（変異は復元済み）

## 既知の注意 {#caveats}

- WebKitWebDriver の key action は連続する同一キーの 2 打目を落とすことがある（`install` の `ll` が 1 文字になった）。
  入力文字列に同じ文字を連続させず、入力後は editor の DOM 反映を待つ
- 再起動で WebDriver session が失われるため、wdio の afterSession で `Failed to clear mock store: A sessionId is required` の
  WARN が出る（テスト結果には影響しない）
