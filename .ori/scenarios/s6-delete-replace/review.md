# Review: s6-delete-replace {#review-s6-delete-replace}

## Pass 1 {#pass-1}

### Syntax checks

- tsc (`tsc --noEmit -p tsconfig.json`): PASS（テスト spec は 0 error。既知の `wdio.conf.ts TS2353 'tauri:options'` のみ — s5 と同一の WDIO v9 型定義起因、許容）
- docker-compose: SKIP — compose-service 参加者ゼロ (app `promptnotes` is mode=local)
- E2E: PASS (`wdio run wdio.conf.ts` → 1 passing, ~7.5s)

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

- **PASS**: Given（A/B 表示・未削除・スタック空）→ `before` で 2 block wait + 初期 `.md` 2 件存在 + toast 0 件を assert ✓
- **PASS**: t0（A 削除）→ A の `.md` 不在 / block 2→1 / toast 1 件 / `data-toast-id=A` 存在 ✓
- **PASS**: t1（B 削除）→ B の `.md` 不在 / block 0 / **toast 2 件**（A が置換されていない）/ A・B 両 `data-toast-id` 存在 ✓
- **PASS**: t1 積み上げ順 → `getBoundingClientRect().top` を `browser.execute` で取得し **B(top) < A(top)** を assert。DOM 順ではなく **画面上の Y 座標** で「最新が上」を判定しており、`screen-1.md#cross-toast-display` に忠実 ✓
- **PASS**: t2（A の Undo）→ A の `.md` 復元 + body=`alpha` / B の `.md` は absent のまま / toast A のみ消え **B は残存** / block 1（per-toast 独立性）✓
- **PASS**: t3（B の Undo）→ B の `.md` 復元 + body=`bravo` / toast 0（スタック空）/ block 2 ✓
- **PASS**: 特定トースト操作は `[data-testid="screen-1-toast-undo"][data-toast-id="<id>"]` で狙い撃ちし、複数トースト併存下でも誤操作しない ✓
- **PASS**: テストデータ独立性 — `mkdtempSync` の temp storageDir に A/B を seed、`onComplete` で `rmSync` ✓
- **PASS**: spec.md frontmatter の `coherence.source: derived` + upstream 5 件（hash 付き）✓
- **MEDIUM**: event 発行順序 (`NoteDeletedToTrash(A/B)` → `NoteRestoredFromTrash(A/B)`) の明示検証は無い。ただし production 配線が `NoOpBus` のため E2E 観測不可であり、各 slice の unit test で担保する方針（`notes.md#event-verification-policy` に制約として文書化）。scenario レベルでは許容 ✓
- **LOW**: Toast 有効期間 (5s) 超過後の no-op は S7 (`s7-undo-after-toast`) の責務。本 scenario は t0→t3 で最大 ~3s + 各操作 pause 1s の範囲で TTL 内に収めており、スコープ外 ✓
- **LOW**: `data-testid` セレクタの存在は実装依存（`@ori-generated` marker でドリフト管理）✓

### Production fix discovery {#pass-1-production-fix}

E2E 初回実行が **t1 積み上げ順で RED**（B が A より下 = Y 座標で B > A）。
原因は production `ToastRegion.svelte` のトーストスタックが `flex-col-reverse` であり、
`store.entries`（新しい順 = 先頭）を `flex-col-reverse` で描画するため **最新トーストが最下段**に
なっていたこと。これは `screen-1.md#cross-toast-display`
「新しい Toast は **上に積む**（最新が画面上側）」に違反する production 側の不具合。

- 最小修正: `flex-col-reverse` → `flex-col`（+ 根拠コメント）。store の順序（先頭 = 最新）は不変。
- 回帰確認: 変更後 E2E **1 passing**、frontend unit suite **142 passed / 15 files**、
  `svelte-check` 0 errors、変更ファイルの prettier/eslint clean。
- 備考: `bun run lint` 全体は `PageMain.svelte` / `stores/toasts.test.ts` の **既存** prettier
  警告で fail するが、本変更とは無関係（未変更ファイル）。

### Disposition {#pass-1-disposition}

- 指摘は MEDIUM/LOW のみで、いずれもスコープ外または制約として文書化済み。
- production 不足は本 scenario の正当な RED 要因であり、最小修正で GREEN 化済み。
- Verdict: **PASS**
