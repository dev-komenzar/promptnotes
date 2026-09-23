# Review: s12-startup-state {#review-s12-startup-state}

## Pass 1 {#pass-1}

> reviewer agent (`ori-reviewer`) は spawn せず、main session の objective 検証で代替
> （orchestrator 指示: reviewer spawn がハングする既知問題があるため。s1〜s11 と同方針）。

### Syntax checks {#pass-1-syntax}

- tsc (`tsc --noEmit -p tsconfig.json`): PASS（テスト spec は 0 error。既知の
  `wdio.conf.ts TS2353 'tauri:options'` のみ — s1〜s11 と同一の WDIO v9 型定義起因、許容）
- docker-compose: SKIP — compose-service 参加者ゼロ (app `promptnotes` is mode=local)
- E2E: PASS (`wdio run wdio.conf.ts` → **1 passing**, ~0.7s)

### Mode checklist (local tauri) {#mode-checklist}

| item | status | note |
|---|---|---|
| spec `runner: wdio` ↔ test 実行 ↔ wdio.conf.ts | ✅ PASS | 一致 |
| テストコード内に service lifecycle が無い | ✅ PASS | seed は onPrepare、テストは assert のみ |
| local app が docker-compose に含まれない | ✅ PASS | compose 不在が正しい |
| wdio `tauri:options.application` == runtime `binary` | ✅ PASS | `target/debug/app` |
| `@wdio/tauri-service` が services に含まれる | ✅ PASS | `driverProvider: 'external'` |

### Semantic findings (main session objective verification) {#pass-1-findings}

spec ↔ テストコード ↔ domain 整合性:

- **PASS**: **step 1（本 scenario の核）** — `settings.json` の
  `sort_preference = { field: "updated_at", direction: "asc" }` が `load_settings` →
  `feedStore.hydrateSort` 経由で復元され、`[data-block-id]` の並びが `[beta, gamma, alpha]`
  （updatedAt 昇順）になる。seed は updatedAt 昇順 ≠ createdAt 昇順 ≠ createdAt 降順
  （I-S3 default）に設計されており、sort 復元の有無を判別できる ✓
- **PASS**: **起動時 filter 空（Q3 / I-F6）** — `screen-1-toolbar-search-query` の value が空、
  `screen-1-toolbar-date-range-all` が `aria-pressed=true`、`screen-1-toolbar-tag-chip` が不在。
  前回セッションの `"gpt"` / `"coding"` を持ち越さない ✓
- **PASS**: **toolbar の sort 状態** — `screen-1-toolbar-sort-field-updated_at` が active、
  `screen-1-toolbar-sort-direction` の `aria-label` が `Ascending`。`screen-1.md#cross-sort-immediate`
  の「sort は Settings に永続化 / 復元」に対応 ✓
- **PASS**: **Settings 永続状態** — `settings.json` の `storage_dir` / `sort_preference` が seed 値の
  まま（起動が書き換えない）✓
- **PASS**: **filter 非永続（対照）** — `settings.json` に `query` / `tag` / `date_range` / `filter`
  キーが無い。filter が揮発（Q3 / I-F6）である設計を裏付ける ✓
- **PASS**: spec.md frontmatter の `coherence.source: derived` + upstream 9 件（hash 付き:
  validation / load-settings / list-feed / change-sort-order / update-feed-filter /
  settings-aggregate / note-feed-aggregate / sort-preference-changed / screen-1）✓
- **PASS**: テストデータ独立性 — `mkdtempSync` の temp dir に config/data/storage を隔離し
  `onComplete` で `rmSync`。`XDG_CONFIG_HOME`/`XDG_DATA_HOME` の分離により実 user 環境を汚さない ✓
- **PASS**: step 1 の表示順 assertion が空振りでないことを変異テストで実測（後述 Production fix
  discovery 変異 A）✓
- **MEDIUM**: `Settings::load_or_default` は validation.md 上の概念名で、実 Rust 関数は
  `LoadSettingsUseCase::execute`。挙動（sort_preference の field 単位 fallback）は一致。spec
  `#then` に実装名を併記済み ✓
- **LOW**: filter の「リセット」は webview reload ではなく**プロセス新規起動**でのみ観測できる。
  E2E は新規セッション起動で post-restart 状態を再現し、filter 非永続は `settings.json` 検査で
  確認する方式。spec `#impl-notes` / notes.md#known-issues に明記済み ✓
- **LOW**: sort は backend `list_notes` と frontend `visibleNotes` の二重適用。ユーザー可視順を
  支配するのは frontend `hydrateSort`（変異 A で実測）。冗長だが無害で、E2E は可視 state を検証 ✓

### Production fix discovery {#pass-1-production-fix}

production 側は S12 の前提を**既に満たしていた**（追加実装は不要）:

1. `load_settings` が `settings.json` の `sort_preference` を field 単位 fallback 付きで復元
   （`apps/promptnotes/src-tauri/src/user_preferences/slices/load_settings/`）
2. `list_notes` が `settings.sort_preference()` を feed に適用（`note_feed/slices/list_feed/commands.rs`）
3. `page-main` が起動時に `hydrateSort(settings.sort_preference)` を実行
   （`apps/promptnotes/src/ui-page/page-main/PageMain.svelte`）
4. 起動時 filter は `feedStore` の `DEFAULT_FILTER`（空）のまま = 揮発

RED 実測（テストが空振りでないことの確認）:

- **変異 A（`PageMain.svelte` の起動時 `feedStore.hydrateSort(settings.sort_preference)` を削除）**
  → `VITE_WDIO_TEST=1 bun run tauri build --debug --no-bundle` → E2E **1 failing**（exit 1）。
  step1 が表示順で RED: `Expected ["20260201100000","20260301100000","20260101100000"]`
  （updatedAt 昇順）に対し `Received ["20260301100000","20260201100000","20260101100000"]`
  （createdAt 降順 = I-S3 default）。
- **復元後** → 再ビルド → E2E **1 passing (~0.7s)** で GREEN。

production への正味の変更は **0 files**（`git diff apps/` 空）。

回帰確認:

- frontend unit test: **143 passed / 15 files**
- svelte-check: **0 errors / 0 warnings**
- E2E 回帰: s11-storage-dir-change **3 passing**、s4-tag-assign-normalize **2 passing**、
  s10-tag-invalid-char **3 passing**
- Rust: 変更なし

### Disposition {#pass-1-disposition}

- 指摘は MEDIUM/LOW のみで、いずれも概念名の差・restart 観測方法・sort 二重適用の注記。
  spec / notes に明記済みで E2E の観測点は満たす。
- production 変更は不要（既存実装で充足）。RED 検出力は変異 A の実測で確認済み。
- Verdict: **PASS**
