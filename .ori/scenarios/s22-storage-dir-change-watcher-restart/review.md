# Review: s22-storage-dir-change-watcher-restart {#review-s22-storage-dir-change-watcher-restart}

## Pass 1 {#pass-1}

> reviewer agent (`ori-reviewer`) は spawn せず、main session の objective 検証で代替
> （orchestrator 指示: reviewer spawn がハングする既知問題があるため。s1〜s21 と同方針）。

### Syntax checks {#pass-1-syntax}

- tsc (`tsc --noEmit -p tsconfig.json`): PASS（テスト spec は 0 error。既知の
  `wdio.conf.ts TS2353 'tauri:options'` のみ — s1〜s21 と同一の WDIO v9 型定義起因、許容）
- svelte-check (`bun run check`): PASS（0 errors / 0 warnings）
- docker-compose: SKIP — compose-service 参加者ゼロ (app `promptnotes` is mode=local)。
  `generate-docker-compose.sh` も「compose-service 系参加者ゼロ — docker-compose.yml は省略」で exit 0
- E2E: **RED を実測**（`wdio run wdio.conf.ts` → **2 passing / 2 failing**, ~31s。2 回連続で同一）
  - ✓ step 1（old dir watcher 稼働の前提実測）
  - ✓ step 2（S11 回帰: 永続化 + restart-prompt + フィード据え置き）
  - ✖ step 3（核: StorageDirChanged 後の new dir 変更検知）— **本 scenario が固定する RED 契約**
  - ✖ step 4（核: new dir の新規 `.md` 検知 / Created 経路）
  - **runtime RED は本 scenario の想定結果**（後述 Production fix discovery）。review の判定は
    「spec ↔ テストコードの整合」で行う（RED 自体は FAIL 理由にしない）

### Mode checklist (local tauri) {#pass-1-mode-checklist}

| item | status | note |
|---|---|---|
| spec `runner: wdio` ↔ test 実行 ↔ wdio.conf.ts | ✅ PASS | 一致（derive chain step 2: 参加 local app の runtime.runner） |
| テストコード内に service lifecycle が無い | ✅ PASS | seed / temp dir 隔離は onPrepare、テストは node:fs 操作 + assert のみ |
| local app が docker-compose に含まれない | ✅ PASS | compose 不在が正しい |
| wdio `tauri:options.application` == runtime `binary` | ✅ PASS | `apps/promptnotes/src-tauri/target/debug/app` |
| `@wdio/tauri-service` が services に含まれる | ✅ PASS | `driverProvider: 'external'` |

### Semantic findings (main session objective verification) {#pass-1-findings}

spec ↔ テストコード ↔ domain 整合性:

- **PASS**: **step 1（Given 前提の実測）** — old dir の Note を外部上書きすると、手動操作なしで
  当該 Block の `.cm-content` が更新されることを実測。`validation.md#s22-given`「ファイルウォッチャーが
  `/old/path` を監視中」を前提として実測している。あわせて storage_dir 未変更では
  `restart-prompt` が出ないことも確認 ✓
- **PASS**: **step 2（S11 回帰）** — 境界 invoke で `storage_dir = new dir` に変更 →
  `settings.json#storage_dir` 更新、`StorageDirChanged` により `restart-prompt` 出現（I-S4）、
  フィードは old dir 3 件のまま・new dir の Note は現れないことを実測。
  `validation.md#s22-then`「UI: 再起動を促すモーダル表示（S11 と同様、I-S4）」および
  `domain-events.md#storage-dir-changed-subscribers`（UI 層）を再確認 ✓
- **PASS**: **step 3（核）の仕様 ↔ テスト整合** — `validation.md#s22-then`
  「ウォッチャー再起動後、`/new/path` の変更が検知対象になる」を、
  `[data-block-id="20260701100000"] .cm-content` が更新後 body になること（＝ manual 操作なしの
  DOM 自動更新）で検証している。`detect-external-changes.md#steps` step 4 `onFileModified` /
  `#notes` watcher 再起動の要求と整合 ✓
