# Review: s18-external-file-deleted {#review-s18-external-file-deleted}

## Pass 1 {#pass-1}

> reviewer agent (`ori-reviewer`) は spawn せず、main session の objective 検証で代替
> （orchestrator 指示: reviewer spawn がハングする既知問題があるため。s1〜s17 と同方針）。

### Syntax checks {#pass-1-syntax}

- tsc (`tsc --noEmit -p tsconfig.json`): PASS（テスト spec は 0 error。既知の
  `wdio.conf.ts TS2353 'tauri:options'` のみ — s1〜s17 と同一の WDIO v9 型定義起因、許容）
- docker-compose: SKIP — compose-service 参加者ゼロ (app `promptnotes` is mode=local)。
  `generate-docker-compose.sh` も「compose-service 系参加者ゼロ — docker-compose.yml は省略」で exit 0
- E2E: PASS (`wdio run wdio.conf.ts` → **4 passing**, ~4.5s)

### Mode checklist (local tauri) {#mode-checklist}

| item | status | note |
|---|---|---|
| spec `runner: wdio` ↔ test 実行 ↔ wdio.conf.ts | ✅ PASS | 一致（derive chain step 2: 参加 local app の runtime.runner） |
| テストコード内に service lifecycle が無い | ✅ PASS | seed / temp dir 隔離は onPrepare、テストは node:fs 削除 + assert + invoke のみ |
| local app が docker-compose に含まれない | ✅ PASS | compose 不在が正しい |
| wdio `tauri:options.application` == runtime `binary` | ✅ PASS | `apps/promptnotes/src-tauri/target/debug/app` |
| `@wdio/tauri-service` が services に含まれる | ✅ PASS | `driverProvider: 'external'` |

### Semantic findings (main session objective verification) {#pass-1-findings}

spec ↔ テストコード ↔ domain 整合性:

- **PASS**: **step 1（本 scenario の核）** — Given で `storage_dir` に Note A / Note B の 2 件を
  seed し、フィード `[B, A]`（既定 sort `createdAt desc`）を実測した後、テストプロセスが
  `node:fs` の `unlinkSync` で `storage_dir/20260620120000.md` を外部削除する。
  `list_notes` の手動 invoke / 再起動を一切行わず、`waitForBlockIds([B])` が成立し
  Block A が DOM から消える → DOM が自動で 2 件 → 1 件に更新。validation#s18-when 1-2 /
  Then「フィード表示から Note A が消える」を実測 ✓
- **PASS**: **step 1（id 特異性）** — Block A のみが消え Block B は残る。`remove_note(&A.id)` が
  blanket clear ではなく該当 `note_id` のみを除外すること（I-F8「該当 `note_id` のみ」）を実測。
  2 件 seed は validation Given（「Note A が表示されている」＝「のみ」ではない）に整合 ✓
- **PASS**: **step 1（UI 通知不要）** — 反映後に `[data-testid="screen-1-toast"]` が 0 件。
  `note-file-deleted-externally-subscribers`「UI 層: 通知不要（フィードの自然な更新で十分）」を実測 ✓
- **PASS**: **step 2（read DTO の正しさ）** — 反映後の read DTO で Note A が含まれず、
  Note B のみが `body="world"` / `tags=[]` / `created_at=updated_at="2026-06-21T09:00:00Z"` で残る ✓
- **PASS**: **step 3（追跡 / 非空虚性）** — 残りの Note B も `unlinkSync` で外部削除し、
  やはり手動操作なしで Block B も消えフィードが空（`screen-1-feed-empty` 表示）になる。
  1 回限りの偶然ではなく watcher pipeline が継続的に機能することを実測（validation Then の一般性）✓
- **PASS**: **step 4（非 Note ファイル名 skip）** — `^\d{14}$` に一致しない `notanote.md` を
  外部作成 → 削除し、フィードが空のまま不変なことを assert。validation#s18-then 補足
  「ファイル名が `^\d{14}$` に一致しない場合は event 非発行」に対応 ✓
- **PASS**: **watcher pipeline の因果性（RED 実測）** — `PageMain.svelte` の
  `invoke('start_file_watcher')` を一時無効化して test build → E2E は **1 passing / 3 failing**。
  step 1 / 3 / 4 が RED になり、DOM 自動除去判定が watcher pipeline に因果依存することを確認。
  復元・再ビルド後は **4 passing** で GREEN ✓（詳細 notes.md#mutation-red）
- **PASS**: **`list_notes` 経路の非識別性を明示的に扱っている** — `list-feed/commands.rs` は毎回
  `FsNoteRepository::list_all` で disk を再 hydrate するため、`list_notes` の結果だけでは
  watcher を証明できない。上記 RED 実測で **watcher 無効時も step 2（read DTO 検証）だけが
  PASS のまま** だったことがこれを実証している。spec.md#test-points / notes.md#event-observation
  に明記済みで、step 1 / 3 は DOM 自動除去のみで判定する設計になっている ✓
