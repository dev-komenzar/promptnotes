---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s18-external-file-deleted
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
    - path: domain/glossary.md#glossary-timestamp
      hash: 9405b16bb835
---

# s18-external-file-deleted — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s18-external-file-deleted phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

外部プログラム（Syncthing / vim / VSCode 等）が `storage_dir/<A.id>.md` を **削除**したとき、
アプリを再起動したり手動 Refresh したりしなくても **NoteFeed から当該 Block が自動的に除去される**
ことを E2E で検証する scenario。

経路は `domain/workflows/detect-external-changes.md#detect-external-changes` の
OS ファイルウォッチャー → 500ms debounce → ファイル名（basename、拡張子除く）から `NoteId` 解決
（`^\d{14}$`）→ domain event `NoteFileDeletedExternally` 発行 → 購読者 NoteFeed の
`remove_note`（I-F8）、という pipeline。UI 通知（トースト / モーダル）は不要（フィードの自然な更新で十分）。
Note A が EDITING の場合は S20 の挙動（別 scenario）であり、本 scenario は IDLE 前提。

- 対応 workflow: `domain/workflows/detect-external-changes.md#detect-external-changes`
  （step 1 `resolveStorageDir` / step 2 `startWatcher` + debounce 500ms / step 5 `onFileDeleted` /
  step 6 `emitEvent`）
- 対応 event: `domain/domain-events.md#note-file-deleted-externally`
  （payload `{ note_id, file_path, detected_at }`。Subscribers に NoteFeed `remove_note` と
  「UI 層: 通知不要」）
- 対応 aggregate (read side): `domain/aggregates.md#note-feed-aggregate`
  （I-F8: 外部ファイル削除を `remove_note` で受け付ける。該当 `note_id` のみ除外）
- 対応 aggregate (source): `domain/aggregates.md#note-aggregate`
  （削除 event は Note を再構築しない。`NoteId` はファイル名から解決）
- 対応語彙: `domain/glossary.md#glossary-timestamp`
- 対応 page: `page-main`（`start_file_watcher` の起動元 + `notes-changed` 購読元の composition root）

> domain/validation.md#s18-external-file-deleted より:

- Given:
  - Note A が表示されている（IDLE 状態）
  - ファイルウォッチャー稼働中
- When:
  1. `t0` に外部プログラムが `storage_dir/<A.id>.md` を削除
  2. ファイルウォッチャーが削除を検知（debounce 500ms 後、`t0 + 0.5s = t1`）
  3. infrastructure 層がファイル名から `NoteId` を解決（`^\d{14}$` に一致）
  4. event **NoteFileDeletedExternally** `{ note_id: A.id, file_path, detected_at: t1 }` 発行
- Then:
  - NoteFeed: `remove_note(&A.id)` で source から除外（I-F8）
  - フィード表示から Note A が消える
  - UI 通知は不要（自然な更新）
  - Note A が EDITING 状態だった場合は S20 の挙動に従う
  - 補足: ファイル名が `^\d{14}$` に一致しない場合（非 Note ファイル）は event 非発行

> domain/workflows/detect-external-changes.md#steps より:
> 「step 5 `onFileDeleted`: ファイル名（basename、拡張子除く）から `NoteId` を解決（`^\d{14}$` に一致するか）。
> 解決成功 → `NoteFileDeletedExternally { note_id, file_path, detected_at }` を返す。
> 解決失敗（非 Note ファイル名）→ `None`（skip）」

> domain/domain-events.md#note-file-deleted-externally-subscribers より:
> 「**Note Feed**: `remove_note(&note_id)` で source から除外（I-F8） / **UI 層**: 通知不要
> （フィードの自然な更新で十分）。編集中に削除された場合は application service が別途判断」

> domain/aggregates.md#note-feed-aggregate-invariants より:
> 「**I-F8**: NoteFeed は外部ファイル変更の検知を契機とした差分更新を受け付ける。
> `upsert_note` 操作により、変更された `.md` ファイルに対応する Note のみを部分更新できる。
> ファイル削除 / 新規作成も upsert または後続の remove で反映する。
> 検知機構そのもの（OS レベルのファイルウォッチャー）は infrastructure 層の責務」

## シナリオステップ {#scenario-steps}

