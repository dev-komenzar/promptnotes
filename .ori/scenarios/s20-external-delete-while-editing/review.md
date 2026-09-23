# Review: s20-external-delete-while-editing {#review-s20-external-delete-while-editing}

## Pass 1 {#pass-1}

> reviewer agent (`ori-reviewer`) は spawn せず、main session の objective 検証で代替
> （orchestrator 指示: reviewer spawn がハングする既知問題があるため。s1〜s19 と同方針）。

### Syntax checks {#pass-1-syntax}

- tsc (`tsc --noEmit -p tsconfig.json`): PASS（テスト spec は 0 error。既知の
  `wdio.conf.ts TS2353 'tauri:options'` のみ — s1〜s19 と同一の WDIO v9 型定義起因、許容）
- svelte-check (`bun run check`): PASS（0 errors / 0 warnings）
- eslint（変更 file）: PASS
- cargo clippy（新 slice `recreate_note`）: PASS（新規指摘ゼロ。既存ファイルの warning のみ）
- docker-compose: SKIP — compose-service 参加者ゼロ (app `promptnotes` is mode=local)。
  `generate-docker-compose.sh` も「compose-service 系参加者ゼロ — docker-compose.yml は省略」
- E2E: PASS (`wdio run wdio.conf.ts` → **5 passing**, ~9–10s。3 回連続実行で安定)

### Mode checklist (local tauri) {#mode-checklist}

| item | status | note |
|---|---|---|
| spec `runner: wdio` ↔ test 実行 ↔ wdio.conf.ts | ✅ PASS | 一致（derive chain step 2: 参加 local app の runtime.runner） |
| テストコード内に service lifecycle が無い | ✅ PASS | seed / temp dir 隔離は onPrepare、テストは node:fs 操作 + assert + dialog 操作のみ |
| local app が docker-compose に含まれない | ✅ PASS | compose 不在が正しい |
| wdio `tauri:options.application` == runtime `binary` | ✅ PASS | `apps/promptnotes/src-tauri/target/debug/app` |
| `@wdio/tauri-service` が services に含まれる | ✅ PASS | `driverProvider: 'external'` |

### Semantic findings (main session objective verification) {#pass-1-findings}

spec ↔ テストコード ↔ domain 整合性:

- **PASS**: **step 1（本 scenario の核）** — Given で Block A を EDITING にし `"hello local"` に
  編集（AutoSave 永続化を待機）、その後テストプロセスが `node:fs` の `unlinkSync` で
  `storage_dir/20260620120000.md` を外部削除する。`list_notes` の手動 invoke / 再起動を
  一切行わず `[data-testid="widget-external-delete-notice"]` が表示され、Block A が
  `data-block-state="EDITING"` のままであることを実測。validation#s20-when 1-3 /
  Then「編集中のノートが外部で削除されました」通知を満たす ✓
- **PASS**: **step 2（通知内容）** — `delete-notice-title` = `20260620120000.md`、
  `delete-notice-body-local` value = `"hello local"`。validation Then の「現在の編集中 body を
  提示」を実測 ✓
- **PASS**: **step 3（Save as new file）** — `delete-notice-save-as-new` で通知が閉じ、
  `storage_dir/20260620120000.md` が **元の id のまま** 編集中内容 `"hello local"` で再作成され
  （FS 実測）、Block A が IDLE に遷移することを実測。validation Then「現在の編集中内容で
  `.md` を再作成 / Block は IDLE」に対応 ✓
- **PASS**: **step 4（Discard）** — 再度 EDITING + 外部削除で通知を表示させ、
  `delete-notice-discard` で通知が閉じ、フィードから Block A が除去される（`remove_note` /
  I-F8）ことを実測。validation Then「NoteFeed: `remove_note(&A.id)` 実行 / 編集中の内容は破棄」✓
- **PASS**: **step 5（非編集時は通知なし / S18 境界）** — Note A を外部作成して IDLE に戻し、
  その状態で外部削除すると削除通知は表示されず、フィードから Block A が消えることを実測
  （`editing.noteId === null` では検知しない = I-DEL6）✓
- **PASS**: **非空虚性（RED 実測）** — pre-S20 の test binary で E2E は **5 failing (~54s)**。
  step 1 が `delete notice ... did not appear` で落ち、通知 UI が存在しないことを実証。
  配線後に **5 passing**（詳細 notes.md#mutation-red）✓
- **PASS**: **spec.md frontmatter の coherence** — `coherence.source: derived` + upstream 9 件
  （hash 付き: validation / detect-external-changes / external-file-change-events /
  note-file-deleted-externally / note-feed-aggregate / note-aggregate / screen-1 /
  page-groups / glossary-timestamp）✓
- **PASS**: **テストデータ独立性** — `mkdtempSync` の temp dir に config/data/storage を隔離し
  `onComplete` で `rmSync`。`TAURI_TEST_STORAGE_DIR` override により実 user 環境を汚さない ✓
- **PASS**: **production 最小性** — Rust は新 slice `recreate_note`（4 file + mod 登録 + lib.rs
  登録）+ 既存 `create_note::FsNoteRepository` 再利用。frontend は新規 4 file（wrapper / store /
  component / store test）+ 既存 bridge / PageMain の拡張。既存の conflict widget は無変更 ✓
