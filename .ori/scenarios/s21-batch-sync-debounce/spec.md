---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s21-batch-sync-debounce
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
    - path: domain/ui-fields/screen-1.md#screen-1
      hash: 4bc0f83f71f3
    - path: domain/glossary.md#glossary-timestamp
      hash: 9405b16bb835
---

# s21-batch-sync-debounce — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s21-batch-sync-debounce phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

外部プログラム（Syncthing 等）が **複数の `.md` ファイルを一括同期**したとき、すなわち
短時間（debounce 窓 500ms 以内）に Note A / B / C の 3 ファイルが連続して変更されたとき、
ファイルウォッチャーが各ファイルの変更を **個別に** debounce 処理し、手動 Refresh / 再起動なしで
**NoteFeed のすべての対応 Block の body が自動更新される**ことを E2E で検証する scenario。

経路は `domain/workflows/detect-external-changes.md#detect-external-changes` の
OS ファイルウォッチャー → 500ms debounce（**path 単位**）→ 各ファイルを
`NoteRepository::load_by_id` で再 parse + `BodyHash` 計算 → domain event
`NoteFileModifiedExternally` を **3 回**発行 → 購読者 NoteFeed の `upsert_note`（I-F8）→
Tauri event `notes-changed` → frontend の再 hydrate、という pipeline。
S17（単一ファイルの外部変更）を **複数ファイルの一括同期**に拡張した scenario であり、
S16〜S18 の逐次適用と最終結果が一致すること（バッチでも取りこぼしがないこと）を確認する。

- 対応 workflow: `domain/workflows/detect-external-changes.md#detect-external-changes`
  （step 1 `resolveStorageDir` / step 2 `startWatcher` + debounce 500ms / step 4 `onFileModified` /
  step 6 `emitEvent`）
- 対応 event: `domain/domain-events.md#note-file-modified-externally`
  （payload `{ note_id, disk_body_hash, note, file_path, detected_at }`。Subscribers は
  NoteFeed `upsert_note`（I-F8）。Note A/B/C は IDLE のため競合検出は発火しない）
- 対応 aggregate (read side): `domain/aggregates.md#note-feed-aggregate`
  （I-F8: 外部ファイル変更を `upsert_note` で受け付ける。該当 `note_id` のみ部分更新。
  filter / sort は維持）
- 対応 aggregate (source): `domain/aggregates.md#note-aggregate`
  （parse 経路で再構築される Note。`created_at` はファイル名 / frontmatter から復元）
- 対応 UI: `domain/ui-fields/screen-1.md#screen-1`（Block の `IDLE | FOCUSED | EDITING`
  ステートマシン。本 scenario は全 Block IDLE）
- 対応 page: `page-main`（`start_file_watcher` の起動元 + `notes-changed` 購読元）
- 対応語彙: `domain/glossary.md#glossary-timestamp`

> domain/validation.md#s21-batch-sync-debounce より:

- Given:
  - Note A, B, C が表示されている（いずれも IDLE 状態）
  - ファイルウォッチャー稼働中（debounce 500ms）
  - 別デバイスで Note A, B, C の 3 ファイルすべてが変更され、Syncthing が一括同期を開始
- When:
  1. `t0`〜`t0 + 0.1s` の間に `A.md`, `B.md`, `C.md` の 3 ファイルが連続して変更される
  2. ファイルウォッチャーが `A.md`, `B.md`, `C.md` の変更イベントをほぼ同時に受信
     （それぞれに debounce 500ms が適用）
  3. `t0 + 0.5s = t1` に A の debounce が完了 → **NoteFileModifiedExternally(A)** 発行
  4. `t0 + 0.6s = t2` に B の debounce が完了 → **NoteFileModifiedExternally(B)** 発行
  5. `t0 + 0.7s = t3` に C の debounce が完了 → **NoteFileModifiedExternally(C)** 発行
- Then:
  - NoteFeed は各 event を順次処理:
    - `t1`: `upsert_note(A)` → source 内の A を更新
    - `t2`: `upsert_note(B)` → source 内の B を更新
    - `t3`: `upsert_note(C)` → source 内の C を更新
  - 各 `upsert_note` は独立かつ冪等（I-F8）
  - フィードは 3 回の部分更新が行われる（全体再ハイドレートは不要）
  - `visible_notes` の結果は各 upsert 後に再計算されるが、フィルター・ソート条件が
    変わらなければ最終結果は S16〜S18 の逐次適用と同じ
  - 補足: Syncthing の `.tmp` ファイル → rename パターンは infrastructure 層の watcher が
    `.tmp` を無視し、rename 完了後の `.md` のみ処理する

