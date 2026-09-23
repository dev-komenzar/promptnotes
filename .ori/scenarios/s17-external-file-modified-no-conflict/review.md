# Review: s17-external-file-modified-no-conflict {#review-s17-external-file-modified-no-conflict}

## Pass 1 {#pass-1}

> reviewer agent (`ori-reviewer`) は spawn せず、main session の objective 検証で代替
> （orchestrator 指示: reviewer spawn がハングする既知問題があるため。s1〜s16 と同方針）。

### Syntax checks {#pass-1-syntax}

- tsc (`tsc --noEmit -p tsconfig.json`): PASS（テスト spec は 0 error。既知の
  `wdio.conf.ts TS2353 'tauri:options'` のみ — s1〜s16 と同一の WDIO v9 型定義起因、許容）
- docker-compose: SKIP — compose-service 参加者ゼロ (app `promptnotes` is mode=local)。
  `generate-docker-compose.sh` も「compose-service 系参加者ゼロ — docker-compose.yml は省略」で exit 0
- E2E: PASS (`wdio run wdio.conf.ts` → **4 passing**, ~3s)

### Mode checklist (local tauri) {#mode-checklist}

| item | status | note |
|---|---|---|
| spec `runner: wdio` ↔ test 実行 ↔ wdio.conf.ts | ✅ PASS | 一致（derive chain step 2: 参加 local app の runtime.runner） |
| テストコード内に service lifecycle が無い | ✅ PASS | seed / temp dir 隔離は onPrepare、テストは node:fs 上書き + assert + invoke のみ |
| local app が docker-compose に含まれない | ✅ PASS | compose 不在が正しい |
| wdio `tauri:options.application` == runtime `binary` | ✅ PASS | `apps/promptnotes/src-tauri/target/debug/app` |
| `@wdio/tauri-service` が services に含まれる | ✅ PASS | `driverProvider: 'external'` |

### Semantic findings (main session objective verification) {#pass-1-findings}

spec ↔ テストコード ↔ domain 整合性:

- **PASS**: **step 1（本 scenario の核）** — Given で `storage_dir` に Note A (`body="hello"`) のみを
  実測した後、テストプロセスが `node:fs` で既存 `storage_dir/20260620120000.md` を外部上書きし
  body を `"hello world"` にする。`list_notes` の手動 invoke / 再起動を一切行わず、
  既存 Block A の `.cm-content` が `"hello world"` になるのを `waitUntil` で実測
  → validation#s17-when 1-2 / Then「フィード表示が "hello world" に更新される」✓
- **PASS**: **step 1（upsert 差し替え / UI 通知なし）** — 反映後も Block 数は 1 のまま
  （append されない）、`[data-testid="screen-1-toast"]` は 0 件。
  `note-file-modified-externally-subscribers`「Note Feed: `upsert_note(note)` で
  source 内の該当 Note を差し替え（I-F8）」/「UI 層: 通知不要」を実測 ✓
- **PASS**: **step 2（再 parse 結果の正しさ）** — 反映後の read DTO で
  `body="hello world"` / `tags=[]` / `created_at=updated_at="2026-06-20T12:00:00Z"`。
  外部 `.md` の frontmatter（秒精度 + tags）が `FsNoteRepository::parse_note_md` 経由で
  Note に再構築されることを実測。`updatedAt` が Given の `t0` のまま = アプリ内編集
  (`applyAutoSave`) を経由していないことの間接確認にもなる（Then「NoteBodyEdited は発行されない」）✓
- **PASS**: **step 3（差し替え / 非空虚性）** — さらに body を `"hello world 2"` に外部上書きし、
  やはり手動操作なしで DOM body が更新され、かつ Block 数が 1 のままであることを実測。
  `upsert_note` が append ではなく差し替え（I-F8）として機能し、watcher pipeline が
  1 回限りの偶然でなく継続的に動くことを示す ✓
