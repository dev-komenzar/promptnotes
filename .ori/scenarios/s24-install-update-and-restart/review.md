# Review: s24-install-update-and-restart {#review-s24-install-update-and-restart}

## Pass 1 {#pass-1}

### Syntax checks {#syntax-checks}
- test code: PASS (tsc --noEmit, main session 実行済み)
- docker-compose: N/A (compose-service 参加者ゼロ)

### Then ↔ assertion coverage {#then-coverage}

`spec.md#scenario-steps` の Then は 4 件で、main session 提示の件数と一致する（抜けなし）。
`domain/validation.md#s24-then` の 4 bullet とも一致する。

| Then (source#anchor) | 期待 assertion | テスト (file:line) | 状態 |
|---|---|---|---|
| spec.md#scenario-steps (L61) Then UpdateChannel::install() が最新リリースを取得し直し、0.2.5 > 0.2.4 を確認する（I-U7） | install 時に latest.json の request 数が起動時の確認時点より増える（再取得）。新しい版のときだけ install に進む（marker = 9.9.8 で install まで到達したことを確認） | tests/s24-install-update-and-restart.spec.ts:107（基準値の採取）, :143（`requestCount('/latest.json') > latestJsonBeforeInstall`）, :140 | VERIFIED（再取得）。`ensureNewer` の比較ロジック自体は E2E では観測しない。`N/A(代替担保)`: install_update/tests.rs:133 `tp_s1_installs_newer_release_and_returns_its_version`, :220 `tp_n1_same_or_older_release_is_not_installed`（実在確認済み） |
| spec.md#scenario-steps (L62) Then updater plugin が download + 署名検証 + install を行い、0.2.5 を返す | `/download` が 1 回、AppImage の中身がテストパッケージと byte 一致（署名検証を通ったパッケージが install された） | tests/s24-install-update-and-restart.spec.ts:145, :146 | VERIFIED（download + install）。「0.2.5 を返す」は E2E で観測不能。`N/A(代替担保)`: install_update/tests.rs:133（`assert_eq!(result, Ok(v("0.2.5")))`、実在確認済み） |
| spec.md#scenario-steps (L63) Then 処理中は Toast が「Updating…」を表示し、ボタンは押せない | `install-status` のテキストが `Updating…`、install ボタンと dismiss が `isEnabled() === false`。観測は download 保留中に行う | tests/s24-install-update-and-restart.spec.ts:119-122（保留の根拠は :124-127 と wdio.conf.ts:89-100） | VERIFIED |
| spec.md#scenario-steps (L64) Then インストール成功後、Note A を Flush（NoteBodyEdited）し、永続化を待ってからアプリを再起動する（I-U8） | install 中の未保存編集が、再起動後に起動されたパッケージが書く snapshot に含まれる | tests/s24-install-update-and-restart.spec.ts:132-133（未保存の確認）, :138-140（再起動の観測）, :150-151（snapshot の assert） | VERIFIED（Flush → 永続化 → 再起動の結果）。「await してから」の厳密な順序保証は下記 HIGH-1 を参照 |

観測方法の評価:
- 「Updating…」の観測は、`/download` を `/release-download` が叩かれるまで server 側で保留する設計（wdio.conf.ts:89-100）で、処理中に確実に行われる。観測前に download は完了し得ない。空振りにならない。
- 保留中に `requestCount('/download') === 1` を待つ（spec.ts:124）ので、「処理中」に入ったことの裏取りもある。
- Flush の観測は、クリック時の blur flush と AutoSave debounce（500ms）で説明できない編集（`EDIT_DURING_INSTALL`）だけを判定に使っており、Then の空振り対策として妥当。
- notes.md の変異 A（`restartAfterUpdate` から `flushAll` を削除すると step 2 が RED）の実測結果は、`PageMain.svelte:140-147` の実装と整合する。`restart_app`（commands.rs:77）は `app_handle.restart()` のみで、Rust 側に独自の flush は無い。したがって snapshot に編集が含まれるのは page-main の `flushAll` 経由に限られる。

### Test-points coverage {#test-points-coverage}

`spec.md#test-points` は 8 件（TP1〜TP8）で、`test-points-map.md` の行と 1:1 で一致する（表に無い項目なし）。

| TP | map 状態 | 実際の assert | 判定 |
|---|---|---|---|
| TP1 Given の成立 | COVERED (spec.ts:95) | `latest-version` = `9.9.8`、install ボタンが displayed かつ enabled、Note A が EDITING、`readNoteA()` に編集が無い、`/latest.json >= 1`、`/download === 0`（:97-109） | OK |
| TP2 処理中表示 | COVERED (spec.ts:112) | `Updating…` と install / dismiss の disabled（:119-122） | OK |
| TP3 最新リリースを取得し直す | COVERED (spec.ts:130) | `/latest.json` の request 増加（:143） | OK |
| TP4 download + 署名検証 + install | COVERED (spec.ts:130) | `/download === 1`、AppImage bytes = package（:145-146） | OK（LOW-1 を参照） |
| TP5（核）Flush してから再起動 | COVERED (spec.ts:130) | snapshot に `alpha body S24-pending-edit S24-typed-while-updating`（:151） | OK（MEDIUM-1 / HIGH-1 を参照） |
| TP6 インストールしたバージョンで再起動 | COVERED (spec.ts:130) | marker = `9.9.8`（:140） | OK |
| TP7 比較（I-U7）と版を返すこと | N/A(代替担保) | tests.rs:133 は `Ok(v("0.2.5"))` と fetch / install 1 回ずつを assert。tests.rs:220 は同版 / 旧版で `NoUpdateAvailable` と install 0 回を assert（実在確認済み） | OK |
| TP8 Flush → 再起動の呼出順（page-main が install 成功後に `flushAll` を **await** してから `restart_app` を呼ぶ） | N/A(代替担保) | PageMain.update-check.svelte.test.ts:140 は `invoke:install_update → flushAll → invoke:restart_app` の **呼出順** のみ assert。fake の `flushAll`（:89-91）は呼ばれた瞬間に即 resolve し、完了を記録しない | **UNCOVERED**（HIGH-1） |

### Semantic findings (reviewer: sonnet, capability=deep, fresh context) {#semantic-findings}

- **HIGH-1** apps/promptnotes/src/ui-page/page-main/tests/PageMain.update-check.svelte.test.ts:85-93,140-157 （TP8 の代替担保）: TP8 は「`flushAll` を **await** してから `restart_app`」と規定している。代替担保のテストは呼出順のみ assert しており、`makePendingFlush().flushAll` は即 resolve する fake である。そのため `PageMain.svelte:142` の `await` を外した実装（`void pendingFlush.flushAll(...)` の後に `restartAppFn()`）でもこのテストは GREEN のまま通る。E2E 側にも await の有無を決定的に検出する手段は無い。snapshot の取得は新プロセス起動後なので、IPC の書込み完了が間に合うかどうかの競争に依存する（変異 A は await ではなく `flushAll` 呼出自体の削除であり、await 欠落は検証されていない）。I-U8 の「永続化を待ってから再起動」の核心部分が決定的には assert されていない。gate 規則により UNCOVERED は HIGH（LOW 不可）。
  推奨修正: fake の `flushAll` を deferred promise にする（例: `flushAll:start` を記録し、tick を待って `flushAll:done` を記録）。そのうえで `['invoke:install_update','flushAll:start','flushAll:done','invoke:restart_app']` の順を assert する。または、restart_app が flush 完了前に呼ばれていないことを deferred の resolve 前後で直接 assert する。修正対象は page-main の component test で、scenario の E2E コードの変更は不要。変更後に test-points-map の TP8 行の file:line を更新すること。
- **MEDIUM-1** tests/s24-install-update-and-restart.spec.ts:132-134: 「編集が未保存である」ことの確認（`readNoteA()` が `EDIT_DURING_INSTALL` を含まない）は時点判定であり、直後の `fetch(RELEASE_DOWNLOAD_URL)` で download を解放するまでの間と、解放から install 完了までの間に、AutoSave debounce（500ms）が成立し得る窓が残る。窓の間に debounce が成立すると、`flushAll` を外しても snapshot に編集が含まれ GREEN になる（空振り）。逆の方向（入力から確認までに 500ms を超えて保存済みになる）は spec.ts:133 が RED にするため安全側である。直近の実測（3 回連続 GREEN と変異 A の RED）では顕在化していないが、負荷の高い CI では確率的に偽 GREEN になり得る。
  推奨修正: 次のいずれか。(a) debounce 時間に対し十分余裕のある入力〜解放間隔を保証するため、入力完了の直前でなく直後に未保存確認と `fetch` を連続して行い、その間に待機を挟まない（現状に近いので、この前提を spec 実装ノートへ明記する）。(b) 入力と解放の間に経過時間を測り、400ms 未満であることを assert して、窓を超えた場合は偽 GREEN ではなく RED / retry にする。なお、これは generate 側の改善提案であり、現状を直ちにブロックするものではない。
- **MEDIUM-2** .ori/scenarios/s24-install-update-and-restart/status.yaml / test-points-map.md: TP8 が HIGH-1 のため、`/ori-finalize` に進む前に map の更新が必要（上記のとおり）。台帳（status.yaml）の `current_phase: review` はこの review の結果次第で進めること。
- **LOW-1** tests/s24-install-update-and-restart.spec.ts:145-146 （Then 2 の「署名検証」）: 検証を通った場合（正しい鍵と正しい署名）のみを assert しており、署名検証が実際に働いていること（不正署名は拒否される）はこの scenario では区別できない。署名検証は workflow 上 updater plugin への委譲（install-update step 5）であり、失敗系は S25 の守備範囲と読めるため、本 scenario では LOW とする。`TAURI_TEST_UPDATER_PUBKEY` を seam 経由で差し替えているため、「鍵が使われている」ことは暗黙には裏付けられている（infrastructure.rs の `builder.pubkey(pubkey)`）。
- **LOW-2** tests/s24-install-update-and-restart.spec.ts:134: テストコード内の `fetch(RELEASE_DOWNLOAD_URL)` は、runner config が所有する update server の gate を開けるだけで、サービスの起動ではない。`scenario-test` の「テスト内で service を起動しない」規約には抵触しない。ただし URL を環境変数経由で受ける暗黙の契約なので、spec 実装ノートの「Flush の観測」に既に記載がある点を確認済み（追加対応不要）。

#### local 系 wdio checklist {#wdio-checklist}

| 項目 | 結果 | 根拠 |
|---|---|---|
| `tauri:options.application` が `runtime.binary` と一致 | PASS | wdio.conf.ts:13 `resolve(__dirname,'../../../apps/promptnotes/src-tauri/target/debug/app')` = architecture.md の `apps/promptnotes/src-tauri/target/debug/app` |
| `@wdio/tauri-service` を使用 | PASS | wdio.conf.ts:136 `services: [['@wdio/tauri-service', { driverProvider: 'external' }]]` |
| onPrepare の非 Linux fail fast（`SevereServiceError`） | PASS | wdio.conf.ts:148-152 |
| onPrepare の :4444 使用中 fail fast（`SevereServiceError`） | PASS | wdio.conf.ts:153-158 |
| XDG 4 種の per-run temp と `ORI_SCENARIO_TMP` | PASS | wdio.conf.ts:160,213-217（`HOME` は未変更） |
| `maxInstances: 1` | PASS | wdio.conf.ts:135 |
| fixture seed（Note A、settings.json の `storage_dir`） | PASS | wdio.conf.ts:169-183 |
| テストに service 起動処理が無い | PASS | spec.ts は `fetch` による gate 解放のみ（LOW-2 参照） |
| `browser.execute` の結果を `undefined` 比較していない | PASS | spec.ts は `browser.execute` を使用していない |
| spec 実装ノートの前提条件記録（build 契約 / plugin / XDG / symlink / seed / 署名ツール） | PASS | spec.md:131-141 |
| node_modules symlink | PASS | `.ori/scenarios/node_modules -> ../../apps/promptnotes/node_modules`（実在確認済み） |
| 鍵・署名済みパッケージを commit しない | PASS | onPrepare が temp に実行時生成（wdio.conf.ts:48-57,207）。`.omc/` は gitignore 済み |
| 観測しやすさの前提（debug 限定 seam） | PASS | `#[cfg(debug_assertions)]` で release から compile out（updater.rs / infrastructure.rs の diff で確認） |

### Verdict {#verdict}
**NEEDS_FIX**

理由:
1. 全 Then 4 件はテストの assertion に対応付けられ、E2E 内の UNVERIFIED は 0 件。「Updating…」の観測は download 保留中に行われ、Flush の観測は blur flush / debounce で説明できない編集に限定されており、未実装でも GREEN になる構図は確認されなかった（`flushAll` 削除の変異で RED になる実測も実装と整合）。
2. ただし TP8（page-main が `flushAll` を **await** してから `restart_app`）の代替担保が、呼出順しか assert しておらず、fake の `flushAll` が即 resolve するため `await` を外しても GREEN になる。test-points 網羅 gate により UNCOVERED は HIGH、LOW 不可（HIGH-1）。E2E 側でも await の有無は決定的に検出できない。
3. 差し戻し先: PageMain component test（`PageMain.update-check.svelte.test.ts` の fake `flushAll` を deferred にして完了順を assert）の強化と、test-points-map の TP8 行の file:line 更新。E2E コード自体の assertion 追加は不要。MEDIUM-1（未保存確認の時間窓）は generate 側の改善提案で、直ちにブロックしない。

## Pass 2 {#pass-2}

Pass 1 の HIGH-1 / MEDIUM-1 / MEDIUM-2 への対応を再審査した（single-pass 規則により最後の往復）。

### Syntax checks {#pass-2-syntax-checks}
- test code: PASS 見込み（`statSync` の import 追加、`typeIntoNoteA` の戻り値型を `Promise<number>` に変更。呼出側 3 箇所は整合。tsc は main session 実行済みと報告）
- docker-compose: N/A (compose-service 参加者ゼロ)

### Then ↔ assertion coverage {#pass-2-then-coverage}

Then は引き続き 4 件（`spec.md#scenario-steps` L61-64）。抜けなし。

| Then (source#anchor) | 期待 assertion | テスト (file:line) | 状態 |
|---|---|---|---|
| spec.md#scenario-steps (L61) Then UpdateChannel::install() が最新リリースを取得し直し、0.2.5 > 0.2.4 を確認する（I-U7） | `/latest.json` の request 数が起動時の確認時点より増える | tests/s24-install-update-and-restart.spec.ts:116（基準値）, :153 | VERIFIED（再取得）。比較ロジックは `N/A(代替担保)`: install_update/tests.rs:133 / :220（実在確認済み） |
| spec.md#scenario-steps (L62) Then updater plugin が download + 署名検証 + install を行い、0.2.5 を返す | `/download === 1`、AppImage がパッケージと byte 一致 | tests/s24-install-update-and-restart.spec.ts:155, :156 | VERIFIED（download + install）。「0.2.5 を返す」は `N/A(代替担保)`: install_update/tests.rs:133 |
| spec.md#scenario-steps (L63) Then 処理中は Toast が「Updating…」を表示し、ボタンは押せない | `Updating…`、install / dismiss が disabled（download 保留中に観測） | tests/s24-install-update-and-restart.spec.ts:129-131（保留の裏取り :133-136） | VERIFIED |
| spec.md#scenario-steps (L64) Then インストール成功後、Note A を Flush（NoteBodyEdited）し、永続化を待ってからアプリを再起動する（I-U8） | 再起動後 snapshot に install 中の編集（最後の打鍵 `!` を含む）が存在し、かつ Note A の最終書き込みが最後の打鍵から 500ms 未満 | tests/s24-install-update-and-restart.spec.ts:161（snapshot）, :164-166（mtime − 打鍵時刻 < `AUTOSAVE_DEBOUNCE_MS`）。await の厳密な順序は PageMain.update-check.svelte.test.ts:162-198 | VERIFIED |

### Test-points coverage {#pass-2-test-points-coverage}

TP1〜TP8 の 8 件が `test-points-map.md` と 1:1 で一致。map の行番号は現行コードと一致することを確認した（spec.ts:104 given / :121 step 1 / :139 step 2）。

| TP | 状態 | 実際の assert | 判定 |
|---|---|---|---|
| TP1 | COVERED (spec.ts:104) | :108-118 | OK |
| TP2 | COVERED (spec.ts:121) | :129-131 | OK |
| TP3 | COVERED (spec.ts:139) | :153 | OK |
| TP4 | COVERED (spec.ts:139) | :155-156 | OK |
| TP5（核） | COVERED (spec.ts:139) | :161 と :164-166 | OK（MEDIUM-1 解消、下記） |
| TP6 | COVERED (spec.ts:139) | :150 | OK |
| TP7 | N/A(代替担保) | tests.rs:133 / :220（実在、内容合致） | OK |
| TP8 | N/A(代替担保) | PageMain.update-check.svelte.test.ts:162-198: fake `flushAll` を deferred にし、`flushAll:start` 後 50ms 待っても `invoke:restart_app` が呼ばれないことを assert（:182-185）。`finishFlush()` 後に `install_update → flushAll:start → flushAll:done → restart_app` の順を assert（:187-197）。:140 の呼出順テストも残存（実在） | OK（HIGH-1 解消） |

### Pass 1 指摘の解消確認 {#pass-2-resolution}

- **HIGH-1（TP8）: 解消**。新テストは `flushAll` の完了と `restart_app` の呼出を区別して assert している。`await` を `void` にすると、`flushAll:start` 直後に `restart_app` が呼ばれて :185 が RED になる。この論理は実装（`PageMain.svelte:140-147`）と整合し、coordinator の変異実測（このケースだけ RED）とも一致する。50ms の待機は time-based な否定判定だが、`void` 変異では microtask 内で restart が呼ばれるため余裕は十分で、偽 GREEN の余地は無い。
- **MEDIUM-1（未保存確認の時間窓）: 解消**。次の理由で判定は健全（偽 GREEN 方向に誤らない）。
  - `beforeLastKey` は最後の打鍵送信「前」の時刻（spec.ts:81）なので、実際の打鍵時刻 ≥ `beforeLastKey` であり、`mtime − beforeLastKey` は打鍵からの経過時間を過大評価する。したがって `< 500ms` が成立すれば、実際の経過も 500ms 未満が保証される（誤るとすれば偽 RED 側のみ）。
  - 最後の打鍵 `!` を含む内容を書けるのは Flush か debounce のみ。debounce が書く場合の mtime ≥ 打鍵 + 500ms になるため :166 で RED になる。Flush の外し忘れや、debounce が先に成立する空振りを時刻で排除できる。
  - 最後の打鍵を `typeIntoNoteA(LAST_KEY)` として別呼出に分離したことで、debounce が打鍵ごとにリセットされる前提が明確。余裕は 500ms に対し実測 26〜70ms で十分。
  - 点判定の :143（`not.toContain`）は残るが、窓は :164-166 の時刻判定で補完された。
- **MEDIUM-2（map 更新）: 解消**。TP8 行に新テスト（:162）が追加され、spec.ts の行番号も現行と一致する。

### Semantic findings (reviewer: sonnet, capability=deep, fresh context) {#pass-2-semantic-findings}

- **LOW-1（継続）** tests/s24-install-update-and-restart.spec.ts:155-156: 署名検証は「正しい署名が通る」ことしか示さない。不正署名の拒否は S25 の守備範囲。変更なし、ブロックしない。
- **LOW-3** tests/s24-install-update-and-restart.spec.ts:165: `console.log` による実測値の出力は診断用として許容できる。assert ではない（削除は不要）。
- **LOW-4** .ori/scenarios/s24-install-update-and-restart/spec.md:89,130-132: `spec.md` は派生文書で直接編集禁止（ori-conventions）。今回の追記が `/ori-derive` の再実行経由か手編集かはこのレビューでは判別できない。手編集であれば、`/ori-finalize` 前に正規手順（source 側を直して再 derive）に揃えること。内容自体は spec・テスト・notes と整合している。
- HIGH / MEDIUM の残指摘は無い。

### Verdict {#pass-2-verdict}
**PASS**

理由:
1. Then 4 件はすべて VERIFIED、test-points 8 件のうち UNCOVERED は 0 件。TP7 / TP8 の代替担保は実在し、assert の内容も TP の記述と合致する。
2. Pass 1 の HIGH-1（TP8 の await 順序が未 assert）は、deferred な fake `flushAll` により完了前に `restart_app` が呼ばれないことを決定的に assert するテストで解消された。
3. Pass 1 の MEDIUM-1（debounce が先に成立する偽 GREEN の窓）は、最後の打鍵時刻と Note A の mtime の差による 500ms 判定で解消された。判定は保守側（過大評価）で、偽 GREEN にならない。
4. 残るのは LOW のみ（LOW-1 署名の否定系は S25 の守備範囲、LOW-4 は spec.md の派生手順の確認）。
