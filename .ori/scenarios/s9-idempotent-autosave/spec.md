---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s9-idempotent-autosave
      hash: 4a02c17cc025
    - path: domain/workflows/auto-save-note.md#auto-save-note
      hash: 642c5094fd1a
    - path: domain/aggregates.md#note-aggregate
      hash: 56f7a54a8ab2
    - path: domain/domain-events.md#note-body-edited
      hash: 71db66eafe03
---

# s9-idempotent-autosave — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s9-idempotent-autosave phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

既存 Note を EDITING 状態にしても本文が変化していなければ、AutoSave 経路は
**application service レベルで早期 return し、何も永続化しない**ことを検証する scenario。

- 対応 workflow: `domain/workflows/auto-save-note.md`（`compareBody` → `BodyDiff::Unchanged` →
  早期 return。event 非発行）
- 対応 aggregate: `domain/aggregates.md#note-aggregate`（I-N4: body 変更操作のみ `updatedAt` を
  更新する。Aggregate 自身は呼ばれれば素朴に更新する）
- 対応 event: `domain/domain-events.md#note-body-edited`（永続化成功時のみ発行。no-op では発行しない）
- 対応 page: `page-main`（Block をクリックして EDITING 状態に入る）

> domain/validation.md#s9-idempotent-autosave より:

- Given: Note A (`body="hello"`, `updatedAt=t0`)
- When:
  1. ユーザがブロックをクリックして EDITING 状態に
  2. body を一切変更せず 500ms 経過
- Then:
  - AutoSave 経路は body 変化を検知 → **何もしない**
  - `Note::edit_body` は呼び出されない
  - event **NoteBodyEdited** は発行されない
  - `updatedAt = t0` のまま
- 補足: 「同一 body 編集」を application service レベルで弾く方針。
  Note Aggregate 自体は呼ばれれば `updatedAt` を更新する（I-N4 通り）。

> domain/workflows/auto-save-note.md#steps より:
> 「3. `compareBody: (Note, NoteBody) → BodyDiff`（**S9 の application service レベル冪等性ガード**）」
> 「4. `branchOnDiff:` `Unchanged` → 早期 return（event 非発行）」

> domain/domain-events.md#note-body-edited-trigger より:
> 「`Note::edit_body(new_body, now)` の永続化成功時。発行経路は AutoSave (debounce) または
> Flush (focus 喪失 / blur / quit)。キー入力ごとには発行しない（永続化が完了して初めて event）」

## シナリオステップ {#scenario-steps}

### Given {#given}

1. Note A（ID: `20260620120000`、body: `"hello"`、`updatedAt: 20260620120000` = t0）が表示されている
2. A の `.md` ファイルは `TAURI_TEST_STORAGE_DIR` に seed 済み
3. Tauri App が起動済み

### When {#when}

1. ユーザがブロック A をクリックして EDITING 状態に入る（`data-block-state="EDITING"`）
2. body を一切変更せず、debounce 区間（500ms）を超えて待機する

### Then {#then}

- EDITING への遷移だけでは AutoSave は発火しない（UI debounce はキー入力の `onChange` でのみ
  schedule される）。待機後も A の `.md` の body は `"hello"`、`updatedAt` は `t0` のまま
- AutoSave 経路（application service）に同一 body が到達した場合:
  - `compareBody` が `BodyDiff::Unchanged` を返し、**早期 return**（`Ok(None)`）
  - `Note::edit_body` は呼び出されない
  - `NoteRepository::write` は呼び出されない（**write なし**）
  - event `NoteBodyEdited` は発行されない
  - `updatedAt = t0` のまま
- 対照（ガードが過剰抑制でないこと）: body が実際に変化した場合は `NoteBodyEdited` の永続化成功と
  して `updatedAt` が更新される（`Saved`）

### 補足（責務分離） {#when-responsibility}

