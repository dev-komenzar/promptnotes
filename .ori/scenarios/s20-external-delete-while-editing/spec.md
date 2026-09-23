---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s20-external-delete-while-editing
      hash: 4a02c17cc025
    - path: domain/workflows/detect-external-changes.md#detect-external-changes
      hash: a66910b0d892
    - path: domain/domain-events.md#external-file-change-events
      hash: 71db66eafe03
    - path: domain/domain-events.md#note-file-deleted-externally
      hash: 71db66eafe03
    - path: domain/aggregates.md#note-feed-aggregate
      hash: 56f7a54a8ab2
    - path: domain/aggregates.md#note-aggregate
      hash: 56f7a54a8ab2
    - path: domain/ui-fields/screen-1.md#screen-1
      hash: 4bc0f83f71f3
    - path: domain/ui-fields/page-groups.md#widget-external-change-conflict
      hash: 8986615249ac
    - path: domain/glossary.md#glossary-timestamp
      hash: 9405b16bb835
---

# s20-external-delete-while-editing — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s20-external-delete-while-editing phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

外部プログラム（Syncthing / vim / VSCode 等）が `storage_dir/<A.id>.md` を **削除**したとき、
Note A の Block が **EDITING 状態（ユーザが編集中）** である場合に、フロントエンドが
**削除通知**を表示し、ユーザに「新規ファイルとして保存」（現在の編集中内容で `.md` を再作成）か
「破棄」（編集を諦めて `remove_note`）の選択肢を提示することを E2E で検証する scenario。

経路は `domain/workflows/detect-external-changes.md#detect-external-changes` の
OS ファイルウォッチャー → 500ms debounce → ファイル名（basename、拡張子除く）から `NoteId` 解決
（`^\d{14}$`）→ domain event `NoteFileDeletedExternally` 発行 → 購読者 NoteFeed の
`remove_note`（I-F8）→ Tauri event `notes-changed` → frontend の EDITING 検出
→ 削除通知、という pipeline。S18（IDLE での外部削除 → Feed 自動更新のみ）とは対照的に、
本 scenario は EDITING 中の削除を扱う（validation.md#notes-external-change-conflict）。

- 対応 workflow: `domain/workflows/detect-external-changes.md#detect-external-changes`
  （step 1 `resolveStorageDir` / step 2 `startWatcher` + debounce 500ms / step 5 `onFileDeleted` /
  step 6 `emitEvent`）
- 対応 event: `domain/domain-events.md#note-file-deleted-externally`
  （payload `{ note_id, file_path, detected_at }`。Subscribers は NoteFeed `remove_note` と
  「UI 層: 通知不要（フィードの自然な更新で十分）。**編集中に削除された場合は application service が
  別途判断**」。本 scenario は後者の EDITING ケース）
- 対応 aggregate (read side): `domain/aggregates.md#note-feed-aggregate`
  （I-F8: 外部ファイル削除を `remove_note` で受け付ける。該当 `note_id` のみ除外）
- 対応 aggregate (source): `domain/aggregates.md#note-aggregate`
  （削除 event は Note を再構築しない。`NoteId` はファイル名から解決）
- 対応 UI: `domain/ui-fields/screen-1.md#screen-1`（Block の `IDLE | FOCUSED | EDITING`
  ステートマシン / 削除トーストスタック）
- 対応 page: `page-main`（`start_file_watcher` の起動元 + `notes-changed` 購読元）
- 対応語彙: `domain/glossary.md#glossary-timestamp`

> domain/validation.md#s20-external-delete-while-editing より:

- Given:
  - Note A をユーザが **EDITING 状態** で編集中
  - ユーザは body を `"hello 編集中"` に変更済み
- When:
  1. `t0` に外部プログラムが `storage_dir/<A.id>.md` を削除
  2. ファイルウォッチャーが削除を検知 → **NoteFileDeletedExternally** 発行
     `{ note_id: A.id, ... }`
  3. フロントエンドが event 受信 → Block A の状態を確認 → **EDITING** を検出
