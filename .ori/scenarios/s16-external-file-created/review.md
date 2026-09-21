# Review: s16-external-file-created {#review-s16-external-file-created}

## Pass 1 {#pass-1}

> reviewer agent (`ori-reviewer`) は spawn せず、main session の objective 検証で代替
> （orchestrator 指示: reviewer spawn がハングする既知問題があるため。s1〜s15 と同方針）。

### Syntax checks {#pass-1-syntax}

- tsc (`tsc --noEmit -p tsconfig.json`): PASS（テスト spec は 0 error。既知の
  `wdio.conf.ts TS2353 'tauri:options'` のみ — s1〜s15 と同一の WDIO v9 型定義起因、許容）
- docker-compose: SKIP — compose-service 参加者ゼロ (app `promptnotes` is mode=local)。
  `generate-docker-compose.sh` も「compose-service 系参加者ゼロ — docker-compose.yml は省略」で exit 0
- E2E: PASS (`wdio run wdio.conf.ts` → **4 passing**, ~3.5s)

### Mode checklist (local tauri) {#mode-checklist}

| item | status | note |
|---|---|---|
| spec `runner: wdio` ↔ test 実行 ↔ wdio.conf.ts | ✅ PASS | 一致（derive chain step 2: 参加 local app の runtime.runner） |
| テストコード内に service lifecycle が無い | ✅ PASS | seed / temp dir 隔離は onPrepare、テストは node:fs 書き込み + assert + invoke のみ |
| local app が docker-compose に含まれない | ✅ PASS | compose 不在が正しい |
| wdio `tauri:options.application` == runtime `binary` | ✅ PASS | `apps/promptnotes/src-tauri/target/debug/app` |
| `@wdio/tauri-service` が services に含まれる | ✅ PASS | `driverProvider: 'external'` |

### Semantic findings (main session objective verification) {#pass-1-findings}

spec ↔ テストコード ↔ domain 整合性:

- **PASS**: **step 1（本 scenario の核）** — Given で `storage_dir` に Note A のみ（フィード 1 件）を
  実測した後、テストプロセスが `node:fs` で `storage_dir/20260630120000.md` を外部作成する。
  `list_notes` の手動 invoke / 再起動を一切行わず、`waitForBlockIds([NEW, A])` が成立
  → DOM が自動で 1 件 → 2 件に更新。validation#s16-when 1-2 / Then「フィード表示が 1 件 → 2 件に
  更新」を実測 ✓
- **PASS**: **step 1（sort 維持）** — 期待順序を `createdAt desc` の `[20260630120000, 20260620120000]`
  として assert。末尾 append ではなく現在の sort に従った位置に挿入される
  （I-F3 の `id` tiebreak / Then「現在の filter / sort が維持されたまま新規 Note が表示される」）✓
- **PASS**: **step 1（UI 通知不要）** — 反映後に `[data-testid="screen-1-toast"]` が 0 件。
  `note-file-created-externally-subscribers`「UI 層: 通知不要（フィードの自然な更新で十分）」を実測 ✓
- **PASS**: **step 2（parse 結果の正しさ）** — 反映後の read DTO で
  `body="外部から作成"` / `tags=["rust"]` / `created_at=updated_at="2026-06-30T12:00:00Z"`。
  外部 `.md` の frontmatter（`createdAt/updatedAt` 秒精度 + inline tags）が
  `FsNoteRepository::parse_note_md` 経由で Note に再構築されることを実測 ✓
- **PASS**: **step 3（追跡 / 非空虚性）** — 2 つ目の外部ファイル `20260701090000.md` を作成し、
  やはり手動操作なしで UI が 3 件 `[NEW2, NEW, A]` に更新。1 回限りの偶然ではなく
  watcher pipeline が継続的に機能することを実測（validation Then の一般性）✓
- **PASS**: **step 4（parse 失敗 skip）** — 壊れた frontmatter（閉じ `---` なし + `tags: [broken`）の
  `.md` を外部作成し、debounce 窓 (500ms) を超える 1500ms 待機後もフィードは
  `[NEW2, NEW, A]` のまま、当該 block は非存在。validation#s16-then 補足
  「parse 失敗時は event を発行せず skip」を実測 ✓
- **PASS**: **watcher pipeline の因果性（RED 実測）** — `PageMain.svelte` の
  `invoke('start_file_watcher')` を一時無効化して test build → E2E は **1 passing / 3 failing**。
  step 1 / 3 / 4 が RED になり、DOM 自動更新判定が watcher pipeline に因果依存することを確認。
  復元・再ビルド後は **4 passing** で GREEN ✓（詳細 notes.md#mutation-red）
- **PASS**: **`list_notes` 経路の非識別性を明示的に扱っている** — `list-feed/commands.rs` は毎回
  `FsNoteRepository::list_all` で disk を再 hydrate するため、`list_notes` の結果だけでは
  watcher を証明できない。上記 RED 実測で **watcher 無効時も step 2（read DTO 検証）だけが
  PASS のまま** だったことがこれを実証している。spec.md#test-points / notes.md#event-observation
  に明記済みで、step 1 / 3 は DOM 自動更新のみで判定する設計になっている ✓
