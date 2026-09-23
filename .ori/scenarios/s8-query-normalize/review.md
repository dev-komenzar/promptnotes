# Review: s8-query-normalize {#review-s8-query-normalize}

## Pass 1 {#pass-1}

> reviewer agent (`ori-reviewer`) は spawn せず、main session の objective 検証で代替
> （orchestrator 指示: reviewer spawn がハングする既知問題があるため。s1 Pass 2 / s7 と同方針）。

### Syntax checks {#pass-1-syntax}

- tsc (`tsc --noEmit -p tsconfig.json`): PASS（テスト spec は 0 error。既知の
  `wdio.conf.ts TS2353 'tauri:options'` のみ — s1〜s7 と同一の WDIO v9 型定義起因、許容）
- docker-compose: SKIP — compose-service 参加者ゼロ (app `promptnotes` is mode=local)
- E2E: PASS (`wdio run wdio.conf.ts` → **3 passing**, ~5.9s)

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

- **PASS**: Given（A/B 表示・検索バー空・filter 初期状態）→ `before` で 2 block wait +
  両 `.md` 存在 + 検索入力 `getValue() === ''` を assert ✓
- **PASS**: **step 1（本 scenario の核）** — 半角 `"gpt"` を検索バーに入力 →
  **A（半角 body `GPT を試す`）/ B（全角 body `ｇｐｔ のメモ`）の 2 Block が両方残る**。
  filter.query を `update_feed_filter` 直接 invoke で観測すると `"gpt"`（NFKC + lowercase、
  半角入力なので変化なし）✓
  → `validation.md#s8-then`「B の body を NFKC + lowercase: `gpt のメモ`（全角 → 半角化）→ match
  / 両者表示」を直接検証
- **PASS**: **step 2** — 全角大文字 `"ＧＰＴ"` を入力しても A / B が両方表示。Rust 境界の
  `NormalizedQuery` が `"gpt"` へ正規化することを DTO で確認（query 正規化 + body 正規化の合成）✓
- **PASS**: **step 3** — 検索バーを空に戻すと `NormalizedQuery::from_raw("")` が `None` に降格し
  A / B が再表示される（C-UF2 と整合）✓
- **PASS**: spec.md frontmatter の `coherence.source: derived` + upstream 4 件（hash 付き:
  validation / update-feed-filter / note-feed-aggregate / screen-1-toolbar-search-query）✓
- **PASS**: テストデータ独立性 — `mkdtempSync` の temp storageDir に A/B を seed、
  `onComplete` で `rmSync` ✓
- **MEDIUM**: event 非発行の明示検証は無い。NoteFeed は read model かつ `update-feed-filter` は
  揮発 filter で **構造的に domain event を発行しない**（`workflows/update-feed-filter.md#output`
  「domain event: なし」）。E2E で観測すべき event が存在しないため scenario レベルでは許容 ✓
- **LOW**: step 3 の解除は `clearValue()` の input 発火に依存する。WebKitWebDriver で実測 GREEN
  のため実害なし（`setValue('')` への置換は不要）
- **LOW**: `data-testid` / `data-block-id` セレクタの存在は実装依存（`@ori-generated` marker
  でドリフト管理）✓

### Production fix discovery {#pass-1-production-fix}

初回 E2E は **step 1 で RED**（`count([data-block-id="B"])` が `Expected: 1, Received: 0`）。
半角 query に対して A（半角 body）は残るが B（全角 body）が消えた。原因は frontend の
in-memory filter `applyFeedFilter`（`apps/promptnotes/src/ui-page/page-main/stores/feed.svelte.ts`）
が query / body / tag を lowercase するだけで **NFKC 正規化していなかった**こと。
Rust `NoteFeed::visible_notes` の `matches_query` は `nfkc().collect().to_lowercase()` して
比較する（I-F1 / I-F5）ため、frontend の暫定実装だけが乖離していた。

- 最小修正: `applyFeedFilter` の query matching で `normalize('NFKC')` を query / body / tag の
  全てに適用（Rust 側と同一の NFKC + lowercase）。他軸（date_range / tag / sort）は不変。
- 回帰確認:
  - frontend unit test: 新規 `spec#I-F1/I-F5`（全角 body `ｇｐｔ のメモ` が半角 query `gpt` に
    match）を `feed.test.ts` に追加。
  - 全 frontend suite: **143 passed / 15 files**（feed store 単体 23 passed）。
  - E2E: 修正前 **3 failing**（step 1/2/3 すべて B 消失）→ 修正後 **3 passing (5.9s)**。
  - `prettier --check` / `eslint`: 変更 2 file に指摘なし。
  - Rust 変更なし（`cargo` 再ビルドのみ。binary は `tauri build --debug --no-bundle` +
    `VITE_WDIO_TEST=1` で再生成済み）。

### Disposition {#pass-1-disposition}

- 指摘は MEDIUM/LOW のみで、いずれも read model の構造的制約または実行基盤の既知制約。
- production 不足（frontend の NFKC 欠落）は本 scenario の正当な RED 要因であり、
  最小修正で GREEN 化済み（RED → GREEN を実測で確認）。
- Verdict: **PASS**
