# s9-idempotent-autosave — Scenario implementation notes

## Event 検証の方針 (EventBus 境界の制約) {#event-verification-policy}

`auto_save_note` の application service は `EventBus` に `NoteBodyEdited` を publish するが、
Tauri の production 配線（`commands.rs`）は `NoOpBus` を注入している
（`// The Note Feed BC will subscribe here once it lands.`）。したがって E2E で domain event を
直接観測する対象は無い。event **非発行**の担保は slice unit test
（`auto_save_note/tests.rs` の TP-I3 `body_unchanged_skips_publish`）に委ね、E2E では
`auto_save_note` の返却 `outcome` と `.md` の不変で間接検証する。

## テスト方式: UI 遷移 + Tauri 境界 invoke {#test-approach}

S9 の核は **application service レベルの冪等性ガード**（`compare_body` →
`BodyDiff::Unchanged` → `Ok(None)`）であり、その観測点は Tauri command 境界が最も直接的。
`window.__TAURI_INTERNALS__.invoke('auto_save_note', { noteId, newBody })` を同期
`browser.execute` で kick-off し、window 上の結果を poll する（`@wdio/tauri-service`
(driverProvider=external) の `patchedExecute` は `browser.executeAsync` を扱えないため。s7 / s8 と同方式）。

前半（step 1）は UI 駆動: Block をクリックして `data-block-state="EDITING"` に遷移させ、
debounce 区間（500ms）を超えて待機しても `.md` が変化しないことを確認する。
後半（step 2 / 3）は Tauri 境界 invoke で「同一 body → no_op」「body 変化 → saved」を対比する。
seed の Note A は `wdio.conf.ts` の `onPrepare` が `TAURI_TEST_STORAGE_DIR` に
unquoted inline frontmatter 形式で投入する。

## テストカバレッジ {#coverage}

| 検証項目 | 検証方法 |
|---|---|
| 初期状態（A 表示 / body `hello` / `updatedAt` t0） | E2E (`before`: block wait + `readFileSync`) |
| **EDITING 遷移のみでは AutoSave 不発火（本 scenario の核・前半）** | E2E step1（クリック → `data-block-state=EDITING` → 900ms 待機 → `.md` 完全一致） |
| **同一 body の `auto_save_note` は no_op（本 scenario の核・後半）** | E2E step2（DTO `outcome=no_op` + `.md` 不変 + `updatedAt=t0`） |
| 対照: body 変化時は Saved + `updatedAt` 更新 | E2E step3（DTO `outcome=saved` + body `hello world` + `updatedAt≠t0`） |
| `Note::edit_body` 非呼出 / write なし | no_op 時の `.md` 不変（write が起きたら `updatedAt` が変わる）で間接検証 |
| event `NoteBodyEdited` 非発行 | slice unit test TP-I3（EventBus 境界 NoOpBus のため E2E 不可） |

## RED → GREEN 検証 {#red-green}

production 側は初回から S9 ガード（`compare_body` の `BodyDiff::Unchanged → return Ok(None)`）を
備えており、**初回 E2E は GREEN（3 passing）**。テストが空振りでないことを示すため、
`AutoSaveNoteUseCase::execute` のガードを一時的に無効化（`BodyDiff::Unchanged` を
`Changed` と同様に `edit_body` へ流す）して `bun run build:test` で再ビルドし、RED を実測した:

- ガード無効時: **step 2 が RED** — `expect(outcome.outcome).toBe('no_op')` に対し
  `Expected: "no_op" / Received: "saved"`（step 1 / 3 は pass）
- ガード復元後（`git checkout` + `bun run build:test`）: **3 passing (2.6s)** で GREEN

したがって本 scenario のテストは S9 ガードの欠落を実際に検出する。

## production 変更 {#production-change}

**なし**。S9 ガードは slice `auto-save-note` 実装時（S2/S9 対応）に既に導入済みで、
`apps/promptnotes/src-tauri/src/note_capture/slices/auto_save_note/application.rs` の
`compare_body` / `BodyDiff::Unchanged => return Ok(None)` が該当する。slice unit test
TP-I1 / TP-I2 / TP-I3 が write なし / publish なしを既に固定している。

回帰確認:

- frontend unit test: **143 passed / 15 files**（変更なしのため既存 GREEN を再確認）
- E2E 回帰: s2-autosave-debounce **2 passing**（`no-op autosave` テスト含む）
- Rust: 変更なし（RED 検証時の一時 mutation は `git checkout` で復元済み。`git diff` clean）
- バイナリ: RED/GREEN 検証は `VITE_WDIO_TEST=1 bun run tauri build --debug --no-bundle`
  （`bun run build:test`）で再生成済み
