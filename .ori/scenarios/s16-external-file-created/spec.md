---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s16-external-file-created
      hash: 4a02c17cc025
    - path: domain/workflows/detect-external-changes.md#detect-external-changes
      hash: a66910b0d892
    - path: domain/domain-events.md#external-file-change-events
      hash: 71db66eafe03
    - path: domain/domain-events.md#note-file-created-externally
      hash: 71db66eafe03
    - path: domain/aggregates.md#note-feed-aggregate
      hash: 56f7a54a8ab2
    - path: domain/aggregates.md#note-aggregate
      hash: 56f7a54a8ab2
    - path: domain/glossary.md#glossary-timestamp
      hash: 9405b16bb835
---

# s16-external-file-created — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s16-external-file-created phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

外部プログラム（vim / VSCode / Syncthing 等）が `storage_dir/` に **新しい `.md` ファイルを
作成**したとき、アプリを再起動したり手動 Refresh したりしなくても **NoteFeed が自動的に
更新される**（フィード表示が 1 件 → 2 件）ことを E2E で検証する scenario。

経路は `domain/workflows/detect-external-changes.md#detect-external-changes` の
OS ファイルウォッチャー → 500ms debounce → `NoteRepository::load_by_id` での parse →
domain event `NoteFileCreatedExternally` 発行 → 購読者 NoteFeed の `upsert_note`（I-F8）、
という pipeline。UI へのトースト / モーダル通知は不要（フィードの自然な更新で十分）で、
**現在の filter / sort を維持したまま**新規 Note が表示される。

- 対応 workflow: `domain/workflows/detect-external-changes.md#detect-external-changes`
  （step 1 `resolveStorageDir` / step 2 `startWatcher` + debounce 500ms / step 3 `onFileCreated` /
  step 6 `emitEvent`）
- 対応 event: `domain/domain-events.md#note-file-created-externally`
  （payload `{ note_id, note, file_path, detected_at }`。Subscribers に NoteFeed `upsert_note` と
  「UI 層: 通知不要」）
- 対応 aggregate (read side): `domain/aggregates.md#note-feed-aggregate`
  （I-F8: 外部ファイル変更の差分更新を `upsert_note` で受け付ける。filter / sort は維持）
- 対応 aggregate (source): `domain/aggregates.md#note-aggregate`
  （parse 経路で再構築される Note。`createdAt` / `updatedAt` は `YYYYMMDDhhmmss` の秒精度）
- 対応語彙: `domain/glossary.md#glossary-timestamp`
- 対応 page: `page-main`（`start_file_watcher` の起動元 + `notes-changed` 購読元の composition root）

> domain/validation.md#s16-external-file-created より:

- Given:
  - アプリ起動済み、ファイルウォッチャー稼働中
  - `storage_dir` に Note A のみ存在（フィード表示 1 件）
  - ユーザは別プログラム（vim 等）で `storage_dir/` を開いている
- When:
  1. `t0` に外部プログラムが `storage_dir/20260630120000.md` を作成
     （frontmatter: `tags: [rust]`, body: `"外部から作成"`）
  2. ファイルウォッチャーが作成を検知（debounce 500ms 後、`t0 + 0.5s = t1`）
  3. infrastructure 層が `.md` を parse、Note 構築に成功
  4. event **NoteFileCreatedExternally** `{ note_id: 20260630120000, note, file_path, detected_at: t1 }` 発行
- Then:
  - NoteFeed: `upsert_note(note)` で `source` に Note を追加（I-F8）
  - フィード表示が 1 件 → 2 件に更新
  - UI 通知は不要（フィードの自然な更新で十分）
  - 現在の filter / sort が維持されたまま新規 Note が表示される
  - 補足: parse 失敗時（malformed frontmatter 等）は event を発行せず skip

> domain/workflows/detect-external-changes.md#steps より:
> 「step 3 `onFileCreated`: parse 成功 → `NoteFileCreatedExternally { note_id, note, file_path, detected_at }`
> を返す。parse 失敗 → `None`（skip）」

> domain/domain-events.md#note-file-created-externally-subscribers より:
> 「**Note Feed**: `upsert_note(note)` で source に追加（I-F8） / **UI 層**: 通知不要
> （フィードの自然な更新で十分）」

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
3. UI は起動時 `list_notes` で hydrate 済み → フィード表示は Note A の 1 件

### When {#when}

1. テストプロセス（= 外部プログラムの代役）が `node:fs` で
   `storage_dir/20260630120000.md` を作成する
   （frontmatter: `createdAt: 20260630120000` / `updatedAt: 20260630120000` / `tags: [rust]`,
   body: `外部から作成`）
2. ファイルウォッチャーが作成を検知（debounce 500ms 後）
3. infrastructure 層が `.md` を parse し Note を構築（成功）
4. event **NoteFileCreatedExternally** が発行される

### Then {#then}

- 購読者（`start_file_watcher` が wire した subscriber）が
  `InMemoryNoteFeedState::upsert_one` → Tauri event `notes-changed` emit
- `PageMain` の `notes-changed` listener が `list_notes` を再取得 → `feedStore.hydrateNotes`
- **手動 Refresh / 再起動なしで** UI のフィード表示が 1 件 → 2 件に更新される
- 新規 Note は現在の sort（既定 `createdAt desc`）に従って Note A の前に挿入される
  （filter / sort は維持）
