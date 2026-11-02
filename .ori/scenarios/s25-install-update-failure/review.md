# Review: s25-install-update-failure {#review-s25-install-update-failure}

## Pass 1 {#pass-1}

### Syntax checks {#syntax-checks}
- test code: PASS (tsc --noEmit, main session 実行済み)
- docker-compose: N/A (compose-service 参加者ゼロ)

### Then ↔ assertion coverage {#then-coverage}

spec.md#scenario-steps の Then は 4 件 (spec.md:53-56)。domain/validation.md#s25-then の bullet 4 件と一致し、main session 提示の件数とも一致する。

| Then (source#anchor) | 期待 assertion | テスト (file:line) | 状態 |
|---|---|---|---|
| spec.md#scenario-steps Then `InstallUpdateError::DownloadFailed`（または `InstallFailed`）が UI に返る (spec.md:53) | 失敗後に同じ `install_update` を Tauri 境界で invoke し `kind` が step 1 では `DownloadFailed`、step 2 では `InstallFailed`（文字列の完全一致） | tests/s25-install-update-failure.spec.ts:127-129 (DownloadFailed) / :145-147 (InstallFailed)。`invokeInstall` は :47-58 | VERIFIED |
| spec.md#scenario-steps Then アプリは再起動しない (spec.md:54 前半) | 失敗表示後も app プロセスが生存して応答する。install されたパッケージが起動されていない（marker なし）。偽 AppImage が不変 | :84-85 (marker 不在 / AppImage 不変)。生存性は :76-82 の DOM 問い合わせ、:93-94 の 3 秒後の再確認、:127 / :145 の `browser.tauri.execute` が成立することに依存（[MEDIUM-1] 参照） | VERIFIED（生存性は暗黙の観測） |
| spec.md#scenario-steps Then Note の Flush も行わない (spec.md:54 後半) | install 失敗時に `flushAll` と `restart_app` が呼ばれない。`onInstalled` hook が呼ばれない | E2E では不可能（spec.md:92-94: 「Update & restart」のクリック自体が editor の blur flush を起こす）。代替担保: apps/promptnotes/src/ui-page/page-main/tests/PageMain.update-check.svelte.test.ts:162-189（`expect(tauri.calls).not.toContain('flushAll')` :182、`'invoke:restart_app'` :183）/ apps/promptnotes/src/ui-widget/update-toast/tests/store.test.ts:286-304（`onInstalled` not called :297、再実行で `onInstalled` が 1 回 :302）。いずれも実在し assert を確認した | N/A(代替担保) |
| spec.md#scenario-steps Then Toast は失敗メッセージを表示し、「Update & restart」を再度押せる状態に戻る（I-U7） (spec.md:55) | 「Updating…」を経て `FAILED_TEXT` に一致。`role="alert"`。install ボタンが enabled。step 2 で実際に再クリックして処理が走る（request +1） | :61-71 (Updating… → FAILED_TEXT)、:76-77 (role=alert / isEnabled)、:119 / :137 (download +1) | VERIFIED |
| spec.md#scenario-steps Then 「View release」は引き続き使える (spec.md:56 前半) | view-release が displayed かつ enabled。toast は閉じられていない（latest-version=9.9.8、dismiss enabled） | :79-82 | VERIFIED |
| spec.md#scenario-steps Then 自動リトライはしない (spec.md:56 後半) | 失敗確認後 3 秒待っても `/latest.json` と `/download` の request 数が不変。クリック 1 回で `/download` がちょうど +1 | :89-95 (`expectNoAutomaticRetry`)、:119 / :137（`toBe(before + 1)`） | VERIFIED |

複合 Then の分解結果:
- 「再起動しない」と「Flush しない」は別々に検証方法を確認した。前者は E2E、後者は代替担保。
- 「View release は使える」と「自動リトライしない」は別々の assertion で検証されている。
- UNVERIFIED は 0 件。

### Test-points coverage {#test-points-coverage}

`awk '/^```/{c=!c} /^## .*\{#test-points\}/{f=1;next} /^## /{f=0} f && !c && /^- /' spec.md` の相当で、項目は 7 件（TP1〜TP7）。test-points-map.md の行も 7 件で、順序と見出しが一致する。

| TP | map の状態 | 実測 | 判定 |
|---|---|---|---|
| TP1 Given の成立 | COVERED (spec.ts:104) | :104-112。Toast 表示、`9.9.8`、install ボタンが displayed かつ enabled（`store.installable` のときだけ描画される。I-U6）、status 無し | OK |
| TP2（核）download 失敗 | COVERED (:114) | :114-130。失敗表示、再起動なし、再度押せる、kind=DownloadFailed | OK |
| TP3 install 失敗（署名検証） | COVERED (:132) | :132-148。再押下で処理が走る、失敗表示、AppImage 不変、kind=InstallFailed。notes.md の変異 C で RED を実測済み | OK |
| TP4 View release | COVERED (:114 / :132) | :79-81 が両 step で実行される | OK |
| TP5 自動リトライしない | COVERED (:114 / :132) | :89-95 が両 step で実行される | OK |
| TP6 Flush しない | N/A(代替担保) | PageMain.update-check.svelte.test.ts:162、store.test.ts:286。実在し、assert の内容も妥当 | OK |
| TP7 pkexec 拒否の InstallFailed | N/A(代替担保) | install_update/tests.rs:273 (`tp_f3_...`: `InstallOutcome::InstallFails` → `InstallFailed`)、:319 (`tp_f4_...`: 署名系は `InstallFailed`、他は `DownloadFailed`)。実在を確認した（infrastructure.rs:184 の `map_download_error` と整合） | OK |

UNCOVERED は 0 件。

### Semantic findings (reviewer: sonnet, capability=deep, fresh context) {#semantic-findings}

HIGH: なし。

- **MEDIUM-1**（非ブロッキング） tests/s25-install-update-failure.spec.ts:83-85 「アプリは再起動しない」の観測が暗黙的。
  - `MARKER` は install されたパッケージが起動されたときだけ書かれる。S25 の `APPIMAGE` は `exit 0` の偽 AppImage で marker を書かない（wdio.conf.ts:188）。「失敗したのに `restart_app` が呼ばれる」不具合では、再起動先が偽 AppImage になり marker は書かれず、:84 の assertion 単体では検出できない。
  - 現状は、再起動すると app プロセスが死に、:93-94 の後の `invokeInstall`（:127 / :145）や step 2 の `$(INSTALL).click()` が失敗する、という副次効果で RED になる。検出は成立しているが、assertion 自体は「再起動しない」を直接表していない。
  - 推奨: Given（:104 の `it`）で `browser.execute` により window 上に sentinel を置く（例: `window.__s25 = 1`）。失敗後と 3 秒待機後に `!= null` で比較して同一プロセス・同一ページを直接検証する。`!== undefined` 比較にしないこと（scenario-test.md の注意）。ただし `.claude/rules` の「テスト用 window 公開を production に残さない」方針は、テストが注入する値なら production 側の変更ではないため抵触しない。
  - あわせて :83 のコメント「インストールしたパッケージは起動されていない」は、marker の意味（install 成功後の再起動の検出）に限定された記述に直すと誤読が減る。
  - 判定: 副次効果で RED になるため Then は VERIFIED とした。UNVERIFIED としたときの HIGH 規則は適用しない。
- **LOW-1** tests/s25-install-update-failure.spec.ts:127-129 / :145-147 「UI に返る」エラー種別は、UI のクリックで得た応答ではなく、別途 invoke した応答で確認している。
  - spec.md#then（:80-82）が明示した方法であり、同一の `install_update` command なので妥当。
  - 補足: invoke は `/latest.json` と `/download` の request 数を増やすが、`expectNoAutomaticRetry` の後に実行されているため干渉しない。順序を変えないこと。
- **LOW-2** tests/s25-install-update-failure.spec.ts:89-95 自動リトライの観測窓は 3 秒。即時リトライと、失敗直後から周期的に走るリトライは検出できる。3 秒を超える backoff 付きリトライは検出できない。
  - 現実装にリトライ処理は無く（infrastructure.rs には再試行なし）、domain も「ユーザ操作を起点とした再試行のみ」としか規定しない。窓の長さは spec.md#then が 3 秒と明記しているため、乖離ではない。
- **LOW-3** test-points-map.md の表書式が scenario-test.md#test-points-map のテンプレート（TP / test-point / テストケース / 代替担保 / 状態 の 5 列）と異なる。
  - 3 列（test point / 状態 / テストケース・代替担保）で、`file:line` は第 3 列に併記されている。内容は全項目で充足しており、`N/A(代替担保)` の 2 行は `path:数字` を持つ。
  - 機械 checker が「代替担保」列を厳密に読む場合に備え、再生成時に 5 列へ揃えることを推奨する。
- **LOW-4** 「current_version = 0.2.4」の Given は、test build の current より新しい `9.9.8` を配信する形に読み替えている（spec.md#impl-notes）。`current-version` の表示値は assert されていないが、「新しい版が Toast に出る」という本質は :108 で検証されている。

### local 系 wdio checklist {#wdio-checklist}

| 項目 | 結果 |
|---|---|
| `tauri:options.application` が `runtime.binary` と一致 | OK。wdio.conf.ts:13 は `resolve(__dirname,'../../../apps/promptnotes/src-tauri/target/debug/app')` で、architecture.md の `binary: apps/promptnotes/src-tauri/target/debug/app` と一致。`build: bun run build:test` も契約どおり |
| `@wdio/tauri-service`（`driverProvider: 'external'`） | OK (:137) |
| `onPrepare` の非 Linux fail fast | OK。`SevereServiceError`（:149-153）。import 元は `webdriverio`（:10） |
| `onPrepare` の :4444 占有 fail fast | OK (:154-159、`pkill tauri-driver` の案内あり) |
| XDG 4 種の per-run temp と `ORI_SCENARIO_TMP` | OK (:161、:208-212)。`HOME` は不変 |
| `maxInstances: 1` | OK (:136、理由コメントあり) |
| テストに service 起動処理が無い | OK。updater server と鍵生成は `onPrepare` 側 |
| `browser.execute` の結果を `undefined` 比較していない | OK。`browser.execute` は未使用。`browser.tauri.execute` の結果は `ok` / `kind` で判定 |
| spec 実装ノートの前提条件 | OK。spec.md:114-124 に build 契約 / plugin / storage 隔離 / node_modules / fixture / 署名ツールを記録 |
| `.ori/scenarios/node_modules` の symlink | OK（`apps/promptnotes/node_modules` を指す） |
| 後始末 | OK。`onComplete` で server を close し、temp を削除 |
| 実ファイル保護 | OK。`APPIMAGE` を temp の偽 AppImage に向け、:85 で不変を確認 |

### 未実装でも GREEN になる構図の探索結果 {#green-without-impl}

- 失敗を握りつぶして UI に何も出さない場合: `Updating…` の後に `FAILED_TEXT` へ遷移しないため :67 で RED。
- 失敗後にボタンが disabled のまま、または toast が消える場合: :76-82 で RED。
- 失敗時に `restart_app` を呼ぶ場合: プロセスが死ぬため後続の `browser.tauri.execute` が失敗して RED（MEDIUM-1 のとおり間接的）。
- 署名エラーを `DownloadFailed` に写す場合: step 2 の `kind` 比較で RED（notes.md の変異 C で実測済み）。
- 古い `FAILED_TEXT` が残った状態での誤検知: step 2 は `Updating…` を先に待つため、step 1 の表示では通らない（:63-66）。
- 失敗時の自動リトライ: 3 秒窓の request 数比較と `download === before + 1` で RED。
- `getText()` が要素不在で throw する点: `waitUntil` の中でリトライされるため問題ない。
- 変異 C のみ実測済みで、「失敗表示なし」「failure 時に再起動」「自動リトライ」の変異は本 review では実行していない（読解による評価）。

### Verdict {#verdict}
**PASS**

理由:
1. spec.md#scenario-steps の全 Then 4 件が assertion に対応している。VERIFIED が 3 件、複合 Then のうち E2E 不能な「Flush しない」は実在する unit test 2 件を代替担保とする N/A(代替担保)。UNVERIFIED は 0 件で、domain の数値・定数（3 秒の観測窓、エラー種別）とテストが一致している。
2. test-points は 7 件すべてが map と 1 対 1 で、COVERED の根拠となる assertion が実在し、N/A(代替担保) の 3 つの file:line（PageMain test :162、store.test.ts :286、tests.rs :273 / :319）も実在し内容が妥当である。
3. local 系 wdio checklist の全項目を満たしている。
4. 指摘は MEDIUM 1 件（「再起動しない」の観測が副次効果依存。sentinel の追加を推奨）と LOW 4 件のみで、すべて非ブロッキング。必要なら `/ori-generate` で改善する。

### Disposition (main session) {#disposition}

- verdict=PASS のため差し戻しなし。非ブロッキングの MEDIUM（「再起動しない」の観測が副次効果頼み）は推奨どおり対応した:
  Given でテストが window に目印 (`__s25Sentinel`) を置き、各失敗後と自動リトライ観測の待機後に同じ値が残っていることを assert する
  （同じ webview / process の継続を直接観測）。E2E 再実行で 3 passing。テスト行番号は test-points-map.md を更新済み
  （given :116 / step 1 :132 / step 2 :150）
- LOW（観測窓 3 秒 / map の列構成 / 種別確認の別 invoke）は spec.md に根拠があるため変更しない
