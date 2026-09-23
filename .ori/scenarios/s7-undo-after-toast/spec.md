---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s7-undo-after-toast
      hash: 4a02c17cc025
    - path: domain/ui-fields/screen-1.md#cross-toast-display
      hash: 4bc0f83f71f3
    - path: domain/aggregates.md#notes-undo
      hash: 56f7a54a8ab2
    - path: domain/workflows/delete-note.md
      hash: b727f18ad614
    - path: domain/workflows/restore-deleted-note.md
      hash: e32a07cd279b
---

# s7-undo-after-toast — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s7-undo-after-toast phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

Note A を削除して Undo トーストが表示されてから **Toast の有効期間（仮 5 秒）が経過して
Toast が消失した後**に、A に対する復元 API を試行しても **`NoUndoAvailable` で reject
される**ことを検証する E2E シナリオ。

- 対応 workflow: `domain/workflows/delete-note.md`（Toast 有効期間 / per-element TTL）+
  `domain/workflows/restore-deleted-note.md`（`NoUndoAvailable` 二重防御 / per-toast 独立性）
- 対応 page: `page-main`（Toast region / Block delete）

> domain/validation.md#s7-undo-after-toast より:

- `t1`（`t0 + 5s` = Toast 有効期間）で A の Toast が消失
- `t2`（`t1 + 1s`）に何らかの方法で A に対する復元 API 呼び出し試行
- `t1`: A の Toast 消失と同時に application service が Undo スタックから A を除去 → `[]`
- `t2`: 復元 API は **対応する DeletedNote がスタックに無い** ため reject
  （event **NoteRestoredFromTrash** は発行されない / workflow restore-deleted-note は
  `NoUndoAvailable` を返す）
- A はゴミ箱に残る
- 補足: 他の Toast（例: B の Toast が同時に存在）がある場合、B の Undo は影響を受けず
  引き続き有効（per-toast 独立性）

## シナリオステップ {#scenario-steps}

### Given {#given}

1. Note A（ID: `20260620120000`、body: `"alpha"`）が表示されている
2. Note B（ID: `20260620130000`、body: `"bravo"`）が表示されている
3. A / B の削除トーストは未表示
4. Undo スタックは空

### When {#when}

1. **t0**: ユーザが Note A のブロックにホバー → 削除ボタン（🗑️）をクリック
2. **t0 + 3s**: A の Toast 表示中に、ユーザが Note B の削除ボタンをクリック
3. **t0 + 5s = t1**: A の Toast が有効期間切れで消失（B の Toast はまだ有効）
4. **t1 + 1s = t2**: A に対する復元 API 呼び出しを試行する
5. **t2 直後**: B の Toast 内「元に戻す」ボタンをクリックする

### Then {#then}

**t1（A の Toast 消失）**:
- UI: A の Toast が画面から消える（A の `data-toast-id` が DOM から消える）
- Application service: A に対応する `DeletedNote` が Undo スタックから除去される（TTL 経過）
- A の `.md` は原パスに復帰しない（ゴミ箱に残ったまま）

**t2（A への復元 API 試行）**:
- workflow restore-deleted-note は **`NoUndoAvailable`** を返す（reject）
- event **NoteRestoredFromTrash** は発行されない
- A の `.md` は `<storageDir>/trash/20260620120000.md` に残ったまま（原パスには復帰しない）

**t2 直後（B の Undo、per-toast 独立性）**:
- B の Toast は A の消失後も表示維持されている
- `DeletedNote::restore()` が B について実行され、event **NoteRestoredFromTrash** が発行される
- B の `.md` が原パス `<storageDir>/20260620130000.md` に復帰（body=`"bravo"`）
- NoteFeed: 表示に B が再登場する
- UI: B の Toast が閉じ、トーストスタックが空になる
- A の Undo スタック除去は B に影響しない（per-toast 独立性）

> domain/ui-fields/screen-1.md#cross-toast-display より:
> 「各 Toast は独立した `DeletedNote` を保持し、それぞれ独立した有効期間を持つ」
> 「各 Toast の消失条件: 仮 5 秒経過 / 明示クローズ / 対応する Undo クリック
>   — いずれかで対応する `DeletedNote` の Undo 保持を破棄」
> 「各 Toast の `screen-1-toast-undo` は **その Toast の有効期間中のみ enable**」
> 「期限切れ後はその 1 つの Undo のみが reject される（他の Toast は影響しない）」

> domain/aggregates.md#notes-undo より:
> 「各 DeletedNote は対応する Toast と 1:1 対応し、独立した有効期間 (TTL) を持つ」
> 「Toast 消失 / Undo 成功 / 明示クローズ のいずれかで該当 DeletedNote のみスタックから除去」

> domain/workflows/restore-deleted-note.md#errors より:
> "`NoUndoAvailable` — 指定 `NoteId` の `DeletedNote` が Undo スタックに存在しない
> （Toast 消失後、または別 workflow で既に Undo 済み、S7）"

