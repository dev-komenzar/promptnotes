---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s22-storage-dir-change-watcher-restart
      hash: 4a02c17cc025
    - path: domain/workflows/detect-external-changes.md#detect-external-changes
      hash: a66910b0d892
    - path: domain/workflows/update-settings.md#update-settings
      hash: f420da94bd93
    - path: domain/aggregates.md#settings-aggregate
      hash: 56f7a54a8ab2
    - path: domain/domain-events.md#storage-dir-changed
      hash: 71db66eafe03
    - path: domain/domain-events.md#note-file-modified-externally
      hash: 71db66eafe03
    - path: domain/domain-events.md#note-file-created-externally
      hash: 71db66eafe03
    - path: domain/ui-fields/screen-2.md#screen-2
      hash: f1e4fd24bf05
---

# s22-storage-dir-change-watcher-restart — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s22-storage-dir-change-watcher-restart phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

設定モーダルで `storage_dir` を変更したとき、**Infrastructure 層（ファイルウォッチャー）が
`StorageDirChanged` を購読して監視対象ディレクトリを切り替える**ことを E2E で検証する scenario。
S11（`s11-storage-dir-change`）を拡張したもので、S11 が扱わなかった
「`StorageDirChanged` の Infrastructure 層 subscriber」を追加した形。

S11 では UI 層 subscriber（再起動を促すモーダル）と Feed 据え置き guard（I-S4）のみを実装・検証し、
ファイルウォッチャーの監視対象切替は **明示的に S22 の範囲** として先送りされた
（`s11/spec.md#when-watcher-out-of-scope`）。本 scenario はその未実装部分を検証対象とする。

経路は `domain/workflows/detect-external-changes.md#detect-external-changes` のとおり:

1. 起動時: `resolve_storage_dir`（Settings から `old_dir` を解決）→ `startWatcher` →
   `old_dir/*.md` を non-recursive / 500ms debounce で監視
2. 設定変更: `update-settings` workflow が `Settings::change_storage_dir(new_dir)` を適用し
   `settings.json` へ永続化 → event `StorageDirChanged { old_dir, new_dir }` を発行
3. **切り替え（本 scenario の核）**: Infrastructure 層が `StorageDirChanged` を受信し、
   旧 `WatcherHandle` を Drop（監視停止）→ `new_dir` で新規 watcher を起動
4. 以降は `new_dir/*.md` の外部変更が
   `NoteFileCreatedExternally` / `NoteFileModifiedExternally` として検知される

> domain/validation.md#s22-storage-dir-change-watcher-restart より:

- Given:
  - Settings (`storage_dir = /old/path`)、ファイルウォッチャーが `/old/path` を監視中
  - フィードに `/old/path` の Note 3 件表示中
- When:
  1. 設定モーダルで `storage_dir = /new/path` に変更して保存
  2. event **StorageDirChanged** `{ old_dir: /old/path, new_dir: /new/path }` 発行
  3. Infrastructure 層（ファイルウォッチャー）が subscriber として受信
- Then:
  - `/old/path` のファイルウォッチャーを停止（`WatcherHandle` の Drop）
  - `/new/path` で新規ファイルウォッチャーを起動
    - 監視対象: `/new/path/*.md` / debounce: 500ms（S21 と同じ）
  - ウォッチャー再起動後、`/new/path` の変更が検知対象になる
  - UI: 再起動を促すモーダル表示（S11 と同様、I-S4）
  - ウォッチャー再起動に失敗した場合:
    - infrastructure 層が retry（最大 3 回、1 秒間隔）
    - 全 retry 失敗時はユーザにアプリ再起動を促す
  - 補足: このシナリオは S11 を拡張したもの

> domain/workflows/detect-external-changes.md#detect-external-changes より:

> 「トリガー: **StorageDirChanged event 購読時**: 旧ディレクトリの監視を停止し、新ディレクトリで再開」

> 「**watcher 再起動**: `StorageDirChanged` event 購読時、旧ディレクトリの watcher を
> 停止し、新ディレクトリで新規に起動する。watcher の停止は `WatcherHandle` の Drop で保証。
> 再起動失敗時はアプリ全体の再起動を促す（I-S4 と整合）」

