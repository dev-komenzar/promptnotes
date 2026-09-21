---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s17-external-file-modified-no-conflict
      hash: 4a02c17cc025
    - path: domain/workflows/detect-external-changes.md#detect-external-changes
      hash: a66910b0d892
    - path: domain/domain-events.md#external-file-change-events
      hash: 71db66eafe03
    - path: domain/domain-events.md#note-file-modified-externally
      hash: 71db66eafe03
    - path: domain/aggregates.md#note-feed-aggregate
      hash: 56f7a54a8ab2
    - path: domain/aggregates.md#note-aggregate
      hash: 56f7a54a8ab2
    - path: domain/glossary.md#glossary-timestamp
      hash: 9405b16bb835
---

# s17-external-file-modified-no-conflict — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s17-external-file-modified-no-conflict phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

外部プログラム（Syncthing / vim / VSCode 等）が `storage_dir/<A.id>.md` の **body を変更**したとき、
対応する Note A のブロックが **IDLE 状態（編集中でない）** であれば、アプリを再起動したり
手動 Refresh したりしなくても **NoteFeed の表示 body が自動的に更新される**（`"hello"` →
`"hello world"`）ことを E2E で検証する scenario。

経路は `domain/workflows/detect-external-changes.md#detect-external-changes` の
OS ファイルウォッチャー → 500ms debounce → `NoteRepository::load_by_id` での再 parse +
`BodyHash` 計算 → domain event `NoteFileModifiedExternally` 発行 → 購読者 NoteFeed の
`upsert_note`（I-F8）→ Tauri event `notes-changed` → `PageMain` の再 hydrate、という pipeline。
Note A は IDLE のため **競合ダイアログは出ない**（S19 の EDITING ケースは範囲外）。
UI 通知も不要（フィードの自然な更新で十分）。

- 対応 workflow: `domain/workflows/detect-external-changes.md#detect-external-changes`
  （step 1 `resolveStorageDir` / step 2 `startWatcher` + debounce 500ms / step 4 `onFileModified` /
  step 6 `emitEvent`）
- 対応 event: `domain/domain-events.md#note-file-modified-externally`
  （payload `{ note_id, disk_body_hash, note, file_path, detected_at }`。Subscribers に
  NoteFeed `upsert_note` と Note Capture の競合検出。後者は IDLE のため発火しない）
- 対応 aggregate (read side): `domain/aggregates.md#note-feed-aggregate`
  （I-F8: 外部ファイル変更の差分更新を `upsert_note` で受け付ける。既存 Note は差し替え）
- 対応 aggregate (source): `domain/aggregates.md#note-aggregate`
  （parse 経路で再構築される Note。`body_hash` は body から決定論的に導出 — I-N9）
- 対応語彙: `domain/glossary.md#glossary-timestamp`
- 対応 page: `page-main`（`start_file_watcher` の起動元 + `notes-changed` 購読元の composition root）

> domain/validation.md#s17-external-file-modified-no-conflict より:

- Given:
  - Note A (`body="hello"`, `updatedAt=t0`) が表示されている
  - Note A のブロックは **IDLE 状態**（編集中ではない）
  - ファイルウォッチャー稼働中
- When:
  1. `t1` に外部プログラム（Syncthing 経由等）が `storage_dir/<A.id>.md` の body を `"hello world"` に変更
  2. ファイルウォッチャーが変更を検知（debounce 500ms 後、`t1 + 0.5s = t2`）
  3. infrastructure 層が `.md` を再 parse、`BodyHash` を計算
  4. event **NoteFileModifiedExternally**
     `{ note_id: A.id, disk_body_hash, note, file_path, detected_at: t2 }` 発行
- Then:
  - フロントエンドが event 受信 → Block A の状態を確認 → **IDLE** のため競合なし
  - NoteFeed: `upsert_note(note)` で source 内の Note A を差し替え（I-F8）
  - フィード表示が `"hello world"` に更新される
  - `updatedAt` ソート時、表示順が変わる可能性がある
  - event **NoteBodyEdited** は発行されない（これは外部変更であり、アプリ内編集ではない）

> domain/workflows/detect-external-changes.md#steps より:
> 「step 4 `onFileModified`: `.md` を再 parse し、ディスクから読んだ body から `BodyHash` を計算。
> parse 成功 → `NoteFileModifiedExternally { note_id, disk_body_hash, note, file_path, detected_at }`
> を返す。parse 失敗 → `None`（skip）」

> domain/domain-events.md#note-file-modified-externally-subscribers より:
> 「**Note Feed**: `upsert_note(note)` で source 内の該当 Note を差し替え（I-F8） /
> **Note Capture (application service)**: 当該 `note_id` が現在編集中の場合、
> `Note::is_stale(&disk_body_hash)` で競合を判定し、ユーザに選択肢を提示する」