- **PASS**: **step 4（核・Created 経路）の仕様 ↔ テスト整合** — `detect-external-changes.md#steps`
  step 3 `onFileCreated` / `NoteFileCreatedExternally` を、new dir に新規 `.md` を作成し
  新規 Block が出現し body が反映されることで検証 ✓
- **PASS**: **`StorageDirChanged` の Infrastructure 層 subscriber を検証対象にしている** —
  `domain-events.md#storage-dir-changed-subscribers` の「**Infrastructure 層（ファイルウォッチャー）**:
  監視対象ディレクトリを `new_dir` に切り替え。旧ディレクトリの監視は停止」を本 scenario の核に据え、
  S11 が明示的に範囲外とした当該部分（`s11/spec.md#when-watcher-out-of-scope`）を引き受けている ✓
- **PASS**: **spec.md frontmatter の coherence** — `coherence.source: derived` + upstream 8 件
  （hash 付き: validation / detect-external-changes / update-settings / settings-aggregate /
  storage-dir-changed / note-file-modified-externally / note-file-created-externally / screen-2）✓
- **PASS**: **テストデータ独立性** — `mkdtempSync` の temp dir に config / data / old dir / new dir を
  隔離し `onComplete` で `rmSync`。`XDG_CONFIG_HOME` / `XDG_DATA_HOME` の分離により実 user 環境を
  汚さない。`TAURI_TEST_STORAGE_DIR` override は使わず、`settings.json` 経由で `storage_dir` を
  解決させる（override を使うと dir が固定され S22 の切替を検証できない — spec.md#impl-notes）✓
- **PASS**: **watcher 切替の因果性を step 1 が担保している（非空虚性）** — step 1 は step 3 / 4 と
  同じ DOM 観測機構（外部 `.md` 上書き → watcher → `notes-changed` → `list_notes` → DOM 更新）を
  **old dir** で実測して PASS している。step 3 / 4 の RED は「同じ機構が new dir では発火しない」
  = **watcher が new dir を監視していない**ことに帰着する（ハーネス起因ではない）✓
- **MEDIUM**: **旧 watcher 停止（`WatcherHandle` の Drop）は E2E で直接観測できない** —
  in-process の後始末であり frontend に露出する signal を持たない（TP5）。E2E の判定は
  「new dir の変更が検知されること」に置いており、旧 watcher 停止の直接的検証は slice / unit の
  領分。spec.md#test-points / notes.md#known-issues に明記済み ✓
- **MEDIUM**: **retry（最大 3 回、1 秒間隔）/ 全失敗時の再起動要求は E2E 観測対象外** —
  watcher 起動を失敗させる外部条件を E2E から注入できない。spec.md#test-points / notes.md#known-issues
  に制約として明記済み ✓
- **LOW**: **`StorageDirChanged` の event payload（`old_dir` / `new_dir`）は E2E で assert しない** —
  frontend には `settings:storage_dir_changed`（payload あり）と `notes-changed`（payload なし）が
  emit されるのみで、domain event payload は露出しない。`settings.json` の内容と UI 状態で間接検証する
  （notes.md#event-observation）✓
- **LOW**: **フィードの「部分更新回数」は観測しない** — `list_notes` は毎回 disk から全件 re-hydrate
  するため、E2E は最終状態（new dir の内容へ切替）のみを観測する（s21 と同じ制約。
  notes.md#known-issues）✓
- **LOW**: **テストは Gherkin ステップ順に依存** — step 1（old dir 変更）→ step 2（storage_dir 変更）
  → step 3（new dir 変更）→ step 4（new dir 作成）。`maxInstances: 1` + mocha 逐次実行が前提
  （s1〜s21 と同方針）✓

### Production fix discovery {#pass-1-production-fix}

**本 scenario は RED-first の契約であり、production 実装は行っていない**（orchestrator 指示:
実装は次のワーカーの責務。本ワーカーは scenario = 検証軸のみを作る）。