### Given {#given}

1. 実 user 環境から隔離した temp dir を `storage_dir` として Tauri App（test build, wdio）を起動。
   `storage_dir` には Note A (`id=20260620120000`, `createdAt=updatedAt=20260620120000`,
   `tags=[]`, `body="hello"`) と Note B (`id=20260621090000`, `createdAt=updatedAt=20260621090000`,
   `tags=[]`, `body="world"`) を seed 済み。validation Given は「Note A が表示されている」であり
   「のみ」とは規定しないため、`remove_note(&A.id)` が A だけを除外し B を残すこと（＝
   blanket clear ではないこと）を観測できるよう 2 件を seed する
2. `PageMain` の mount effect が `start_file_watcher` を invoke し、`storage_dir` の
   OS ファイルウォッチャー（`notify`, non-recursive, `.md` のみ）が稼働している
3. UI は起動時 `list_notes` で hydrate 済み → フィードは既定 sort（`createdAt desc`）で
   `[B, A]` の 2 件、両 Block は IDLE 状態（`data-block-state="IDLE"`）

### When {#when}

1. テストプロセス（= 外部プログラムの代役）が `node:fs` で既存の
   `storage_dir/20260620120000.md` を **削除**する（`unlinkSync`）
2. ファイルウォッチャーが削除を検知（Linux inotify では `EventKind::Remove`
   → `RawFileEvent::Deleted`。debounce 500ms 窓で同一パスの連続イベントを集約）
3. infrastructure 層がファイル名（basename `20260620120000`）から `NoteId` を解決（成功）
4. event **NoteFileDeletedExternally** が発行される

### Then {#then}

- 購読者（`start_file_watcher` が wire した subscriber）が
  `InMemoryNoteFeedState::remove_one` → Tauri event `notes-changed` emit
- `PageMain` の `notes-changed` listener が `list_notes` を再取得 → `feedStore.hydrateNotes`
- **手動 Refresh / 再起動なしで** フィードから Block A が消え、Block B は残る（`[B, A]` → `[B]`）
- NoteFeed は `remove_note(&A.id)` で **該当 `note_id` のみ** 除外（I-F8。blanket clear ではない）
- Block A が IDLE のため **競合ダイアログは表示されない**（S20 の EDITING ケースは範囲外）
- UI 通知（トースト）は出ない
- 補足: ファイル名が `^\d{14}$` に一致しない `.md`（非 Note ファイル）の削除は event 非発行で skip される

## テスト観点 {#test-points}

- **TP1（核・本 scenario の主目的）— 外部削除が手動操作なしで UI に反映される**:
  フィードに Note A / Note B が表示されている状態で、テストプロセスが
  `storage_dir/20260620120000.md` を外部削除する。`list_notes` の手動 invoke / 再起動を
  一切行わずに、Block A (`[data-block-id="20260620120000"]`) が DOM から消え、
  Block B (`[data-block-id="20260621090000"]`) が残るのを待つ。これが watcher 検知 →
  `NoteFileDeletedExternally` → 購読者 `remove_one` + `notes-changed` → 再 hydrate の
  縦断 pipeline が動いていることの間接観測になる（UI/FS 状態で検証）
- **TP2（核）— 除去後の read DTO が正しい**:
  反映後の NoteFeed を `list_notes` の read DTO で確認し、Note A が含まれず、
  Note B のみ残り `body="world"` / `tags=[]` / `created_at=updated_at=2026-06-21T09:00:00Z`
  であることを assert する
- **TP3（差し替え / 追跡・非空虚性）— 連続する外部削除もそれぞれ反映される**:
  さらに Note B も外部削除し、やはり手動操作なしで Block B が消えてフィードが空
  （`[data-testid="screen-1-feed-empty"]` 表示）になることを assert する。`remove_note` が
  該当 id のみを除外しつつ pipeline が継続的に動くことを示す
- **TP4（非 Note ファイル名 skip / 弱い観測）— `^\d{14}$` に一致しない `.md` の削除は無視される**:
  `storage_dir/notanote.md` を外部作成 → 削除し、フィードが空のまま変化しないことを assert する。
  `resolve_note_id` が `None` を返すため event 非発行で skip される
  （validation#s18-then 補足）。ただしこの観測は弱い: 非 Note 名ファイルは `list_all` 側でも
  そもそも feed に現れないため、「UI が変化しない」だけでは watcher 側 skip を単独では
  識別しない（notes.md#known-issues に明記）