- Then:
  - フロントエンドが通知を表示:
    - 「編集中のノートが外部で削除されました」
    - 選択肢: 「新規ファイルとして保存」（現在の内容で `.md` を再作成）、「破棄」（編集を諦める）
  - ユーザが「新規ファイルとして保存」を選択:
    - 現在の編集中内容で `storage_dir/<A.id>.md` を再作成
    - NoteFeed: `upsert_note(note)` 実行
    - Block は IDLE 状態に遷移
  - ユーザが「破棄」を選択:
    - NoteFeed: `remove_note(&A.id)` 実行
    - 編集中の内容は破棄
  - NoteFeed の `remove_note` は S18 と同様に発動するが、
    EDITING 検出がある場合は上記の通知が優先される

> domain/domain-events.md#note-file-deleted-externally-subscribers より:
> 「**Note Feed**: `remove_note(&note_id)` で source から除外（I-F8） / **UI 層**: 通知不要
> （フィードの自然な更新で十分）。編集中に削除された場合は application service が別途判断」

> domain/workflows/detect-external-changes.md#steps より:
> 「step 5 `onFileDeleted`: ファイル名（basename、拡張子除く）から `NoteId` を解決（`^\d{14}$` に一致するか）。
> 解決成功 → `NoteFileDeletedExternally { note_id, file_path, detected_at }` を返す。
> 解決失敗（非 Note ファイル名）→ `None`（skip）」

> domain/aggregates.md#note-feed-aggregate-invariants より:
> 「**I-F8**: NoteFeed は外部ファイル変更の検知を契機とした差分更新を受け付ける。
> `upsert_note` 操作により、変更された `.md` ファイルに対応する Note のみを部分更新できる。
> ファイル削除 / 新規作成も upsert または後続の remove で反映する」

> domain/ui-fields/page-groups.md#widget-external-change-conflict より:
> 「**注意**: `NoteFileCreatedExternally` / `NoteFileDeletedExternally` では mount されない
> （Feed 自動更新で十分）」

## シナリオステップ {#scenario-steps}

### Given {#given}

1. 実 user 環境から隔離した temp dir を `storage_dir` として Tauri App（test build, wdio）を起動。
   `storage_dir` には Note A (`id=20260620120000`, `createdAt=updatedAt=20260620120000`,
   `tags=[]`, `body="hello"`) のみを seed 済み
2. `PageMain` の mount effect が `start_file_watcher` を invoke し、`storage_dir` の
   OS ファイルウォッチャー（`notify`, non-recursive, `.md` のみ）が稼働している
3. UI は起動時 `list_notes` で hydrate 済み → フィードは Note A の 1 件、body は `"hello"`、
   Block A は IDLE 状態（`data-block-state="IDLE"`）
