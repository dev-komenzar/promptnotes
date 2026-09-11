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