> domain/workflows/restore-deleted-note.md#notes より:
> 「**Toast UI 側の guard**: 各 Toast は対応する DeletedNote の TTL に同期して
> Undo ボタンを disable する。本 workflow の `NoUndoAvailable` は二重防御」
> 「**per-toast 独立性**: 1 つの Toast の Undo 実行は他の Toast / DeletedNote に
> 影響を与えない」

> domain/workflows/delete-note.md#dependencies より:
> 「`UndoStack` — `Vec<DeletedNote>` を保持する application service
> （TTL 管理付き、各要素ごとに個別タイマー）」

## テスト観点 {#test-points}

- **t0 — A 削除**: `<storageDir>/20260620120000.md` が存在しない（`<storageDir>/trash/` へ移動）/
  NoteFeed の A ブロックが消える / A の削除トーストが 1 件表示される
- **t0+3s — B 削除**: A の Toast 表示中に B を削除 → トーストが **2 件**（A 置換なし）
- **t1 — A の Toast 消失**: A の `data-toast-id` を持つトーストが DOM から消える（5s 経過）/
  B の Toast は表示維持 / A の `.md` は依然として原パスに不在（ゴミ箱のまま）
- **t2 — 復元 API の reject（本 scenario の核）**: `restore_deleted_note` を A について
  呼び出すと **`no_undo_available`** で reject される / A の `.md` は原パスに復帰しない /
  `<storageDir>/trash/20260620120000.md` が残存
- **per-toast 独立性**: A の Undo が reject された後も、B の Toast 内 Undo は成功し
  B の `.md` が原パスに復帰（body=`"bravo"`）/ B の Toast が閉じる / NoteFeed に B が再登場
- **event 非発行（間接）**: A については `NoteRestoredFromTrash` が発行されない（`.md` が
  復帰しないことで間接検証。NoOpBus 制約により E2E では event 自体は観測不可 —
  `notes.md#event-verification-policy`）

## 実装ノート {#impl-notes}

- **runner**: `wdio`（優先チェーン 2 — 参加 `local` 系 app `promptnotes` の `runtime.runner` = wdio。
  manifest に `runner:` 明示なし。app `promptnotes` は `mode: local` のため compose に含めず、
  ビルド済み binary を runner が直接起動）
- **infrastructure**: `promptnotes` のみ（local mode）。docker-compose は不要（compose-service 系 app なし）
- **runner config**: `wdio.conf.ts` が `/ori-generate` により生成される
- **Note A / B の準備**: `wdio.conf.ts` の `onPrepare` が `TAURI_TEST_STORAGE_DIR` に
  Note A（`20260620120000`, body=`"alpha"`）と Note B（`20260620130000`, body=`"bravo"`）を seed する
  （app の unquoted inline 形式）
- **アサーション戦略**:
  - ファイルの存在/不在・内容は**テストプロセスの `node:fs`**（module スコープ）で検証する
  - NoteFeed / トーストの DOM 状態は `[data-testid]` セレクタで検証する
  - A の Toast 消失は `browser.waitUntil` で `[data-testid="screen-1-toast"][data-toast-id=A]` が
    0 件になるまで待機する（timeout を 5s TTL より十分大きく取る）
- **復元 API 呼び出し試行（t2）**: Toast 消失後は Undo ボタンが DOM に無いため、
  `browser.executeAsync` から `window.__TAURI_INTERNALS__.invoke('restore_deleted_note', { noteId })`
  を直接呼び出す。reject payload の `kind` が `no_undo_available` であることを assert する
  （UI の disable guard と domain の二重防御のうち、後者を直接検証する）
- **削除ボタン操作**: delete ボタンは hover 時のみ操作可能（`opacity-0` / `pointer-events-none`）なため、
  `dispatchEvent(new MouseEvent('click'))` で直接 dispatch する
  （WebKitWebDriver の `moveTo` は `:hover` を安定発火しない）。対象ブロックは
  `[data-block-id="<id>"] [data-testid="screen-1-block-delete"]` で特定する
- **待機戦略**: 削除 / Undo 後に `browser.pause(1000)` で FS・UI 反映を待つ。A の Toast 消失後に
  Rust 側 Undo スタックの TTL（5s）経過を確実にするため 1s の追加 pause を挟む。B の Undo は
  B の Toast 有効期間（5s）内に実行する（A 消失後 ~1.7s + 1s pause で B の TTL 内）
- **テストデータのクリーンアップ**: `wdio.conf.ts` の `onComplete` が temp `storageDir` を
  `rmSync` で削除する
- **production 側の前提（本 scenario が検証する不変条件）**: application service の
  `UndoStack` は per-element TTL（仮 5 秒、frontend Toast と同期）を持ち、TTL 経過後の
  `restore_deleted_note` は `NoUndoAvailable` を返す（`delete-note.md#dependencies` /
  `restore-deleted-note.md#errors`）。TTL 未実装の場合、Toast 消失後も Undo が成功してしまい
  S7 は RED になる
