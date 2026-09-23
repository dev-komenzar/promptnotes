---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s19-external-modify-while-editing
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
    - path: domain/ui-fields/screen-4.md#screen-4
      hash: 4ee02719430d
    - path: domain/ui-fields/screen-1.md#screen-1
      hash: 4bc0f83f71f3
    - path: domain/ui-fields/page-groups.md#widget-external-change-conflict
      hash: 8986615249ac
    - path: domain/glossary.md#glossary-timestamp
      hash: 9405b16bb835
---

# s19-external-modify-while-editing — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s19-external-modify-while-editing phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

外部プログラム（Syncthing / vim / VSCode 等）が `storage_dir/<A.id>.md` の **body を変更**したとき、
Note A の Block が **EDITING 状態（ユーザが編集中）** であり、かつディスク上の body ハッシュが
メモリ上のハッシュと異なる（`is_stale == true`）場合に、**競合ダイアログ**
`widget-external-change-conflict`（screen-4）が表示されることを E2E で検証する scenario。

経路は `domain/workflows/detect-external-changes.md#detect-external-changes` の
OS ファイルウォッチャー → 500ms debounce → `NoteRepository::load_by_id` での再 parse +
`BodyHash` 計算 → domain event `NoteFileModifiedExternally` 発行 → 購読者 NoteFeed の
`upsert_note`（I-F8）→ Tauri event `notes-changed` → frontend の競合判定
（Block ステートマシンの EDITING 検出 + `is_stale`）→ screen-4 ダイアログ、という pipeline。
S17（IDLE での外部変更 → 自動反映）とは対照的に、本 scenario は EDITING の競合解決を扱う。

- 対応 workflow: `domain/workflows/detect-external-changes.md#detect-external-changes`
  （step 1 `resolveStorageDir` / step 2 `startWatcher` + debounce 500ms / step 4 `onFileModified` /
  step 6 `emitEvent`）
- 対応 event: `domain/domain-events.md#note-file-modified-externally`
  （payload `{ note_id, disk_body_hash, note, file_path, detected_at }`。Subscribers に
  NoteFeed `upsert_note` と Note Capture の競合検出。本 scenario は後者 = 競合検出側）
- 対応 aggregate (read side): `domain/aggregates.md#note-feed-aggregate`
  （I-F8: 外部ファイル変更の差分更新を `upsert_note` で受け付ける）
- 対応 aggregate (source): `domain/aggregates.md#note-aggregate`
  （I-N9: `BodyHash` により競合検出。`Note::is_stale(&disk_body_hash)`）
- 対応 UI: `domain/ui-fields/screen-4.md#screen-4`（競合解決ダイアログ、compare-layout）+
  `domain/ui-fields/screen-1.md#screen-1`（Block の `IDLE | FOCUSED | EDITING` ステートマシン）
- 対応 page: `page-main`（`start_file_watcher` の起動元 + `notes-changed` 購読元 +
  widget の mount 元）/ `widget-external-change-conflict`（subscribe + 競合判定 + 解決）
- 対応語彙: `domain/glossary.md#glossary-timestamp`

> domain/validation.md#s19-external-modify-while-editing より:

- Given:
  - Note A (`body="hello"`, `body_hash=H1`) をユーザが **EDITING 状態** で編集中
  - ユーザは body を `"hello 編集中"` に変更済み（AutoSave 未発火）
  - ファイルウォッチャー稼働中
- When:
  1. `t1` に外部プログラム（Syncthing 経由等）が `storage_dir/<A.id>.md` の body を `"hello world"` に変更
  2. ファイルウォッチャーが変更を検知 → **NoteFileModifiedExternally** 発行
     `{ note_id: A.id, disk_body_hash: H2, note, ... }`
  3. フロントエンドが event 受信 → Block A の状態を確認 → **EDITING** を検出
  4. `Note::is_stale(&H2)` を呼出 → `H1 ≠ H2` → `true`（I-N9）
- Then:
  - フロントエンドが**競合ダイアログ**を表示:
    - 「外部でこのノートが変更されました」
    - 選択肢: 「外部変更を適用」（編集中の内容は破棄）、「編集中を保持」（外部変更を無視）
  - ユーザが「外部変更を適用」を選択: 編集中の内容を破棄し、ディスクの内容で Note A を置換。
    NoteFeed `upsert_note(note)` 実行。Block は IDLE 状態に遷移
  - ユーザが「編集中を保持」を選択: 外部変更を無視、編集中の内容を維持。
    NoteFeed への upsert は行わない（次回の AutoSave/Flush 時に上書き）。Block は EDITING 状態を維持
  - 補足: ダイアログ表示中もファイルウォッチャーは稼働継続（後続の変更も検知）

> domain/workflows/detect-external-changes.md#steps より:
> 「step 4 `onFileModified`: `.md` を再 parse し、ディスクから読んだ body から `BodyHash` を計算。
> parse 成功 → `NoteFileModifiedExternally { note_id, disk_body_hash, note, file_path, detected_at }`
> を返す。parse 失敗 → `None`（skip）」

