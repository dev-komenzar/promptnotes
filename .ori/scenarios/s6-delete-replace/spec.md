---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s6-delete-replace
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

# s6-delete-replace — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s6-delete-replace phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

Note A を削除して Undo トーストが表示されている最中に Note B を削除したとき、
**既存の A のトーストが置換されずに B のトーストが縦パイルの上に積まれ**、
かつ **A / B それぞれの Undo が互いに独立して機能する**ことを検証する E2E シナリオ。

- 対応 workflow: `domain/workflows/delete-note.md`（連続削除時の挙動 / S6 改訂）+
  `domain/workflows/restore-deleted-note.md`（per-toast 独立性 / S6 改訂の核）
- 対応 page: `page-main`（Toast region）

> domain/validation.md#s6-delete-replace より:

- `t0`: A の `NoteDeletedToTrash` 発行 / A の Toast を画面下部に表示 / Undo スタック `[DeletedNote(A)]`
- `t1`: B の `NoteDeletedToTrash` 発行 / B の Toast を **A の上に積む**（縦パイル、最新が上）/
  Undo スタック `[DeletedNote(A), DeletedNote(B)]` / **A の Toast は維持（置換されない、独立 TTL）**
- `t2`: A の Toast の Undo クリック → A が復元 / Undo スタックから A を除去 `[DeletedNote(B)]` /
  B の Toast は表示維持（有効期間内）
- `t3`: B の Toast の Undo クリック → B が復元 / Undo スタックから B を除去 `[]`
- 補足: A の Toast が `t0 + 5s` で時間切れ消失しても A のみが破棄され、B の Toast / Undo は影響を受けない（各 Toast 独立 TTL）

## シナリオステップ {#scenario-steps}

### Given {#given}

1. Note A（ID: `20260620120000`、body: `"alpha"`）が表示されている
2. Note B（ID: `20260620130000`、body: `"bravo"`）が表示されている
3. どちらも未削除で、A / B の削除トーストは未表示
4. Undo スタックは空

### When {#when}

1. **t0**: ユーザが Note A のブロックにホバー → 削除ボタン（🗑️）をクリック
2. **t0 + 1s = t1**: A の Undo トースト表示中に、ユーザが Note B の削除ボタンをクリック
3. **t1 + 1s = t2**: A の Toast 内「元に戻す」ボタンをクリック
4. **t2 + 1s = t3**: B の Toast 内「元に戻す」ボタンをクリック

### Then {#then}

**t0（A 削除直後）**:
- `Note::delete_to_trash()` が実行される
- event **NoteDeletedToTrash** `{ note_id: A, original_path: <storageDir>/20260620120000.md, deleted_at: t0 }` 発行
- NoteFeed: 表示から Note A が除外される
- UI: 画面下部に A の「元に戻す」トーストが 1 件表示される（有効期間 5 秒）
- Application service: Undo スタック `[DeletedNote(A)]`

**t1（B 削除直後）**:
- event **NoteDeletedToTrash** `{ note_id: B, ... }` 発行
- NoteFeed: 表示から Note B が除外される（A は既に非表示）
- UI: **B のトーストが A の上に積まれる（縦パイル、最新が画面上側）**
- UI: **A のトーストは置換されず表示維持**（独立 TTL）
- Application service: Undo スタック `[DeletedNote(A), DeletedNote(B)]`

**t2（A の Undo 実行）**:
- `DeletedNote::restore()` が A について実行される
- event **NoteRestoredFromTrash** `{ note_id: A, restored_at: t2 }` 発行
- A の `.md` が原パス `<storageDir>/20260620120000.md` に復帰
- NoteFeed: 表示に A が再登場
- UI: A のトーストが閉じ、**B のトーストは表示維持**
- Application service: Undo スタック `[DeletedNote(B)]`

**t3（B の Undo 実行）**:
- `DeletedNote::restore()` が B について実行される
- event **NoteRestoredFromTrash** `{ note_id: B, restored_at: t3 }` 発行
- B の `.md` が原パス `<storageDir>/20260620130000.md` に復帰
- NoteFeed: 表示に B が再登場
- UI: B のトーストが閉じ、トーストスタックが空になる
- Application service: Undo スタック `[]`

