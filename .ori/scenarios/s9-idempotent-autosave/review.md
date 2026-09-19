# Review: s9-idempotent-autosave {#review-s9-idempotent-autosave}

## Pass 1 {#pass-1}

> reviewer agent (`ori-reviewer`) は spawn せず、main session の objective 検証で代替
> （orchestrator 指示: reviewer spawn がハングする既知問題があるため。s1 Pass 2 / s7 / s8 と同方針）。

### Syntax checks {#pass-1-syntax}

- tsc (`tsc --noEmit -p tsconfig.json`): PASS（テスト spec は 0 error。既知の
  `wdio.conf.ts TS2353 'tauri:options'` のみ — s1〜s8 と同一の WDIO v9 型定義起因、許容）
- docker-compose: SKIP — compose-service 参加者ゼロ (app `promptnotes` is mode=local)
- E2E: PASS (`wdio run wdio.conf.ts` → **3 passing**, ~2.6s)

### Mode checklist (local tauri) {#mode-checklist}

| item | status | note |
|---|---|---|
| spec `runner: wdio` ↔ test 実行 ↔ wdio.conf.ts | ✅ PASS | 一致 |
| テストコード内に service lifecycle が無い | ✅ PASS | seed は onPrepare、テストは操作のみ |
| local app が docker-compose に含まれない | ✅ PASS | compose 不在が正しい |
| wdio `tauri:options.application` == runtime `binary` | ✅ PASS | `target/debug/app` |
| `@wdio/tauri-service` が services に含まれる | ✅ PASS | `driverProvider: 'external'` |

### Semantic findings (main session objective verification) {#pass-1-findings}

spec ↔ テストコード ↔ domain 整合性:

- **PASS**: Given（A 表示 / body `hello` / `updatedAt=t0`）→ `before` で block wait +
  `readFileSync` の body / `updatedAt` を assert ✓
- **PASS**: **step 1** — Block をクリックして `data-block-state="EDITING"` へ遷移させ、
  debounce 区間（500ms）を超えて 900ms 待機しても A の `.md` は完全一致（body `hello` /
  `updatedAt=t0`）。EDITING 遷移のみでは AutoSave が発火しないことを UI 経路で確認 ✓
- **PASS**: **step 2（本 scenario の核）** — `auto_save_note { noteId, newBody: "hello" }`
  （現在 body と同値）の返却 DTO が `{ outcome: "no_op" }`。`.md` は完全一致のまま。
  → `validation.md#s9-then`「AutoSave 経路は body 変化を検知 → 何もしない / Note::edit_body は
  呼び出されない / event NoteBodyEdited は発行されない / updatedAt = t0 のまま」を直接検証 ✓
- **PASS**: **step 3（対照）** — `auto_save_note { noteId, newBody: "hello world" }` は
  `{ outcome: "saved", id: A, updated_at ≠ t0 }` を返し、`.md` の body が `hello world`、
  `updatedAt` が更新される。→ ガードが同値のみを抑制し、変化を取りこぼさないことを確認 ✓
- **PASS**: spec.md frontmatter の `coherence.source: derived` + upstream 4 件（hash 付き:
  validation / auto-save-note / note-aggregate / note-body-edited）✓
- **PASS**: テストデータ独立性 — `mkdtempSync` の temp storageDir に A を seed、
  `onComplete` で `rmSync` ✓
- **MEDIUM**: event `NoteBodyEdited` 非発行の E2E 直接観測は無い。Tauri command 配線が
  `NoOpBus` を注入するため（`commands.rs`: 「The Note Feed BC will subscribe here once it lands.」）
  E2E で観測すべき event が存在しない。担保は slice unit test TP-I3
  （`body_unchanged_skips_publish`）に委ね、E2E では `outcome=no_op` と `.md` 不変で間接検証 ✓
- **LOW**: step 1 の debounce 待機は固定 `browser.pause(900)`。WebKitWebDriver で実測 GREEN
  のため実害なし（`waitUntil` への置換は不要）✓
- **LOW**: `data-block-id` セレクタの存在は実装依存（`@ori-generated` marker でドリフト管理）✓

### Production fix discovery {#pass-1-production-fix}

初回 E2E は **3 passing（GREEN on first run）**。production 側は既に S9 ガード
（`AutoSaveNoteUseCase::execute` の `compare_body` → `BodyDiff::Unchanged => return Ok(None)`、
`apps/promptnotes/src-tauri/src/note_capture/slices/auto_save_note/application.rs`）を備えており、
**不足は無かった**。テストが空振りでないことを示すため、ガードを一時無効化して RED を実測:

- ガード無効（`BodyDiff::Unchanged` を `edit_body` へ流す）→ `bun run build:test` →
  **step 2 RED**: `Expected: "no_op" / Received: "saved"`（step 1 / 3 は pass）
- `git checkout` で復元 → `bun run build:test` → **3 passing (2.6s)** で GREEN

したがって本 scenario のテストは S9 ガードの欠落・regression を実際に検出する。
production への正味の変更は **無し**（RED 検証の一時 mutation は復元済み、`git diff` clean）。

回帰確認:

- frontend unit test: **143 passed / 15 files**
- E2E 回帰: s2-autosave-debounce **2 passing**（`no-op autosave` = S9 idempotency 含む）
- Rust: 変更なし

### Disposition {#pass-1-disposition}

- 指摘は MEDIUM/LOW のみで、いずれも EventBus 配線（NoOpBus）の構造的制約または
  実行基盤の既知制約。E2E で観測不能な event 非発行は slice unit test が担保する。
- production 不足は無く、テストの RED 検出力（ガード無効時に step 2 が落ちる）を実測で確認済み。
- Verdict: **PASS**