- UI 通知（トースト / モーダル）は出ない
- 補足: malformed frontmatter を持つ `.md` は parse 失敗で event 発行されず skip される

## テスト観点 {#test-points}

- **TP1（核・本 scenario の主目的）— 外部新規作成が手動操作なしで UI に反映される**:
  Note A のみ表示されている状態で、テストプロセスが `storage_dir/20260630120000.md` を
  外部作成する。`list_notes` の手動 invoke / 再起動を一切行わずに、
  `[data-block-id="20260630120000"]` が DOM に出現するのを待つ。
  これが watcher 検知 → `NoteFileCreatedExternally` → `notes-changed` → 再 hydrate の
  縦断 pipeline が動いていることの間接観測になる（UI/FS 状態で検証）
- **TP2（核）— 新規 Note の内容が正しく反映される**:
  反映されたブロックに対応する Note を `list_notes` の read DTO で確認し、
  `body="外部から作成"` / `tags=["rust"]` / `created_at=updated_at=2026-06-30T12:00:00Z`
  であることを assert する
- **TP3（sort 維持 / 非空虚性）— 挿入位置が現在の sort に従う**:
  表示順が `[20260630120000, 20260620120000]`（`createdAt desc`）であることを assert する。
  末尾 append ではなく sort が維持されたまま正しい位置に挿入されることを示す
- **TP4（追跡）— 連続する外部作成もそれぞれ検知される**:
  2 つ目の外部ファイル `20260701090000.md` を作成し、UI が 3 件に増えることを、
  やはり手動操作なしで確認する。1 回限りの偶然ではなく pipeline が継続的に機能することを示す
- **TP5（parse 失敗 skip）— malformed `.md` はフィードに現れない**:
  `storage_dir/20260630130000.md` を壊れた frontmatter で外部作成し、
  フィード件数が増えないこと（skip）を assert する
  （`list-feed` と同じ「読めるものだけ」原則。event 未発行の直接観測はできないため
  件数不変で間接検証）
- **E2E 観測の制約**: domain event `NoteFileCreatedExternally` 自体は frontend に露出しない
  （購読者が NoteFeed 更新 + `notes-changed` Tauri event を emit するのみ）。そのため
  event payload の直接 assert は行わず、UI/FS 状態（DOM ブロック出現 + read DTO）で
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
  Note A を seed した後 app を起動するため、watcher は隔離 dir を監視する
- **外部作成の再現**: テストプロセスから `node:fs` の `writeFileSync` で
  `TAURI_TEST_STORAGE_DIR/<id>.md` を直接作成する（＝外部プログラムの代役）。
  `.md` 以外 / `.tmp` は watcher 側で ignore される
- **UI 観測点**: `Block.svelte` が `data-block-id={note.id}` を付与する。テストは
  `$('[data-block-id="<id>"]')` の出現を `waitForExist` で待つ。手動 `list_notes` invoke を
  挟まないことが「watcher 経由の自動反映」の判定条件
- **event 観測の制約（重要）**: `start_file_watcher` は内部で `AppEventBus` を生成して
  `DomainEvent::NoteFileCreatedExternally` を publish する。subscriber は
  `InMemoryNoteFeedState::upsert_one` と Tauri event `notes-changed` の emit のみを行い、
  domain event 自体を frontend に露出しない。また `list_notes` は毎回 disk から
  `FsNoteRepository::list_all` で再 hydrate するため、「`list_notes` の結果に新規 Note が
  含まれる」ことは watcher の動作証明にはならない（disk 直読みでも同じ結果になる）。
  したがって **watcher pipeline の証明は「手動操作なしで UI（DOM）が自動更新されること」**
  に置く。これは `notes-changed` が当該 domain event の subscriber からのみ emit される
  という構造に依拠する
- **parse 失敗 skip**: watcher は `load_by_id` 失敗時に event を発行しない。加えて
  `list_all` も malformed ファイルを skip するため、malformed ファイルは UI に現れない
  （TP5 は両経路の一致を確認する弱い観測。notes.md に明記）
- **production 側の前提（本 scenario が満たすべき不変条件）**:
  1. `PageMain` の mount effect が `start_file_watcher` を invoke すること
     — `src/ui-page/page-main/PageMain.svelte`
  2. `start_file_watcher` が `storage_dir` を `resolve_storage_dir`（= `TAURI_TEST_STORAGE_DIR`
     override 尊重）で解決し、`notify` watcher を non-recursive / `.md` のみで起動し、
     `WatcherHandle` を managed state に保持すること
     — `src-tauri/src/note_feed/slices/detect_external_changes/commands.rs`
  3. `FsWatcher::run_event_loop` が 500ms debounce を適用し、`Created` を
     `RawFileEvent::Created` として渡すこと — `.../infrastructure.rs`
  4. `DetectExternalChangesUseCase::start_watcher` が `Created` で `load_by_id` →
     `NoteFileCreatedExternally` を publish すること — `.../application.rs`
  5. subscriber が `upsert_one` + `notes-changed` emit を行うこと — `commands.rs`
  6. `PageMain` が `notes-changed` を listen し `listNotesFn` → `feedStore.hydrateNotes` する
     こと — `PageMain.svelte`
  これらが揃っていれば production 変更なしで GREEN になる見込み。欠落があれば最小実装を追加する
