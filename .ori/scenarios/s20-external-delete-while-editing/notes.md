# s20-external-delete-while-editing — Scenario implementation notes

## 検証対象 {#target}

`validation.md#s20-external-delete-while-editing` — ユーザが Block A を **EDITING 中**に
外部プログラムが `storage_dir/<A.id>.md` を削除したとき、削除通知
`widget-external-delete-notice` が表示され、Discard / Save as new file の解決が
UI/FS に反映されること。

経路: OS ファイルウォッチャー（`start_file_watcher` / `notify`）→ `NoteFileDeletedExternally`
→ NoteFeed `remove_one` + Tauri event `notes-changed` → frontend の EDITING 検出
→ 削除通知 → 解決。

## 実行方法 {#how-to-run}

```bash
export PATH=$HOME/.cargo/bin:$PATH
export DISPLAY=:0
# test build (VITE_WDIO_TEST=1 で tauri-plugin-wdio を有効化)
cd apps/promptnotes && bun run build:test
cd ../../.ori/scenarios/s20-external-delete-while-editing
../../../apps/promptnotes/node_modules/.bin/wdio run wdio.conf.ts
```

結果: **5 passing (~9–10s)**（3 連続実行で安定）。

## event 観測の制約（重要） {#event-observation}

`NoteFileDeletedExternally` の domain event は frontend / E2E に直接露出しない:

- `start_file_watcher` は内部で `AppEventBus`（in-process）を生成し、
  `DetectExternalChangesUseCase` 経由で `DomainEvent::NoteFileDeletedExternally` を publish する
- その subscriber は `InMemoryNoteFeedState::remove_one` と Tauri event `notes-changed`
  （**payload なし**）の emit のみを行う（domain event payload は frontend に渡さない）
- したがって削除検知は **presence / absence**（`list_notes` の結果に編集対象 note が
  存在するか）で行い、body hash / `crypto.subtle` には依存しない

E2E は event payload を直接 assert せず、UI 状態（通知表示/非表示 + body 表示 +
Block ステート）と FS 状態（`.md` の再作成）で間接検証する。

## RED 実測（テストの非空虚性検証） {#mutation-red}

pre-S20 の test binary（production 配線前）で E2E を実行:

- **結果: 5 failing (~54s)**
  - step 1: `delete notice widget-external-delete-notice did not appear`
  - step 2–5: 通知が出現しないため連鎖的に失敗
- これは「EDITING 中の外部削除で Block が黙って消え、通知 UI が存在しない」という
  production 欠落が原因の RED

production 実装後に再ビルド → **5 passing (~9–10s)**。3 回連続実行して全て 5 passing
（flaky でないことを確認）。

## production 変更 {#production-change}

**Rust 新規 1 slice + frontend 6 file**:

### Rust: 新 slice `note_capture/slices/recreate_note/`

1. `domain.rs`（新規）: `RecreateNoteCommand { created_at, raw_body, raw_tags }` +
   `RecreateNoteError { InvalidBody | InvalidTag | PersistError }`
2. `application.rs`（新規）: `RecreateNoteUseCase` — parseBody → parseTags →
   `Note::from_persisted(body, tags, created_at, now)` → `repo.write`。`created_at` を
   note_id から復元するため **元の NoteId のまま**再作成される（I-N2）。domain event は
   発行しない（persistence repair。NoteFeed は frontend が直接更新し、watcher の Created で自己修復）
3. `commands.rs`（新規）: `#[tauri::command] recreate_note(note_id, raw_body, raw_tags)`。
   `note_id` の parse 失敗は `InvalidNoteId` として区別（restore_deleted_note と同方針）
4. `mod.rs`（新規）/ `tests.rs`（新規・4 件）
5. `note_capture/slices/mod.rs`: `pub mod recreate_note;` を追加
6. `lib.rs`: `recreate_note` を `generate_handler!` に登録

> specta / `bindings.ts` / `export-types` は本 repo には存在しない（architecture の
> cross_root 記述は未配線）。frontend は手書き `invoke` wrapper で呼ぶ。

### Frontend

1. `src/lib/note-capture/slices/recreate-note/index.ts`（新規）:
   `recreateNote(noteId, rawBody, rawTags)` の invoke wrapper
2. `src/ui-widget/external-delete-notice/store.svelte.ts`（新規）:
   `createExternalDeleteNoticeStore` — 削除 payload 保持 / `saveAsNew` / `discard` /
   重複通知 debounce
3. `src/ui-widget/external-delete-notice/WidgetExternalDeleteNotice.svelte`（新規）:
   `<dialog data-testid="widget-external-delete-notice">`。`delete-notice-title` /
   `delete-notice-message` / `delete-notice-body-local` / `delete-notice-save-as-new` /
   `delete-notice-discard`。Esc は `preventDefault`（2 択必須）
4. `src/ui-widget/external-delete-notice/tests/store.test.ts`（新規・8 件）
5. `src/ui-page/page-main/stores/external-change-bridge.ts`:
   `subscribeDeletion` / `notifyExternalDeletion` / `deleteDeps` を追加。EDITING 中の
   note が disk snapshot に存在しない場合のみ削除 payload を emit し、local snapshot
   （body/tags/created_at/updated_at）を返す。既存 conflict path と排他
