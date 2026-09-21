# s19-external-modify-while-editing — Scenario implementation notes

## 検証対象 {#target}

`validation.md#s19-external-modify-while-editing` — ユーザが Block A を **EDITING 中**に
外部プログラムが `storage_dir/<A.id>.md` の body を変更したとき、競合ダイアログ
`widget-external-change-conflict`（screen-4）が表示され、KeepEditing / ApplyExternal の
解決が UI に反映されること。

経路: OS ファイルウォッチャー（`start_file_watcher` / `notify`）→ `NoteFileModifiedExternally`
→ NoteFeed `upsert_one` + Tauri event `notes-changed` → frontend の競合判定
（Block ステートマシンの EDITING 検出 + body hash 比較）→ screen-4 ダイアログ。

## 実行方法 {#how-to-run}

```bash
export PATH=$HOME/.cargo/bin:$PATH
export DISPLAY=:0
# test build (VITE_WDIO_TEST=1 で tauri-plugin-wdio を有効化)
cd apps/promptnotes && bun run build:test
cd ../../.ori/scenarios/s19-external-modify-while-editing
../../../apps/promptnotes/node_modules/.bin/wdio run wdio.conf.ts
```

結果: **5 passing (~7s)**。

## event 観測の制約（重要） {#event-observation}

`NoteFileModifiedExternally` の domain event は frontend / E2E に直接露出しない:

- `start_file_watcher` は内部で `AppEventBus`（in-process）を生成し、
  `DetectExternalChangesUseCase` 経由で `DomainEvent::NoteFileModifiedExternally` を publish する
- その subscriber は `InMemoryNoteFeedState::upsert_one` と Tauri event `notes-changed`
  （**payload なし**）の emit のみを行う（domain event payload は frontend に渡さない）
- frontend の `widget-external-change-conflict` の `defaultSubscribeFn` は no-op
  （`store.svelte.ts` OQ-WC1: Real event bridge (Rust → TS) is not yet wired）

したがって E2E は event payload（`disk_body_hash` / `note`）を直接 assert できない。
本 scenario では **frontend の最小 bridge** を追加して競合を観測可能にした（後述）。
E2E の判定は UI 状態（ダイアログ表示/非表示 + 両バージョン表示 + Block ステート）に置いた。

## RED 実測（テストの非空虚性検証） {#mutation-red}

test build 後に production 配線を入れる前の状態で E2E を実行:

- **結果: 5 failing (~54s)**
  - step 1: `conflict dialog widget-external-change-conflict did not appear`
  - step 2: dialog 不在のため note title 取得で失敗
  - step 3: dialog 不在のため Cancel クリックで失敗
  - step 4: dialog 不在のため失敗
  - step 5: step 3/4 が dialog を閉じられず Block が EDITING のまま残るため失敗
- これは「widget の `defaultSubscribeFn` が no-op で、EDITING 中の外部変更でも
  ダイアログが決して表示されない」という production 欠落が原因の RED

frontend 最小配線を追加 → 再ビルド → **5 passing (~7s)** で GREEN。
3 回連続実行して全て 5 passing（flaky でないことを確認）。

## production 変更 {#production-change}

**frontend のみ 5 file**（Rust 変更ゼロ）:

1. `apps/promptnotes/src/ui-page/page-main/stores/editing-note.svelte.ts`
   - 単なる singleton から `createEditingNoteStore()` factory + singleton へ。
     `clearIfCurrent(noteId)` を追加（従来 `setEditing` はどこからも呼ばれていなかった）
2. `apps/promptnotes/src/ui-page/page-main/stores/body-hash.ts`（新規）
   - `hashBody(body)`: Web Crypto SHA-256（Rust `BodyHash::from_body` と同一）。
     `crypto.subtle` 不可の環境では `plain:<body>` にフォールバック（両辺同一関数なので等価比較は保たれる）
3. `apps/promptnotes/src/ui-page/page-main/stores/external-change-bridge.ts`（新規）
   - `notes-changed` → `list_notes` の差分から、EDITING 中の note のディスク body が
     ローカル body と異なる場合に `NoteFileModifiedExternally` 形の payload を
     dialog store へ emit。衝突時は `{ noteId, localBody }` を返し、呼び出し元に
     「ローカル body を保持すべき」ことを伝える
