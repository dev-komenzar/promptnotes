# Review: s21-batch-sync-debounce {#review-s21-batch-sync-debounce}

## Pass 1 {#pass-1}

> reviewer agent (`ori-reviewer`) は spawn せず、main session の objective 検証で代替
> （orchestrator 指示: reviewer spawn がハングする既知問題があるため。s1〜s20 と同方針）。

### Syntax checks {#pass-1-syntax}

- tsc (`tsc --noEmit -p tsconfig.json`): PASS（テスト spec は 0 error。既知の
  `wdio.conf.ts TS2353 'tauri:options'` のみ — s1〜s20 と同一の WDIO v9 型定義起因、許容）
- svelte-check (`bun run check`): PASS（0 errors / 0 warnings）
- docker-compose: SKIP — compose-service 参加者ゼロ (app `promptnotes` is mode=local)。
  `generate-docker-compose.sh` も「compose-service 系参加者ゼロ — docker-compose.yml は省略」で exit 0
- E2E: PASS (`wdio run wdio.conf.ts` → **5 passing**, ~7–8s。3 回連続実行で安定)

### Mode checklist (local tauri) {#mode-checklist}

| item | status | note |
|---|---|---|
| spec `runner: wdio` ↔ test 実行 ↔ wdio.conf.ts | ✅ PASS | 一致（derive chain step 2: 参加 local app の runtime.runner） |
| テストコード内に service lifecycle が無い | ✅ PASS | seed / temp dir 隔離は onPrepare、テストは node:fs 操作 + assert のみ |
| local app が docker-compose に含まれない | ✅ PASS | compose 不在が正しい |
| wdio `tauri:options.application` == runtime `binary` | ✅ PASS | `apps/promptnotes/src-tauri/target/debug/app` |
| `@wdio/tauri-service` が services に含まれる | ✅ PASS | `driverProvider: 'external'` |

### Semantic findings (main session objective verification) {#pass-1-findings}

spec ↔ テストコード ↔ domain 整合性:

- **PASS**: **step 1（本 scenario の核）** — Given で Note A / B / C の 3 件（順序 `[C, B, A]`・
  全て IDLE）を実測した後、テストプロセスが debounce 窓（500ms）以内に `A.md` / `B.md` / `C.md` を
  連続上書きする。`list_notes` の手動 invoke / 再起動を一切行わず、3 つの既存 Block の
  `.cm-content` が **すべて** sync body になることを `waitUntil` で実測 →
  validation#s21-when 1-5 / Then「フィードは 3 回の部分更新 / 最終結果は S16〜S18 の逐次適用と同じ」✓
- **PASS**: **step 1（upsert 差し替え / 表示順不変）** — 反映後も Block 数は 3・表示順 `[C, B, A]`
  のまま（append されない）、`screen-1-toast` は 0 件。
  `note-file-modified-externally-subscribers`「Note Feed: `upsert_note` で差し替え（I-F8）」を実測 ✓
- **PASS**: **step 2（再 parse 結果の正しさ）** — 反映後の read DTO で A / B / C それぞれ
  `body` が sync 値 / `tags=[]` / `created_at = updated_at = <seed 時刻>`。
  外部 `.md` の frontmatter が `FsNoteRepository::parse_note_md` 経由で再構築されることを実測。
  `updatedAt` が Given の値のまま = アプリ内編集 (`applyAutoSave`) を経由していないことの
  間接確認にもなる（Then「`NoteBodyEdited` は発行されない」）✓
- **PASS**: **step 3（冪等 / 非空虚性）** — debounce 窓を超えて待ってから同じ 3 ファイルを
  同一 body で再バッチし、Block が重複せず 3 のまま・body / 表示順も不変であることを実測。
  `upsert_note` が等（I-F8）であることを示す ✓
- **PASS**: **step 4（競合なし / IDLE）** — バッチ反映後も全 Block は `data-block-state="IDLE"`、
  競合ダイアログ (`widget-external-change-conflict`) とトースト (`screen-1-toast`) は
  いずれも非存在。validation Then の IDLE 前提（S19 の EDITING ケースは範囲外）を実測
  （ただし強い検証は下記 MEDIUM の制約付き）✓