- **PASS**: **domain event の E2E 直接観測不可の扱い** — `NoteFileCreatedExternally` は
  `start_file_watcher` 内部の `AppEventBus` に publish され、subscriber が
  `upsert_one` + `notes-changed` emit を行うのみ（domain event payload は frontend に露出しない）。
  E2E では event payload を assert せず、UI/FS 状態で間接検証する方針を
  spec.md#test-points「E2E 観測の制約」/ notes.md#event-observation に明記 ✓
  （task 指示「NoOpBus のため観測不可なら UI/FS 状態で間接検証し notes.md に明記」に対応。
  正確には note-capture 系 command の NoOpBus とは別に、watcher は in-process `AppEventBus` を
  持つが frontend には `notes-changed` 以外を出さない、という構造を notes.md に記載）
- **PASS**: **spec.md frontmatter の coherence** — `coherence.source: derived` + upstream 7 件
  （hash 付き: validation / detect-external-changes / external-file-change-events /
  note-file-created-externally / note-feed-aggregate / note-aggregate / glossary-timestamp）✓
- **PASS**: **テストデータ独立性** — `mkdtempSync` の temp dir に config/data/storage を隔離し
  `onComplete` で `rmSync`。`TAURI_TEST_STORAGE_DIR` override により実 user 環境を汚さない ✓
- **MEDIUM**: **step 4 は弱い観測** — malformed ファイルが UI に現れないことは、watcher の
  `load_by_id` 失敗（event 未発行）と `list_all` の parse 失敗 skip の両経路で同じ結果になるため、
  件数不変 assert は watcher 側 skip を単独では識別しない。notes.md#known-issues に明記済み。
  event 未発行の直接観測は slice unit test（`detect_external_changes/tests.rs`）の領分 ✓
- **LOW**: **テストは Gherkin ステップ順に依存** — step 1 で作成 → step 2 で読み → step 3 で追加作成
  → step 4 で malformed。`maxInstances: 1` + mocha 逐次実行が前提（s1〜s15 と同方針）。✓
- **LOW**: **vim atomic write 相当（`.tmp` → rename）は未検証** — 本テストは直接 `.md` を作成する。
  rename（delete+create 2 イベント）と `.tmp` ignore の扱いは S21/S22 / slice unit test の領分
  （notes.md#known-issues に明記）✓

### Production fix discovery {#pass-1-production-fix}

production 側は S16 の不変条件を **既に満たしていた**。E2E を GREEN にするための
production 実装追加は **不要（変更 0 file）**:

- `start_file_watcher`（`note_feed/slices/detect_external_changes/commands.rs`）が
  `note_capture::shared::storage::resolve_storage_dir`（`TAURI_TEST_STORAGE_DIR` override 尊重）で dir を解決し、
  `notify` watcher を non-recursive / `.md` のみで起動、`WatcherHandle` を managed state に保持
- `FsWatcher::run_event_loop` が 500ms debounce を適用して `Created` を
  `RawFileEvent::Created` として渡す
- `DetectExternalChangesUseCase::start_watcher` が `load_by_id` 成功時に
  `DomainEvent::NoteFileCreatedExternally` を publish
- subscriber が `InMemoryNoteFeedState::upsert_one` + `app_handle.emit("notes-changed", ())`
- `PageMain.svelte` の mount effect が `start_file_watcher` を invoke し、`notes-changed` を
  listen して `listNotesFn` → `feedStore.hydrateNotes`

RED 実測（テストが空振りでないことの確認）:

- **変異（`PageMain.svelte` の `invoke('start_file_watcher')` を無効化）**:
  test build → E2E **1 passing / 3 failing**
  - step 1: `feed blocks did not become [20260630120000, 20260620120000]`
  - step 2: **PASS のまま**（`list_notes` は disk 直読みのため watcher 無効でも新規 Note が見える）
  - step 3: 2 つ目の外部作成も未反映
  - step 4: フィード件数が期待 `[NEW2, NEW, A]` ではなく `[A]`
- **復元後** → 再ビルド → E2E **4 passing (~3.5s)** で GREEN。

production への正味の変更は **0 file**（変異は完全に revert 済み。`git diff` clean）。

回帰確認:

- frontend unit test: **143 passed / 15 files**
- 既存 E2E 回帰: s15-same-second-edits **4 passing**
- Rust: production 変更ゼロのため追加の `cargo test` 影響なし

### Disposition {#pass-1-disposition}

- 指摘は MEDIUM/LOW のみで、いずれも観測の弱さ / スコープ注記であり、notes.md / spec.md に
  明記済み。E2E の観測点（手動操作なしの DOM 自動更新 / sort 維持 / 内容一致 / 連続検知 /
  malformed skip）は満たす。
- production 変更は 0 file。RED 検出力は watcher 無効化変異の実測で確認済み（step 2 のみ PASS が
  read DTO 経路の非識別性の実証にもなっている）。
- Verdict: **PASS**

<!-- verdict=PASS -->
