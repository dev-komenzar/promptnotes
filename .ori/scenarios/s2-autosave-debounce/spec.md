---
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s2-autosave-debounce
      hash: s2-autosave-debounce
    - path: domain/workflows/auto-save-note.md
      hash: auto-save-note
    - path: domain/aggregates.md#note-aggregate
      hash: note-aggregate
    - path: domain/domain-events.md#note-body-edited
      hash: note-body-edited
---

# s2-autosave-debounce — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s2-autosave-debounce phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

Note ブロックが EDITING 状態のとき、ユーザのキー入力に対して 500ms debounce が成立すると
AutoSave が発火し、Note 本文が永続化される。

> domain/validation.md#s2-autosave-debounce より:
> 既存 Note 編集後 500ms debounce で AutoSave

この scenario では以下を検証する:

1. EDITING 状態の Note にキー入力を与える
2. キー入力を止めて 500ms 経過後に AutoSave が完了する
3. `.md` ファイルが更新され、updatedAt が更新される
4. NoteBodyEdited イベントが発行される

> domain/workflows/auto-save-note.md より:
> EDITING ブロックでのキー入力後 500ms debounce が成立した時に、Note 本文を永続化する。

## シナリオステップ {#scenario-steps}

### Given {#given}

- 既存 Note A が存在し (`body="hello"`, `updatedAt=t0`)、フィードに表示されている
- ブロック A が EDITING 状態（ユーザがクリックまたは Cmd+Enter 等で編集を開始済み）
- Tauri App が起動済み、ファイルウォッチャーは稼働中
- テスト環境: `storage_dir` に Note A の `.md` ファイルが事前に存在する

### When {#when}

1. `t1` 時点でブロック A に `" world"` を追記（body = `"hello world"` となる）
2. キー入力を止め、500ms 経過（`t1 + 0.5s = t2`）

### Then {#then}

1. `Note::edit_body(new_body="hello world", now=t2)` が実行される
2. `storageDir/<A.id>.md` の body が `"hello world"` に、frontmatter `updatedAt` が `t2` に更新される
3. domain event **NoteBodyEdited** `{ note_id: A.id, updated_at: t2 }` が発行される
4. NoteFeed: updatedAt sort 時、A が最上部へ移動する

## テスト観点 {#test-points}

- **auto-save 発火タイミング**: キー入力停止後、500ms ± 許容範囲（100ms 程度）で永続化が完了すること
  - 許容範囲: トータルの待ち時間が 500ms〜700ms に収まる（処理オーバーヘッド込み）
- **ファイル更新**: `.md` ファイルの body 行が更新後の値と一致すること
- **updatedAt 更新**: frontmatter `updatedAt` が編集後の時刻を反映していること
- **event 発行**: NoteBodyEdited が適切なペイロードで発行されること
- **NoteFeed 反映**: updatedAt sort で対象 Note が最上部に移動すること
- **冪等性（S9）**: body が変化していない場合は AutoSave が発火しないこと（イベント非発行、ファイル非更新）

## 実装ノート {#impl-notes}

- **runner**: `wdio`（優先チェーン step 2 — 参加 app `promptnotes` の `runtime.runner=wdio` から導出。「1 scenario = 1 UI runner」制約を満たす）
- **起動方式**: `local` — ビルド済み Tauri binary を runner（wdio）が直接起動する。docker-compose は不要（compose-service 系 app が参加しないため）
- **テスト環境のセットアップ**:
  - storage_dir をテスト用の一時ディレクトリに設定
  - Note A の `.md` ファイルを事前作成（frontmatter: `updatedAt: t0`）
- **debounce 待機**: キー入力後に `browser.pause(600)` または wdio の `waitUntil` で AutoSave 完了を待つ。キー入力 → 500ms debounce + α で合計 600〜800ms 程度を目安に
- **関連 workflow**: `auto-save-note` workflow（Phase 9）。ステップ: load → parse body → compareBody（冪等ガード）→ updateBody → persist → emit
- **関連 scenario**: S9 (idempotent-autosave) — body が変化していない場合は event 非発行