> domain/aggregates.md#note-feed-aggregate-invariants より:
> 「**I-F8**: NoteFeed は外部ファイル変更の検知を契機とした差分更新を受け付ける。
> `upsert_note` 操作により、変更された `.md` ファイルに対応する Note のみを部分更新できる。
> ファイル削除 / 新規作成も upsert または後続の remove で反映する。
> 検知機構そのもの（OS レベルのファイルウォッチャー）は infrastructure 層の責務」

## シナリオステップ {#scenario-steps}

### Given {#given}

1. 実 user 環境から隔離した temp dir を `storage_dir` として Tauri App（test build, wdio）を起動。
   `storage_dir` には Note A (`id=20260620120000`, `createdAt=updatedAt=20260620120000`,
   `tags=[]`, `body="hello"`) のみを seed 済み
2. `PageMain` の mount effect が `start_file_watcher` を invoke し、`storage_dir` の
   OS ファイルウォッチャー（`notify`, non-recursive, `.md` のみ）が稼働している
3. UI は起動時 `list_notes` で hydrate 済み → フィードは Note A の 1 件、body は `"hello"`、
   Block A は IDLE 状態（`data-block-state="IDLE"`）

### When {#when}

1. テストプロセス（= 外部プログラムの代役）が `node:fs` で既存の
   `storage_dir/20260620120000.md` を **上書き変更**する
   （frontmatter は `createdAt`/`updatedAt`/`tags` を維持し、body のみ `"hello world"` に変更）
2. ファイルウォッチャーが変更を検知（debounce 500ms 後）
3. infrastructure 層が `.md` を再 parse し Note を再構築（成功）、`BodyHash` を計算
4. event **NoteFileModifiedExternally** が発行される

### Then {#then}

- 購読者（`start_file_watcher` が wire した subscriber）が
  `InMemoryNoteFeedState::upsert_one` → Tauri event `notes-changed` emit
- `PageMain` の `notes-changed` listener が `list_notes` を再取得 → `feedStore.hydrateNotes`
- **手動 Refresh / 再起動なしで** 既存 Block A の表示 body が `"hello"` → `"hello world"` に自動更新される
- NoteFeed は `upsert_note` で **差し替え**（新規追加ではない）: Block 数は 1 のまま
- Block A は IDLE のため **競合ダイアログは表示されない**（S19 の EDITING ケースは範囲外）
- UI 通知（トースト）は出ない
- event **NoteBodyEdited** は発行されない（アプリ内編集ではないため）
- 補足: malformed frontmatter を持つ `.md` は parse 失敗で event 発行されず skip される

## テスト観点 {#test-points}

- **TP1（核・本 scenario の主目的）— 外部変更が手動操作なしで UI に反映される**:
  Note A の body `"hello"` が表示されている状態で、テストプロセスが
  `storage_dir/20260620120000.md` を外部上書きし body を `"hello world"` にする。
  `list_notes` の手動 invoke / 再起動を一切行わずに、既存 Block A の CodeMirror body
  （`[data-block-id="20260620120000"] .cm-content`）の text が `"hello world"` になるのを待つ。
  これが watcher 検知 → 再 parse → `NoteFileModifiedExternally` → `notes-changed` → 再 hydrate の
  縦断 pipeline が動いていることの間接観測になる（UI/FS 状態で検証）
- **TP2（核）— 再 parse された Note の内容が正しい**:
  反映後の Note A を `list_notes` の read DTO で確認し、
  `body="hello world"` / `tags=[]` / `created_at=updated_at=2026-06-20T12:00:00Z`
  であることを assert する。`updatedAt` は Given の `t0` のまま（When は body のみ変更）
- **TP3（差し替え / 非空虚性）— 連続する外部変更もそれぞれ反映され、Block は増えない**:
  さらに body を `"hello world 2"` に外部上書きし、やはり手動操作なしで DOM body が更新され、
  かつ Block 数が 1 のままであることを assert する。`upsert_note` が append ではなく
  **既存 Note の差し替え**（I-F8）として機能し、pipeline が継続的に動くことを示す
- **TP4（競合なし / IDLE）— 競合ダイアログもトーストも出ない**:
  変更反映後、Block A が IDLE 状態（`data-block-state="IDLE"`）で、競合ダイアログ
  （`[data-testid="widget-external-change-conflict"]`）とトースト（`screen-1-toast`）が
  いずれも非存在であることを assert する。S19 の EDITING 競合挙動は範囲外であることを示す