> 「watcher の再起動失敗は `StorageDirChanged` の subscriber として infrastructure 層が
> retry またはユーザーに再起動を促す」

> domain/domain-events.md#storage-dir-changed-subscribers より:
> 「**UI 層**: 再起動を促すモーダルを表示（即時マイグレーションは行わない、I-S4）」
> 「**Infrastructure 層（ファイルウォッチャー）**: 監視対象ディレクトリを `new_dir` に切り替え。
> 旧ディレクトリの監視は停止（Phase 9 workflow で詳細設計）」

> domain/aggregates.md#settings-aggregate より:
> 「**I-S4**: `change_storage_dir` 操作は即時には Note の引っ越しを起こさない（再起動を要求する想定）」

> domain/ui-fields/screen-2.md#cross-storage-dir-restart より:
> 「保存ボタン押下で `update-settings` workflow が走り、`StorageDirChanged` 発行」「直後に
> 『設定を反映するにはアプリを再起動してください』というモーダル表示（即時マイグレーションは
> しない、I-S4 / S11）」

## シナリオステップ {#scenario-steps}

### Given {#given}

1. `settings.json` の `storage_dir` は **old dir**（`XDG_CONFIG_HOME/<identifier>/settings.json` に
   seed 済み）。`TAURI_TEST_STORAGE_DIR` override は使わない（使うと `storage_dir` が固定され、
   新 dir への切替を検証できない）
2. old dir に Note 3 件（`20260601100000` / `20260615100000` / `20260626100000`）が seed されている
3. new dir に **別の** Note 1 件（`20260701100000` "new-delta"）が seed されている
4. Tauri App が起動済みで、フィードに old dir の 3 件が表示されている
5. `PageMain` の mount effect が `start_file_watcher` を invoke し、`resolve_storage_dir` が
   `settings.json` から old dir を解決 → old dir の OS ファイルウォッチャー（`notify`,
   non-recursive, `.md` のみ, 500ms debounce）が稼働している

### When {#when}

