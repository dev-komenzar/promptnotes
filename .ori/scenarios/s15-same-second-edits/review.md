# Review: s15-same-second-edits {#review-s15-same-second-edits}

## Pass 1 {#pass-1}

> reviewer agent (`ori-reviewer`) は spawn せず、main session の objective 検証で代替
> （orchestrator 指示: reviewer spawn がハングする既知問題があるため。s1〜s14 と同方針）。

### Syntax checks {#pass-1-syntax}

- tsc (`tsc --noEmit -p tsconfig.json`): PASS（テスト spec は 0 error。既知の
  `wdio.conf.ts TS2353 'tauri:options'` のみ — s1〜s14 と同一の WDIO v9 型定義起因、許容）
- docker-compose: SKIP — compose-service 参加者ゼロ (app `promptnotes` is mode=local)
- E2E: PASS (`wdio run wdio.conf.ts` → **4 passing**, ~1.3s)

### Mode checklist (local tauri) {#mode-checklist}

| item | status | note |
|---|---|---|
| spec `runner: wdio` ↔ test 実行 ↔ wdio.conf.ts | ✅ PASS | 一致（derive chain step 2: 参加 local app の runtime.runner） |
| テストコード内に service lifecycle が無い | ✅ PASS | seed / fixed-now file は onPrepare、テストは assert + invoke のみ |
| local app が docker-compose に含まれない | ✅ PASS | compose 不在が正しい |
| wdio `tauri:options.application` == runtime `binary` | ✅ PASS | `target/debug/app` |
| `@wdio/tauri-service` が services に含まれる | ✅ PASS | `driverProvider: 'external'` |

### Semantic findings (main session objective verification) {#pass-1-findings}

spec ↔ テストコード ↔ domain 整合性:

- **PASS**: **step 1（本 scenario の核・前半）** — seed と同一秒 `now=12:00:00.100` で
  `auto_save_note` を Tauri 境界 invoke → `outcome=saved` / `updated_at="2026-06-20T12:00:00Z"`。
  `.md` は `body="hello v1"` に更新されつつ `updatedAt="20260620120000"` のまま。
  「persist は実行されるが `updatedAt` は同一秒内で同値」（I-N4 補足 / S15 Then 1 回目）を実測 ✓
- **PASS**: **step 2（本 scenario の核・後半）** — 依然として同一秒 `now=12:00:00.800` で再度
  invoke → `outcome=saved` / `updated_at="2026-06-20T12:00:00Z"`。`.md` は `body="hello v2"` /
  `updatedAt="20260620120000"` のまま（S15 Then 2 回目）✓
- **PASS**: **step 3（NoteFeed 冪等）** — `list_notes` を 2 回 invoke し、いずれも
  `[A, B]`（CreatedAt desc、I-F3 の `id` tiebreak）で完全一致。A の `updated_at` は
  `"2026-06-20T12:00:00Z"`（据え置き）。「2 回 sort 再計算しても結果は同じ」（S15 Then）
  を read DTO レベルで実測 ✓
- **PASS**: **step 4（陽性対照 / 非空虚性）** — `now=12:00:01`（次の秒）で invoke すると
  `updated_at="2026-06-20T12:00:01Z"` / `.md` `updatedAt="20260620120001"` に進む。
  これにより step 1-2 の据え置きが「更新されない実装」ではなく「秒精度 truncate による同値」
  であることを保証 ✓
- **PASS**: **persist + publish** — `AutoSaveNoteUseCase::execute` は step 6 persist 成功後に
  step 7 `bus.publish(NoteBodyEdited { updated_at })` を実行してから `Ok(Some(note))` を返す。
  `outcome=saved` はその分岐と同値で、event payload の `updated_at` は返却値と同一式。
  E2E で直接観測できないのは production が `NoOpBus` を注入している構造的制約（`commands.rs`）で、
  slice unit test が担保する（notes.md#event-observation / S9 と同方針）✓
- **PASS**: **`now` が実 HTTP / 実時計ではなく seam 経由** — 同一秒の再現は flaky になるため、
  debug build 限定の `TAURI_TEST_FIXED_NOW_FILE` clock seam で `now` を pin。テストはステップ間に
  ファイル（RFC3339）を書き換えて `12:00:00.100` → `12:00:00.800` → `12:00:01` を制御 ✓
- **PASS**: **production 影響なし** — seam は `#[cfg(debug_assertions)]` ガードで release build は
  compile out。本番の `SystemClock`（`OffsetDateTime::now_utc()`）挙動は不変。env 未設定の
  s14/s9 E2E が回帰 GREEN であることがフォールバック挙動の実測にもなっている ✓