- **PASS**: **step 5（補足 `.tmp` → rename）** — `D.md.tmp` を書き込み debounce 窓を超えて
  待っても Block D は出現せず（feed 3 件のまま）、`D.md.tmp` → `D.md` の rename 後に
  Block D が新規 Note として出現し body が反映されることを実測。
  `detect-external-changes.md#notes`「watcher は `.tmp` を無視し rename 先が `.md` の場合のみ
  Created / Modified として扱う」を実測 ✓（notes.md#tmp-rename に notify 6.1.1 のイベント分解を記載）
- **PASS**: **debounce の path 単位性を実測** — 3 ファイルは互いに集約されず、それぞれ独立に
  `NoteFileModifiedExternally` として処理される（全て反映）。実装は `last_seen: HashMap<PathBuf,
  Instant>` による path 単位の leading-edge + 500ms window 抑制であり、validation の
  `t1/t2/t3` 段階タイミングは近似である旨を spec.md#impl-notes / notes.md#debounce-semantics に
  明記済み ✓
- **PASS**: **watcher pipeline の因果性（RED 実測）** — `PageMain.svelte` の
  `invoke('start_file_watcher')` を一時無効化（`MUTATION_DISABLED_start_file_watcher`）して
  test build → E2E は **2 passing / 3 failing**。step 1 / 3 / 5 が RED になり、DOM 自動更新判定が
  watcher pipeline に因果依存することを確認。復元・再ビルド後は **5 passing** で GREEN
  ✓（詳細 notes.md#mutation-red）
- **PASS**: **`list_notes` 経路の非識別性を明示的に扱っている** — `list-feed/commands.rs` は毎回
  `FsNoteRepository::list_all` で disk を再 hydrate するため、`list_notes` の結果だけでは
  watcher を証明できない。RED 実測で **watcher 無効時も step 2（read DTO 検証）だけが PASS の
  まま** だったことがこれを実証している。spec.md#test-points / notes.md#event-observation に
  明記済みで、step 1 / 3 / 5 は DOM 自動更新のみで判定する設計になっている ✓
- **PASS**: **domain event の E2E 直接観測不可の扱い** — `NoteFileModifiedExternally` は
  `start_file_watcher` 内部の **in-process `AppEventBus`** に publish され、subscriber が
  `upsert_one` + `notes-changed`（payload なし）emit を行うのみ（domain event payload /
  `disk_body_hash` は frontend に露出しない。NoOpBus ではない）。E2E では event payload を
  assert せず、UI/FS 状態で間接検証する方針を spec.md#test-points / notes.md#event-observation に
  明記 ✓（task 指示「イベントは NoOpBus のため E2E 観測不可なら UI/FS 状態で間接検証し notes.md に
  明記」に対し、正確には watcher は in-process `AppEventBus` を持ち frontend には `notes-changed`
  以外を出さない、という構造を notes.md に記載）
- **PASS**: **spec.md frontmatter の coherence** — `coherence.source: derived` + upstream 8 件
  （hash 付き: validation / detect-external-changes / external-file-change-events /
  note-file-modified-externally / note-feed-aggregate / note-aggregate / screen-1 /
  glossary-timestamp）✓
- **PASS**: **テストデータ独立性** — `mkdtempSync` の temp dir に config/data/storage を隔離し
  `onComplete` で `rmSync`。`TAURI_TEST_STORAGE_DIR` override により実 user 環境を汚さない ✓
- **MEDIUM**: **frontend は全件 re-hydrate（部分更新回数は未観測）** — ドメイン仕様（I-F8）は
  `upsert_note` による部分更新を規定し Rust `InMemoryNoteFeedState` は `upsert_one` で差分更新するが、
  frontend は `notes-changed` のたびに `listNotesFn()`（disk 全件 re-read）→ `hydrateNotes` で
  全件差し替えする。E2E は最終状態（3 Block の body 更新 + Block 数不変）を観測し、
  「3 回の部分更新」そのものは観測しない。notes.md#known-issues に明記済み ✓
- **MEDIUM**: **step 4 は弱い観測（競合ダイアログ）** — `WidgetExternalChangeConflict` の
  `defaultSubscribeFn` は no-op（`store.svelte.ts` OQ-WC1: Real event bridge 未配線）。
  IDLE では当然ダイアログ非表示だが、EDITING（S19）でも E2E では区別できない。S19 の競合検出は
  slice / widget unit の領分。notes.md#known-issues に明記済み ✓
