# Review: s13-quit-flush {#review-s13-quit-flush}

## Pass 1 {#pass-1}

> reviewer agent (`ori-reviewer`) は spawn せず、main session の objective 検証で代替
> （orchestrator 指示: reviewer spawn がハングする既知問題があるため。s1〜s12 と同方針）。

### Syntax checks {#pass-1-syntax}

- tsc (`tsc --noEmit -p tsconfig.json`): PASS（テスト spec は 0 error。既知の
  `wdio.conf.ts TS2353 'tauri:options'` のみ — s1〜s12 と同一の WDIO v9 型定義起因、許容）
- docker-compose: SKIP — compose-service 参加者ゼロ (app `promptnotes` is mode=local)
- E2E: PASS (`wdio run wdio.conf.ts` → **3 passing**, ~3.9s)

### Mode checklist (local tauri) {#mode-checklist}

| item | status | note |
|---|---|---|
| spec `runner: wdio` ↔ test 実行 ↔ wdio.conf.ts | ✅ PASS | 一致 |
| テストコード内に service lifecycle が無い | ✅ PASS | seed は onPrepare、テストは assert + invoke のみ |
| local app が docker-compose に含まれない | ✅ PASS | compose 不在が正しい |
| wdio `tauri:options.application` == runtime `binary` | ✅ PASS | `target/debug/app` |
| `@wdio/tauri-service` が services に含まれる | ✅ PASS | `driverProvider: 'external'` |

### Semantic findings (main session objective verification) {#pass-1-findings}

spec ↔ テストコード ↔ domain 整合性:

- **PASS**: **step 3（本 scenario の核）** — 実ブロックを EDITING にして debounce 中の pending_body
  (`delta body!`) を抱えた状態で `flush_note(app_quit)` を発火 → `outcome=flushed` が返り、`.md` が
  `delta body!` に更新される。`flushed` は「debounce が先に fire していれば `no_op` になる」ため、
  **debounce を待たず Flush が永続化した**ことの決定的証拠。debounce 後の重複永続化も無い（冪等）✓
- **PASS**: **step 1（連続 Flush の逐次性）** — A → B → C を順次 await し、各段階で
  「当該 Note のみ更新・後続は seed のまま」を `.md` 読み分けで確認（domain/workflows/flush-note.md#notes
  「AppQuit はすべての EDITING Note に flush-note を発行、順序は処理順、並列性なし」に対応）✓
- **PASS**: **step 2（冪等性ガード C-FL4）** — 同じ pending_body で app_quit Flush → `outcome=no_op`、
  ファイル bytes（updatedAt 含む）不変。`Note::edit_body` のバイト等価比較が境界で機能 ✓
- **PASS**: **trigger=app_quit 受理** — `flush_note` command が `trigger: "app_quit"`（FlushTriggerDto）
  を受理して `Flushed { id, updated_at }` / `NoOp` を返す。Tauri 境界（specta bindings 相当）経由 invoke ✓
- **PASS**: **I-N4** — Flush 後の `updatedAt` が seed 値から進む（step1/step3 で assert）✓
- **PASS**: spec.md frontmatter の `coherence.source: derived` + upstream 6 件（hash 付き:
  validation / flush-note / auto-save-note / note-aggregate / note-body-edited / screen-1）✓
- **PASS**: テストデータ独立性 — `mkdtempSync` の temp dir に config/data/storage を隔離し `onComplete` で
  `rmSync`。`TAURI_TEST_STORAGE_DIR` override により実 user 環境を汚さない ✓
- **PASS**: step1/step3 の永続化 assertion が空振りでないことを変異テストで実測（後述 Production fix
  discovery 変異 A）✓
- **MEDIUM**: **quit trigger の E2E 再現制約** — capability に `core:window:allow-close` が無いため、
  E2E は `getCurrentWindow().close()` で CloseRequested を発火できず、`flush_note(app_quit)` invoke で
  代替している。frontend orchestration（`pendingFlushRegistry` の順序 / `onCloseRequested` intercept /
  `destroy()`）自体は E2E 未検証で、既存 unit test（`pending-flush.test.ts`）+ production code が担保する。
  spec `#impl-notes` / `#test-points` / notes.md#test-approach / #known-issues に明記済み。E2E としての
  観測点（app_quit Flush の永続化・逐次性・冪等性）は満たす ✓
- **MEDIUM**: **I-PM10 との整合** — validation の Given「A, B, C が EDITING」は `screen-1` の同時 EDITING
  高々 1 つ制約により UI では再現不可。連続 Flush を Tauri 境界で再現する方式を spec に明記済み ✓
- **LOW**: `FlushTrigger` は event payload / use case 内部状態に影響しない（C-FL8）。E2E は trigger 差を
  app_quit 経路の受理で確認するに留める（BlockBlur 経路は s3 が担当）✓
- **LOW**: `NoteBodyEdited` の発行・購読冪等は in-process EventBus（現状 `NoOpBus`、ori-znq で prod wiring
  が follow-up）。E2E の観測は `.md` 永続化（event の前提）に置いている ✓

### Production fix discovery {#pass-1-production-fix}

production 側は S13 の前提を**既に満たしていた**（追加実装は不要）:

1. `flush_note` Tauri command が `trigger: app_quit` を受理し `Note::edit_body` を永続化
   （`apps/promptnotes/src-tauri/src/note_capture/slices/flush_note/commands.rs`）
2. body 不変時の冪等性ガードが `FlushOutcome::NoOp` を返す（`flush_note/application.rs` C-FL4）
3. quit orchestration（`pendingFlushRegistry.flushAll('app_quit')` の逐次 await +
   `PageMain.svelte` の `onCloseRequested` intercept）は ori-73q で実装済み

RED 実測（テストが空振りでないことの確認）:

- **変異 A（`flush_note/application.rs` step 7 の `repository.write` を skip）**
  → `VITE_WDIO_TEST=1 bun run tauri build --debug --no-bundle`（`bun run build:test`）→ E2E **3 failing**:
  step1 `Expected "alpha quit-A"` が seed のまま / step2 `Expected "no_op"` Received `"flushed"` /
  step3 `Expected "delta body!"` が seed のまま。
- **復元後** → 再ビルド → E2E **3 passing (~3.9s)** で GREEN。

production への正味の変更は **0 files**（`git diff apps/` 空）。

回帰確認:

- frontend unit test: **143 passed / 15 files**
- svelte-check: 変更なし（production 0 files）
- E2E 回帰: s3-flush-on-blur **3 passing**、s12-startup-state **1 passing**
- Rust: 変更なし

### Disposition {#pass-1-disposition}

- 指摘は MEDIUM/LOW のみで、いずれも E2E 再現制約（capability による quit trigger 代替）と
  I-PM10 由来の検証方式の注記。spec / notes に明記済みで、E2E の観測点（app_quit Flush の
  永続化 / 逐次性 / 冪等性 / trigger 受理）は満たす。
- production 変更は不要（既存実装で充足）。RED 検出力は変異 A の実測で確認済み。
- Verdict: **PASS**
