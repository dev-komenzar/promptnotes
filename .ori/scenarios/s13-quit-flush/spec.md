---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s13-quit-flush
      hash: 4a02c17cc025
    - path: domain/workflows/flush-note.md#flush-note
      hash: 06ace0dff2ff
    - path: domain/workflows/auto-save-note.md#auto-save-note
      hash: 642c5094fd1a
    - path: domain/aggregates.md#note-aggregate
      hash: 56f7a54a8ab2
    - path: domain/domain-events.md#note-body-edited
      hash: 71db66eafe03
    - path: domain/ui-fields/screen-1.md#screen-1
      hash: 4bc0f83f71f3
---

# s13-quit-flush — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s13-quit-flush phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

アプリ quit 時に **debounce timer 待ちの編集（pending_body）を取りこぼさず永続化する** ことを検証する scenario。
ユーザが編集直後（500ms debounce 未満）に quit した場合、AutoSave の debounce timer は fire しないため、
quit シグナルを契機に **Flush 経路**（`flush-note` workflow, trigger=AppQuit）で全 EDITING Note を順次永続化し、
quit 完了までそれを待つ（最大欠損 500ms を許容、event-storming Q4 補足）。

- 対応 workflow: `domain/workflows/flush-note.md#flush-note`（debounce timer を待たず即時永続化。
  トリガーは Q4 決定の 3 種。AppQuit は全 EDITING Note に flush-note を発行。順序は処理順、並列性なし）
- 対応 workflow: `domain/workflows/auto-save-note.md#auto-save-note`（通常経路の 500ms debounce。quit 時に
  fire しない場合の欠損を Flush が補償する。cancel 責務は flush-note 側）
- 対応 aggregate: `domain/aggregates.md#note-aggregate`（`Note::edit_body` 経由の永続化。I-N1 / I-N3 / I-N4 / I-N8。
  冪等性ガードは body バイト等価比較）
- 対応 event: `domain/domain-events.md#note-body-edited`（body 変化時のみ 1 回発行。quit 時の連続 Flush は
  複数 Note の NoteBodyEdited を連続発行しうる。購読側 NoteFeed は 1 個ずつ処理しても結果が同じ = 冪等）
- 対応 page: `page-main`（`screen-1` の EDITING ブロックが pending flush を保持する composition root）

> domain/validation.md#s13-quit-flush より:

- Given: Note A, B, C が EDITING 状態（複数ブロック編集を許容する場合の想定）／いずれも AutoSave debounce
  timer 中
- When: ユーザが Cmd+Q で quit
- Then:
  - quit シグナル受信 → 全 EDITING ブロックを Flush
  - `Note::edit_body` を A, B, C の順に同期実行
  - event **NoteBodyEdited** が A, B, C 順に連続発行
  - NoteFeed の購読は冪等（1 個ずつ処理しても結果は同じ）
  - quit 完了まで永続化を待つ（最大欠損 500ms を許容）

> domain/workflows/flush-note.md#notes より:
> 「**AppQuit trigger** はすべての EDITING Note に対して flush-note を発行する（S13: 連続 Flush）。
> 順序は処理順（並列性は持たない）」「永続化が完了するまで quit を待つ。最大欠損 500ms を許容」

> domain/aggregates.md#note-aggregate-invariants より:
> 「**I-N4**: `body` を変更する操作は `updatedAt` を現在時刻（秒精度）に更新する」

> domain/domain-events.md#notes-event-ordering より:
> 「例外: アプリ quit 時の Flush は複数 Note の `NoteBodyEdited` を連続発行する可能性。購読側 (NoteFeed) は
> 1 個ずつ処理しても結果が同じ（冪等）であるべき」

## シナリオステップ {#scenario-steps}

### Given {#given}

1. `storage_dir`（隔離 temp dir）に Note A, B, C が seed されている。各 Note は既知の body を持つ
2. ユーザが Note を編集し、AutoSave debounce timer（500ms）が **まだ fire していない** pending_body を抱えた
   状態にある（= quit 時に取りこぼすとデータ欠損になる状態）
3. Tauri App が起動済み