- **PASS**: spec.md frontmatter の `coherence.source: derived` + upstream 6 件（hash 付き:
  validation / auto-save-note / note-aggregate / note-body-edited / note-feed-aggregate /
  glossary-timestamp）✓
- **PASS**: テストデータ独立性 — `mkdtempSync` の temp dir に config/data/storage/fixed-now を
  隔離し `onComplete` で `rmSync`。`TAURI_TEST_STORAGE_DIR` override により実 user 環境を
  汚さない ✓
- **MEDIUM**: **event `NoteBodyEdited` の E2E 直接観測は無い** — Tauri command 配線が `NoOpBus` を
  注入するため（`commands.rs`: 「The Note Feed BC will subscribe here once it lands.」）観測対象が
  存在しない。担保は slice unit test（`auto_save_note/tests.rs` TP-H3 /
  `tp_h3_body_changed_publishes_note_body_edited_once_with_correct_payload`）に委ね、E2E では
  `outcome=saved` + `.md` 差分で間接検証 ✓（notes.md#event-observation に明記。prod wiring は
  `ori-znq` の follow-up）
- **LOW**: `TAURI_TEST_FIXED_NOW_FILE` の clock seam は `auto_save_note` のみに効く。flush-note は
  validation 上も S15 の対象だが、本 scenario は AutoSave 経路（workflow step 5 = `edit_body`）で
  `Note::edit_body` の不変条件を検証しており、flush 経路の同一 `edit_body` 呼び出しは同一コードを
  通る（notes.md#known-issues）✓
- **LOW**: 固定時刻ファイルは app 起動時に継承した env 経由で解決される。1 プロセス 1 ファイル
  共有のため、テストはファイル内容の書き換えで時刻を切替える（env 再設定は不可、notes.md 明記）✓

### Production fix discovery {#pass-1-production-fix}

production 側は S15 の不変条件（`Timestamp` の秒 truncate / `Note::edit_body` の `updatedAt = now` /
`FsNoteRepository::write` の `YYYYMMDDhhmmss` 永続化）を**既に満たしていた**。E2E を決定論的に
するため、**debug build 限定の clock seam** のみを追加した:

- `apps/promptnotes/src-tauri/src/note_capture/slices/auto_save_note/commands.rs`:
  `SystemClock::now()` に `TAURI_TEST_FIXED_NOW_FILE`（`#[cfg(debug_assertions)]`）分岐を追加。
  ファイルの RFC3339 を parse して `Timestamp::from_offset_datetime`（秒 truncate）で返す。
  空 / `system` / 不正値は実時計にフォールバック。release build はこの分岐を compile out するため
  本番挙動は不変（S14 の `TAURI_TEST_UPDATER_ENDPOINT` seam と同型）。

RED 実測（テストが空振りでないことの確認）:

- **変異（`Timestamp::from_offset_datetime` の `replace_nanosecond(0)` を除去）**:
  秒 truncate が失われる → `bun run build:test` → E2E **2 failing / 2 passing**:
  - step 1: `Expected: "2026-06-20T12:00:00Z"` に対し `Received: "2026-06-20T12:00:00.1Z"`
  - step 2: `Expected: "2026-06-20T12:00:00Z"` に対し `Received: "2026-06-20T12:00:00.8Z"`
  - step 3 / step 4 は pass（file は `YYYYMMDDhhmmss` 書き出しのため sub-second が落ちる）
- **復元後** → 再ビルド → E2E **4 passing (~1.3s)** で GREEN。

production への正味の変更は **1 file / 30 行**（debug seam のみ）。

回帰確認:

- frontend unit test: **143 passed / 15 files**
- Rust `auto_save_note` unit: **20 passed**
- E2E 回帰: s14-update-check-failure **4 passing**、s9-idempotent-autosave **3 passing**
- Rust 全体 (`cargo test`): 328 passed / 2 failed。失敗 2 件は fixture が 2026-06 固定で `now` が
  実時計依存の date-range テスト（`list_feed::tp_f5_last_7_days` / `tp_f6_and_composition`）。
  本変更は `auto_save_note` のみで `list_feed` に触れておらず、s14 でも同一の既存 time-bomb
  として記録済み（本変更とは無関係）。

### Disposition {#pass-1-disposition}

- 指摘は MEDIUM/LOW のみで、いずれも EventBus 配線（NoOpBus）の構造的制約または seam の
  スコープ注記。spec / notes に明記済みで、E2E の観測点（同一秒の据え置き / persist 実行 /
  NoteFeed 冪等 / 陽性対照）は満たす。
- production 変更は debug seam 1 file のみ。RED 検出力は truncate 変異の実測で確認済み。
- Verdict: **PASS**

<!-- verdict=PASS -->