> domain/domain-events.md#note-file-modified-externally-subscribers より:
> 「**Note Feed**: `upsert_note(note)` で source 内の該当 Note を差し替え（I-F8） /
> **Note Capture (application service)**: 当該 `note_id` が現在編集中の場合、
> `Note::is_stale(&disk_body_hash)` で競合を判定し、ユーザに選択肢を提示する」

> domain/workflows/detect-external-changes.md#notes より:
> 「**debounce 戦略**: Syncthing は一時ファイル（`.syncthing.xxx.tmp`）への書き込み → rename の
> パターンを使う。infrastructure 層の watcher は `.tmp` ファイルを無視し、rename 先が `.md` の
> 場合のみ `Created` / `Modified` として扱う。debounce 窓: 500ms
> （同一ファイルへの連続イベントを 1 つに集約）」

> domain/aggregates.md#note-feed-aggregate-operations より:
> 「`NoteFeed::upsert_note(self, note: Note) -> NoteFeed`
> — `source` 内の `note.id` と一致する要素があれば置換、なければ末尾に追加（I-F8）。
> 変更後も現在の filter / sort は維持される」

## シナリオステップ {#scenario-steps}

### Given {#given}

1. 実 user 環境から隔離した temp dir を `storage_dir` として Tauri App（test build, wdio）を起動。
   `storage_dir` には Note A / B / C の 3 件のみを seed 済み
   （それぞれ `id = 20260620120000 / 20260620130000 / 20260620140000`、
   `createdAt = updatedAt = id`、`tags=[]`、body は初期値）
2. `PageMain` の mount effect が `start_file_watcher` を invoke し、`storage_dir` の
   OS ファイルウォッチャー（`notify`, non-recursive, `.md` のみ）が稼働している
3. UI は起動時 `list_notes` で hydrate 済み → フィードは 3 件、既定 sort（created_at desc）で
   表示順は `[C, B, A]`、いずれの Block も IDLE 状態（`data-block-state="IDLE"`）

### When {#when}

1. テストプロセス（= 外部プログラム / Syncthing の代役）が `node:fs` で
   `storage_dir/A.md`, `storage_dir/B.md`, `storage_dir/C.md` を **debounce 窓（500ms）以内に
   連続して上書き変更**する（frontmatter は `createdAt`/`updatedAt`/`tags` を維持し body のみ変更）。
   `list_notes` の手動 invoke / 再起動は一切行わない
2. ファイルウォッチャーが 3 ファイルの変更イベントをほぼ同時に受信
   （debounce は **path 単位** — 別ファイルは互いに集約されない）
3. infrastructure 層が各 `.md` を再 parse し Note を再構築（成功）、`BodyHash` を計算
4. event **NoteFileModifiedExternally** が A / B / C についてそれぞれ発行される

### Then {#then}

- 購読者（`start_file_watcher` が wire した subscriber）が各 event で
  `InMemoryNoteFeedState::upsert_one` → Tauri event `notes-changed` emit
- `PageMain` の `notes-changed` listener が `list_notes` を再取得 → `feedStore.hydrateNotes`
- **手動 Refresh / 再起動なしで** 既存 Block A / B / C の表示 body がすべて自動更新される
  （3 ファイルとも取りこぼしなし）
- NoteFeed は各 event を `upsert_note` で **差し替え**（新規追加ではない）: Block 数は 3 のまま。
  同一 id の重複 Block は生じない（冪等 / I-F8）
- `created_at` は不変、body のみの外部変更なので `updated_at` も Given の値のまま
  （アプリ内編集 `NoteBodyEdited` は発行されない）
- 全 Block は IDLE のため **競合ダイアログ / トーストは表示されない**（S19 の EDITING ケースは範囲外）
- 補足: `.tmp` ファイル（Syncthing の一時ファイル）は watcher に無視され、rename 完了後の
  `.md` のみが `Created` / `Modified` として処理される

## テスト観点 {#test-points}