「同一 body 編集の抑制」は **application service（`AutoSaveNoteUseCase::execute`）** の責務。
`Note::edit_body`（aggregate）は呼ばれれば `updatedAt` を更新する素朴な設計を維持する
（`domain/validation.md#notes-idempotent-save`）。これにより aggregate は薄く保たれ、IO 最適化は
application 層に閉じる。

## テスト観点 {#test-points}

- **初期状態**: A の Block が表示され、`.md` の body が `"hello"`、`updatedAt` が `t0`
- **EDITING 遷移のみで AutoSave 不発火（本 scenario の核・前半）**: ブロックをクリックして
  `data-block-state="EDITING"` になり、500ms 超待機しても A の `.md` は不変
  （body `"hello"` / `updatedAt` `t0`）
- **同一 body の AutoSave は no-op（本 scenario の核・後半）**:
  `auto_save_note { noteId, newBody: "hello" }`（現在 body と同値）を返却 DTO で観測すると
  `{ outcome: "no_op" }`。`.md` は不変のまま（write なし・event 非発行の直接観測）
- **対照: body 変化時は Saved**: `auto_save_note { noteId, newBody: "hello world" }` は
  `{ outcome: "saved" }` を返し、`.md` の body が `"hello world"`、`updatedAt` が `t0` から更新される
  （→ ガードは同値のみを抑制し、変化は取りこぼさない）
- **event 非発行**: `NoteBodyEdited` は永続化成功時のみ発行されるため、no-op 経路では発行されない
  （`NoOpBus` の境界のため E2E では直接観測せず、`auto_save_note` の `outcome` と `.md` 不変で
  間接検証する。application service 単体検証は slice `auto-save-note` の TP-I1..I3 が担当）

## 実装ノート {#impl-notes}

- **runner**: `wdio`（優先チェーン step 2 — manifest に `runner:` 明示なし。参加 app `promptnotes` は
  `local` 系で `runtime.runner = wdio`。`.ori/architecture.md` の `scenario_test_runner: wdio` とも一致）
- **infrastructure**: `promptnotes` のみ（`mode: local`）。compose-service 系 app が参加しないため
  docker-compose は不要
- **runner config**: `wdio.conf.ts` が `/ori-generate` により生成される
- **Note A の準備**: `wdio.conf.ts` の `onPrepare` が `TAURI_TEST_STORAGE_DIR` に Note A
  （`20260620120000`, body=`hello`, frontmatter `updatedAt: 20260620120000`）を seed する
- **アサーション戦略**:
  - EDITING 遷移は Block の `[data-block-id="20260620120000"]` の `data-block-state` 属性で判定する
  - `.md` の不変 / 変化は `node:fs` の `readFileSync` で body 行と frontmatter `updatedAt` を比較する
    （`auto_save_note` の直接 invoke は Tauri 境界の実コマンドを叩くため、ファイル反映が最も直接的）
  - `auto_save_note` の結果は `window.__TAURI_INTERNALS__.invoke('auto_save_note', { noteId, newBody })`
    を同期 `browser.execute` で kick-off → window 上の結果を poll する方式で検証する
    （`@wdio/tauri-service` (driverProvider=external) の `patchedExecute` は `browser.executeAsync` を
    扱えないため。s7 / s8 と同方式）
- **テストデータのクリーンアップ**: `wdio.conf.ts` の `onComplete` が temp `storageDir` を
  `rmSync` で削除する
- **production 側の前提（本 scenario が検証する不変条件）**: `AutoSaveNoteUseCase::execute` の
  `compareBody` が `BodyDiff::Unchanged` のとき `Ok(None)` を返し、`write` / `emit` に到達しないこと
  （`apps/promptnotes/src-tauri/src/note_capture/slices/auto_save_note/application.rs`）。
  この冪等性ガードが欠落すると、同一 body の AutoSave が `updatedAt` を bump し、
  `NoteBodyEdited` を重複発行して S9 が RED になる
