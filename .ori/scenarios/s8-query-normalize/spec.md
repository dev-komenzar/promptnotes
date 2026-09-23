---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s8-query-normalize
      hash: 4a02c17cc025
    - path: domain/workflows/update-feed-filter.md#update-feed-filter
      hash: 51cdf06a33f9
    - path: domain/aggregates.md#note-feed-aggregate
      hash: 56f7a54a8ab2
    - path: domain/ui-fields/screen-1.md#screen-1-toolbar-search-query
      hash: 4bc0f83f71f3
---

# s8-query-normalize — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s8-query-normalize phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

検索バー入力による NoteFeed filter の **query 正規化（NFKC (compatibility normalization) +
lowercase）** を、UI 駆動 E2E で検証する scenario。

- 対応 workflow: `domain/workflows/update-feed-filter.md`（`SetQuery` → `normalizeQuery` →
  `applyQuery`。read model のため event 非発行）
- 対応 aggregate: `domain/aggregates.md#note-feed-aggregate`（I-F1 / I-F5: query は常に
  NFKC + lowercase 済み、マッチング時に再正規化しない）
- 対応 page: `page-main`（Toolbar region の検索入力 `screen-1-toolbar-search-query`）

> domain/validation.md#s8-query-normalize より:

- Given: Note A（`body="GPT を試す"`）、Note B（`body="ｇｐｔ のメモ"` 全角）、検索バーは空
- When: 検索バーに `"gpt"`（半角）を入力
- Then:
  - 入力を NFKC + lowercase: `"gpt"`（変化なし）
  - A の body を NFKC + lowercase: `"gpt を試す"` → match
  - B の body を NFKC + lowercase: `"gpt のメモ"`（全角 → 半角化、NFKC により互換等価変換） → match
  - **両者表示**
  - event は発行されない（NoteFeed は read model）

> domain/aggregates.md#note-feed-aggregate-invariants より:
> 「**I-F1**: `query` は常に **NFKC 正規化済み + lowercase 済み**（マッチング時に再正規化しない）。
> NFKC を使う理由: 全角 Latin / 半角 Latin、半角カナ / 全角カナ等の互換等価文字を同一視するため。
> canonical decomposition のみの NFC では半角化が起きず、S8 シナリオが成立しない」
> 「**I-F5**: マッチング対象は `body` 全文 + `tags[*].name` のみ」

> domain/workflows/update-feed-filter.md#steps-set-query より:
> 「`normalizeQuery: String → Option<NormalizedQuery>` — NFKC (compatibility normalization)
> 正規化 + lowercase 化。空文字なら `None`、それ以外は `Some(query)`」

> domain/ui-fields/screen-1.md#screen-1-toolbar-search-query より:
> 「text input (icon 付) | `Cmd+F` で focus。1 文字入力ごと即時 filter」

## シナリオステップ {#scenario-steps}

### Given {#given}

1. Note A（ID: `20260620120000`、body: `"GPT を試す"`）が表示されている
2. Note B（ID: `20260620130000`、body: `"ｇｐｔ のメモ"` 全角）が表示されている
3. 検索バー（`screen-1-toolbar-search-query`）は空
4. filter は初期状態（`query=None, date_range=All, tag=None`）

### When {#when}

1. ユーザが検索バーに `"gpt"`（半角小文字）を入力する
2. 1 文字入力ごとに即時 filter が適用される（debounce なし、Q7）

### Then {#then}

- `NoteFeed::filter_by_query("gpt")`: 入力を NFKC + lowercase → `"gpt"`（半角入力なので変化なし）
- NoteFeed.visible_notes:
  - A body NFKC + lowercase → `"gpt を試す"` → `"gpt"` を含む → match
  - B body NFKC + lowercase → `"gpt のメモ"`（全角 `ｇｐｔ` が NFKC で半角 `gpt` へ互換等価変換）
    → `"gpt"` を含む → match
  - **A / B の 2 件が両方表示される**