- **PASS**: **step 4（競合なし / IDLE）** — 変更反映後も Block A は `data-block-state="IDLE"` のまま、
  競合ダイアログ (`widget-external-change-conflict`) とトースト (`screen-1-toast`) は
  いずれも非存在。validation Then「Block A の状態を確認 → IDLE のため競合なし」を実測
  （ただし強い検証は下記 MEDIUM の制約付き）✓
- **PASS**: **debounce 窓 (500ms) と同一ファイル連続変更の集約を実測** — 初回 E2E は
  **2 passing / 2 failing** だった。原因は step 3 の 2 回目上書きが step 1 のイベント処理から
  500ms 以内に発行され、`FsWatcher::run_event_loop` の debounce（`last_seen` による同一 path の
  集約、`infrastructure.rs`）で **意図的に skip** されたため。診断 spec で 2s 間隔なら
  modify#1/#2/#3 + Create がすべて反映されることを確認した上で、step 3 に
  `await browser.pause(700)`（debounce 超え）を追加して GREEN 化した。
  これは production の欠落ではなく **domain 仕様どおりの debounce 挙動**
  （`domain/workflows/detect-external-changes.md#notes`: debounce 窓 500ms）を
  E2E が正しく炙り出した証拠であり、テスト側のタイミング修正が正しい対処 ✓
- **PASS**: **watcher pipeline の因果性（RED 実測）** — `PageMain.svelte` の
  `invoke('start_file_watcher')` を一時無効化（`MUTATION_DISABLED_start_file_watcher`）して
  test build → E2E は **1 passing / 3 failing**。step 1 / 3 / 4 が RED になり、
  DOM 自動更新判定が watcher pipeline に因果依存することを確認。復元・再ビルド後は
  **4 passing** で GREEN ✓（詳細 notes.md#mutation-red）
- **PASS**: **`list_notes` 経路の非識別性を明示的に扱っている** — `list-feed/commands.rs` は毎回
  `FsNoteRepository::list_all` で disk を再 hydrate するため、`list_notes` の結果だけでは
  watcher を証明できない。上記 RED 実測で **watcher 無効時も step 2（read DTO 検証）だけが
  PASS のまま** だったことがこれを実証している。spec.md#test-points / notes.md#event-observation
  に明記済みで、step 1 / 3 は DOM 自動更新のみで判定する設計になっている ✓
- **PASS**: **domain event の E2E 直接観測不可の扱い** — `NoteFileModifiedExternally` は
  `start_file_watcher` 内部の `AppEventBus` に publish され、subscriber が
  `upsert_one` + `notes-changed` emit を行うのみ（domain event payload / `disk_body_hash` は
  frontend に露出しない）。E2E では event payload を assert せず、UI/FS 状態
  （DOM body 更新 + read DTO + Block 数不変 + ダイアログ非表示）で間接検証する方針を
  spec.md#test-points「E2E 観測の制約」/ notes.md#event-observation に明記 ✓
  （task 指示「NoOpBus のため観測不可なら UI/FS 状態で間接検証し notes.md に明記」に対応。
  正確には watcher は in-process `AppEventBus` を持ち、frontend には `notes-changed` 以外を
  出さない、という構造を notes.md に記載）
- **PASS**: **spec.md frontmatter の coherence** — `coherence.source: derived` + upstream 7 件
  （hash 付き: validation / detect-external-changes / external-file-change-events /
  note-file-modified-externally / note-feed-aggregate / note-aggregate / glossary-timestamp）✓
- **PASS**: **テストデータ独立性** — `mkdtempSync` の temp dir に config/data/storage を隔離し
  `onComplete` で `rmSync`。`TAURI_TEST_STORAGE_DIR` override により実 user 環境を汚さない ✓
- **MEDIUM**: **step 4 は弱い観測（競合ダイアログ）** — `WidgetExternalChangeConflict` の
  `defaultSubscribeFn` は no-op（`store.svelte.ts` OQ-WC1: Real event bridge (Rust → TS) is not
  yet wired）。そのため IDLE では当然ダイアログ非表示だが、EDITING でも S19 の bridge 未実装の
  ため E2E では区別できない。S19 の競合検出は slice / widget unit の領分。
  notes.md#known-issues に明記済み ✓