- **TP1（核・本 scenario の主目的）— 一括変更が手動操作なしで全件 UI に反映される**:
  Given で Block A / B / C の 3 件を実測（順序 `[C, B, A]`・全て IDLE）した後、
  テストプロセスが debounce 窓（500ms）以内に `A.md`, `B.md`, `C.md` を連続上書きする。
  `list_notes` の手動 invoke / 再起動を一切行わず、3 つの既存 Block の CodeMirror body
  （`[data-block-id="<id>"] .cm-content`）が **すべて** 更新されるのを待つ。これが
  watcher 検知 → 再 parse → `NoteFileModifiedExternally`（×3）→ `notes-changed` → 再 hydrate の
  縦断 pipeline がバッチでも取りこぼしなく動いていることの間接観測になる（UI/FS 状態で検証）
- **TP2（核）— 再 parse された各 Note の内容が正しい**:
  反映後の read DTO で A / B / C それぞれの `body` が更新後 body、`tags=[]`、
  `created_at = updated_at = <seed 値>` であることを assert する。body のみの外部変更なので
  `updated_at` は Given の値のまま（`NoteBodyEdited` 非発行の間接確認）
- **TP3（冪等・非空虚性）— 同一内容の再バッチでも Block が増えず表示が安定する**:
  同じ 3 ファイルを再度同一 body で上書きし、やはり手動操作なしで（debounce 窓を超えた後に）
  表示が変わらず、Block 数が 3 のままであることを assert する。`upsert_note` が append ではなく
  **既存 Note の差し替え**（I-F8）として機能し、pipeline が 1 回限りの偶然でなく継続的に
  動くことを示す
- **TP4（競合なし / IDLE）— 競合ダイアログもトーストも出ない**:
  バッチ反映後、全 Block が `data-block-state="IDLE"` で、競合ダイアログ
  （`[data-testid="widget-external-change-conflict"]`）とトースト（`screen-1-toast`）が
  いずれも非存在であることを assert する。S19 の EDITING 競合挙動は範囲外であることを示す
- **TP5（補足・`.tmp` → rename）— `.tmp` は無視され rename 後の `.md` のみ処理される**:
  `storage_dir/D.md.tmp`（D = `20260630120000`）を書き込み、debounce 窓を超えて待っても
  Block D が出現しない（`.tmp` は無視）ことを assert する。その後 `D.md.tmp` を `D.md` へ
  rename し、Block D が出現し body が反映されることを assert する。
  Syncthing の一時ファイル → rename パターンに対する watcher の取り扱いを検証する