1. 設定モーダルで `storage_dir = new dir` に変更して保存する
   （保存ディレクトリは read-only 表示 + OS native folder picker のため、E2E は
   `update_settings` を Tauri 境界で直接 invoke する。S11 と同方式 — 詳細は [#when-boundary-invoke](#when-boundary-invoke)）
2. `update_settings` が `settings.json` を永続化し、event `StorageDirChanged { old_dir, new_dir }`
   を発行する
3. Infrastructure 層（ファイルウォッチャー）が subscriber として受信する

### Then {#then}

- `/old/path` のファイルウォッチャーが停止する（`WatcherHandle` の Drop）
- `/new/path` で新規ファイルウォッチャーが起動する（`/new/path/*.md`, 500ms debounce）
- ウォッチャー再起動後、`/new/path` の外部変更（Modify / Create）が検知され、
  `NoteFileModifiedExternally` / `NoteFileCreatedExternally` → subscriber（NoteFeed
  `upsert_one` + Tauri event `notes-changed`）→ frontend の再 hydrate、という pipeline が動く
- UI: 再起動を促すモーダル（`restart-prompt`）が表示される（S11 / I-S4 の回帰）
- 再起動に失敗した場合: infrastructure 層が retry（最大 3 回、1 秒間隔）、全失敗でユーザに
  アプリ再起動を促す（E2E 観測対象外 — [#impl-notes](#impl-notes) 参照）

### 補足（storage_dir 変更の駆動方法） {#when-boundary-invoke}

保存ディレクトリは read-only 表示 + OS native folder picker（screen-2）のため dialog 自体は
駆動できない。そこで E2E は storage_dir の変更を `update_settings { input: { storage_dir } }` の
Tauri 境界 invoke で行う（s10 / s11 の境界 invoke と同方式）。本 scenario の主眼は watcher 再起動
（`StorageDirChanged` の infrastructure subscriber）にあり、設定モーダルの Save 経路
（`handleSettingsSaved` / Feed 据え置き guard）は S11 で検証済みであるため、E2E は設定モーダルを
開かず境界 invoke のみで storage_dir を変更する。production コードに test seam は入れない。

### 補足（S11 の回帰 — フィード据え置き） {#when-s11-regression}

`StorageDirChanged` の UI 層 subscriber（再起動モーダル）と Feed 据え置き guard（I-S4）は
S11 で実装済み。本 scenario はそれらを **回帰として再確認** する: storage_dir 変更直後は
`restart-prompt` が表示され、フィードは old dir のまま（new dir の Note は現れない）。

## テスト観点 {#test-points}

- **TP1（Given 前提の実測）— old dir の watcher が稼働中**:
  Given で old dir の Note 3 件を実測した後、テストプロセスが old dir の `.md` を外部上書きする。
  手動操作なしで当該 Block の DOM body が自動更新されることを確認する。
  これが「watcher が old dir を監視している」ことの前提実測になる（RED / GREEN どちらでも PASS）
- **TP2（S11 回帰）— storage_dir 変更で永続化 + 再起動モーダル + フィード据え置き**:
  設定モーダルを開いた状態で `update_settings { storage_dir: newDir }` を境界 invoke する。
  `settings.json#storage_dir` が new dir に更新され、`StorageDirChanged` 発行により
  `restart-prompt` が出現する（I-S4）。この時点でフィードは old dir の 3 件のままで、
  new dir の Note は現れない（S11 の Feed 据え置き guard の回帰）
- **TP3（核・本 scenario の主目的）— StorageDirChanged 後に new dir の外部変更が検知される**:
  TP2 の変更後、テストプロセスが **new dir** の既存 Note（`20260701100000`）を外部上書きする。
  手動 Refresh / 再起動なしで、UI が new dir の変更を検知して反映する
  （`[data-block-id="20260701100000"] .cm-content` が更新後 body になる）ことを待つ。
  これが「infrastructure subscriber が旧 watcher を停止し新 dir で watcher を起動した」
  ことの間接観測になる（watcher が old dir のままなら new dir の変更は検知されない）。
  **本 step は RED になるのが想定**（watcher 再起動が未実装）
- **TP4（核・Created 経路）— StorageDirChanged 後に new dir の新規ファイルが検知される**:
  TP2 の変更後、new dir に新しい `.md`（`20260702100000`）を外部作成する。
  手動操作なしで新規 Block が出現し body が反映されることを確認する。
  新 dir watcher の `Created` 経路（`NoteFileCreatedExternally`）の間接観測。
  **本 step も RED になるのが想定**
- **TP5（非空虚性 / 機構の保存）— 旧 watcher の停止は E2E では直接観測できない**:
  「`/old/path` の監視停止」は `WatcherHandle` の Drop という in-process の後始末であり、
  frontend に露出する signal を持たない。よって E2E の判定は
  **「new dir の変更が検知されること（TP3 / TP4）」** に置く。
  旧 watcher 停止の直接的検証は slice / unit の領分（notes.md#known-issues に明記）
- **E2E 観測の制約**:
  - domain event `StorageDirChanged` / `NoteFileModifiedExternally` / `NoteFileCreatedExternally`
    は frontend に直接露出しない（UI 層は `settings:storage_dir_changed`、NoteFeed 購読者は
    payload なし `notes-changed` のみを受け取る）。そのため event payload
    （`old_dir` / `new_dir` / `disk_body_hash`）は assert せず、UI/FS 状態
    （`settings.json` の内容 + `restart-prompt` の出現 + フィードの Block id 集合 +
    new dir 変更の DOM 反映）で間接検証する
  - **retry（最大 3 回、1 秒間隔）/ 全 retry 失敗時の「アプリ再起動を促す」挙動**は、
    watcher 起動を失敗させる外部条件を E2E から注入できないため観測対象外とする
    （notes.md#known-issues に制約として明記）

## 実装ノート {#impl-notes}

- **runner**: `wdio`（優先チェーン step 2 — manifest に `runner:` 明示なし。参加 app
  `promptnotes` は `local` 系で `runtime.runner = wdio`。`.ori/architecture.md` の
  `scenario_test_runner: wdio` とも一致）
- **infrastructure**: `promptnotes` のみ（`mode: local`）。compose-service 系 app / infra が
  参加しないため `docker-compose.yml` は生成しない
- **runner config**: `wdio.conf.ts`。`tauri:options.application` は runtime block の `binary`
  = `apps/promptnotes/src-tauri/target/debug/app`（build-then-test。
  `VITE_WDIO_TEST=1 bun run tauri build --debug --no-bundle` = `bun run build:test`）
- **config dir の隔離**: `onPrepare` が `XDG_CONFIG_HOME` / `XDG_DATA_HOME` を temp dir に設定する。
  Tauri の `app_config_dir()` は `$XDG_CONFIG_HOME/com.komenzar.promptnotes` に解決されるため、
  そこへ `settings.json`（`storage_dir = old dir`）を seed すれば Given の Settings を再現できる。
  **`TAURI_TEST_STORAGE_DIR` override は使わない**（`storage::resolve_storage_dir` が override を
  最優先するため `storage_dir` が固定され、新 dir への切替を検証できない）
- **Note の seed**: old dir に 3 件（`20260601100000` / `20260615100000` / `20260626100000`）、
  new dir に別の 1 件（`20260701100000`）を `onPrepare` が書き込む
- **storage_dir 変更の駆動方法**: `update_settings { input: { storage_dir } }` を Tauri 境界で
  直接 invoke する（同期 `browser.execute` で kick-off → window 上の結果を poll。
  `@wdio/tauri-service` (driverProvider=external) の patchedExecute は `browser.executeAsync` を
  扱えないため。s10 / s11 と同方式）
- **UI 観測点**: フィードは `[data-block-id]` の集合で判定し、body は
  `[data-block-id="<id>"] .cm-content` の text で判定する。`restart-prompt` は
  `[data-testid="restart-prompt"]`。`settings.json` は `node:fs` の `readFileSync` で直接 read する
- **new dir 変更の観測（重要）**: `list_notes` は毎回 `settings.json` の `storage_dir` を解決して
  disk を再 hydrate する（`note_feed/slices/list_feed/commands.rs`）。したがって new dir の変更が
  watcher に検知されると subscriber が `notes-changed` を emit → `PageMain` が `list_notes` →
  new dir の内容で feed を更新する。**この経路は watcher 再起動なしには発生しない**ため、
  TP3 / TP4 は watcher 切替の間接観測として成立する
- **production 側の前提（本 scenario が満たすべき不変条件）**:
  1. `PageMain.svelte` の mount effect が `start_file_watcher` を invoke し、`notes-changed` を
     listen すること（既存 — S17/S21 で実証済み）
  2. `start_file_watcher`（`note_feed/slices/detect_external_changes/commands.rs`）が
     `resolve_storage_dir` で dir を解決し、`notify` watcher を non-recursive / `.md` のみで
     起動、`WatcherHandle` を managed state (`WatcherState`) に保持すること（既存）
  3. `update_settings` が `storage_dir` 差分時に `settings.json` へ永続化し
     `StorageDirChanged` を `app.emit('settings:storage_dir_changed')` すること（既存 — S11）
  4. `PageMain` が `settings:storage_dir_changed` を購読して `restart-prompt` を表示し、
     `storage_dir` 変更時に Feed を再 hydrate しないこと（既存 — S11 / I-S4）
  5. **【未実装・本 scenario の核】** `StorageDirChanged` の Infrastructure 層 subscriber が
     旧 `WatcherHandle` を Drop し、`new_dir` で新規 watcher を起動すること。
     現状 `WatcherState` は `StorageDirChanged` を購読しておらず、`start_file_watcher` /
     `stop_file_watcher` は frontend から明示的に呼ばれるのみ。`WatcherState.handle` を
     差し替える infrastructure subscriber（+ 失敗時 retry 最大 3 回 / 1 秒間隔）が必要
- **RED の想定**: 前提 5 が未実装のため、TP3 / TP4 は **RED**（new dir の変更が検知されない）
  になる。TP1 / TP2 は既存実装で PASS する。この RED が「watcher 再起動の実装が必要である」
  ことの契約になる（詳細は review.md / notes.md）
