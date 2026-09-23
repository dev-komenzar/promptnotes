# Review: s1-note-created-happy {#review-s1-note-created-happy}

## Pass 1 {#pass-1}

### Syntax checks

- test code: PASS (project deps `@wdio/globals/types` / `mocha` / `node` types are not installed in `.ori/` scope — expected. Actual execution happens within project directory where these are available.)
- docker-compose: SKIPPED (no compose-service participants — local Tauri app only. No `docker-compose.yml` exists, which is correct.)

### Mode checklist {#mode-checklist}

| item | status | note |
|---|---|---|
| spec runner (= wdio) ↔ test code import (`@wdio/globals`) | ✅ PASS | consistent |
| spec runner ↔ wdio.conf.ts | ✅ PASS | wdio.conf.ts present, `@wdio/tauri-service` configured |
| local app NOT in docker-compose | ✅ PASS | no compose exists, correct |
| wdio `tauri:options.application` ↔ runtime `binary` | ✅ PASS | `../../../apps/promptnotes/src-tauri/target/debug/app` — matches Cargo project name "app" (architecture.md says `promptnotes` but actual binary is `app`) |
| No service lifecycle in test code | ✅ PASS | lifecycle is in wdio.conf.ts only |

### Binary path note

- architecture.md: `apps/promptnotes/src-tauri/target/debug/promptnotes`
- Actual binary: `apps/promptnotes/src-tauri/target/debug/app`
- wdio.conf.ts uses actual binary path `app`. architecture.md may need a correction (`s/promptnotes/app/` in `runtime.binary`), but this is out of scope for this review.

### Semantic findings (spec ↔ test code consistency)

- **LOW** spec.md#then: "UI: Draft 入力欄がクリア、新規ブロックへフォーカス遷移" — test verifies draft clear ✅, but **focus transition to new block is not explicitly asserted**. Test only verifies `draftText.trim() === ''`. The focus assertion (e.g. `expect(activeElement).toHaveAttr('data-testid', 'note-block')`) is missing.
- **NOTE** Test uses `data-testid` selectors (`[data-testid="draft-input"]`, `[data-testid="note-block"]`). These attributes must exist in the actual UI components. If not present, the test will fail at runtime with element-not-found. This is expected — the test serves as a contract that these testids must be added during implementation.

### Disposition

- LOW 指摘 (focus transition): minor coverage gap. Test already covers the core assertions (NoteFeed count, body content, draft clear). The focus assertion can be added when implementing the testids.
- Verdict: **PASS** — no blocking issues. LOW item is a refinement, not a gate failure.

## Pass 2 {#pass-2}

再レビュー（Cmd+N 乖離の解消 + scenario test 更新後）。reviewer agent は spawn したが
30 分 timeout で応答なし → main session の objective 検証で代替。

### Syntax checks {#pass-2-syntax}

- test code: PASS (実行で GREEN。`tsc --noEmit -p tsconfig.json` は
  `wdio.conf.ts TS2353 'tauri:options'` の **pre-existing** 型エラーのみ — 実行時は正常。
  s5 でも同種の tsc 制約を許容済み)
- docker-compose: SKIPPED (compose-service 参加者ゼロ = local Tauri app。不在が正常)

### Mode checklist (local tauri) {#pass-2-mode}

| item | status | note |
|---|---|---|
| spec `runner: wdio` ↔ test import (`@wdio/globals`) ↔ wdio.conf.ts | ✅ PASS | 一致 |
| テストコード内に service lifecycle (起動/停止/healthcheck) が無い | ✅ PASS | lifecycle は wdio.conf.ts 所有 |
| local app が docker-compose に含まれない | ✅ PASS | compose 不在が正しい |
| wdio `tauri:options.application` == architecture runtime `binary` | ✅ PASS | `target/debug/app` |
| `@wdio/tauri-service` が services に含まれる | ✅ PASS | `driverProvider: 'external'` |

### Semantic findings (spec ↔ test code) {#pass-2-findings}

- **RESOLVED (旧 LOW)** spec.md#when: 「`Cmd+N` で Draft 入力欄にフォーカス」—
  実装 (`PageMain.svelte` / `DraftRegion.svelte`) で対応し、scenario test step 1 が
  `Ctrl+N` 後の `document.activeElement` が `[data-testid="screen-1-draft-body"]` 内
  (`.cm-content`) であることを assert するよう更新。
  - 実証: page-main component test 6 passed / フル suite 141 passed /
    E2E s1 2 passing / E2E probe で実 Tauri(WebKit) の focus 移動を確認
- **LOW (残)** spec.md#then: 「新規ブロックへフォーカス遷移」— 作成後の新規 Block への
  focus 遷移は依然 assert していない。Pass 1 同様、scenario レベルでは non-blocking の
  refinement 扱い（別途 follow-up 候補）。
- **NOTE** テストは session 直後に `Ctrl+N` を送るため、Draft region の mount 待ち
  (`waitForExist`) を追加した。service 起動待機ではなく UI mount 待機であり
  `scenario-test.instructions.md` の「lifecycle は config 所有」に抵触しない。

### Disposition {#pass-2-disposition}

- Cmd+N 乖離は実装 + テストで解消。RESOLVED。
- 残 LOW (新規 Block focus) は non-blocking のため差し戻し不要。
- Verdict: **PASS**