- **E2E 観測の制約**: domain event `NoteFileModifiedExternally` 自体は frontend に露出しない
  （購読者が NoteFeed 更新 + `notes-changed` Tauri event を emit するのみ）。`disk_body_hash` /
  `NoteBodyEdited` 未発行も直接 assert できない。そのため event payload の直接 assert は行わず、
  UI/FS 状態（DOM body 更新 + read DTO + Block 数不変 + ダイアログ非表示）で間接検証する
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
  （`storage::resolve_storage_dir` が `storage_dir_override()` を最優先で解決する）。
  Note A を seed した後 app を起動するため、watcher は隔離 dir を監視する
- **外部変更の再現**: テストプロセスから `node:fs` の `writeFileSync` で
  既存 `TAURI_TEST_STORAGE_DIR/20260620120000.md` を上書きする（＝外部プログラムの代役）。
  既存ファイルへの truncate + write は Linux inotify で `EventKind::Modify` として通知され、
  watcher 側で `RawFileEvent::Modified` に分類される
- **UI 観測点**: `Block.svelte` が `data-block-id={note.id}` を付与し、body は CodeMirror の
  `[data-block-id="<id>"] .cm-content` に描画される。`feedStore.hydrateNotes` が同一 id の
  Note を差し替えると、keyed `{#each (note.id)}` により同じ `Block` インスタンスの `note` prop が
  更新され、`Block.svelte` の `$effect`（`note.body` → CM doc 同期）で `.cm-content` の text が
  書き換わる。テストはこの text を `waitUntil` で待つ。手動 `list_notes` invoke を挟まないことが
  「watcher 経由の自動反映」の判定条件
- **event 観測の制約（重要）**: `start_file_watcher` は内部で `AppEventBus` を生成して
  `DomainEvent::NoteFileModifiedExternally` を publish する。subscriber は
  `InMemoryNoteFeedState::upsert_one` と Tauri event `notes-changed` の emit のみを行い、
  domain event 自体を frontend に露出しない。また `list_notes` は毎回 disk から
  `FsNoteRepository::list_all` で再 hydrate するため、「`list_notes` の結果に変更後 body が
  含まれる」ことは watcher の動作証明にはならない（disk 直読みでも同じ結果になる）。
  したがって **watcher pipeline の証明は「手動操作なしで UI（DOM）が自動更新されること」**
  に置く。これは `notes-changed` が当該 domain event の subscriber からのみ emit される
  という構造に依拠する
- **競合ダイアログの観測制約**: `WidgetExternalChangeConflict` の `defaultSubscribeFn` は
  no-op（`store.svelte.ts` OQ-WC1: Real event bridge (Rust → TS) is not yet wired）。
  そのため IDLE では当然ダイアログは出ないが、EDITING でも S19 の bridge 未実装のため
  E2E では区別できない。TP4 は「IDLE で競合 UI が出ない」ことの弱い確認であり、
  S19 の競合検出は slice / widget unit の領分（notes.md#known-issues に明記）
- **`NoteBodyEdited` 非発行**: 外部変更はアプリ内編集ではないため `NoteBodyEdited` は
  発行されない（Then）。event の直接観測はできないが、`applyAutoSave` 経由の
  `updated_at` 更新が起きないこと（TP2 で `updated_at` が `t0` のまま）で間接的に確認する
- **production 側の前提（本 scenario が満たすべき不変条件）**:
  1. `PageMain` の mount effect が `start_file_watcher` を invoke すること
     — `src/ui-page/page-main/PageMain.svelte`
  2. `start_file_watcher` が `storage_dir` を `resolve_storage_dir`（= `TAURI_TEST_STORAGE_DIR`
     override 尊重）で解決し、`notify` watcher を non-recursive / `.md` のみで起動し、
     `WatcherHandle` を managed state に保持すること
     — `src-tauri/src/note_feed/slices/detect_external_changes/commands.rs`
  3. `FsWatcher::run_event_loop` が 500ms debounce を適用し、`Modify` を
     `RawFileEvent::Modified` として渡すこと — `.../infrastructure.rs`
  4. `DetectExternalChangesUseCase::start_watcher` が `Modified` で `load_by_id` →
     `note.body_hash()` を計算し `NoteFileModifiedExternally` を publish すること
     — `.../application.rs`
  5. subscriber が `NoteFileModifiedExternally` で `upsert_one` + `notes-changed` emit を
     行うこと — `commands.rs`
  6. `PageMain` が `notes-changed` を listen し `listNotesFn` → `feedStore.hydrateNotes` する
     こと — `PageMain.svelte`
  これらが揃っていれば production 変更なしで GREEN になる見込み。欠落があれば最小実装を追加する