- **E2E 観測の制約**: domain event `NoteFileDeletedExternally` 自体は frontend に露出しない
  （購読者が NoteFeed `remove_one` + `notes-changed` Tauri event を emit するのみ）。そのため
  event payload の直接 assert は行わず、UI/FS 状態（DOM Block 消滅 + read DTO + Block 数）で
  間接検証する（詳細は [#impl-notes](#impl-notes) と notes.md）

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
  Note A / Note B を seed した後 app を起動するため、watcher は隔離 dir を監視する
- **外部削除の再現**: テストプロセスから `node:fs` の `unlinkSync` で
  既存 `TAURI_TEST_STORAGE_DIR/20260620120000.md` を削除する（＝外部プログラムの代役）。
  Linux inotify では `IN_DELETE` が `EventKind::Remove` として通知され、watcher 側で
  `RawFileEvent::Deleted` に分類される。`.md` 以外 / `.tmp` は watcher 側で ignore される
- **UI 観測点**: `Block.svelte` が `data-block-id={note.id}` を付与する。テストは
  `[data-block-id="<id>"]` の消滅を `waitForExist({ reverse: true })` ／ 件数変化で待つ。
  `feedStore.hydrateNotes` が同一 id の Note を差し替えると、keyed `{#each (note.id)}` により
  削除された Note の `Block` インスタンスが DOM から除去される。手動 `list_notes` invoke を
  挟まないことが「watcher 経由の自動反映」の判定条件
- **event 観測の制約（重要）**: `start_file_watcher` は内部で `AppEventBus` を生成して
  `DomainEvent::NoteFileDeletedExternally` を publish する。subscriber は
  `InMemoryNoteFeedState::remove_one` と Tauri event `notes-changed` の emit のみを行い、
  domain event 自体を frontend に露出しない。また `list_notes` は毎回 disk から
  `FsNoteRepository::list_all` で再 hydrate するため、「`list_notes` の結果から A が消えて
  いる」ことは watcher の動作証明にはならない（disk 直読みでも同じ結果になる）。
  したがって **watcher pipeline の証明は「手動操作なしで UI（DOM）から Block A が消えること」**
  に置く。これは `notes-changed` が当該 domain event の subscriber からのみ emit される
  という構造に依拠する
- **IDLE 競合なし**: `WidgetExternalChangeConflict` の `defaultSubscribeFn` は no-op
  （`store.svelte.ts` OQ-WC1: Real event bridge (Rust → TS) is not yet wired）。そのため
  IDLE では当然ダイアログは出ない。S20 の EDITING 削除競合挙動は本 scenario の範囲外
  （notes.md#known-issues に明記）
- **production 側の前提（本 scenario が満たすべき不変条件）**:
  1. `PageMain` の mount effect が `start_file_watcher` を invoke すること
     — `src/ui-page/page-main/PageMain.svelte`
  2. `start_file_watcher` が `storage_dir` を `resolve_storage_dir`（= `TAURI_TEST_STORAGE_DIR`
     override 尊重）で解決し、`notify` watcher を non-recursive / `.md` のみで起動し、
     `WatcherHandle` を managed state に保持すること
     — `src-tauri/src/note_feed/slices/detect_external_changes/commands.rs`
  3. `FsWatcher::run_event_loop` が 500ms debounce を適用し、`Remove` を
     `RawFileEvent::Deleted` として渡すこと — `.../infrastructure.rs`
  4. `DetectExternalChangesUseCase::start_watcher` が `Deleted` で `resolve_note_id` し
     `NoteFileDeletedExternally` を publish すること — `.../application.rs`
  5. subscriber が `NoteFileDeletedExternally` で `remove_one` + `notes-changed` emit を
     行うこと — `commands.rs`
  6. `PageMain` が `notes-changed` を listen し `listNotesFn` → `feedStore.hydrateNotes` する
     こと — `PageMain.svelte`
  これらが揃っていれば production 変更なしで GREEN になる見込み。欠落があれば最小実装を追加する