4. `apps/promptnotes/src/ui-page/page-main/PageMain.svelte`
   - `WidgetExternalChangeConflict` に実 deps（`subscribeFn` / `currentNoteId` /
     `currentBodyHash` / `onApplyExternal`）を注入（従来は `localBody=""` + no-op）
   - EDITING 中のみ `editingNote` に note_id と body hash を保持する `$effect` を追加
   - `notes-changed` listener で競合を検出し、衝突 note のローカル body を保持したまま
     他 note を hydrate（従来は無条件 hydrate で編集中 body が clobber されていた）
   - `onApplyExternal` で feed body を外部版に置換し `focusStore.clear()`（IDLE へ）
5. `apps/promptnotes/src/ui-widget/external-change-conflict/*`
   - **変更なし**（store / component は既存のまま。既存 vitest 9 件も変更なし）

### Rust 側（変更なし）

`infrastructure.rs`（`Modify` → `RawFileEvent::Modified` / 500ms debounce）、
`application.rs`（`load_by_id` → `NoteFileModifiedExternally` publish）、
`commands.rs`（`upsert_one` + `notes-changed` emit）は s17 で実証済みの既存挙動で充足。

## 追加 unit test {#unit-tests}

- `src/ui-page/page-main/stores/body-hash.test.ts`（2 件）:
  SHA-256 既知ベクタ（`""` / `"abc"`）+ 決定性・差分
- `src/ui-page/page-main/stores/external-change-bridge.test.ts`（6 件）:
  EDITING でない / body 一致 / body 不一致（emit + hash 更新） / disk に当該 note なし /
  handler 未購読 / `currentLocalBody` の境界

## 回帰確認 {#regression}

- frontend unit test: `bun run test` → **151 passed / 17 files**（s18 baseline 143 + 新規 8）
- 既存 E2E 回帰: `s17-external-file-modified-no-conflict` **4 passing** /
  `s18-external-file-deleted` **4 passing**
- `tsc --noEmit -p tsconfig.json`（scenario）: 既知の `wdio.conf.ts TS2353 'tauri:options'` のみ
- `cargo test`: **328 passed / 2 failed**。失敗 2 件
  （`list_feed::tests::tp_f5_last_7_days` / `tp_f6_and_composition`）は
  テスト内の固定日付（2026-06-26 等）に対する `Last7Days` / `Last30Days` を
  wall-clock `now`（本日 2026-09-21）で評価する **既存の time-bomb test** であり、
  本 scenario の変更（Rust 変更ゼロ）とは無関係の pre-existing failure

## 既知の制約 {#known-issues}

- **domain event bridge は未配線（OQ-WC1 継続）**: `NoteFileModifiedExternally` payload を
  Rust → TS へ渡す経路は無く、本 bridge は `notes-changed`（payload なし）→ `list_notes` の
  body 差分から payload を構成している。payload の `file_path` は `${id}.md`、
  `detected_at` は frontend の `new Date()` で、domain の実値ではない。
  真の Rust bridge は `detect-external-changes` / `widget-external-change-conflict` の
  follow-up（OQ-WC1）として残る
- **ダイアログ解決ボタンは DOM click で操作**: modal `<dialog>` は browser top layer にあり、
  本 headless WebKitGTK では WebDriver のネイティブ pointer click が
  window-focus 管理の影響で flaky（`cancel` handler が発火しない run を実測）だったため、
  `screen-4-*` の radio / button は `browser.execute` の DOM click で dispatch している。
  競合のトリガー自体は実ファイル書き込み（`node:fs`）で、そこは native 経路
- **IME 制約**: wdio から日本語（`"hello 編集中"`）を入力すると IME composition を再現できず
  不安定なため、ローカル編集は ASCII `"hello local"` で表現する。競合判定の意味は同一
- **AutoSave の順序**: validation Given は「AutoSave 未発火」だが、pending write が外部変更を
  上書きして race するのを避けるため、タイプ後に AutoSave の永続化を待ってから外部変更を起こす
- **I-WC7（ダイアログ表示中の Editor キー入力ブロック）**: store コメント上、呼び出し元の責務。
  本 scenario の E2E 観測対象外（未実装）
- **テストは Gherkin ステップ順に依存**（step 1 表示 → step 2 内容 → step 3 KeepEditing →
  step 4 ApplyExternal → step 5 非編集時）。`maxInstances: 1` + mocha 逐次実行が前提
  （s1〜s18 と同方針。`.claude/rules/scenario-test.md`「テスト間独立」からの既知の逸脱）
- **`crypto.subtle` の可否**: hash は Web Crypto を優先し、不可なら `plain:` トークンへ
  フォールバックする。Tauri の custom scheme が non-secure context でも
  ローカル/ディスク両辺が同一関数を通るため等価比較は成立する
- **s20（編集中の外部削除）は範囲外**: 削除通知の設計は別 scenario
