# Review: s3-flush-on-blur {#review-s3-flush-on-blur}

## Pass 1 {#pass-1}

### Syntax checks

- test code: skipped (wdio globals not available in vanilla tsc — verified manually)
- docker-compose: N/A (local tauri app only — no compose-service participants)
- wdio.conf.ts: valid TypeScript ✓

### Mode-specific checks

| Check | Result |
|---|---|
| spec runner (wdio) ↔ test code imports | ✓ test uses wdio globals (`describe`/`it`/`browser`/`expect`/`$$`) |
| test code contains NO service start/stop/healthcheck | ✓ lifecycle is in wdio.conf.ts only |
| local tauri app NOT in docker-compose.yml | ✓ (no compose needed) |
| wdio.conf.ts `tauri:options.application` matches runtime binary | ✓ `apps/promptnotes/src-tauri/target/debug/promptnotes` |
| `@wdio/tauri-service` is in wdio services | ✓ |

### Semantic findings

| # | Severity | Source | Finding |
|---|---|---|
| **L1** | LOW | spec.md#scenario-steps vs Block.svelte | spec の記述では debounce timer は **500ms** (`autoSaveDebounceMs = 500`) だが、実際の Block.svelte コードは `autoSaveDebounceMs = 600` でデフォルトタイムアウト。これは non-blocking（600msがシナリオのロジックにに Flush パスルートに影響しない — テストは100ms 後に click → Flush は 500ms でも 600ms でも動作する）|

### Disposition

- **LOW 指摘** L1: spec vs code の 500ms / 600ms 差分 → `/ori-propose` でどちらかに統一することを提案。ただし scenario テストの PASS/FAIL には影響しないため、この review 自体は PASS

### Verdict: PASS

3 test cases cover all specified behaviors:
1. ✓ 別ブロッククリックで debounce 待たず即時保存 → state が EDITING → IDLE に戻る
2. ✓ debounce timer timeout 前に Flush 発火 → 100ms クリックで IDLE 確認
3. ✓ 未編集時のクリックでは Flush しない → 編集せずに click away → no-op

spec ↔ test ↔ config の整合に重大な乖離なし。

## Pass 2 {#pass-2}

再レビュー（テスト強化 + spec frontmatter 是正 + status 復旧後）。

### Pass 1 の訂正 {#pass-2-corrections}

- **binary path の誤検証**: Pass 1 は `tauri:options.application` を
  `target/debug/promptnotes` と記載し「runtime binary と一致 ✓」と判定していたが、
  実バイナリは `target/debug/app`（`964db65` で architecture.md も `app` に修正済み）。
  本 Pass で `wdio.conf.ts` が `.../debug/app` を指すことを確認。
- **L1 (spec 500 / Block 600ms) は解消**: `Block.svelte` の `autoSaveDebounceMs` を
  500ms に修正（`6ce4da6`）。
- **syntax check の skip 記述は陳腐化**: 現在は wdio deps 導入済みで実行可能。
  `tsc` は既知の `wdio.conf.ts TS2353 ('tauri:options')` のみ（実行時は正常）。

### Mode checklist (local tauri) {#pass-2-mode}

| item | status | note |
|---|---|---|
| spec `runner: wdio` ↔ test 実行 ↔ wdio.conf.ts | ✅ PASS | 一致 |
| テストコード内に service lifecycle が無い | ✅ PASS | seed は onPrepare、テストは操作のみ |
| local app が docker-compose に含まれない | ✅ PASS | compose 不在が正しい |
| wdio `tauri:options.application` == runtime `binary` | ✅ PASS | `target/debug/app` |
| `@wdio/tauri-service` が services に含まれる | ✅ PASS | `driverProvider: 'external'` |

### Semantic findings (spec ↔ test code) {#pass-2-findings}

- **RESOLVED（最重要）**: 旧テストは `data-block-state` の IDLE 遷移しか見ておらず、
  flush の実体（永続化 / debounce 前発火 / 重複なし）を検証していなかった。step1 を
  `.md` 永続化（body + updatedAt）/ `editedAt` から <500ms で flush 完了 / flush 後
  800ms 経過しても updatedAt 不変（debounce キャンセル）を assert するよう実質化。
  step2 は body を元に戻す no-op、step3 は I-PM10（同時 EDITING は高々 1）を検証。
- **RESOLVED**: spec frontmatter の非標準フィールド（`runner` / `trigger`）を標準
  `{ path, hash }` に是正。hash は `sha256(domain/validation.md)[:12] = 4a02c17cc025`
  （s5 と同規約）。
- **INFO（制約・文書化）**: event `NoteBodyEdited` は `flush-note` も `NoOpBus`
  (`flush_note/commands.rs`) のため E2E 観測不可。`notes.md#event-verification-policy`
  に明記し、slice unit test (`flush_note/tests.rs`) で担保。
- **NOTE（proposal 候補）**: spec test-point 4「2 ブロック同時 EDITING」は
  I-PM10（同時 EDITING は高々 1）と矛盾。step3 は I-PM10 として検証しており、
  spec 記述の修正は `/ori-propose` 候補。

### Disposition {#pass-2-disposition}

- Pass 1 の誤検証（binary path / test の実質性）を訂正し、テストを実質化。残りは
  NoOpBus 制約と spec test-point 4 の記述問題のみで、scenario レベルでは non-blocking。
- Verdict: **PASS**