- event は発行されない（NoteFeed は read model、`update-feed-filter` は揮発 filter のため）

### 検索解除（補助） {#when-clear}

- 検索バーを空文字に戻すと `NormalizedQuery::from_raw("")` は `None` に降格し、
  filter.query が解除されて A / B の 2 件が再び表示される（C-UF2 と整合）。

## テスト観点 {#test-points}

- **初期状態**: A / B の 2 Block が表示され、検索バーは空、file は両方存在する
- **半角 query "gpt"（本 scenario の核）**: 検索バーに `"gpt"` を入力 → **A / B の 2 Block が
  表示され続ける**（B の全角 body `ｇｐｔ のメモ` が NFKC により `gpt` へ互換等価変換されて match）
  → 正規化が欠落していれば B が消え **1 Block** になる（RED）
- **filter.query の正規化**: `update_feed_filter { kind: "set_query", raw: "ＧＰＴ" }`（全角大文字）を
  直接 invoke すると、返却 DTO の `query` が `"gpt"`（NFKC + lowercase）である
- **query 正規化 + body 正規化の合成**: 検索バーに全角 `"ＧＰＴ"` を入力しても A（半角 `GPT`）/
  B（全角 `ｇｐｔ`）が両方 match する
- **解除**: 検索バーを空に戻すと A / B が再表示される
- **event 非発行（間接）**: NoteFeed は read model かつ `update-feed-filter` は揮発 filter のため、
  domain event は構造的に発行されない（`workflows/update-feed-filter.md#output` 「domain event: なし」）

## 実装ノート {#impl-notes}

- **runner**: `wdio`（優先チェーン 2 — 参加 `local` 系 app `promptnotes` の `runtime.runner` = wdio。
  manifest に `runner:` 明示なし。app `promptnotes` は `mode: local` のため compose に含めず、
  ビルド済み binary を runner が直接起動）
- **infrastructure**: `promptnotes` のみ（local mode）。docker-compose は不要（compose-service 系 app なし）
- **runner config**: `wdio.conf.ts` が `/ori-generate` により生成される
- **Note A / B の準備**: `wdio.conf.ts` の `onPrepare` が `TAURI_TEST_STORAGE_DIR` に
  Note A（`20260620120000`, body=`GPT を試す`）と Note B（`20260620130000`, body=`ｇｐｔ のメモ` 全角）を
  seed する（app の unquoted inline frontmatter 形式）
- **アサーション戦略**:
  - Block の識別は `[data-block-id="<id>"]` で行う（body は CodeMirror 内に描画されるため
    テキスト直接参照ではなく Block の存在で検証する）
  - 検索バーは `[data-testid="screen-1-toolbar-search-query"]`。`setValue` で `oninput` を発火させる
  - filter 適用は reactivity のため、入力後に `browser.pause` で反映を待つ
  - `update_feed_filter` の正規化は `window.__TAURI_INTERNALS__.invoke('update_feed_filter', {...})`
    を同期 `browser.execute` で kick-off → window 上の結果を poll する方式で検証する
    （`@wdio/tauri-service` (driverProvider=external) の `patchedExecute` は `browser.executeAsync`
    を扱えないため。s7 と同方式）
- **file の存在確認**: A / B の `.md` は seed 後も変化しないため、`node:fs` で存在を確認する
- **テストデータのクリーンアップ**: `wdio.conf.ts` の `onComplete` が temp `storageDir` を
  `rmSync` で削除する
- **production 側の前提（本 scenario が検証する不変条件）**: UI の NoteFeed query マッチング
  （frontend `applyFeedFilter` / Rust `NoteFeed::visible_notes`）は、**query だけでなく Note body /
  tag も NFKC (compatibility normalization) + lowercase してから substring 比較する**（I-F1 / I-F5）。
  正規化が欠落している場合、全角 body の Note が半角 query に match せず S8 は RED になる
