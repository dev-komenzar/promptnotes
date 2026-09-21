# s22-storage-dir-change-watcher-restart — Scenario implementation notes

## 検証対象 {#target}

`validation.md#s22-storage-dir-change-watcher-restart` — 設定で `storage_dir` を変更したとき、
`StorageDirChanged` の **Infrastructure 層 subscriber（ファイルウォッチャー）** が旧 watcher を
停止し、new dir で watcher を起動し直すこと。S11 が明示的に範囲外とした部分
（`s11/spec.md#when-watcher-out-of-scope`）を引き継ぐ scenario。

経路: `PageMain` mount → `start_file_watcher`（`resolve_storage_dir` = old dir）→
`update_settings` → `settings.json` 永続化 + `StorageDirChanged` 発行 → **infrastructure subscriber
が旧 `WatcherHandle` を Drop → new dir で新規 watcher 起動** → new dir の外部変更が
`NoteFileModifiedExternally` / `NoteFileCreatedExternally` として検知 → subscriber が
`InMemoryNoteFeedState::upsert_one` + Tauri event `notes-changed` → `PageMain` の listener が
`list_notes` 再取得（`settings.json` = new dir）→ `feedStore.hydrateNotes` → DOM 更新。

## 実行方法 {#how-to-run}

```bash
export PATH=$HOME/.cargo/bin:$PATH
export DISPLAY=:0
# test build (VITE_WDIO_TEST=1 で tauri-plugin-wdio を有効化)
cd apps/promptnotes && bun run build:test
cd ../../.ori/scenarios/s22-storage-dir-change-watcher-restart
../../../apps/promptnotes/node_modules/.bin/wdio run wdio.conf.ts
```

