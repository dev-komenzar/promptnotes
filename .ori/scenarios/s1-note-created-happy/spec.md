---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  derives_from:
    - domain/validation.md#s1-note-created-happy
---

# s1-note-created-happy — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s1-note-created-happy phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

`Cmd+N` → Draft 入力欄にテキスト入力 → `Cmd+Enter` 押下によって
新規 Note が作成され、NoteFeed に反映される E2E 検証。

対象 workflow: `create-note` slice (handles Note creation + NoteCreated event emission)。
対象 page: `page-main` (Draft 入力欄 + NoteFeed 表示)。

## シナリオステップ {#scenario-steps}

> `domain/validation.md#s1-note-created-happy` より:

### Given {#given}

- アプリ起動済み、保存先は OS 慣習パス
- フィードに既存 Note は 0 件
- Draft 入力欄が空

### When {#when}

1. `Cmd+N` で Draft 入力欄にフォーカス
2. `"docs を書く"` を入力
3. `Cmd+Enter` を押下

### Then {#then}

- `Note::create(body="docs を書く", tags=∅, now=t0)` が実行される
- `storageDir/<note_id>.md` が書き出される（frontmatter: `tags: []`, `createdAt`, `updatedAt`）
- event **NoteCreated** `{ note_id, created_at: t0, initial_tags: ∅ }` 発行
- NoteFeed: 表示 1 件、フィード最上部に新規ブロック
- UI: Draft 入力欄がクリア、新規ブロックへフォーカス遷移

## テスト観点 {#test-points}

### データ検証 {#data-verification}

- [ ] 作成された `.md` ファイルの frontmatter に `tags: []` が正しく書き込まれている
- [ ] `createdAt` と `updatedAt` が同じタイムスタンプである
- [ ] body が入力テキスト（`"docs を書く"`）と一致する
- [ ] ファイル名が `<note_id>.md` 形式（14 桁タイムスタンプ）である

### UI 状態検証 {#ui-state-verification}

- [ ] Draft 入力欄が Cmd+Enter 後にクリアされる
- [ ] NoteFeed に新規 Note が 1 件表示される
- [ ] フォーカスが新規作成された Note ブロックに遷移する
- [ ] 既存 Note が 0 件の場合、作成後は 1 件表示

### 境界条件 {#boundary-conditions}

- [ ] 空ボディ（入力欄が空のまま Cmd+Enter）→ Note 作成されない
- [ ] 前回終了時に Note が存在する場合 → 起動時フィードに既存 Note が表示された状態から新規作成が正常動作

## 実装ノート {#impl-notes}

- **runner**: `wdio`（優先チェーン step 2 — `promptnotes` app は `mode: local` / `runner: wdio` のため）
- **参加 app**: `promptnotes`（Tauri desktop、`mode: local` → `compose-service` 非該当のため docker-compose 生成不要。`wdio` runner がホスト上のビルド済み binary を直接起動）
- **build-then-test**: `/ori-generate` が生成する `wdio.conf.ts` が `nix build` → ビルド済み binary の起動・停止を管理
- **依存 slice**: `create-note` — Draft 入力 + Cmd+Enter → Note 作成 + NoteFeed 反映。この scenario 実行時点で `create-note` slice は GREEN（phase 4 完了済み）
- **依存 page**: `page-main` — Draft 入力欄と NoteFeed を提供。UI region として `draft-input-area` と `note-feed-area` を持つ