4. ユーザが Block A をクリックして **EDITING 状態** に遷移し、body を編集する
   （テストでは ASCII で追記する — [#impl-notes](#impl-notes) の IME 制約を参照）。
   編集後の body を `"hello local"` とし、AutoSave debounce を待って
   `storage_dir/<A.id>.md` にローカル編集が永続化された状態を準備する
   （＝ この時点でディスクとローカルは同一 body。以降の外部削除が単一起点になる）

### When {#when}

1. テストプロセス（= 外部プログラムの代役）が `node:fs` の `unlinkSync` で既存の
   `storage_dir/20260620120000.md` を **削除**する
2. ファイルウォッチャーが削除を検知（debounce 500ms 後）
3. infrastructure 層がファイル名から `NoteId` を解決（`^\d{14}$` に一致）
4. event **NoteFileDeletedExternally** が発行される
5. frontend は `notes-changed` 受信後に `list_notes` を再取得し、Block A が
   **EDITING** のままディスク上から消えていることを検出する

### Then {#then}

- フロントエンドが**削除通知**を表示:
  - 「編集中のノートが外部で削除されました」
  - 現在の編集中 body（`"hello local"`）を提示
  - 選択肢: 「新規ファイルとして保存」（Save as new file）/「破棄」（Discard）
- Block A は通知中も EDITING 状態を維持する（削除検出時にローカル body を保持し、
  黙って Block を消さない）
- ユーザが **Save as new file** を選択 →
  現在の編集中内容で `storage_dir/<A.id>.md` を再作成し、フィードは Note A を保持、
  Block A は **IDLE** に遷移する
- ユーザが **Discard** を選択 →
  フィードから Block A が除去され、編集中の内容は破棄される
- NoteFeed の `remove_note` は S18 と同様に発動するが、EDITING 検出がある場合は
  上記の通知が優先される
- 補足: 通知表示中もファイルウォッチャーは稼働継続する

## テスト観点 {#test-points}

- **TP1（核・本 scenario の主目的）— EDITING 中の外部削除で削除通知が表示される**:
  Block A を EDITING にし `"hello local"` に編集（AutoSave 済み）した後、テストプロセスが
  `storage_dir/20260620120000.md` を外部削除する。`list_notes` の手動 invoke / 再起動を一切行わず、
  `[data-testid="widget-external-delete-notice"]` が表示されるのを待つ。Block A は
  `data-block-state="EDITING"` のまま（黙って消えない）。これが watcher 検知 →
  `NoteFileDeletedExternally` → frontend の EDITING 検出 → 削除通知、の縦断 pipeline が
  動いていることの間接観測になる
- **TP2（核）— 通知が現在の編集中 body を提示する**:
  表示された通知の `delete-notice-title` が `20260620120000.md`、
  `delete-notice-body-local` の value が `"hello local"` であることを assert する
- **TP3（解決・Discard）— 破棄でフィードから除去される**:
  再度 EDITING に入れて外部削除し通知を表示させ、`delete-notice-discard` をクリックすると、
  通知が非表示になり、フィードから Block A が消える（`remove_note` / I-F8）ことを assert する
- **TP4（解決・Save as new file）— 再作成で IDLE へ**:
  再度 EDITING に入れて外部削除し通知を表示させ、`delete-notice-save-as-new` をクリックすると、
  通知が非表示になり、`storage_dir/20260620120000.md` が再作成され（FS 観測）、
  フィードに Block A が残り、Block A が IDLE に遷移することを assert する
- **TP5（非編集時は通知なし）— IDLE の外部削除では通知が出ない**:
  Block A が IDLE の状態で外部削除を起こし、削除通知が表示されないままフィードから
  Block A が消えることを assert する（S18 との境界）
- **E2E 観測の制約**: domain event `NoteFileDeletedExternally` 自体は frontend に露出しない
  （購読者が NoteFeed `remove_one` + payload なし Tauri event `notes-changed` を emit するのみ）。
  そのため event payload の直接 assert は行わず、UI 状態（通知表示/非表示 + body 表示 +
  Block ステート）と FS 状態（`.md` の再作成）で間接検証する
  （詳細は [#impl-notes](#impl-notes) と notes.md）

## 実装ノート {#impl-notes}

- **runner**: `wdio`（優先チェーン step 2 — manifest に `runner:` 明示なし。参加 app
  `promptnotes` は `local` 系で `runtime.runner = wdio`。`.ori/architecture.md` の
  `scenario_test_runner: wdio` とも一致）
- **infrastructure**: `promptnotes` のみ（`mode: local`）。compose-service 系 app / infra が
  参加しないため `docker-compose.yml` は生成しない
- **runner config**: `wdio.conf.ts`。`tauri:options.application` は runtime block の `binary`
  = `apps/promptnotes/src-tauri/target/debug/app`（build-then-test。
  `VITE_WDIO_TEST=1 bun run tauri build --debug --no-bundle` = `bun run build:test`）
- **storage_dir の隔離**: wdio `onPrepare` が `XDG_CONFIG_HOME` / `XDG_DATA_HOME` と
  専用 `storage_dir` を temp dir に隔離し、`TAURI_TEST_STORAGE_DIR` で app に伝える
  （`storage::resolve_storage_dir` が `storage_dir_override()` を最優先で解決する）
- **外部削除の再現**: テストプロセスから `node:fs` の `unlinkSync` で
  既存 `TAURI_TEST_STORAGE_DIR/20260620120000.md` を削除する（= 外部プログラムの代役）。
  Linux inotify では `IN_DELETE` が `EventKind::Remove` → `RawFileEvent::Deleted` に分類され、
  s18 と同一経路
- **UI 観測点**:
  - Block ステート: `Block.svelte` が `data-block-state={blockState}`（`IDLE|FOCUSED|EDITING`）
  - 編集中 body: `[data-block-id="<id>"] .cm-content`（CodeMirror）
  - 削除通知: `[data-testid="widget-external-delete-notice"]`（`<dialog>`、`showModal()`）。
    body 表示は `delete-notice-body-local`、note title は `delete-notice-title`、
    解決ボタンは `delete-notice-save-as-new` / `delete-notice-discard`
  - textarea の value は `getValue()` で読む
- **event 観測の制約（重要）**: `start_file_watcher` は内部で `AppEventBus`（in-process）を生成し
  `DomainEvent::NoteFileDeletedExternally` を publish する。subscriber は
  `InMemoryNoteFeedState::remove_one` と Tauri event `notes-changed`（payload なし）の emit のみを
  行い、domain event payload を frontend に露出しない。したがって E2E は event payload を
  直接 assert せず、UI/FS 状態で間接検証する
- **frontend 最小配線（本 scenario が満たすべき不変条件）**:
  1. `PageMain` の `notes-changed` listener が、EDITING 中の note がディスクから消えている場合に
     削除通知 payload を widget へ届け、**ローカル body / Block を保持する**こと
     — 現状は無条件 `hydrateNotes(feed.notes)` で Block が黙って消える（= 本 scenario の RED）
  2. `widget-external-delete-notice` が EDITING 中の note の外部削除でのみ mount されること
     （IDLE では S18 の Feed 自動更新のみ。page-groups の「conflict widget は deleted で
     mount されない」とは別 widget）
  3. **Discard** で `feedStore.applyDelete(noteId)` + `focusStore.clear()` を実行すること
  4. **Save as new file** で現在の編集中内容を `storage_dir/<A.id>.md` に再作成し、
     フィードを保持したまま Block を IDLE へ遷移させること
  5. Rust 側は既存（s18 で実証済み）: `EventKind::Remove` → `RawFileEvent::Deleted` →
     `NoteFileDeletedExternally` publish → `remove_one` + `notes-changed` emit
- **IME / キー入力の制約**: wdio からの日本語入力（`"hello 編集中"`）は IME composition を
  再現できず不安定なため、テストでは ASCII（`"hello local"`）でローカル編集を表現する
  （notes.md#known-issues に明記）
- **AutoSave debounce と削除の順序**: validation Given は「AutoSave 未発火」だが、E2E では
  ローカル編集の pending write が外部削除を race するのを避けるため、タイプ後に AutoSave の
  永続化を待ってから外部削除を起こす（notes.md#known-issues に明記）
- **ドメイン文書ギャップ**: `page-groups.md#widget-external-change-conflict` /
  `screen-4.md#notes-lifecycle` は `NoteFileDeletedExternally` で conflict widget を mount しない
  と規定するが、EDITING 中の外部削除に対する通知 UI（本 scenario の `widget-external-delete-notice`）
  は Phase 11a で設計されていない。本 scenario は validation S20 を優先し、新 widget として
  最小実装する。ui-fields への反映は `/ori-propose` 対象（notes.md#known-issues に明記）
