# Review: s7-undo-after-toast {#review-s7-undo-after-toast}

## Pass 1 {#pass-1}

### Syntax checks

- tsc (`tsc --noEmit -p tsconfig.json`): PASS（テスト spec は 0 error。既知の
  `wdio.conf.ts TS2353 'tauri:options'` のみ — s1〜s6 と同一の WDIO v9 型定義起因、許容）
- docker-compose: SKIP — compose-service 参加者ゼロ (app `promptnotes` is mode=local)
- E2E: PASS (`wdio run wdio.conf.ts` → 1 passing, ~9.9s)

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

- **PASS**: Given（A/B 表示・未削除・toast 0 件）→ `before` で 2 block wait +
  初期 `.md` 2 件存在 + toast 0 件を assert ✓
- **PASS**: t0（A 削除）→ A の `.md` 不在 / `trash/A.md` 存在 / block 2→1 / toast A 1 件 ✓
- **PASS**: t0+3s（B 削除）→ B の `.md` 不在 / block 0 / **toast 2 件**（A 置換なし）✓
- **PASS**: t1（A の Toast 消失）→ `waitUntil` で `data-toast-id=A` が 0 件になるまで
  待機（TTL 5s）/ **B の Toast は残存（1 件）** / A の `.md` は依然 absent ✓
- **PASS**: t2（復元 API 試行）→ `window.__TAURI_INTERNALS__.invoke('restore_deleted_note', {noteId:A})`
  が reject し、payload の `kind === 'no_undo_available'`。A の `.md` は原パスに復帰せず
  `trash/A.md` が残存（= `NoteRestoredFromTrash` の副作用が起きていない）✓
  → `validation.md#s7-then` の「対応する DeletedNote がスタックに無いため reject」を直接検証
- **PASS**: per-toast 独立性（t2 直後）→ B の Toast undo が成功し B の `.md` が復元
  （body=`bravo`）/ B の Toast 消滅 / toast stack 0 / block 1（B のみ）✓
  → `screen-1.md#cross-toast-display`「1 つの Undo 期限切れ後はその Undo のみ reject される」に忠実
- **PASS**: テストデータ独立性 — `mkdtempSync` の temp storageDir に A/B を seed、
  `onComplete` で `rmSync` ✓
- **PASS**: spec.md frontmatter の `coherence.source: derived` + upstream 5 件（hash 付き）✓
- **MEDIUM**: event `NoteRestoredFromTrash` 非発行の明示検証は無い。production 配線が
  `NoOpBus` のため E2E 観測不可であり、slice unit test（`restore_deleted_note/tests.rs`
  I-RDN1）+ A の `.md` 非復帰で間接検証。`notes.md#event-verification-policy` に制約として
  文書化済み → scenario レベルでは許容 ✓
- **LOW**: `data-testid` / `data-toast-id` セレクタの存在は実装依存（`@ori-generated` marker
  でドリフト管理）✓
- **LOW**: `@wdio/tauri-service` (`driverProvider: 'external'`) の `patchedExecute` は
  `browser.executeAsync` を扱えないため、t2 の直接 invoke は同期 `browser.execute` で
  kick-off → window 上の結果を poll する方式（常に文字列を返し WebKit の serialization 制約を回避）。
  s1〜s6 も同期 `browser.execute` のみ使用しており既知の実行基盤制約 ✓

### Production fix discovery {#pass-1-production-fix}

E2E 初回実行が **t2 で RED**: 期限切れ後も `restore_deleted_note` が成功し
`attempt.ok === true`（`Expected: false, Received: true` @ `spec.ts:147`）。
原因は application service 側 `InMemoryUndoStack` に TTL が無く、frontend の Toast
timeout（5s）で UI エントリが消えても Rust 側 `DeletedNote` が残り続けていたこと。
`delete-note.md#dependencies`「`UndoStack` は TTL 管理付き、各要素ごとに個別タイマー」/
`restore-deleted-note.md#errors`「Toast 消失後は `NoUndoAvailable`」に違反する production の不足。

- 最小修正: `InMemoryUndoStack` を `Mutex<Vec<Entry>>`（`Entry { deleted, expires_at }`）へ変更。
  push 時に `Instant::now() + UNDO_TTL (5s)` を付与し、`push` / `find_by_id` / `remove_by_id`
  のアクセス時に期限切れを lazy prune。`UndoStack` trait 形は不変・frontend 変更なし。
- 回帰確認:
  - Rust unit test `note_capture::shared::adapters::undo_stack`: **4/4 pass**
    （新規 `expired_entry_is_pruned_and_not_found` / `expiry_is_per_element` 含む）
  - E2E: 修正前 **1 failing**（`Received: true`）→ 修正後 **1 passing (9.9s)**
  - `cargo clippy --all-targets`: 変更ファイルに警告なし
  - `cargo test --lib`: 328 passed / 2 failed。失敗 2 件
    (`note_feed::list_feed::tests::tp_f5_last_7_days` / `tp_f6_and_composition`) は
    固定日付 (2026-06) × 現在日付 (2026-09) に依存する**既存** failure で本変更と無関係
    （未変更モジュール・time-dependent）

### Disposition {#pass-1-disposition}

- 指摘は MEDIUM/LOW のみで、いずれも制約として文書化済みまたは実行基盤の既知制約。
- production 不足は本 scenario の正当な RED 要因であり、最小修正で GREEN 化済み
  （RED → GREEN を実測で確認）。
- Verdict: **PASS**