6. `src/ui-page/page-main/stores/external-change-bridge.test.ts`: 削除パス 5 件を追加
7. `src/ui-page/page-main/PageMain.svelte`:
   - `notes-changed` handler に削除分岐を追加。削除検知時は **local snapshot を feed に
     保持** して Block / editor を維持（従来は無条件 hydrate で Block が黙って消えた）
   - `WidgetExternalDeleteNotice` に実 deps を注入
   - Save as new file: `recreate_note` invoke → `feedStore.applyAutoSave(id, updated_at)` →
     `editingNote` clear → `focusStore.clear()`（IDLE）
   - Discard: `feedStore.applyDelete(id)` → `editingNote` clear → `focusStore.clear()`

## 追加 unit test {#unit-tests}

- Rust `recreate_note/tests.rs`（4 件）: 元 id 保持 / 1 回 write / invalid body / invalid tag /
  persist failure
- frontend `external-delete-notice/tests/store.test.ts`（8 件): silent / 表示 / 非編集 ignore /
  重複 ignore / saveAsNew / discard / stop / subscribe failure
- frontend `external-change-bridge.test.ts`（+5 件): 非編集 silent / disk 生存 silent /
  feed 不在 silent / EDITING+absent で emit + snapshot / deleteDeps 配線

## ドメイン文書ギャップ {#domain-gap}

`ui-fields/page-groups.md#widget-external-change-conflict` と `screen-4.md#notes-lifecycle` は
「`NoteFileDeletedExternally` では widget を mount しない（Feed 自動更新で十分）」と規定するが、
validation S20 は **EDITING 中の外部削除**に対する通知を要求しており、その UI は Phase 11a で
設計されていない。本 scenario は validation S20 を優先し、conflict widget（screen-4）とは
別の新 widget `widget-external-delete-notice` として最小実装した。ui-fields への正式反映
（screen-5 新設 or screen-4 への `deleted` display state 追加）は `/ori-propose` の follow-up。

## 回帰確認 {#regression}

- frontend unit test: `bun run test` → **164 passed / 18 files**（s19 baseline 151/17 + 新規 13）
- 既存 E2E 回帰: `s18-external-file-deleted` **4 passing** /
  `s19-external-modify-while-editing` **5 passing**
- `tsc --noEmit -p tsconfig.json`（scenario）: 既知の `wdio.conf.ts TS2353 'tauri:options'` のみ
- `svelte-check`: 0 errors / 0 warnings
- `cargo test`: **332 passed / 2 failed**。失敗 2 件
  （`list_feed::tests::tp_f5_last_7_days` / `tp_f6_and_composition`）は
  テスト内の固定日付（2026-06-26 等）に対する `Last7Days` / `Last30Days` を
  wall-clock `now`（本日 2026-09-21）で評価する **既存の time-bomb test** であり、
  本 scenario の変更とは無関係の pre-existing failure
- `cargo clippy --all-targets`: 新 slice `recreate_note` への指摘ゼロ（既存ファイルの
  warning のみ）
- `cargo fmt --check`: 既存 drift（`lib.rs` / `shared/events.rs` 等、本変更以前から）あり。
  新規 slice は隣接 DTO enum と同じ multi-line variant スタイルに合わせた

## 既知の制約 {#known-issues}

- **IME 制約**: wdio から日本語（`"hello 編集中"`）を入力すると IME composition を再現できず
  不安定なため、ローカル編集は ASCII `"hello local"` で表現する。競合/削除判定の意味は同一
- **AutoSave の順序**: validation Given は「AutoSave 未発火」だが、pending write が外部削除を
  race するのを避けるため、タイプ後に AutoSave の永続化を待ってから外部削除を起こす
- **debounce 窓と外部削除**: watcher は同一パスの 500ms 以内の後続イベントを破棄する。
  Save as new file の自書き込み（Created）直後に同一パスを削除すると窓内で破棄されるため、
  E2E は削除前に `browser.pause(700)` を置く（s18 step 3 と同方針）
- **window blur flakiness**: headless WebKitGTK の focus 管理で EDITING → FOCUSED に落ちる
  ことがあるため、通知を必要とする直前は `ensureEditing()` で EDITING を保証する
- **ダイアログ操作は DOM click**: modal `<dialog>` は browser top layer にあり、本 headless
  WebKitGTK では WebDriver のネイティブ pointer click が flaky なため、`delete-notice-*` は
  `browser.execute` の DOM click で操作する。外部削除トリガー自体は実ファイル unlink（native 経路）
- **通知表示中も watcher は稼働継続**: 重複通知は store が同一 note_id で debounce する
- **Esc は無効化**: S20 は明示的な 2 択のみのため、`oncancel` で `preventDefault` し
  DOM だけ閉じて状態が取り残されるのを防ぐ
- **テストは Gherkin ステップ順に依存**（step 1 表示 → step 2 内容 → step 3 Save as new →
  step 4 Discard → step 5 非編集時）。`maxInstances: 1` + mocha 逐次実行が前提
  （s1〜s19 と同方針。`.claude/rules/scenario-test.md`「テスト間独立」からの既知の逸脱）