結果（RED 確立時）: **2 passing / 2 failing (~31s)** — **RED が期待結果**（後述）。
結果（GREEN 化後）: **4 passing / 0 failing（~3.5s）** — [#green](#green) 参照。

## RED 契約（本 scenario の目的） {#red-contract}

本 scenario は **TDD の RED-first** で作られている。対応する production 実装
（`StorageDirChanged` の infrastructure subscriber = watcher 再起動）は **未実装**であり、
E2E の step 3 / 4 が RED になることが **本 scenario の契約**である。
GREEN 化（production 実装の追加）は次ワーカーの責務であり、本ワーカーは scenario（検証軸）のみを作った。
→ **GREEN 化は完了済み**。実装と実測は [#green](#green) を参照（テスト assertion は非改変）。

- **RED になる step**: step 3（new dir の外部変更検知）/ step 4（new dir の新規 `.md` 作成検知）
- **PASS の step**: step 1（old dir watcher 稼働の前提実測）/ step 2（S11 回帰: 永続化 +
  restart-prompt + フィード据え置き）
- **RED の原因**: `note_feed/slices/detect_external_changes/commands.rs` の `WatcherState` は
  `start_file_watcher` / `stop_file_watcher` を frontend から明示的に呼ぶのみで、
  `StorageDirChanged` を購読する infrastructure subscriber を持たない。`update_settings` で
  `storage_dir` を変更しても watcher は old dir を監視し続ける

## 観測設計（なぜ new dir の DOM 反映で判定するか） {#observation}

domain event は frontend に露出しないため（[#event-observation](#event-observation)）、
watcher 切替の判定は **「new dir の外部変更が手動操作なしで UI に反映されること」** に置く。

- `list_notes`（`note_feed/slices/list_feed/commands.rs`）は毎回 `settings.json` の
  `storage_dir` を解決して disk を再 hydrate する。したがって new dir の変更が watcher に
  検知されると subscriber が `notes-changed` を emit → `PageMain` が `list_notes` → new dir の
  内容で feed が更新される
- **この経路は watcher 再起動なしには発生しない**（old dir watcher は new dir の変更を検知しない）。
  よって step 3 / 4 の DOM 自動更新は watcher 切替の間接観測として成立する
- step 3 は body の更新（`20260701100000` の `.cm-content` が更新後 body）を待つ。これは
  「feed が new dir に切り替わっただけ（`list_notes` の手動 invoke 等）」では成立せず、
  watcher → `notes-changed` が実際に発火したことを要求する

## event 観測の制約（重要） {#event-observation}

- `StorageDirChanged` は `TauriEventBus`（`settings:storage_dir_changed`）として UI 層に届き、
  その購読者は `PageMain` の再起動モーダル表示のみ。**Infrastructure 層 subscriber は現状存在しない**
- `start_file_watcher` は内部で `AppEventBus`（in-process）を生成し、`DomainEvent::
  NoteFileModifiedExternally` / `NoteFileCreatedExternally` を publish する。その subscriber は
  `InMemoryNoteFeedState::upsert_one` と Tauri event `notes-changed`（**payload なし**）の emit のみを
  行い、domain event payload / `disk_body_hash` を frontend に露出しない
- したがって E2E は event payload（`old_dir` / `new_dir` / `disk_body_hash`）を直接 assert できず、
  UI/FS 状態（`settings.json` の内容 + `restart-prompt` の出現 + フィードの Block id 集合 +
  new dir 変更の DOM 反映）で間接検証する（spec.md#test-points）

## RED 実測（本 scenario の契約） {#red-measurement}

`bun run build:test` → `wdio run wdio.conf.ts`（2 回連続で同一結果）:

- **結果: 2 passing / 2 failing（~31s）**
  - ✓ step 1: old dir の外部変更が DOM に自動反映される（watcher 稼働の前提実測）
  - ✓ step 2: `settings.json#storage_dir` 更新 + `restart-prompt` 出現 + フィードは old dir のまま
  - ✖ step 3: `storage_dir 変更後に new dir の変更が検知されなかった`
    `(StorageDirChanged の infrastructure subscriber = watcher 再起動が未実装)`
  - ✖ step 4: `storage_dir 変更後に new dir の新規 .md が検知されなかった`
    `(new dir watcher の Created 経路が未稼働)`
- **非空虚性**: step 1 が step 3 / 4 と同一の DOM 観測機構（外部 `.md` 上書き → watcher →
  `notes-changed` → `list_notes` → DOM 更新）を old dir で PASS させている。よって step 3 / 4 の
  RED は「ハーネスが壊れている」ではなく「new dir が監視されていない」に起因する。
  また step 2 の PASS により `update_settings` → `StorageDirChanged` → UI subscriber は正常で、
  欠けているのは infrastructure subscriber のみと切り分けられる
- **参考（`list_notes` の非識別性）**: `list-feed/commands.rs` は毎回 `settings.json` を解決して
  disk を再 hydrate するため、`list_notes` を手動 invoke すれば watcher が壊れていても new dir の
  内容が見える（s21 で実証済み）。よって本 scenario は `list_notes` を呼ばず、`notes-changed` 駆動の
  DOM 自動更新のみで判定する

## production 変更 {#production-change}

**なし（0 file）**。本ワーカーは RED-first の契約確立のみを行い、GREEN 化の production 実装は
次ワーカーに委ねる（orchestrator 指示）。`git status` は新規 scenario ディレクトリのみで、
`apps/` 配下に差分はない。

GREEN 化に必要な production 変更（次ワーカー向けメモ、spec.md#impl-notes 前提 5）:

1. `StorageDirChanged` の Infrastructure 層 subscriber を追加し、`WatcherState.handle` を
   旧 watcher の Drop → new dir の `start_watcher` で差し替える
   （`note_feed/slices/detect_external_changes/commands.rs` 近傍）
2. 再起動失敗時は retry（最大 3 回、1 秒間隔）、全失敗時はユーザにアプリ再起動を促す
   （`detect-external-changes.md#errors` / `#notes`）
3. subscriber は `settings:storage_dir_changed`（TauriEventBus）とは別経路である点に注意 —
   本 scenario は `update_settings` の `StorageDirChanged` を起点とする

## 既知の制約 {#known-issues}

- **旧 watcher 停止（`WatcherHandle` の Drop）は E2E で直接観測できない** — in-process の
  後始末であり frontend に露出する signal を持たない。E2E の判定は「new dir の変更が検知されること」
  に置く。旧 watcher 停止の直接的検証は slice / unit の領分（spec.md#test-points TP5）
- **retry（最大 3 回、1 秒間隔）/ 全失敗時の再起動要求は E2E 観測対象外** — watcher 起動を
  失敗させる外部条件を E2E から注入できない（spec.md#test-points）
- **フィードの「部分更新回数」は観測しない** — `list_notes` は毎回 disk から全件 re-hydrate する
  ため、E2E は最終状態（new dir の内容への切替）のみを観測する（s21 と同じ制約）
- **`StorageDirChanged` の event payload（`old_dir` / `new_dir`）は E2E で assert しない** —
  frontend には `settings:storage_dir_changed`（payload あり）と `notes-changed`（payload なし）が
  届くのみで、domain event payload は露出しない。`settings.json` の内容と UI 状態で間接検証する
- **domain 文書の Timing 表現との関係** — `domain-events.md#storage-dir-changed-timing` は
  「Note Capture / Note Feed は再起動まで反映しない」と記す一方、S22 は watcher を即時再起動する。
  本 scenario は後者（S22 の Then）に従い、storage_dir 変更直後はフィード据え置き（step 2 = S11 回帰）、
  **new dir の外部変更が起きた時点で** `notes-changed` 経由で feed が new dir に切り替わる、という
  挙動を観測する。この「変更イベント受信後の feed 切替」は U層/B界の意図的な分割（I-S4 の据え置きは
  storage_dir 変更そのものに対する規定）と解釈した。domain 上流で明示したい場合は `/ori-propose` の
  余地がある（proposal は本 phase では作成しない）
- **テストは Gherkin のステップ順に依存する**（step 1 で old dir 変更 → step 2 で storage_dir 変更
  → step 3 で new dir 変更 → step 4 で new dir 作成）。`maxInstances: 1` + mocha 逐次実行が前提
  （s1〜s21 と同方針）

## GREEN 化（本 scenario の実装） {#green}

本 scenario の核（`StorageDirChanged` の Infrastructure 層 subscriber = watcher 再起動）を実装し、
E2E を **4 passing / 0 failing（~3.5s）** にした。テストコード（assertion）は一切改変していない。

### cross-BC 配線の方式（選定理由） {#cross-bc-wiring}

`StorageDirChanged` は User Preferences BC の `SettingsEvent`（`user_preferences::shared::types`）で、
Note Feed BC の `DomainEvent` / `EventBus` とは別系統。cross-BC のため Note Feed が
`SettingsEvent` 型を直接 import するのは境界違反になる。そこで **publisher が外部に公開している
Tauri event `settings:storage_dir_changed`（`TauriEventBus` が emit）を cross-BC 契約面として使い、
composition root（`lib.rs::setup`）で Rust 側 `app.listen` subscriber を登録する方式**を採った。

- 実装: `note_feed/slices/detect_external_changes/commands.rs::register_storage_dir_changed_subscriber`
  を `lib.rs` の `.setup()` で 1 回だけ呼ぶ
- `app.emit` は JS listener と Rust listener の両方に配送される（Tauri v2 の `Listeners::emit` が
  `EventTarget::App` の Rust handler にも match する — tauri 2.11.3 `src/event/listener.rs` で確認）。
  よって `update_settings` の publish がそのまま Note Feed 側 subscriber を起動できる
- 代替案との比較:
  - **frontend から `start_file_watcher` を再 invoke**: 変更は最小だが、UI 層が watcher 再起動を
    駆動することになり、domain-events.md の「Infrastructure 層 subscriber」と層が食い違う
  - **`update_settings` command から note_feed を直接呼ぶ**: User Preferences → Note Feed の
    逆方向依存が生じ、publisher が subscriber を知る形になるため却下
  - **共有 EventBus への統合**: 2 つの EventBus 型（`SettingsEvent` / `DomainEvent`）の統合は
    影響範囲が大きく回帰リスクが高いため却下
- subscriber の handler では `WatcherState.handle` を差し替える（旧 `WatcherHandle` の Drop が
  先に走る = C-DEC7 の「停止 → 起動」順序を RAII で保証）。失敗時は最大 3 回・1 秒間隔で retry
- 観測点は E2E のとおり「new dir の外部変更（Modify / Create）が手動操作なしで UI に反映されること」

### production 変更ファイル {#green-production-change}

- `apps/promptnotes/src-tauri/src/note_feed/slices/detect_external_changes/commands.rs`
  — `start_watcher_for_current_settings()` 抽出 / `restart_watcher_with_retry()` /
  `register_storage_dir_changed_subscriber()` 追加
- `apps/promptnotes/src-tauri/src/lib.rs` — `.setup()` で subscriber を登録（composition root）
- `apps/promptnotes/src-tauri/src/note_feed/slices/detect_external_changes/tests.rs`
  — `watcher_restart_drops_old_dir_and_detects_new_dir`（C-DEC7 / C-DEC11 / TP-WL4）追加

### GREEN 実測 {#green-measurement}

- S22: `bun run build:test` → `wdio run wdio.conf.ts` → **4 passing / 0 failing（~3.5s）**
  - ✓ step 1（old dir watcher 稼働）/ ✓ step 2（S11 回帰）/ ✓ step 3（new dir Modify 検知・核）
    / ✓ step 4（new dir Create 検知）
- 回帰 E2E: s11 3 passing / s16 4 / s17 4 / s18 4 / s21 5（いずれも 0 failing）
- frontend unit: `bun run test` → 164 passed
- Rust: `cargo test detect_external_changes` → 21 passed。全体は 333 passed / 2 failed（既存
  time-bomb `list_feed::tp_f5_last_7_days` / `tp_f6_and_composition` のみ、本変更と無関係）

### 付随要件の範囲 {#green-scope}

- retry（最大 3 回・1 秒間隔）は実装済み（`restart_watcher_with_retry`）。ただし watcher 起動失敗を
  E2E から注入できないため観測対象外（#known-issues のとおり）
- 全 retry 失敗時の「アプリ再起動を促す」は専用 UX を新設せず、S11 の `restart-prompt` に委譲
  （`StorageDirChanged` で常に表示されるため）。slice spec の Out of scope と整合
