# Review: s26-unbundled-notify-only {#review-s26-unbundled-notify-only}

## Pass 1 {#pass-1}

### Syntax checks {#syntax-checks}
- test code: PASS (tsc --noEmit, main session 実行済み)
- docker-compose: N/A (compose-service 参加者ゼロ)

### Then ↔ assertion coverage {#then-coverage}

main session 列挙の Then は 2 件。`spec.md#scenario-steps` の Gherkin（spec.md:51, :52）と一致。抜け無し。

| Then (source#anchor) | 期待 assertion | テスト (file:line) | 状態 |
|---|---|---|---|
| spec.md#scenario-steps (spec.md:51) Then is_installable()==false のため「Update & restart」は表示されない | `widget.widget-update-toast.install-update` が DOM に存在しない（`isExisting()==false`）。`install-status` も存在しない | tests/s26-unbundled-notify-only.spec.ts:49 (install), :50 (status) | VERIFIED |
| spec.md#scenario-steps (spec.md:51) Then ...（View release と dismiss のみ、I-U6） | `view-release` / `dismiss` が表示かつ有効 | tests/s26-unbundled-notify-only.spec.ts:52-55 (`isDisplayed` / `isEnabled`) | VERIFIED |
| spec.md#scenario-steps (spec.md:51) Then ...（`is_installable()==false` 自体） | `is_update_installable` invoke が `false` | tests/s26-unbundled-notify-only.spec.ts:41 (`toBe(false)`) | VERIFIED |
| spec.md#scenario-steps (spec.md:52) Then 仮に install が要求されても UnsupportedInstallation で拒否する | `install_update` invoke が reject し `ok==false` かつ `kind=='UnsupportedInstallation'` | tests/s26-unbundled-notify-only.spec.ts:74-75 | VERIFIED |
| spec.md#scenario-steps (spec.md:52) Then ...ネットワークには触れない | 拒否の前後で server の path 別 request 数が不変（全 path 対象の `toEqual(before)`）。`/download` は呼ばれたら 500。偽 AppImage の byte 不変 | tests/s26-unbundled-notify-only.spec.ts:77 (counts), :78 (AppImage) | VERIFIED |

補足: 上表の file:line は `tests/s26-unbundled-notify-only.spec.ts` 単体の行番号（grep で確認済み）。test-points-map.md の `:31 / :44 / :58` は各 `it(` の行（given=31, step1=44, step2=58）と一致。

### Test-points coverage {#test-points-coverage}

spec.md#test-points は TP1〜TP5 の 5 件。test-points-map.md も 5 行で項目一致（表に無い項目なし）。

| test point | map の状態 | 実在 / 実 assert の確認 | 判定 |
|---|---|---|---|
| TP1 Given の成立 | COVERED (:31) | `it('given — ...')` が Toast 出現 (`waitForExist`)、`/latest.json` count>=1、`is_update_installable===false` を assert | OK |
| TP2（核）通知のみ | COVERED (:44) | `it('step 1 — ...')` が install-update / install-status の不在、view-release / dismiss の表示・有効を assert | OK |
| TP3 install 要求の拒否 | COVERED (:58) | `it('step 2 — ...')` が `kind=='UnsupportedInstallation'` を assert | OK |
| TP4 ネットワークに触れない | COVERED (:58) | 同 it が `requestCounts()` 前後 `toEqual` を assert | OK |
| TP5 release build の配布形式判定 | N/A(代替担保) | `install_update/tests.rs:176` `tp_u3_bundle_type_maps_to_installation_kind`（`(None, Unbundled)` を含む写像を assert）と `tests.rs:149` `tp_u1_unbundled_is_rejected_without_touching_updater`（`Err(UnsupportedInstallation)` かつ fetch/install 呼び出し 0 回）を実在確認。いずれも実際に assert している | OK |

UNCOVERED なし。

### Semantic findings (reviewer: sonnet, capability=deep, fresh context) {#semantic-findings}

未実装でも GREEN になる構図を能動的に探索した結果:

- 「Update & restart 不在」の空振り（installable 判定前の一時状態を見ていないか）: `store.svelte.ts` の `installable` は初期値 `false` で、`start()` が `listen` → `await installableFn()` → `onSubscribed`（起動時 check）の順に進む。Toast の payload は check 由来なので、`installable` の確定より後に出る。したがって Toast 出現後の不在 assertion は判定前の一時状態を見ていない。さらに notes.md の変異 B（既定を AppImage にする）で 3 件 RED を実測済みで、非空振りを裏付ける。
- 「ネットワークに触れない」の観測: `install_update` は `ensure_installable`（application.rs:22）が最初のステップで、fetch 前に返る。観測は server 側の全 path request 数で、`/latest.json` の再取得も `/download` も拾う（カウントは path を選ばない）。periodic な再確認（setInterval 等）は src / src-tauri/src に存在せず、前後比較が偽陽性になる経路は見当たらない。変異 B では download まで進み `DownloadFailed` になることを実測済みで、観測が機能している。
- invoke 結果の `undefined` 比較: `installable` は `toBe(false)`、`result.ok` は `toBe(false)`、`kind` は `toBe('UnsupportedInstallation')`。`browser.tauri.execute` が undefined を返した場合は fail 側に倒れ、PASS に倒れない。OK。

local 系 wdio checklist:
- `tauri:options.application`: `resolve(__dirname, '../../../apps/promptnotes/src-tauri/target/debug/app')` は `.ori/scenarios/s26-.../` 起点でリポジトリルート直下を指し、architecture.md の `runtime.binary` と一致。PASS
- `@wdio/tauri-service`（`driverProvider: 'external'`）使用、`browserName: 'tauri'`: PASS
- `onPrepare` の fail fast: 非 Linux と `:4444` 使用中の両方で `SevereServiceError` を throw（wdio.conf.ts:109-119）: PASS
- XDG 4 種（CONFIG / DATA / CACHE / STATE）を per-run `mkdtempSync` 配下に向け、`ORI_SCENARIO_TMP` を公開、`storage_dir` を temp に seed: PASS
- `maxInstances: 1`（理由コメント付き）: PASS
- テスト内に service 起動処理なし（server は `onPrepare`、後始末は `onComplete`）: PASS
- `TAURI_TEST_INSTALLATION_KIND` を `delete` して seam を使わず、`APPIMAGE` を temp の偽 AppImage に向ける安全策あり: PASS
- spec.md#impl-notes に build 契約（`bun run build:test`）、plugin 前提、storage 隔離、node_modules symlink（`.ori/scenarios/node_modules` の実在を確認）、fixture seed の前提が記録されている: PASS

指摘:

- **LOW** tests/s26-unbundled-notify-only.spec.ts:54-55: 「dismiss 押下可能」は `isEnabled()` までの確認で実押下はしない。以降の step 2 が Toast の残存を前提にするため押下を省いており、spec の "表示・押下可能" と I-U6 の趣旨（操作を提示しない）に対しては十分。最後の it で押下する案もあるが、必須ではない。
- **LOW** tests/s26-unbundled-notify-only.spec.ts:49-50: 不在 assertion は `installable` の確定を明示的に待たず、`start()` の実行順（installable 確定 → check）と it の実行順に依存している。構造上は安全で、変異 B でも担保されているため現状は blocking ではない。将来 `start()` の順序を変える場合は再検討が必要。
- **LOW** wdio.conf.ts:55: latest.json の platforms key が `linux-x86_64` 固定。Linux aarch64 では起動時 check が失敗して Given が成立しない。notes / impl-notes に platform key の前提が明記されており、s24 / s25 と同方針のため許容。
- **LOW** 「ネットワークに触れない」の観測は更新配信 server への request に限られる。ただし updater の取得先は seam で local server に固定され、他 host への経路は存在しないため、観測として妥当。

HIGH / MEDIUM の指摘は無し。

### Verdict {#verdict}
**PASS**

理由:
1. 全 `Then`（2 件、spec.md:51 / :52）がテストの assertion に 1 対 1 で対応し、UNVERIFIED は 0 件。`N/A(代替担保)` に頼った Then も無い。
2. test-points TP1〜TP5 は test-points-map.md と項目が一致し、COVERED の 4 件は実際に assert されている。TP5 の代替担保 `tests.rs:176` / `:149` は実在し、内容も写像（None → Unbundled）と拒否時の fetch 0 回を assert している。
3. 未実装でも GREEN になる構図は見当たらない。`installable` の確定は Toast 出現より前（`start()` の順序）で、不在 assertion は一時状態を見ていない。ネットワーク不接触は全 path の request 数と偽 AppImage の byte 不変で観測している。変異 B で 3 件 RED を実測済み。
4. local 系 wdio checklist は全項目 PASS。LOW 指摘のみで、差し戻し不要。

### Disposition (main session) {#disposition}

- verdict=PASS のため差し戻しなし。LOW 4 件はいずれも spec.md#impl-notes の方針どおりで変更しない
