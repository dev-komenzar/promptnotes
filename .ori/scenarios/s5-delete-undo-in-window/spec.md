---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s5-delete-undo-in-window
      hash: 4a02c17cc025
---

# s5-delete-undo-in-window — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s5-delete-undo-in-window phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

Note A を削除後、トースト表示時間内に「元に戻す」操作で復元できることを検証する E2E シナリオ。

> domain/validation.md#s5-delete-undo-in-window より

削除時の状態遷移:
- `Note::delete_to_trash()` → OS ゴミ箱へ移動、`NoteDeletedToTrash` 発行
- フィードから表示除去
- トースト表示（5 秒有効、Undo ボタン付き）
- Undo 操作: `DeletedNote::restore()` → OS ゴミ箱から原パスへ復帰、`NoteRestoredFromTrash` 発行

## シナリオステップ {#scenario-steps}

### Given {#given}

1. Note A（ID: `20260620120000`、body: `"hello"`）が表示されている
2. Note A の削除トーストは未表示
3. ユーザが Note A のブロックにマウスホバー可能な状態

### When {#when}

1. **t0**: ユーザが Note A のブロックにホバー → 削除ボタン（x アイコン）をクリック
2. **t0 + 2s = t1**: 表示されたトースト内の「元に戻す」ボタンをクリック

### Then {#then}

**t0（削除直後）**:
- `Note::delete_to_trash()` が実行される
- OS ゴミ箱への移動が成功
- event **NoteDeletedToTrash** `{ note_id: "20260620120000", original_path: "<storageDir>/20260620120000.md", deleted_at: t0 }` 発行
- NoteFeed: 表示から Note A が除外される
- UI: 画面下部に「元に戻す」トーストが表示される（有効期間 5 秒）
- Application service: `DeletedNote { id, original_path }` を 1 件保持

**t1（Undo 実行）**:
- `DeletedNote::restore()` が実行される
- OS ゴミ箱から原パス `<storageDir>/20260620120000.md` に復帰
- event **NoteRestoredFromTrash** `{ note_id: "20260620120000", restored_at: t1 }` 発行
- NoteFeed: 表示に Note A が再登場
- UI: トーストが閉じる

## テスト観点 {#test-points}

- **削除後のファイル不在確認**: t0 後、`<storageDir>/20260620120000.md` が存在しないこと（OS ゴミ箱へ移動されている）
- **Undo 後のファイル復元確認**: t1 後、`<storageDir>/20260620120000.md` が原パスに復元され、body が `"hello"` であること
- **NoteFeed の表示更新**: 削除で非表示 → Undo で再表示されること
- **トーストの表示/非表示**: 削除操作後に「元に戻す」トーストが表示され、Undo 操作後に閉じること
- **トースト有効期間内の Undo**: t0 + 2s（5 秒以内）の Undo が成功すること
- **event 発行確認**: `NoteDeletedToTrash` → `NoteRestoredFromTrash` の順で発行されること
- **トースト有効期間超過後**: t0 + 5s 経過後は Undo 不可（no-op、別シナリオ S7 で検証）

## 実装ノート {#impl-notes}

- **runner**: `wdio`（優先チェーン 3 — global `scenario_test_runner`。app `promptnotes` は `mode: local` のため compose に含めず、ビルド済み binary を runner が直接起動）
- **infrastructure**: `promptnotes` のみ（local mode）。docker-compose は不要（compose-service 系 app なし）
- **runner config**: `wdio.conf.ts` が `/ori-generate` により生成される
- **Note A の準備**: テスト開始前に `promptnotes` アプリ内で Note A を作成しておく（`Cmd+N` → body 入力 → `Cmd+Enter`）。または `storageDir` に直接 `.md` ファイルを配置
- **アサーション戦略**:
  - ファイルの存在/不在は `wdio` の `browser.executeAsync()` で Node.js `fs` を使用（Tauri shell plugin 経由も検討）
  - NoteFeed の DOM 状態は wdio のセレクタで検証
  - トーストの表示状態は ARIA live region のテキスト内容で検証
- **待機戦略**: 削除操作後、ファイル削除とイベント伝播に十分な待機時間を設定（500ms 程度）。トーストの有効期間（5s）内に Undo 操作が実行できるよう、t0 から t1 までの時間を 2s に設定
- **テストデータのクリーンアップ**: テスト終了後、`storageDir` を初期状態に戻す（Note A の `.md` が復元されていることを確認した上で、テスト用に作成したファイルを削除）