- **E2E 観測の制約**: domain event `NoteFileModifiedExternally` 自体は frontend に露出しない
  （購読者が NoteFeed 更新 + payload なし Tauri event `notes-changed` を emit するのみ。
  `disk_body_hash` も渡らない）。そのため event payload の直接 assert は行わず、UI/FS 状態
  （3 Block の DOM body 更新 + read DTO + Block 数不変 + ダイアログ / トースト非表示 +
  `.tmp` 無視 / rename 反映）で間接検証する（詳細は [#impl-notes](#impl-notes) と notes.md）

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
  Note A / B / C を seed した後 app を起動するため、watcher は隔離 dir を監視する
- **一括変更の再現**: テストプロセスから `node:fs` の `writeFileSync` で
  既存 `TAURI_TEST_STORAGE_DIR/<id>.md` を **間隔を置かず 3 件連続で** 上書きする
  （＝ Syncthing の一括同期の代役）。既存ファイルへの truncate + write は Linux inotify で
  `EventKind::Modify` として通知され、watcher 側で `RawFileEvent::Modified` に分類される
- **debounce の粒度（重要）**: 実装（`infrastructure.rs`）の debounce は
  `last_seen: HashMap<PathBuf, Instant>` による **path 単位の leading-edge + 500ms window 抑制**:
  ① 各 path の最初のイベントは即座に `on_event` へ渡す ② 同一 path の 500ms 以内の後続
  イベントは skip ③ 500ms 無イベントで `last_seen` を clear。したがって **別ファイル
  （A / B / C）は互いに集約されず**、それぞれ独立に処理される。validation の
  `t1 = t0 + 0.5s` 等の段階的タイミングは説明上の近似であり、実装は各 path の初回イベントを
  即時処理する。不変条件「3 ファイルすべてが個別に反映され取りこぼしがない」は保持される
  （notes.md#debounce-semantics に明記）
- **UI 観測点**: `Block.svelte` が `data-block-id={note.id}` / `data-block-state` を付与し、
  body は CodeMirror の `[data-block-id="<id>"] .cm-content` に描画される。
  `feedStore.hydrateNotes` が同一 id の Note を差し替えると、keyed `{#each (note.id)}` により
  同じ `Block` インスタンスの `note` prop が更新され、`Block.svelte` の `$effect`
  （`note.body` → CM doc 同期）で `.cm-content` の text が書き換わる。テストはこの text を
  `waitUntil` で待つ。手動 `list_notes` invoke を挟まないことが「watcher 経由の自動反映」の判定条件
- **event 観測の制約（重要）**: `start_file_watcher` は内部で `AppEventBus`（in-process）を生成して
  `DomainEvent::NoteFileModifiedExternally` を publish する。subscriber は
  `InMemoryNoteFeedState::upsert_one` と Tauri event `notes-changed`（payload なし）の emit のみを
  行い、domain event payload / `disk_body_hash` を frontend に露出しない。また `list_notes` は
  毎回 disk から `FsNoteRepository::list_all` で再 hydrate するため、「`list_notes` の結果に
  変更後 body が含まれる」ことは watcher の動作証明にはならない。したがって
  **watcher pipeline の証明は「手動操作なしで UI（DOM）が自動更新されること」** に置く
  （s17 と同方針）
- **frontend の再 hydrate と「部分更新」**: ドメイン仕様（I-F8）は `upsert_note` による
  部分更新を規定し、Rust `InMemoryNoteFeedState` は実際に `upsert_one` で差分更新する。
  一方 frontend は `notes-changed` 受信のたびに `listNotesFn()`（disk 全件 re-read）→
  `hydrateNotes` で全件差し替えする。E2E は最終状態（3 Block の body 更新 + Block 数不変）を
  観測し、「部分更新回数」そのものは観測しない（notes.md#known-issues に明記）
- **競合ダイアログの観測制約**: `WidgetExternalChangeConflict` の `defaultSubscribeFn` は no-op
  （`store.svelte.ts` OQ-WC1: Real event bridge 未配線）。そのため IDLE では当然ダイアログは
  出ないが、EDITING（S19）でも E2E では区別できない。TP4 は「IDLE で競合 UI が出ない」ことの
  弱い確認であり、S19 の競合検出は slice / widget unit の領分（notes.md#known-issues に明記）
- **`.tmp` / rename の取り扱い**: `FsWatcher::is_tmp_file` が拡張子 `tmp` を無視する。
  `D.md.tmp` → `D.md` の rename は、destination（`.md`）イベントが `Modify` として
  `onFileModified` → `load_by_id(D)` → `NoteFileModifiedExternally(D)` に到達し、subscriber の
  `upsert_one` により新規 Note として feed に加わる（=`upsert_note` の「無ければ末尾に追加」）。
  notify の rename イベント分解（`.tmp` 側の From は無視、`.md` 側の To を処理）が前提。
  実装が destination を拾えない場合は最小実装を追加する（notes.md#production-change に記録）
- **production 側の前提（本 scenario が満たすべき不変条件）**:
  1. `PageMain.svelte` の mount effect が `start_file_watcher` を invoke すること
  2. `start_file_watcher` が `storage_dir` を `resolve_storage_dir`（`TAURI_TEST_STORAGE_DIR`
     override 尊重）で解決し、`notify` watcher を non-recursive / `.md` のみで起動し、
     `WatcherHandle` を managed state に保持すること
     — `note_feed/slices/detect_external_changes/commands.rs`
  3. `FsWatcher::run_event_loop` が path 単位 500ms debounce を適用し、`Modify` を
     `RawFileEvent::Modified` として渡すこと — `.../infrastructure.rs`
  4. `DetectExternalChangesUseCase::start_watcher` が `Modified` で `load_by_id` →
     `note.body_hash()` を計算し `NoteFileModifiedExternally` を publish すること
     — `.../application.rs`
  5. subscriber が `NoteFileModifiedExternally` で `upsert_one` + `notes-changed` emit を
     行うこと — `commands.rs`
  6. `PageMain.svelte` が `notes-changed` を listen し `listNotesFn` → `feedStore.hydrateNotes`
     すること
  これらが揃っていれば production 変更なしで GREEN になる見込み（S17 で実証済み）。バッチ固有の
  取りこぼし（別 path の集約誤り等）や `.tmp` / rename の欠落があれば最小実装を追加する