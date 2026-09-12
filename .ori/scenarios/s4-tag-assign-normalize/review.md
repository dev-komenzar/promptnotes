# Review: s4-tag-assign-normalize {#review-s4-tag-assign-normalize}

## Pass 1 {#pass-1}

### Syntax checks

- test code: SKIP (skeleton test, tauri app not built for scenario test)
- docker-compose: SKIP (compose-service 系参加者ゼロ)

### Semantic findings (spec ↔ test code)

- **PASS** spec.md#overview: 概要が 3 つの検証ポイント（Tag 正規化 / 重複 no-op / 変化時のみ event）を明記し、テスト観点に反映されている
- **PASS** spec.md#step-1-assign-duplicate: Step 1（重複 no-op）の Given/When/Then がテストコード `step 1` と一致。TagSet 変化なし・event 非発行を検証
- **PASS** spec.md#step-2-assign-new: Step 2（新規 tag 追加）の検証がテストコード `step 2` でカバー。NoteTagsChanged 発行とファイル更新を検証
- **PASS** runner config: wdio.conf.ts が architecture.md の binary path と一致、tauri-service 設定が正しい
- **NOTE** spec.md#test-points の「禁止文字 reject」は S10 の責務と明記されており、本 scenario の範囲を明確に区別している

### Disposition

- 指摘ゼロ → **verdict=PASS**
- scenario spec ↔ テストコードの整合性に問題なし
- テストコードは WDIO + Tauri integration の skeleton（S2 と同じ状態）。アプリビルド後の実テスト実行は別途

## Pass 2 {#pass-2}

再レビュー（空テストの実装置換 + seed 追加 + spec hash 是正後）。

### Pass 1 の訂正 {#pass-2-corrections}

Pass 1 は「Step 1/2 がテストコードでカバー」と PASS したが、当時のテストは
**アプリに一切触れていない空テスト**だった（IPC invoke はコメントアウト、step2 は
assert ゼロ、テスト自身が別 tmpDir に書いたファイルを読むだけ）。92ms の vacuously
PASS を実カバレッジと誤認していた。本 Pass でテストを UI 駆動に置換した。

### Mode checklist (local tauri) {#pass-2-mode}

| item | status | note |
|---|---|---|
| spec `runner: wdio` ↔ test 実行 ↔ wdio.conf.ts | ✅ PASS | 一致 |
| テストコード内に service lifecycle が無い | ✅ PASS | seed は onPrepare、テストは UI 操作のみ |
| local app が docker-compose に含まれない | ✅ PASS | compose 不在が正しい |
| wdio `tauri:options.application` == runtime `binary` | ✅ PASS | `target/debug/app` |
| `@wdio/tauri-service` が services に含まれる | ✅ PASS | `driverProvider: 'external'` |

### Semantic findings (spec ↔ test code) {#pass-2-findings}

- **RESOLVED（最重要）**: テストを UI 駆動（Block タグ入力 `screen-1-block-tag-input`）に
  実装。step1 = 正規化後重複の no-op（`"  GPT  "` → `gpt` 既存 → file tags / updatedAt
  不変・chip 1 件）、step2 = 新規 `coding` 追加（file tags `[gpt, coding]`・updatedAt
  変化・chip 追加）。
- **RESOLVED**: `wdio.conf.ts onPrepare` に Note A（`tags: [gpt]`）の seed を追加。
  app の `parse_tags_inline` は unquoted inline 形式を期待するため `tags: [gpt]` とした。
- **RESOLVED**: spec.md upstream の 4 エントリに `hash` を追記
  （`sha256(file)[:12]`）。
- **INFO（制約・文書化）**: `NoteTagsChanged` は `assign_tag` も `NoOpBus`
  (`assign_tag/commands.rs`) のため E2E 観測不可。`notes.md#event-verification-policy`
  に明記し、slice unit test で担保。
- **NOTE（proposal 候補）**: spec 実装ノートの「テストコード内で `startApp()`/`stopApp()`
  相当のライフサイクル管理が必要」は `scenario-test.instructions.md`（lifecycle は
  runner config 所有）に反する。spec 記述の修正は `/ori-propose` 候補。

### Disposition {#pass-2-disposition}

- Pass 1 の誤審（空テストを実カバレッジと誤認）を訂正し、テストを実質化。残りは
  NoOpBus 制約と spec 記述のみで non-blocking。
- Verdict: **PASS**