RED 実測（テストの非空虚性と原因）:

- `VITE_WDIO_TEST=1 bun run tauri build --debug --no-bundle`（= `bun run build:test`）で
  test build → `wdio run wdio.conf.ts`
- **結果: 2 passing / 2 failing（~31s、2 回連続で同一）**
  - ✓ step 1: old dir の外部変更が手動操作なしで DOM に反映される（watcher 稼働の前提実測）
  - ✓ step 2: `storage_dir` 変更で `settings.json` 更新 + `restart-prompt` 出現 + フィード据え置き
  - ✖ step 3: `storage_dir 変更後に new dir の変更が検知されなかった`
    `(StorageDirChanged の infrastructure subscriber = watcher 再起動が未実装)`
  - ✖ step 4: `storage_dir 変更後に new dir の新規 .md が検知されなかった`
    `(new dir watcher の Created 経路が未稼働)`
- **RED の原因（コード上の事実）**: `WatcherState` を保持する
  `note_feed/slices/detect_external_changes/commands.rs` は `start_file_watcher` /
  `stop_file_watcher` を frontend から明示的に呼ぶのみで、`StorageDirChanged` を購読する
  Infrastructure 層 subscriber を持たない。したがって `update_settings` で `storage_dir` を
  変更しても watcher は old dir を監視し続け、new dir の変更は検知されない
  （`settings:storage_dir_changed` の購読者は `PageMain` の再起動モーダルのみ）。
  GREEN にするには、`WatcherState.handle` を new dir の watcher に差し替える infrastructure
  subscriber（+ 失敗時 retry 最大 3 回 / 1 秒間隔）が必要
- **RED の非空虚性**: step 1 が同一の DOM 観測機構を old dir で PASS させているため、
  step 3 / 4 の失敗は「機構が壊れている」ではなく「new dir が監視されていない」ことに起因する。
  また step 2 の PASS により `update_settings` → `StorageDirChanged` → UI subscriber は
  正常動作していることが確認できており、欠けているのは infrastructure subscriber のみと切り分けられる
- **参考（`list_notes` の非識別性）**: `list-feed/commands.rs` は毎回 `settings.json` の
  `storage_dir` を解決して disk を再 hydrate するため、`list_notes` を手動 invoke すれば
  watcher が壊れていても new dir の内容が見える（s21 で実証済み）。したがって本 scenario は
  `list_notes` を呼ばず、**`notes-changed` 駆動の DOM 自動更新のみ**で判定している
  （spec.md#test-points / notes.md#event-observation）

production への正味の変更は **0 file**（`git status` は新規 scenario ディレクトリのみ。
`apps/` 配下の変更なし）。

回帰確認:

- frontend unit test: 本ワーカーでは production 変更ゼロのため追加の `bun run test` は実行しない
  （既存 S11 / S21 で担保）
- E2E: s22 単独で 2 passing / 2 failing（RED 契約）。s17 / s18 / s21 等の既存 E2E は
  production 無変更のため影響なし

### Disposition {#pass-1-disposition}

- review の判定対象は「spec ↔ テストコードの整合」。spec.md の 4 テスト観点（TP1〜TP4）が
  テストの step 1〜4 に 1:1 で対応し、validation の Gherkin（Given / When / Then）と
  `detect-external-changes` workflow の要求を満たしている ✓
- runtime RED（step 3 / 4 の 2 failing）は **本 scenario の想定結果**であり、FAIL 理由にしない。
  RED の事実と原因（`StorageDirChanged` の infrastructure subscriber / watcher 再起動が未実装）は
  本 review.md と notes.md#red-contract に明記した。**次ワーカーはこの RED を GREEN にする
  production 実装を追加する**
- 指摘は MEDIUM / LOW のみで、いずれも E2E 観測の原理的制約であり spec.md / notes.md に明記済み
- production 変更は 0 file
- Verdict: **PASS**

<!-- verdict=PASS -->
