# Review: s5-delete-undo-in-window {#review-s5-delete-undo-in-window}

## Pass 1 {#pass-1}

### Syntax checks

- tsc --noEmit: FAIL — `@wdio/globals/types` / `mocha` / `node` types not resolved in isolated scenario tsconfig (expected: project-level node_modules resolution requires `typeRoots`; fixed in tsconfig.json)
- docker-compose: SKIP — compose-service 参加者ゼロ (app promptnotes is mode=local)

### Semantic findings (ori-review — main session)

Spec ↔ テストコード ↔ config 整合性:

- **PASS**: runner (wdio) ↔ spec.md `runner: wdio` ↔ wdio.conf.ts ✓
- **PASS**: wdio.conf.ts: `tauri:options.application` = `runtime.binary` ✓
- **PASS**: local app が docker-compose に含まれていない ✓
- **PASS**: テストコードに compose / healthcheck 待機なし (lifecycle は config 所有) ✓
- **PASS**: delete → feed 除去 → undo → feed 復元 の UI 検証あり ✓
- **PASS**: toast 表示/非表示の検証あり ✓
- **PASS**: トースト有効期間内 Undo (5 秒以内) ✓
- **MEDIUM**: テストコードのファイル操作 (`node:fs`) は `before`/`beforeEach` 内で実行する必要がある。spec.md の実装ノートは `browser.executeAsync()` 経由での fs 使用を示唆しているが、WDIO の browser context では `node:fs` import が解決されない。ただし `before`/`beforeEach` hook は Node.js プロセスで実行されるため、file-level fixture として `storageDir` に `.md` を直接配置する方が実用的。→ spec の修正でカバー（medium、実装時に調整可能）
- **MEDIUM**: event 発行順序 (`NoteDeletedToTrash` → `NoteRestoredFromTrash`) の明示検証がない。UI+FS 検証で間接的にカバーされるため scenario レベルでは許容範囲
- **LOW**: `data-testid` セレクタの存在は実装に依存（`@ori-generated` marker でドリフト防止済）

verdict: **PASS**（指摘は MEDIUM/LOW、scenario レベルでブロックしない）

## Pass 2 {#pass-2}

再レビュー（review.md の tool-call アーティファクト除去 + テスト tsc 修正 +
status schema 統一 + spec 実装ノート是正後）。

### 破損の修復 {#pass-2-repair}

Pass 1 の後ろに AI のツールコールブロック（write 呼び出しの擬似タグ列）がそのまま
書き込まれ、review 本文が二重に埋め込まれた状態になっていた。監査ログとして不正な
ため除去し、本 Pass 2 に置換した。

### Pass 1 指摘の解消 {#pass-2-resolved}

- **MEDIUM（node:fs と browser.executeAsync の齟齬）**: spec 実装ノートを実態
  （テストプロセスの `node:fs`）に修正。解消。
- **MEDIUM（event 発行順序の明示検証なし）**: `delete_note` / `restore_deleted_note` も
  `NoOpBus` のため E2E 不可であることを `notes.md#event-verification-policy` に明記。
  slice unit test で担保。制約として文書化。
- **LOW（data-testid）**: 実装に存在、ドリフトなし。
- **tsc**: テストの `$$(...).length` が `Promise<number>` 型になる 2 件のエラーを
  `count` ヘルパーで解消（残りは既知の `wdio.conf.ts TS2353 'tauri:options'` のみ）。

### Mode checklist (local tauri) {#pass-2-mode}

| item | status | note |
|---|---|---|
| spec `runner: wdio` ↔ test 実行 ↔ wdio.conf.ts | ✅ PASS | 一致 |
| テストコード内に service lifecycle が無い | ✅ PASS | seed は onPrepare、テストは操作のみ |
| local app が docker-compose に含まれない | ✅ PASS | compose 不在が正しい |
| wdio `tauri:options.application` == runtime `binary` | ✅ PASS | `target/debug/app` |
| `@wdio/tauri-service` が services に含まれる | ✅ PASS | `driverProvider: 'external'` |

### Semantic findings (spec ↔ test code) {#pass-2-findings}

- テストは実質的: 削除で `.md` 不在 / feed -1 / toast 表示 → undo で `.md` 復元
  （body=`hello`）/ feed 復元 / toast 消滅 を assert（1 passing 確認済み）。
- event `NoteDeletedToTrash` → `NoteRestoredFromTrash` は NoOpBus のため E2E 不可
  （`notes.md` に明記、slice unit test で担保）。
- spec 実装ノートを実態（seed 方式 / `node:fs` / `dispatchEvent`）に修正。
- status.yaml を s1〜s4 と同形（`beads` + `phases: closed`）に統一。

### Disposition {#pass-2-disposition}

- tool-call 破損を除去。Pass 1 の指摘はすべて解消または制約として文書化。
- Verdict: **PASS**
