# Review: s10-tag-invalid-char {#review-s10-tag-invalid-char}

## Pass 1 {#pass-1}

> reviewer agent (`ori-reviewer`) は spawn せず、main session の objective 検証で代替
> （orchestrator 指示: reviewer spawn がハングする既知問題があるため。s1〜s9 と同方針）。

### Syntax checks {#pass-1-syntax}

- tsc (`tsc --noEmit -p tsconfig.json`): PASS（テスト spec は 0 error。既知の
  `wdio.conf.ts TS2353 'tauri:options'` のみ — s1〜s9 と同一の WDIO v9 型定義起因、許容）
- docker-compose: SKIP — compose-service 参加者ゼロ (app `promptnotes` is mode=local)
- E2E: PASS (`wdio run wdio.conf.ts` → **3 passing**, ~3.2s)

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

- **PASS**: Given（A 表示 / tags `[]` / `updatedAt=t0`）→ `before` で block wait +
  `readFileSync` の tags / `updatedAt` を assert ✓
- **PASS**: **step 1（本 scenario の核・UI 側）** — タグ入力欄に `"foo,bar"` を入力し Enter →
  `[data-testid="screen-1-block-tag-error"]` が出現し文言に `invalid characters` を含む。
  チップ非追加 / `.md` 完全一致（tags `[]` / `updatedAt=t0`）。
  → `validation.md#s10-then`「`Tag::new("foo,bar")` が `TagError::InvalidChar` を返す（I-N6）」
  「`Note::assign_tag` には到達しない」「UI: エラーメッセージ表示」を UI 経路で直接検証 ✓
- **PASS**: **step 2（禁止文字集合全体）** — Tauri 境界 `assign_tag` 直接 invoke で
  `"foo bar"`, `"foo\tbar"`, `"foo\nbar"`, `"foo,bar"`, `"foo[bar"`, `"foo]bar"` の全てが
  `{ kind: "invalid_tag", reason: 〜"invalid character" }` で reject される。加えて空文字
  `"   "` が `{ kind: "invalid_tag" }`（`TagError::Empty`）で reject。
  reject 後も `.md` 完全一致（tags `[]` / `updatedAt=t0`）→ 永続化なし = `Note::assign_tag`
  非到達を Tauri 境界で観測（workflow#notes「`parseTag` 単独で reject 可能、Note を load する前に
  バリデーション」と整合）✓
- **PASS**: **step 3（対照）** — 正常タグ `"valid"` を assign → `.md` の tags が `["valid"]`、
  `updatedAt ≠ t0`、UI チップ `valid` が出現。reject 経路が過剰一般化されていないことを確認 ✓
- **PASS**: spec.md frontmatter の `coherence.source: derived` + upstream 4 件（hash 付き:
  validation / assign-tag / note-aggregate / note-tags-changed）✓
- **PASS**: テストデータ独立性 — `mkdtempSync` の temp storageDir に A を seed、
  `onComplete` で `rmSync`。step 1 / 2 は状態を変異させないため step 間の順序依存は無い ✓
- **PASS**: `validation.md#s10-then` の禁止文字集合 ` `, `\t`, `\n`, `,`, `[`, `]` を
  `FORBIDDEN_TAG_CHARS` と一致する形で全件網羅（`FORBIDDEN_TAGS` const）✓
- **MEDIUM**: event `NoteTagsChanged` 非発行の E2E 直接観測は無い。Tauri command 配線が
  `NoOpBus` を注入するため（`commands.rs`: 「The Note Feed BC will subscribe here once it lands.」）
  E2E で観測すべき event が存在しない。担保は slice `assign-tag` の unit test に委ね、
  E2E では `invalid_tag` reject と `.md` 不変で間接検証 ✓
- **LOW**: UI の `<input type="text">` は `\t` / `\n` を直接入力できないため、これらは
  Tauri 境界 invoke で検証している（spec 実装ノート §when-forbidden-set に明記）。責務分離として妥当 ✓
- **LOW**: `data-block-id` / `data-testid` セレクタの存在は実装依存（`@ori-generated` marker で
  ドリフト管理）✓

### Production fix discovery {#pass-1-production-fix}

初回 E2E は **3 passing（GREEN on first run）**。production 側は既に S10 の全前提
（`Tag::new` の `FORBIDDEN_TAG_CHARS` 検査 / `AssignTagUseCase::execute` Step 1 `parseTag` が
`load_note` より前で `InvalidTag` を返す / `assign_tag` の `AssignTagErrorDto::InvalidTag
{ kind: "invalid_tag" }` / `Block.svelte` の `invalid_tag` catch とエラー表示）を備えており、
**不足は無かった**。テストが空振りでないことを示すため、禁止文字ガードを一時無効化して RED を実測:

- ガード無効（`FORBIDDEN_TAG_CHARS: &[char] = &[]`）→ `bun run build:test` →
  **3 failing**: step 1（エラー表示が出ず assign される）/ step 2（`Expected: false / Received: true`）
  / step 3（`updatedAt` が step1/2 で更新され `Expected: "20260620120000" / Received: "20260919085921"`）
- `git checkout` で復元 → `bun run build:test` → **3 passing (3.2s)** で GREEN

したがって本 scenario のテストは `Tag::new` 禁止文字ガードの欠落・regression を実際に検出する。
production への正味の変更は **無し**（RED 検証の一時 mutation は復元済み、`git diff` clean）。

回帰確認:

- frontend unit test: **143 passed / 15 files**
- E2E 回帰: s4-tag-assign-normalize **2 passing**（タグ正規化 / 永続化）、
  s2-autosave-debounce **2 passing**（`no-op autosave` = S9 idempotency 含む）
- Rust: 変更なし

### Disposition {#pass-1-disposition}

- 指摘は MEDIUM/LOW のみで、いずれも EventBus 配線（NoOpBus）の構造的制約または
  入力 UI の制約。E2E で観測不能な event 非発行は slice unit test が担保する。
- production 不足は無く、テストの RED 検出力（ガード無効時に step 1/2/3 が落ちる）を実測で確認済み。
- Verdict: **PASS**