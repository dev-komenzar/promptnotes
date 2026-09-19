# Review: s11-storage-dir-change {#review-s11-storage-dir-change}

## Pass 1 {#pass-1}

> reviewer agent (`ori-reviewer`) は spawn せず、main session の objective 検証で代替
> （orchestrator 指示: reviewer spawn がハングする既知問題があるため。s1〜s10 と同方針）。

### Syntax checks {#pass-1-syntax}

- tsc (`tsc --noEmit -p tsconfig.json`): PASS（テスト spec は 0 error。既知の
  `wdio.conf.ts TS2353 'tauri:options'` のみ — s1〜s10 と同一の WDIO v9 型定義起因、許容）
- docker-compose: SKIP — compose-service 参加者ゼロ (app `promptnotes` is mode=local)
- E2E: PASS (`wdio run wdio.conf.ts` → **3 passing**, ~3.6s)

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

- **PASS**: Given（old dir 3 件 / `settings.json#storage_dir=old`）→ `before` で
  `[data-block-id]` 集合と `readFileSync` の `storage_dir` を assert ✓
- **PASS**: **step 1（対照・modal UI 統合）** — 設定モーダルが `screen-2-storage-dir` に old dir を
  表示。theme のみを変更して `screen-2-save` → `settings.json#theme=Dark` 更新、`restart-prompt`
  非表示、Feed は old 3 件のまま。`validation.md#s11-then` の「StorageDirChanged は storage_dir
  差分時のみ」を theme 変更との対照で確認 ✓
- **PASS**: **step 2（本 scenario の核）** — modal open 中に境界 invoke で
  `storage_dir = new dir` → `settings.json#storage_dir=new` 更新、`StorageDirChanged` 発行により
  `restart-prompt` 出現（I-S4）。さらに modal から theme を変更して Save（storage_dir 差分ありの
  Save 経路）→ `handleSettingsSaved` が Feed を再 hydrate せず old 3 件のまま維持し、new dir の
  Note は現れない（I-S4 / C-US7）✓
- **PASS**: **step 3** — `restart-prompt-restart` → `window.location.reload()` → settings.json を
  再読込し new dir の Note 1 件のみ表示（`validation.md#s11-then`「再起動後に `/new/path` の
  スキャン結果が表示される」）✓
- **PASS**: spec.md frontmatter の `coherence.source: derived` + upstream 5 件（hash 付き:
  validation / update-settings / settings-aggregate / storage-dir-changed / screen-2）✓
- **PASS**: テストデータ独立性 — `mkdtempSync` の temp dir に config/data/storage を隔離し
  `onComplete` で `rmSync`。`XDG_CONFIG_HOME`/`XDG_DATA_HOME` の分離により実 user 環境を汚さない ✓
- **PASS**: step 2 の Feed 据え置き assertion が空振りでないことを変異テストで実測（後述
  Production fix discovery 変異 B）✓
- **MEDIUM**: event `StorageDirChanged` の Infrastructure 層 subscriber（ファイルウォッチャー
  再起動）は S22 の範囲で本 scenario 対象外（spec `#when-watcher-out-of-scope` に明記）✓
- **LOW**: `handleRestartNow` は `window.location.reload()`（WebView 再読込）で再起動を模す。
  Rust プロセスは継続するが、S11 の観測点（settings.json 再読込 → new dir スキャン）は満たす。
  真のプロセス再起動 / ウォッチャー再起動は S22 ✓
- **LOW**: 設定モーダルは read-only + native folder picker のため storage_dir を UI から直接
  保存できない。境界 invoke を modal open 中に起こし、theme save で guard を踏む方式は
  spec `#when-boundary-invoke` に明記済み（s10 の境界 invoke と同系統）✓

### Production fix discovery {#pass-1-production-fix}

production 側には S11 の UI 層前提が 2 点欠落していた:

1. `StorageDirChanged` を購読する再起動モーダルが無い（`widget-settings-modal` spec OQ-SM2 で先送り）
2. `PageMain.handleSettingsSaved` が保存のたび Feed を再 hydrate し、`storage_dir` 変更時に
   新ディレクトリへ即切替（I-S4 違反）

RED 実測（テストが空振りでないことの確認）:

- **変異 A（PageMain を HEAD に戻す = subscriber / guard なし）** → `bun run build:test` → E2E
  **1 passing / 2 failing**（step 2/3: `restart prompt did not appear`）
- **変異 B（guard のみ無効化 = 常に再 hydrate、subscriber は残す）** → 再ビルド → E2E
  **2 passing / 1 failing**（step 2: Feed が `["20260701100000"]` = new dir の Note に切替）
- **復元後** → `bun run build:test` → E2E **3 passing (3.6s)** で GREEN

production への正味の変更は `PageMain.svelte` のみ（+84 / -10）:

1. `settings:storage_dir_changed` subscriber `$effect`（→ `restartPromptOpen = true`）
2. 再起動モーダル `{#if restartPromptOpen}`（`restart-prompt-restart` / `restart-prompt-later`）
3. `handleSettingsSaved` の `storage_dir` 差分 guard（差分なし時のみ Feed 再 hydrate）
4. `page-main` の `data-restart-prompt-open` 属性

回帰確認:

- frontend unit test: **143 passed / 15 files**
- svelte-check: **0 errors / 0 warnings**
- E2E 回帰: s10-tag-invalid-char **3 passing**、s4-tag-assign-normalize **2 passing**
- Rust: 変更なし

既知の lint 失敗（HEAD 時点で既存、本変更対象外）: `toasts.test.ts` の prettier 非準拠、
`PageMain.svelte` の未使用 import（`editingNote` / `EditingNoteState`）と同ファイルの prettier
非準拠。詳細は notes.md#known-issues。

### Disposition {#pass-1-disposition}

- 指摘は MEDIUM/LOW のみで、いずれも S22 範囲（ウォッチャー再起動）または WebView reload の
  制約。spec に明記済みで E2E の観測点は満たす。
- production 不足（再起動モーダル / Feed 据え置き guard）は最小実装で補い、RED 検出力を
  変異 A / B の実測で確認済み。
- Verdict: **PASS**
