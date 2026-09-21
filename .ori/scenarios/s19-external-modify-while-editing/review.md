# Review: s19-external-modify-while-editing {#review-s19-external-modify-while-editing}

## Pass 1 {#pass-1}

> reviewer agent (`ori-reviewer`) は spawn せず、main session の objective 検証で代替
> （orchestrator 指示: reviewer spawn がハングする既知問題があるため。s1〜s18 と同方針）。

### Syntax checks {#pass-1-syntax}

- tsc (`tsc --noEmit -p tsconfig.json`): PASS（テスト spec は 0 error。既知の
  `wdio.conf.ts TS2353 'tauri:options'` のみ — s1〜s18 と同一の WDIO v9 型定義起因、許容）
- svelte-check (`bun run check`): PASS（0 errors / 0 warnings）
- eslint（変更 file）: PASS
- docker-compose: SKIP — compose-service 参加者ゼロ (app `promptnotes` is mode=local)。
  `generate-docker-compose.sh` も「compose-service 系参加者ゼロ — docker-compose.yml は省略」で exit 0
- E2E: PASS (`wdio run wdio.conf.ts` → **5 passing**, ~7s。3 回連続実行で安定）

### Mode checklist (local tauri) {#mode-checklist}

| item | status | note |
|---|---|---|
| spec `runner: wdio` ↔ test 実行 ↔ wdio.conf.ts | ✅ PASS | 一致（derive chain step 2: 参加 local app の runtime.runner） |
| テストコード内に service lifecycle が無い | ✅ PASS | seed / temp dir 隔離は onPrepare、テストは node:fs 書込 + assert + dialog 操作のみ |
| local app が docker-compose に含まれない | ✅ PASS | compose 不在が正しい |
| wdio `tauri:options.application` == runtime `binary` | ✅ PASS | `apps/promptnotes/src-tauri/target/debug/app` |
| `@wdio/tauri-service` が services に含まれる | ✅ PASS | `driverProvider: 'external'` |

### Semantic findings (main session objective verification) {#pass-1-findings}

spec ↔ テストコード ↔ domain 整合性:

- **PASS**: **step 1（本 scenario の核）** — Given で Block A を EDITING にし `"hello local"` に
  編集（AutoSave 永続化を待機）、その後テストプロセスが `node:fs` で
  `storage_dir/20260620120000.md` を外部上書きし body を `"hello world"` にする。
  `list_notes` の手動 invoke / 再起動を一切行わず `[data-testid="widget-external-change-conflict"]`
  が表示され、Block A が `data-block-state="EDITING"` のままであることを実測。
  validation#s19-when 1-5 / Then「競合ダイアログを表示」を満たす ✓
- **PASS**: **step 2（compare-layout）** — `screen-4-note-title` = `20260620120000.md`、
  `screen-4-body-local` value = `"hello local"`、`screen-4-body-external` value = `"hello world"`。
  screen-4.md#fields / #display-states-compare の local / external 併記を実測 ✓
- **PASS**: **step 3（KeepEditing / I-WC6）** — `screen-4-cancel` でダイアログが閉じ、
  Block A の CodeMirror body が `"hello local"` のまま（外部版に置換されない）で、
  Block A が EDITING を維持することを実測。screen-4.md#cross-field-rules
  「{KeepEditing} → 現在の編集内容を保持」に対応 ✓
- **PASS**: **step 4（ApplyExternal / I-WC5）** — さらに外部変更（`"hello world 2"`）でダイアログを
  再表示させ、`screen-4-resolution-apply-external` 選択 + `screen-4-confirm` で
  ダイアログが閉じ、Block A の body が外部版 `"hello world 2"` に置換され IDLE に遷移することを実測。
  screen-4.md#display-states-resolved「選択後は Feed の表示が自動反映」に対応 ✓
- **PASS**: **step 5（非編集時 silent / I-WC2・I-F8）** — IDLE の Block A に外部変更
  （`"hello world 3"`）を起こしてもダイアログは表示されず、フィード body が自動更新されることを実測。
  S17（IDLE での自動反映）との境界を同一 E2E 内で確認 ✓
- **PASS**: **非空虚性（RED 実測）** — production 配線前の test build で E2E は **5 failing**。
  step 1 が `conflict dialog ... did not appear` で落ち、widget の `defaultSubscribeFn` が
  no-op でダイアログが決して表示されないことを実証。配線後に **5 passing**（詳細 notes.md#mutation-red）✓
- **PASS**: **spec.md frontmatter の coherence** — `coherence.source: derived` + upstream 10 件
  （hash 付き: validation / detect-external-changes / external-file-change-events /
  note-file-modified-externally / note-feed-aggregate / note-aggregate / screen-4 / screen-1 /
  page-groups / glossary-timestamp）✓
- **PASS**: **テストデータ独立性** — `mkdtempSync` の temp dir に config/data/storage を隔離し
  `onComplete` で `rmSync`。`TAURI_TEST_STORAGE_DIR` override により実 user 環境を汚さない ✓
- **PASS**: **production 最小性** — 変更は frontend 5 file（うち新規 3）+ 追加 unit test 2 file。
  `widget-external-change-conflict` の store / component は**無変更**（既存 impl を再利用）✓
- **MEDIUM**: **event bridge は frontend 派生（OQ-WC1 継続）** — Rust → TS の
  `NoteFileModifiedExternally` payload 経路は無く、bridge は `notes-changed` + `list_notes` の
  body 差分から payload を構成している。`file_path` は `${id}.md`、`detected_at` は frontend 時刻で
  domain の実値ではない。真の bridge は follow-up（OQ-WC1）。notes.md#event-observation /
  #known-issues に明記済み ✓
- **MEDIUM**: **ダイアログ解決操作は DOM click** — modal `<dialog>` のネイティブ WebDriver click が
  本 headless WebKitGTK で flaky（handler 未発火 run を実測）だったため、`screen-4-*` は
  `browser.execute` の DOM click で操作。競合トリガー（外部ファイル書込）は native 経路。
  テスト fidelity の制約として notes.md#known-issues と test 内コメントに明記 ✓
- **LOW**: **ローカル編集は ASCII**（`"hello local"`）— IME composition 非再現のため。
  validation Given の `"hello 編集中"` とは文字列が異なるが、競合判定の意味は同一 ✓
- **LOW**: **AutoSave 順序の調整** — Given は「AutoSave 未発火」だが、pending write の race 回避の
  ため AutoSave 永続化を待ってから外部変更。notes.md#known-issues に明記 ✓
- **LOW**: **I-WC7 未検証** — ダイアログ表示中の Editor キー入力ブロックは呼び出し元責務で
  本 E2E 範囲外（notes.md#known-issues）✓
- **LOW**: **テストは Gherkin ステップ順に依存** — `maxInstances: 1` + mocha 逐次実行が前提
  （s1〜s18 と同方針、`.claude/rules/scenario-test.md`「テスト間独立」からの既知の逸脱）✓
- **LOW**: **`crypto.subtle` フォールバック** — Web Crypto 不可時は `plain:` トークン。
  両辺同一関数のため等価比較は成立（notes.md#known-issues）✓

### Production fix discovery {#pass-1-production-fix}

E2E は production 側の配線欠落が原因で RED だった。最小の frontend 配線を追加して GREEN にした:

- `stores/editing-note.svelte.ts`: `createEditingNoteStore()` factory 化 + `clearIfCurrent`
  （`setEditing` は従来未使用の死んだ store だった）
- `stores/body-hash.ts`（新規）: `hashBody` = Web Crypto SHA-256（+ plain fallback）
- `stores/external-change-bridge.ts`（新規）: `notes-changed` → `list_notes` 差分で
  EDITING 中 note の競合を検出し、dialog store へ payload を emit
- `PageMain.svelte`: widget へ実 deps 注入 / EDITING 中の `editingNote` 維持 effect /
  競合時はローカル body を保持して hydrate / `onApplyExternal` で feed 置換 + IDLE 遷移
- Rust: **変更なし**（`Modify` → `RawEvent::Modified` → `NoteFileModifiedExternally` →
  `upsert_one` + `notes-changed` は既存）

RED 実測（テストが空振りでないことの確認）:

- production 配線前: test build → E2E **5 failing (~54s)**（step 1 で dialog 未表示）
- 配線後: 再ビルド → E2E **5 passing (~7s)** × 3 連続

production への正味の変更は frontend 5 file（Rust 0 file）。追加 unit test 8 件は全て PASS。

回帰確認:

- frontend unit test: **151 passed / 17 files**（s18 baseline 143 + 新規 8）
- 既存 E2E 回帰: s17-external-file-modified-no-conflict **4 passing** /
  s18-external-file-deleted **4 passing**
- `cargo test`: 328 passed / 2 failed。失敗 2 件は `list_feed` の
  `tp_f5_last_7_days` / `tp_f6_and_composition` で、テスト内固定日付（2026-06）に対する
  `Last7Days` / `Last30Days` を wall-clock now（2026-09-21）で評価する既存 time-bomb test。
  Rust 変更ゼロのため本 scenario とは無関係の pre-existing failure（notes.md#regression）

### Disposition {#pass-1-disposition}

- 指摘は MEDIUM/LOW のみで、いずれも観測の弱さ / テスト fidelity / follow-up（OQ-WC1）であり、
  notes.md / spec.md に明記済み。E2E の観測点（EDITING 中のダイアログ表示 / compare-layout の
  local・external 併記 / KeepEditing 保持 / ApplyExternal 適用 + IDLE / 非編集時 silent）は満たす。
- production 変更は frontend 5 file（Rust 0 file、widget store/impl 無変更）。
  RED 検出力は配線前 5 failing の実測で確認済み。
- Verdict: **PASS**

<!-- verdict=PASS -->