- **LOW**: **テストは Gherkin ステップ順に依存** — step 1 で変更 → step 2 で読み → step 3 で追加変更
  → step 4 で状態確認。`maxInstances: 1` + mocha 逐次実行が前提（s1〜s16 と同方針）。✓
- **LOW**: **vim atomic write 相当（`.tmp` → rename）は未検証** — 本テストは既存 `.md` を直接
  上書きする。rename（delete+create 2 イベント）や `.tmp` ignore の扱いは S18/S21/S22 /
  slice unit test の領分（notes.md#known-issues に明記）✓
- **LOW**: **`updatedAt` ソートの挙動は未検証** — S17 Then は「`updatedAt` ソート時、表示順が
  変わる可能性がある」と条件付きで述べる。本 scenario の When は body のみ変更（`updatedAt`
  据え置き、Note A 1 件）のため順序は自明。複数 Note × `updatedAt` ソートは S15 / NoteFeed
  sort の領分（notes.md#known-issues）✓

### Production fix discovery {#pass-1-production-fix}

production 側は S17 の不変条件を **既に満たしていた**。E2E を GREEN にするための
production 実装追加は **不要（変更 0 file）**:

- `start_file_watcher`（`note_feed/slices/detect_external_changes/commands.rs`）が
  `note_capture::shared::storage::resolve_storage_dir`（`TAURI_TEST_STORAGE_DIR` override 尊重）で dir を解決し、
  `notify` watcher を non-recursive / `.md` のみで起動、`WatcherHandle` を managed state に保持
- `FsWatcher::run_event_loop` が 500ms debounce を適用し、`Modify` を
  `RawFileEvent::Modified` として渡す
- `DetectExternalChangesUseCase::start_watcher` が `Modified` で `load_by_id` →
  `note.body_hash()` を計算し `DomainEvent::NoteFileModifiedExternally` を publish
- subscriber が `NoteFileModifiedExternally` で `InMemoryNoteFeedState::upsert_one` +
  `app_handle.emit("notes-changed", ())`
- `PageMain.svelte` の mount effect が `start_file_watcher` を invoke し、`notes-changed` を
  listen して `listNotesFn` → `feedStore.hydrateNotes`

RED 実測（テストが空振りでないことの確認）:

- **変異（`PageMain.svelte` の `invoke('start_file_watcher')` を無効化）**:
  test build → E2E **1 passing / 3 failing**
  - step 1: `block 20260620120000 body did not become "hello world"`
  - step 2: **PASS のまま**（`list_notes` は disk 直読みのため watcher 無効でも変更後 body が見える
    — read DTO 経路が非識別であることの実証）
  - step 3: `block ... body did not become "hello world 2"`
  - step 4: 同様に `"hello world 2"` に到達せず
- **復元後** → 再ビルド → E2E **4 passing (~3s)** で GREEN。

production への正味の変更は **0 file**（変異は完全に revert 済み。`git diff` clean。
working tree は新規 scenario ディレクトリのみ）。

回帰確認:

- frontend unit test: **143 passed / 15 files**
- 既存 E2E 回帰: s16-external-file-created **4 passing**
- Rust: production 変更ゼロのため追加の `cargo test` 影響なし

### Disposition {#pass-1-disposition}

- 指摘は MEDIUM/LOW のみで、いずれも観測の弱さ / スコープ注記であり、notes.md / spec.md に
  明記済み。E2E の観測点（手動操作なしの DOM body 自動更新 / upsert 差し替え / 内容一致 /
  連続変更の非空虚性 / 競合 UI 非表示）は満たす。
- production 変更は 0 file。RED 検出力は watcher 無効化変異の実測で確認済み（step 2 のみ PASS が
  read DTO 経路の非識別性の実証にもなっている）。debounce 窓の実測はテスト側のタイミング修正で
  正しく解消した。
- Verdict: **PASS**

<!-- verdict=PASS -->