- **LOW**: **`updatedAt` ソートの挙動は未検証** — When は body のみ変更（`updatedAt` 据え置き）で
  `created_at` desc のため順序は不変。複数 Note × `updatedAt` ソートは S15 / NoteFeed sort の領分
  （notes.md#known-issues）✓
- **LOW**: **`disk_body_hash`（I-N9）は E2E では観測できない** — event payload 非露出のため。
  slice unit test の領分（notes.md#known-issues）✓
- **LOW**: **テストは Gherkin ステップ順に依存** — step 1 で一括変更 → step 2 で読み → step 3 で
  再バッチ → step 4 で状態確認 → step 5 で `.tmp` / rename。`maxInstances: 1` + mocha 逐次実行が
  前提（s1〜s20 と同方針）✓

### Production fix discovery {#pass-1-production-fix}

production 側は S21 の不変条件を **既に満たしていた**。E2E を GREEN にするための
production 実装追加は **不要（変更 0 file）**:

- `start_file_watcher`（`note_feed/slices/detect_external_changes/commands.rs`）が
  `note_capture::shared::storage::resolve_storage_dir`（`TAURI_TEST_STORAGE_DIR` override 尊重）で
  dir を解決し、`notify` watcher を non-recursive / `.md` のみで起動、`WatcherHandle` を managed
  state に保持
- `FsWatcher::run_event_loop` が **path 単位** 500ms debounce を適用し、`Modify` を
  `RawFileEvent::Modified` として渡す。`is_tmp_file` が `.tmp` を除外
- `DetectExternalChangesUseCase::start_watcher` が `Modified` で `load_by_id` →
  `note.body_hash()` を計算し `DomainEvent::NoteFileModifiedExternally` を publish
- subscriber が `NoteFileModifiedExternally` で `InMemoryNoteFeedState::upsert_one` +
  `app_handle.emit("notes-changed", ())`
- `PageMain.svelte` の mount effect が `start_file_watcher` を invoke し、`notes-changed` を
  listen して `listNotesFn` → `feedStore.hydrateNotes`

RED 実測（テストが空振りでないことの確認）:

- **変異（`PageMain.svelte` の `invoke('start_file_watcher')` を無効化）**:
  test build → E2E **2 passing / 3 failing**（~35s）
  - ✖ step 1: 3 Block の DOM body が自動更新されない
  - ✓ step 2: **PASS のまま**（`list_notes` は disk 直読みのため watcher 無効でも変更後 body が
    見える — read DTO 経路が非識別であることの実証）
  - ✖ step 3: sync body に到達しない
  - ✓ step 4: IDLE / ダイアログ・トースト非表示は watcher 非依存のため PASS
  -  step 5: rename 後の Block D が出現しない
- **復元後** → 再ビルド → E2E **5 passing (~7–8s)** で GREEN（2 回連続で安定確認）。

production への正味の変更は **0 file**（変異は完全に revert 済み。`git diff` clean。
working tree は新規 scenario ディレクトリのみ）。

回帰確認:

- frontend unit test: **164 passed / 18 files**
- 既存 E2E 回帰: s17-external-file-modified-no-conflict **4 passing** /
  s18-external-file-deleted **4 passing**
- Rust: production 変更ゼロのため追加の `cargo test` 影響なし

### Disposition {#pass-1-disposition}

- 指摘は MEDIUM/LOW のみで、いずれも観測の弱さ / frontend 再 hydrate の記述 / スコープ注記であり、
  notes.md / spec.md に明記済み。E2E の観測点（debounce 窓内の一括変更で 3 Block 全件自動更新 /
  upsert 差し替え / 内容一致 / 再バッチ冪等 / 競合 UI 非表示 / `.tmp` 無視 + rename 反映）は満たす。
- production 変更は 0 file。RED 検出力は watcher 無効化変異の実測で確認済み（step 2 のみ PASS が
  read DTO 経路の非識別性の実証にもなっている）。
- Verdict: **PASS**

<!-- verdict=PASS -->