> domain/ui-fields/screen-1.md#cross-toast-display より:
> 「削除ごとに新規 Toast を発行し、縦パイルで複数表示」「新しい Toast は **上に積む**
> （最新が画面上側に表示される）」「古い Toast は下に押し下げられ、表示は維持される」
> 「各 Toast の `screen-1-toast-undo` は **その Toast の有効期間中のみ enable**」

> domain/aggregates.md#notes-undo より:
> 「各 DeletedNote は対応する Toast と 1:1 対応し、独立した有効期間 (TTL) を持つ」
> 「Toast 消失 / Undo 成功 / 明示クローズ のいずれかで該当 DeletedNote のみスタックから除去」

> domain/workflows/delete-note.md#notes より:
> 「連続削除時の挙動（S6 改訂）: 各 `DeletedNote` は独立に保持される。
> 対応する Toast が時間切れ / 明示クローズ / Undo 成功 のいずれかで該当要素が
> スタックから除去されるが、他の DeletedNote は影響を受けない」

> domain/workflows/restore-deleted-note.md#notes より:
> 「**per-toast 独立性**: 1 つの Toast の Undo 実行は他の Toast / DeletedNote に影響を与えない」

## テスト観点 {#test-points}

- **t0 — A 削除**: `<storageDir>/20260620120000.md` が存在しない（in-app trash へ移動）/ NoteFeed の A ブロックが消える / A の削除トーストが 1 件表示される
- **t1 — B 削除でスタック**: `<storageDir>/20260620130000.md` が存在しない / 削除トーストが **2 件** 表示される（A が置換されていない）/ トーストスタックに A・B 両方の `data-toast-id` が存在する
- **t1 — 積み上げ順（最新が上）**: B のトーストの画面上の Y 座標が A のトーストより **小さい（上）** こと。逆向き（`flex-col-reverse` による最新が下）は spec 違反として fail させる
- **t2 — A の Undo（per-toast 独立）**: A の `.md` が原パスに復元され body が `"alpha"` / A のトーストのみ消え **B のトーストは残る** / NoteFeed に A が再登場
- **t3 — B の Undo**: B の `.md` が原パスに復元され body が `"bravo"` / B のトーストが消えトーストスタックが空になる / NoteFeed に B が再登場
- **event 発行順序**: `NoteDeletedToTrash(A)` → `NoteDeletedToTrash(B)` → `NoteRestoredFromTrash(A)` → `NoteRestoredFromTrash(B)`（NoOpBus 制約により E2E では観測不可。slice unit test で担保 — `notes.md#event-verification-policy`）
- **独立 TTL**: 一方のトーストの Undo は他方のトースト / `.md` 状態に影響しない（t2 直後に B が残ること・B の `.md` が absent のままであることで間接検証）

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
  - 積み上げ順は `browser.execute` で各トーストの `getBoundingClientRect().top` を取得して比較する
    （DOM 順ではなく**画面上の Y 座標**で「最新が上」を判定する）
- **特定トーストの操作**: トースト / Undo ボタンは `data-toast-id="<note-id>"` を持つため、
  `[data-testid="screen-1-toast-undo"][data-toast-id="<id>"]` で特定の NoteId の Undo を狙う
- **削除ボタン操作**: delete ボタンは hover 時のみ操作可能（`opacity-0` / `pointer-events-none`）なため、
  `dispatchEvent(new MouseEvent('click'))` で直接 dispatch する
  （WebKitWebDriver の `moveTo` は `:hover` を安定発火しない）。対象ブロックは
  `[data-block-id="<id>"] [data-testid="screen-1-block-delete"]` で特定する
- **待機戦略**: 削除 / Undo 後に `browser.pause(1000)` で FS・UI 反映を待つ。各 Undo は
  各トーストの有効期間（5s）内に実行する（t0→t3 で最大 3s + 各操作 1s pause、TTL 内）
- **テストデータのクリーンアップ**: `wdio.conf.ts` の `onComplete` が temp `storageDir` を
  `rmSync` で削除する
- **production 側の前提（本 scenario が検証する不変条件）**: `ToastRegion.svelte` のトーストスタックは
  「最新が画面上側」を満たす必要がある（`screen-1.md#cross-toast-display`）。スタックの DOM 順は
  store の `entries`（新しい順 = 先頭）と一致させ、描画はそれを反転させない
