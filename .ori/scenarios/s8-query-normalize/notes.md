# s8-query-normalize — Scenario implementation notes

## Event 検証の方針 (read model 制約) {#event-verification-policy}

`update-feed-filter` は NoteFeed（read model / 揮発）の filter を更新する workflow であり、
`workflows/update-feed-filter.md#output` が「domain event: **なし**（揮発）」と規定する。
すなわち **発行すべき event が存在しない** ため、E2E で観測する対象も無い。
担保は slice unit test（`note_feed/slices/update_feed_filter/tests.rs`、TP-S8-3 / TP-SE1）に
委ねる。E2E は UI（Block 表示）と Tauri 境界（`update_feed_filter` の query 正規化）でカバーする。

## テスト方式: UI 駆動（検索バー + Block 観測） {#test-approach}

Toolbar の検索入力 `screen-1-toolbar-search-query` に `setValue` で入力し、`oninput` →
`feedStore.setQuery` → `update_feed_filter` invoke → in-memory `visibleNotes` 再計算、という
実際の UI 経路を駆動する。Block の body は CodeMirror 内に描画されるためテキスト直接参照はせず、
`[data-block-id="<id>"]` の存在で可視性を判定する。seed の Note A / B は `wdio.conf.ts` の
`onPrepare` が `TAURI_TEST_STORAGE_DIR` に unquoted inline frontmatter 形式で投入する。

## テストカバレッジ {#coverage}

| 検証項目 | 検証方法 |
|---|---|
| 初期状態（A/B 表示 / 検索バー空 / file 存在） | E2E (`count` + `getValue` + `existsSync`) |
| **半角 query `"gpt"` で全角 body の B も match（NFKC、本 scenario の核）** | E2E step1（A/B 両 Block 残存 = 2 件） |
| query 正規化（`"gpt"` → `"gpt"` 変化なし） | E2E step1（`update_feed_filter` 直接 invoke の DTO.query） |
| 全角大文字 `"ＧＰＴ"` の query 正規化 + body 正規化 | E2E step2（A/B 両 Block + DTO.query=`"gpt"`） |
| query 解除で再表示（C-UF2） | E2E step3（`clearValue` → A/B 再表示） |
| event 非発行 | slice unit test（E2E 不可・上記 read model 制約） |
| NFKC 互換等価 match（frontend filter） | frontend unit test `feed.test.ts` `spec#I-F1/I-F5` |

## production 修正 {#production-fix}

初回 E2E は **step 1 で RED**（半角 query に対し B（全角 body `ｇｐｔ のメモ`）の Block が
`Expected: 1, Received: 0`）。A（半角 body）は残るため、原因は **frontend の in-memory filter が
NFKC 正規化していない**こと。Rust `NoteFeed::visible_notes` の `matches_query` は
`body.nfkc().collect().to_lowercase()` で比較する（`note_feed/shared/types/note_feed.rs`）が、
UI の暫定 filter `applyFeedFilter`
（`apps/promptnotes/src/ui-page/page-main/stores/feed.svelte.ts`）は lowercase のみで乖離していた。
これは I-F1 / I-F5（query / body のマッチングは NFKC + lowercase 済みで行う）に違反する
production 側の不足。

- 最小修正: `applyFeedFilter` の query matching で `String.prototype.normalize('NFKC')` を
  query / body / tag の全てに適用（Rust と同一の NFKC + lowercase）。他軸（date_range / tag /
  sort）は不変。
- 回帰確認:
  - frontend unit test: 新規 `spec#I-F1/I-F5` を追加。feed store 単体 **23/23 pass**。
  - 全 frontend suite: **143 passed / 15 files**。
  - E2E: 修正前 **3 failing** → 修正後 **3 passing (5.9s)**。
  - `prettier --check` / `eslint`: 変更 2 file に指摘なし。
  - binary 再生成: `VITE_WDIO_TEST=1 bun run tauri build --debug --no-bundle`
    （frontend のみ変更のため cargo は差分ビルド数秒）。
