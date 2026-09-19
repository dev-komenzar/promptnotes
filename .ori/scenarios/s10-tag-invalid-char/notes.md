# s10-tag-invalid-char — Scenario implementation notes

## Event 検証の方針 (EventBus 境界の制約) {#event-verification-policy}

`assign_tag` の application service は成功時に `EventBus` へ `NoteTagsChanged` を publish するが、
Tauri の production 配線（`commands.rs`）は `NoOpBus` を注入している
（`// The Note Feed BC will subscribe here once it lands.`）。したがって E2E で domain event を
直接観測する対象は無い。event **非発行**の担保は slice `assign-tag` の unit test に委ね、E2E では
`assign_tag` の reject（`invalid_tag`）と `.md` 不変で間接検証する。

## テスト方式: UI 駆動 + Tauri 境界 invoke {#test-approach}

S10 の核は **`Tag::new` の禁止文字 reject（I-N6）と、`parseTag` が `load_note` より前に走るため
`Note::assign_tag` に到達しないこと**。観測点は 2 つ:

- **UI 経路 (step 1)**: Block をクリックして `data-block-state="EDITING"` に遷移させ、タグ入力欄に
  `"foo,bar"` を入力 → Enter。`Block.svelte#submitTagInput` が `assign_tag` の `invalid_tag` を
  catch し `[data-testid="screen-1-block-tag-error"]` を表示する。チップ非追加と `.md` 不変を確認。
- **Tauri 境界経路 (step 2)**: `window.__TAURI_INTERNALS__.invoke('assign_tag', { noteId, rawTag })`
  を同期 `browser.execute` で kick-off し、window 上の結果を poll する
  （`@wdio/tauri-service` (driverProvider=external) の `patchedExecute` は `browser.executeAsync` を
  扱えないため。s7 / s8 / s9 と同方式）。UI の `<input type="text">` では入力できない `\t` / `\n`
  を含む禁止文字集合全件 + 空文字（`TagError::Empty`）を検証する。

step 3 は対照として正常タグ `"valid"` を assign し、`.md` の tags 更新と `updatedAt` 更新、
UI チップ出現を確認する（reject 経路が過剰一般化されていないことの確認）。

seed の Note A は `wdio.conf.ts` の `onPrepare` が `TAURI_TEST_STORAGE_DIR` に
`tags: []` で投入する。

## テストカバレッジ {#coverage}

| 検証項目 | 検証方法 |
|---|---|
| 初期状態（A 表示 / tags `[]` / `updatedAt` t0） | E2E (`before`: block wait + `readFileSync`) |
| **UI 経由 reject（本 scenario の核・UI 側）** | E2E step1（`foo,bar` → エラーメッセージ `invalid characters` + チップ非追加 + `.md` 完全一致） |
| **禁止文字集合 ` `, `\t`, `\n`, `,`, `[`, `]` 全体の reject** | E2E step2（Tauri 境界 invoke → `kind=invalid_tag` / `reason` に `invalid character`） |
| 空文字（`TagError::Empty`）の reject | E2E step2（Tauri 境界 invoke → `kind=invalid_tag`） |
| `Note::assign_tag` 非到達 / write なし | reject 後の `.md` 完全一致（tags `[]` / `updatedAt=t0`）で間接検証 |
| event `NoteTagsChanged` 非発行 | slice `assign-tag` unit test（EventBus 境界 NoOpBus のため E2E 不可） |
| 対照: 正常タグの永続化 | E2E step3（`valid` → tags `[valid]` + `updatedAt≠t0` + チップ出現） |

## RED → GREEN 検証 {#red-green}

production 側は初回から S10 の全前提を備えており、**初回 E2E は GREEN（3 passing, 3.1s）**。
テストが空振りでないことを示すため、`Tag::new` の禁止文字ガードを一時的に無効化
（`FORBIDDEN_TAG_CHARS: &[char] = &[]` — tag.rs）して `bun run build:test` で再ビルドし、RED を実測した:

- ガード無効時: **3 failing (7.5s)**
  - step 1 RED: `foo,bar` が assign されてしまい、`screen-1-block-tag-error` が出現しない
  - step 2 RED: `expect(res.ok).toBe(false)` に対し `Expected: false / Received: true`
  - step 3 RED: step1/2 で `updatedAt` が更新されたため `Expected: "20260620120000" / Received: "20260919085921"`
- ガード復元後（`git checkout` + `bun run build:test`）: **3 passing (3.2s)** で GREEN

したがって本 scenario のテストは `Tag::new` の禁止文字ガードの欠落を実際に検出する。

## production 変更 {#production-change}

**なし**。S10 の前提はすべて既存実装に存在する:

1. `Tag::new` の `FORBIDDEN_TAG_CHARS` 検査（`apps/promptnotes/src-tauri/src/note_capture/shared/types/tag.rs`）
2. `AssignTagUseCase::execute` の Step 1 `parseTag`（`load_note` より前）→ `AssignTagError::InvalidTag`
   （`apps/promptnotes/src-tauri/src/note_capture/slices/assign_tag/application.rs`）
3. `assign_tag` command の `AssignTagErrorDto::InvalidTag { name, reason }`（`kind: "invalid_tag"`）
   （`apps/promptnotes/src-tauri/src/note_capture/slices/assign_tag/commands.rs`）
4. UI の `invalid_tag` catch + エラー表示
   （`apps/promptnotes/src/ui-page/page-main/components/Block.svelte#submitTagInput`）

回帰確認:

- frontend unit test: **143 passed / 15 files**
- E2E 回帰: s4-tag-assign-normalize **2 passing**（タグ付与の正規化 / 永続化）、
  s2-autosave-debounce **2 passing**（no-op autosave 含む）
- Rust: 変更なし（RED 検証時の一時 mutation は `git checkout` で復元済み。`git diff` clean）
- バイナリ: RED/GREEN 検証は `VITE_WDIO_TEST=1 bun run tauri build --debug --no-bundle`
  （`bun run build:test`）で再生成済み