> domain/domain-events.md#note-file-modified-externally-subscribers より:
> 「**Note Feed**: `upsert_note(note)` で source 内の該当 Note を差し替え（I-F8） /
> **Note Capture (application service)**: 当該 `note_id` が現在編集中の場合、
> `Note::is_stale(&disk_body_hash)` で競合を判定し、ユーザに選択肢を提示する
> （「外部変更を適用」「編集中を保持」）」

> domain/aggregates.md#note-aggregate-invariants より:
> 「**I-N9**: `Note` は `BodyHash` を保持し、ディスク上の body ハッシュとの比較で
> 外部変更による競合を検出できる。`is_stale(&disk_body_hash)` が `true` の間は
> 編集中の内容とディスクの内容が乖離している」

> domain/ui-fields/screen-4.md#purpose より:
> 「表示条件: ユーザが当該 `note_id` を EDITING 状態で開いている / ディスク上の
> `body` ハッシュ (`BodyHash`) がメモリ上のハッシュと異なる（`is_stale`）」

> domain/ui-fields/screen-4.md#cross-field-rules より:
> 「`{ApplyExternal}` 選択 → 確定ボタンで `NoteFeed::upsert_note(external_note)` を呼び、
> Editor の内容を外部バージョンに差し替える。`body_hash` も更新され `is_stale` が `false` になる。
> `{KeepEditing}` 選択 → ダイアログを閉じ、現在の編集内容を保持。
> キャンセル (`Esc` / ×ボタン) → `KeepEditing` と等価」

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
   （テストでは ASCII で追記する — 下記 [#impl-notes](#impl-notes) の IME 制約を参照）。
   編集後の body を `"hello local"` とし、AutoSave debounce を待って
   `storage_dir/<A.id>.md` にローカル編集が永続化された状態を準備する
   （＝ この時点でディスクとローカルは同一 body。以降の外部変更が競合の起点になる）

### When {#when}

1. テストプロセス（= 外部プログラムの代役）が `node:fs` で既存の
   `storage_dir/20260620120000.md` を **上書き変更**し body を `"hello world"` にする
   （frontmatter は `createdAt`/`updatedAt`/`tags` を維持）
2. ファイルウォッチャーが変更を検知（debounce 500ms 後）
3. infrastructure 層が `.md` を再 parse し Note を再構築（成功）、`BodyHash`（H2）を計算
4. event **NoteFileModifiedExternally** が発行される
5. frontend は Note A が EDITING であることを検出し、ローカル body（`"hello local"`）と
   ディスク body（`"hello world"`）のハッシュが異なる（`is_stale == true`）と判定する

### Then {#then}

- `WidgetExternalChangeConflict` の store が競合 payload を保持し
  `data-testid="widget-external-change-conflict"` の `<dialog>` が表示される
- ダイアログは note title（`20260620120000.md`）、ローカル版（`"hello local"`）、
  外部版（`"hello world"`）を提示する（screen-4 compare-layout）
- ユーザが **KeepEditing**（Cancel / Keep my edits）を選択 →
  ダイアログは閉じ、フィード/Editor は編集中の内容 `"hello local"` を維持し、
  Block A は **EDITING のまま**
- ユーザが **ApplyExternal**（Apply external changes + Apply）を選択 →
  ダイアログは閉じ、Editor と NoteFeed が外部版 `"hello world"` で置換され、
  Block A は **IDLE** に遷移する
- Block A が IDLE の間に外部変更が起きても**競合ダイアログは表示されない**（S17 の挙動）。
  フィードは自動更新される（I-F8）
- 競合の検出・解消の過程で UI トーストは表示されない（screen-4 Notes: 追加 toast 不要）
- 補足: ダイアログ表示中もファイルウォッチャーは稼働継続する

## テスト観点 {#test-points}

- **TP1（核・本 scenario の主目的）— EDITING 中の外部変更で競合ダイアログが表示される**:
  Block A を EDITING にし `"hello local"` に編集（AutoSave 済み）した後、テストプロセスが
  `storage_dir/20260620120000.md` を外部上書きし body を `"hello world"` にする。
  `list_notes` の手動 invoke / 再起動を一切行わず、`[data-testid="widget-external-change-conflict"]`
  が表示されるのを待つ。Block A は `data-block-state="EDITING"` のまま。これが
  watcher 検知 → `NoteFileModifiedExternally` → frontend 競合判定（EDITING + is_stale）→
  screen-4 表示、の縦断 pipeline が動いていることの間接観測になる
- **TP2（核）— ダイアログが local / external 両バージョンを提示する**:
  表示されたダイアログの `screen-4-note-title` が `20260620120000.md`、
  `screen-4-body-local` の value が `"hello local"`、`screen-4-body-external` の value が
  `"hello world"` であることを assert する（screen-4 compare-layout）
- **TP3（解決・KeepEditing / 非空虚性）— Cancel で編集中の内容を保持**:
  `screen-4-cancel` をクリックするとダイアログが非表示になり、Block A の
  CodeMirror body（`.cm-content`）が `"hello local"` のまま（外部版に置換されない）で、
  Block A は EDITING を維持することを assert する（I-WC6 / screen-4 cross-field rules）
- **TP4（解決・ApplyExternal）— 外部変更を適用して IDLE へ**:
  さらに外部変更（`"hello world 2"`）を起こしてダイアログを再表示させ、
  `screen-4-resolution-apply-external` を選択して `screen-4-confirm` をクリックすると、
  ダイアログが非表示になり、Block A の body が外部版 `"hello world 2"` に置換され、
  Block A が IDLE に遷移することを assert する（I-WC5 / screen-4 resolution-complete）
- **TP5（非編集時は silent）— IDLE の外部変更ではダイアログが出ずフィードが自動更新**:
  Block A が IDLE の状態で外部変更（`"hello world 3"`）を起こし、競合ダイアログが
  非表示のまま、フィードの body が自動更新されることを assert する（S17 との境界。
  I-WC2 silent on absence）
- **E2E 観測の制約**: domain event `NoteFileModifiedExternally` 自体は frontend に露出しない
  （購読者が NoteFeed `upsert_note` + Tauri event `notes-changed` を emit するのみで、
  event payload を frontend に渡していない。`widget-external-change-conflict` の
  `subscribeFn` は現状 no-op — OQ-WC1）。そのため event payload の直接 assert は行わず、
  UI 状態（ダイアログ表示/非表示 + body 表示 + Block ステート）で間接検証する
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
  watcher 側で `RawFileEvent::Modified` に分類される（s17 と同一経路）
- **UI 観測点**:
  - Block ステート: `Block.svelte` が `data-block-state={blockState}`（`IDLE|FOCUSED|EDITING`）を付与
  - 編集中 body: `[data-block-id="<id>"] .cm-content`（CodeMirror）の text
  - 競合ダイアログ: `[data-testid="widget-external-change-conflict"]`（`<dialog>`、`showModal()`）。
    version textarea は `screen-4-body-local` / `screen-4-body-external`、note title は
    `screen-4-note-title`、解決 radio / button は `screen-4-resolution-*` / `screen-4-confirm` /
    `screen-4-cancel`
  - textarea の value は `getValue()` で読む（Svelte が `value` を DOM property として設定する
    ため `getText()` は空になる）
- **event 観測の制約（重要）**: `start_file_watcher` は内部で `AppEventBus`（in-process）を生成し
  `DomainEvent::NoteFileModifiedExternally` を publish する。subscriber は
  `InMemoryNoteFeedState::upsert_one` と Tauri event `notes-changed`（payload なし）の emit のみを
  行い、domain event payload を frontend に露出しない。さらに frontend 側の
  `widget-external-change-conflict` の `defaultSubscribeFn` は no-op
  （`store.svelte.ts` OQ-WC1: Real event bridge (Rust → TS) is not yet wired）のため、
  **production 配線が無い状態ではダイアログは決して表示されない**（= 本 scenario の RED）。
  E2E は event payload を直接 assert せず、UI 状態で間接検証する
- **frontend 最小配線（本 scenario が満たすべき不変条件）**:
  1. `PageMain` が `WidgetExternalChangeConflict` に実 deps
     （`subscribeFn` / `currentNoteId` / `currentBodyHash` / `onApplyExternal`）を渡すこと
     — 現状は `localBody=""` + default no-op で未配線
  2. `editingNote` store（`noteId` / `bodyHash`）が Block ステートマシンに接続され、
     EDITING 中のみ現在編集中の note_id と body hash を保持すること
     — 現状 `setEditing` はどこからも呼ばれていない
  3. `notes-changed` 受信時、編集対象 note のディスク body がローカル body と異なる場合に
     競合 payload を widget に届け、かつ **ローカル編集中 body を clobber しない**こと
     — 現状は `hydrateNotes` が無条件にディスク内容でフィードを置換し、Block の sync effect が
     CodeMirror doc を外部版で上書きしてしまう（競合が黙って失われる）
  4. Rust 側は既存（s17 で実証済み）: `EventKind::Modify` → `RawFileEvent::Modified` →
     `NoteFileModifiedExternally` publish → `upsert_one` + `notes-changed` emit
  5. これら frontend 配線が揃えば E2E は GREEN になる見込み。欠落があれば最小実装を追加する
- **IME / キー入力の制約**: wdio からの日本語入力（`"hello 編集中"`）は IME composition を
  再現できず不安定なため、テストでは ASCII（`"hello local"`）でローカル編集を表現する。
  ハッシュ比較・競合判定の意味は同一（notes.md#known-issues に明記）
- **AutoSave debounce と競合の順序**: validation Given は「AutoSave 未発火」だが、E2E では
  ローカル編集の pending write が外部変更を上書きして race するのを避けるため、
  タイプ後に AutoSave の永続化を待ってから外部変更を起こす。この時点でディスク == ローカル
  body なので、外部変更が競合の単一起点になる（notes.md#known-issues に明記）
- **I-WC7（ダイアログ表示中の Editor キー入力ブロック）**: store コメント上は
  呼び出し元の責務とされており、本 scenario の E2E 観測対象外（notes.md#known-issues）