> **I-PM10（`screen-1` 由来: 同時に EDITING なブロックは高々 1 つ）** により、UI 操作で同時に複数ブロックを
> EDITING にはできない。したがって validation の「A, B, C 全てが EDITING」は UI 上は再現できない。
> 連続 Flush（A, B, C 順次実行）は **Tauri 境界**（`flush_note` invoke, trigger=app_quit）で再現する
> （[#impl-notes](#impl-notes) 参照）。pending_body を抱えた単一 Note の quit flush は実 UI でも再現する。

### When {#when}

1. pending_body を抱えた状態でアプリ quit 相当の Flush が発生する
   - UI pending ケース: 実ブロックを EDITING にし、debounce 未満で `flush_note(app_quit)` を発火する
   - 連続 Flush ケース: A → B → C の順に `flush_note(app_quit)` を **順次 await** する

### Then {#then}

- debounce timer（500ms）を待たずに pending_body が `.md` に永続化される（`updated_at` 更新、I-N4）
- 連続 Flush は A → B → C の順に **逐次** 実行され、各ステップで当該 Note のみが更新される
  （次に進む前に永続化が完了している = 並列性なし）
- body が変化していなければ Flush は no-op（永続化されず、event 非発行 = 冪等性ガード）
- 全 Note の永続化が完了してから quit が完了する（最大欠損 500ms を許容）

## テスト観点 {#test-points}

- **TP1（核）— quit 時の pending body を欠損させない**: 実ブロックを EDITING にし debounce 中の pending_body を
  抱えた状態で `flush_note(app_quit)` を発火 → 500ms 未満で `.md` に pending_body が書かれ `updated_at` が
  更新される。debounce timer が後から fire しても同じ body のため永続化は重複しない（冪等）
- **TP2 — 連続 Flush の順序（A → B → C 逐次）**: A, B, C の順に `flush_note(app_quit)` を順次 await し、
  各ステップ直後に (i) 当該 Note が更新済み (ii) 後続 Note は未更新 であることを観測する。逐次実行
  （前の永続化完了後に次が走る）を確認
- **TP3 — 冪等性（body 不変で no-op）**: 既に flush 済みの Note に対し同じ body で `flush_note(app_quit)` →
  `outcome = no_op`。`.md` は不変
- **TP4 — quit 完了時の全 Note 反映**: TP2 完了後、A, B, C すべての `.md` が更新後の body を持つ
- **TP5 — trigger が app_quit として受理される**: `flush_note` command が `trigger: "app_quit"` を受理し
  `FlushOutcome::flushed` を返す（Tauri 境界 = specta bindings 経由 invoke）
- **NoteBodyEdited / NoteFeed 冪等 / CloseRequested orchestration**: 本 scenario の E2E は Tauri 境界の
  `flush_note` invoke と実 UI pending で検証する。`NoteBodyEdited` の購読冪等性・`pendingFlushRegistry`
  の順序・`onCloseRequested` intercept は既存 unit test（`pending-flush.test.ts`, flush-note Rust tests）と
  production code が担保する（[#impl-notes](#impl-notes) の制約参照）

## 実装ノート {#impl-notes}

- **runner**: `wdio`（優先チェーン step 2 — manifest に `runner:` 明示なし。参加 app `promptnotes` は
  `local` 系で `runtime.runner = wdio`。`.ori/architecture.md` の `scenario_test_runner: wdio` とも一致）
- **infrastructure**: `promptnotes` のみ（`mode: local`）。compose-service 系 app が参加しないため
  docker-compose は不要
- **runner config**: `wdio.conf.ts` が `/ori-generate` により生成される。`tauri:options.application` は
  runtime block の `binary` = `apps/promptnotes/src-tauri/target/debug/app`（build-then-test）
- **quit トリガーの再現制約（重要）**: 本 app は S13 orchestration を frontend で実装している
  （`PageMain.svelte` の `onCloseRequested` → `preventDefault` → `pendingFlushRegistry.flushAll('app_quit')`
  → `window.destroy()`）。しかし Tauri v2 capability（`src-tauri/capabilities/default.json`）には
  `core:window:allow-close` が無く（`core:window:allow-destroy` のみ）、E2E から
  `getCurrentWindow().close()` で CloseRequested を JS 発火できない。`destroy()` は許可されているが
  flush を伴わず app を即終了するため、quit flush の検証には使えない。よって **app_quit 経路の検証は
  `browser.tauri.execute(({ core }) => core.invoke('flush_note', { ..., trigger: 'app_quit' }))` による
  Tauri 境界 invoke で代替**する（`@wdio/tauri-service` v1.4.0 の公式 API。`tauri-plugin-wdio` は debug
  build で有効）。この代替方法は notes.md にも明記する
- **pending_body の再現**: TP1 は実 UI でブロックを EDITING にし debounce 中の状態を作る。pending_body は
  seed body + 追加入力で決定的に決まる（テストはその文字列を invoke に渡す）
- **A → B → C 逐次性の観測**: 各 `flush_note` を `await` し、直後に `node:fs` で各 `.md` を読み、
  「当該 Note のみ更新済み・後続は未更新」を確認する。これにより並列実行（後続が先に書かれる）を検出できる
- **config dir / storage dir の隔離**: `wdio.conf.ts` の `onPrepare` が `XDG_CONFIG_HOME` / `XDG_DATA_HOME` と
  専用 `storage_dir` を temp dir に隔離する（実 user 環境を汚さない）。`settings.json` に `storage_dir` を
  seed して `resolve_storage_dir` を本 temp dir に向ける。`onComplete` が temp dir を `rmSync` で削除する
- **production 側の前提（本 scenario が満たすべき不変条件）**:
  1. `flush_note` Tauri command が `trigger: app_quit` を受理し、debounce を待たず `Note::edit_body` を永続化
     すること（`apps/promptnotes/src-tauri/src/note_capture/slices/flush_note/`）
  2. body 不変時は `FlushOutcome::no_op` を返す冪等性ガード（C-FL4）
  3. quit orchestration（`pendingFlushRegistry.flushAll('app_quit')` の逐次 await +
     `PageMain.svelte` の `onCloseRequested` intercept）は既存実装（ori-73q）
  これらが欠落すると S13 が RED になる