- **MEDIUM**: **domain event bridge は未配線（OQ-WC1 継続）** — `NoteFileDeletedExternally`
  payload を Rust → TS へ渡す経路は無く、削除検知は `notes-changed` + `list_notes` の存在差分で
  行っている。`detected_at` は frontend の `new Date()` で domain の実値ではない。真の Rust bridge は
  follow-up（OQ-WC1）。notes.md#event-observation に明記済み ✓
- **MEDIUM**: **ドメイン文書ギャップ** — `page-groups.md#widget-external-change-conflict` /
  `screen-4.md#notes-lifecycle` は `NoteFileDeletedExternally` で widget を mount しないと規定する
  （conflict widget については正しい）が、EDITING 中の外部削除通知は Phase 11a 未設計。
  本 scenario は validation S20 を優先し新 widget `widget-external-delete-notice` を最小実装。
  ui-fields への正式反映は `/ori-propose` の follow-up。notes.md#domain-gap に明記済み ✓
- **MEDIUM**: **ダイアログ解決操作は DOM click** — modal `<dialog>` のネイティブ WebDriver click が
  本 headless WebKitGTK で flaky なため、`delete-notice-*` は `browser.execute` の DOM click で操作。
  外部削除トリガー（unlink）は native 経路。notes.md#known-issues に明記 ✓
- **LOW**: **ローカル編集は ASCII**（`"hello local"`）— IME composition 非再現のため。
  validation Given の `"hello 編集中"` とは文字列が異なるが、削除検知は存在判定であり意味は同一 ✓
- **LOW**: **AutoSave 順序の調整 + debounce pause** — Given は「AutoSave 未発火」だが、
  pending write の race 回避のため AutoSave 永続化を待ってから削除。加えて同パス 500ms debounce
  窓内の削除破棄を避けるため `browser.pause(700)` を置く（s18 step 3 と同方針）。
  notes.md#known-issues に明記 ✓
- **LOW**: **window blur flakiness** — headless WebKitGTK の focus 管理で EDITING → FOCUSED に
  落ちるため `ensureEditing()` で直前保証。notes.md#known-issues に明記 ✓
- **LOW**: **テストは Gherkin ステップ順に依存** — `maxInstances: 1` + mocha 逐次実行が前提
  （s1〜s19 と同方針、`.claude/rules/scenario-test.md`「テスト間独立」からの既知の逸脱）✓

### Production fix discovery {#pass-1-production-fix}

E2E は production 側の通知 UI / 再作成機構の欠落が原因で RED だった。最小実装で GREEN にした:

- Rust: 新 slice `note_capture/slices/recreate_note/`（domain / application / commands / mod /
  tests）— 元 id のまま `.md` を再作成する `recreate_note` command。`create_note::FsNoteRepository`
  を再利用し、`Note::from_persisted` の id 導出で元の NoteId を保持。`note_capture/slices/mod.rs`
  と `lib.rs` に登録
- frontend: `lib/note-capture/slices/recreate-note/index.ts`（invoke wrapper）、
  `ui-widget/external-delete-notice/{store.svelte.ts,WidgetExternalDeleteNotice.svelte}`（新 widget）、
  `external-change-bridge.ts` に削除検知パス、`PageMain.svelte` に
  通知 mount + `notes-changed` 削除分岐（local snapshot 保持で Block 維持）+ 解決 callback

RED 実測（テストが空振りでないことの確認）:

- pre-S20 binary: E2E **5 failing (~54s)**（step 1 で通知未表示）
- 実装後: 再ビルド → E2E **5 passing (~9–10s)** × 3 連続

production への正味の変更は Rust 新 slice + frontend 6 file。追加 unit test は Rust 4 件 +
frontend 13 件、全て PASS。

回帰確認:

- frontend unit test: **164 passed / 18 files**（s19 baseline 151/17 + 新規 13）
- 既存 E2E 回帰: s18-external-file-deleted **4 passing** /
  s19-external-modify-while-editing **5 passing**
- `cargo test`: 332 passed / 2 failed。失敗 2 件は `list_feed` の
  `tp_f5_last_7_days` / `tp_f6_and_composition` で、テスト内固定日付（2026-06）に対する
  `Last7Days` / `Last30Days` を wall-clock now（2026-09-21）で評価する既存 time-bomb test。
  本 scenario とは無関係の pre-existing failure（notes.md#regression）

### Disposition {#pass-1-disposition}

- 指摘は MEDIUM/LOW のみで、いずれも観測の弱さ / テスト fidelity / domain 文書ギャップ /
  follow-up（OQ-WC1）であり、notes.md / spec.md に明記済み。E2E の観測点
  （EDITING 中の削除通知 / 編集中 body 提示 / Save as new file の再作成 + IDLE /
  Discard の除去 / 非編集時 silent）は満たす。
- production 変更は Rust 新 slice 1 + frontend 6 file。RED 検出力は pre-S20 binary の
  5 failing 実測で確認済み。
- Verdict: **PASS**

<!-- verdict=PASS -->
