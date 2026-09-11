---
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s3-flush-on-blur
      runner: wdio
      trigger: BlockBlur
---

# s3-flush-on-blur — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s3-flush-on-blur phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

Note を編集（EDITING 状態）中、AutoSave の 500ms debounce timer が完了する前に
ユーザが別の Note ブロックをクリックした（focus 喪失）場合、
debounce timer を即時キャンセルし、即座に pending body を Flush して Note を永続化する。

これは [validation.md#s3-flush-on-blur] で記述されたシナリオに対応する。

## シナリオステップ {#scenario-steps}

| Step | Action | Expected |
|------|--------|----------|
| GIVEN 1 | 既存 Note A（id=`20260620000001`）を load | Note A が IDLE 状態で UI に表示されている |
| GIVEN 2 | ユーザが Note A のブロックをクリック → EDITING 状態に遷移 | Block A が EDITING 状態 |
| GIVEN 3 | 本文を `"hello"` → `"hello world"` に編集（t1 時点） | AutoSave debounce timer が 500ms で起動中（t1+0.5s = t3 に fire） |
| WHEN | `t1+0.2s=t2` 時点で別 Note B のブロックをクリック | Block A の focus 喪失。Block A が EDITING → IDLE に遷移 |
| THEN 1 | debounce timer を即時キャンセル | 重複 AutoSave は発火しない |
| THEN 2 | pending body `"hello world"` で `Note::edit_body` を即時実行（t2 の時刻で） | `Note::edit_body` が `t2` の `Timestamp` で呼ばれる |
| THEN 3 | `.md` ファイル永続化 | `storage_dir/20260620000001.md` の body と updatedAt が `t2` で更新 |
| THEN 4 | event `NoteBodyEdited { note_id: "20260620000001", updated_at: t2 }` 発行 | Note Feed に反映（updatedAt sort 時の再表示） |
| THEN 5 | t2 時点で body が変更されていなければ event は発行されない（flushed body = もともとの body なら no-op、auto-save-note と同形） | 同一の updatedAt、event 非発行 |

## テスト観点 {#test-points}

- **focus 喪失で即時 Flush が発動すること**:
  `t1` に編集して debounce timer を起動、`t1+200ms` に別ブロッククリック → `t1+200ms` 時点で Flush 実行、debounce から t1+500ms には `Note::edit_body` が呼ばれないことを確認
- **Flush が発行する event に updatedAt が Flush 時刻（即時）であること**: t1+200ms の時刻 = `updatedAt`、`NoteBodyEdited` の `updated_at` が即時であることを確認
- **debounce 中ではないときのクリックでは Flush しない**: 既存ブロックを EDITING に遷移していない状態で別ブロックをクリック → Flush は起きない（Idle → Idle で問題ない）
- **2 ブロック同時 EDITING 時は、クリックされた側のブロックに focus が移る前に元ブロックの focus が失われ Flush する**: Block A (EDITING)、Block B (EDITING)、A から B にクリック → A Flush → focus B に移る、の順序になるか検証する

## 実装ノート {#impl-notes}

- runner: `wdio`（参加 app `promptnotes` が `local` 系のため。`.ori/architecture.md` の `workspace.apps[].runtime.runner` から導出）
- テストは build-then-test: wdio の `runner.config`（`wdio.conf.ts`）でビルド・起動・停止を管理。テストコードから直接 app の起動停止は行わない
- Block の状態遷移 (`IDLE` → `EDITING` → `IDLE`) は `page-main` の UI 状態として管理されるため、この scenario は page-main が完成している前提
- `Cancel debounce` は application 層の責務。`DebounceTimer` handle 経由で即時キャンセル → 続けて `flushNote(note_id)` 呼出
- `NoteBodyEdited` の発行は `AutoSave` と `Flush` で同一（flow: `flush-note` workflow に従う）
- validation.md の注記にある通り、3 種の Flush トリガー（BlockBlur / WindowBlur / AppQuit）は同じ `flush-note` workflow を再処理する
- この scenario は BlockBlur trigger の検証が scope。WindowBlur と AppQuit は別 scenario でカバー
- 重複 event を防ぐために body 変化検知は行う (S9 と同じロジック)