- **PASS**: **domain event の E2E 直接観測不可の扱い** — `NoteFileDeletedExternally` は
  `start_file_watcher` 内部の `AppEventBus` に publish され、subscriber が
  `remove_one` + `notes-changed` emit を行うのみ（domain event payload は frontend に露出しない）。
  E2E では event payload を assert せず、UI/FS 状態で間接検証する方針を
  spec.md#test-points「E2E 観測の制約」/ notes.md#event-observation に明記 ✓
  （task 指示「NoOpBus のため観測不可なら UI/FS 状態で間接検証し notes.md に明記」に対応）
- **PASS**: **spec.md frontmatter の coherence** — `coherence.source: derived` + upstream 7 件
  （hash 付き: validation / detect-external-changes / external-file-change-events /
  note-file-deleted-externally / note-feed-aggregate / note-aggregate / glossary-timestamp）✓
- **PASS**: **テストデータ独立性** — `mkdtempSync` の temp dir に config/data/storage を隔離し
  `onComplete` で `rmSync`。`TAURI_TEST_STORAGE_DIR` override により実 user 環境を汚さない ✓
- **MEDIUM**: **step 4 は弱い観測** — 非 Note 名ファイルが UI に現れないことは、watcher の
  `resolve_note_id` 失敗（event 未発行）と `list_all` の非 Note 名 skip の両経路で同じ結果に
  なるため、フィード不変 assert は watcher 側 skip を単独では識別しない。notes.md#known-issues
  に明記済み。event 未発行の直接観測は slice unit test（`detect_external_changes/tests.rs`）の領分 ✓
- **LOW**: **テストは Gherkin ステップ順に依存** — step 1 削除 A → step 2 読み → step 3 削除 B →
  step 4 非 Note 名。`maxInstances: 1` + mocha 逐次実行が前提（s1〜s17 と同方針、
  `.claude/rules/scenario-test.md`「テスト間独立」からの既知の逸脱）。✓
- **LOW**: **vim atomic write / `.tmp` rename は未検証** — 本テストは `unlinkSync` による直接削除。
  rename（delete+create 2 イベント）と `.tmp` ignore の扱いは slice unit test の領分
  （notes.md#known-issues に明記）✓
- **LOW**: **S20（編集中の外部削除）は範囲外** — `WidgetExternalChangeConflict` の
  `defaultSubscribeFn` は no-op（OQ-WC1）のため EDITING 削除競合の E2E は現状識別不能。
  notes.md#known-issues に明記済み ✓

### Production fix discovery {#pass-1-production-fix}

production 側は S18 の不変条件を **既に満たしていた**。E2E を GREEN にするための
production 実装追加は **不要（変更 0 file）**:

- `FsWatcher::new` が `EventKind::Remove(_)` を `RawFileEvent::Deleted(path)` に分類
  （`.md` / 非 `.tmp` のみ通過）
- `FsWatcher::run_event_loop` が 500ms debounce を適用して `Deleted` を渡す
- `DetectExternalChangesUseCase::start_watcher` が `Deleted` で `resolve_note_id`（`^\d{14}$`）
  成功時に `DomainEvent::NoteFileDeletedExternally { note_id, file_path, detected_at }` を publish
- subscriber が `InMemoryNoteFeedState::remove_one(note_id)` + `app_handle.emit("notes-changed", ())`
- `InMemoryNoteFeedState::remove_one` → `NoteFeed::remove_note` が `source.retain(id != note_id)`
- `PageMain.svelte` の mount effect が `start_file_watcher` を invoke し、`notes-changed` を
  listen して `listNotesFn` → `feedStore.hydrateNotes`

RED 実測（テストが空振りでないことの確認）:

- **変異（`PageMain.svelte` の `invoke('start_file_watcher')` を無効化）**:
  test build → E2E **1 passing / 3 failing**
  - step 1: `feed blocks did not become [20260621090000]`
  - step 2: **PASS のまま**（`list_notes` は disk 直読みのため watcher 無効でも削除済み Note A は見えない）
  - step 3: `feed blocks did not become []`
  - step 4: フィード件数が期待 `[]` ではなく `[B, A]` のまま
- **復元後** → 再ビルド → E2E **4 passing (~4.5s)** で GREEN。

production への正味の変更は **0 file**（変異は完全に revert 済み。`git status` は
`.ori/scenarios/s18-external-file-deleted/` の追加のみで clean）。

回帰確認:

- frontend unit test: **143 passed / 15 files**
- 既存 E2E 回帰: s17-external-file-modified-no-conflict **4 passing**
- Rust: production 変更ゼロのため追加の `cargo test` 影響なし

### Disposition {#pass-1-disposition}

- 指摘は MEDIUM/LOW のみで、いずれも観測の弱さ / スコープ注記であり、notes.md / spec.md に
  明記済み。E2E の観測点（手動操作なしの DOM 自動除去 / id 特異性 / 内容一致 / 連続検知 /
  非 Note 名 skip）は満たす。
- production 変更は 0 file。RED 検出力は watcher 無効化変異の実測で確認済み（step 2 のみ PASS が
  read DTO 経路の非識別性の実証にもなっている）。
- Verdict: **PASS**

<!-- verdict=PASS -->
