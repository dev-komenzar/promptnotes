---
ori:
  node_id: workflow:list-feed
  type: workflow
  depends_on:
    - aggregate:NoteFeed
    - aggregate:Note
    - scenario:s12-startup-state
---

# list-feed {#list-feed}

`storage_dir` 配下の Note `.md` を全件読み込んで `NoteFeed.source`
（`Vec<Note>`）を hydrate し、現在の `filter` / `sort` を適用した
`visible_notes` を返す read pipeline。アプリ起動時と手動再読込で同じ pipeline を再利用する。

NoteFeed Aggregate の `visible_notes` query 操作の唯一の実装点であり、Note Feed BC の
read side の **入口** になる。書き込みは伴わず domain event を発行しない（揮発 read model）。

## Input {#input}

```rust
struct ListFeedCommand {
  // 引数なし。実行コンテキストとして storage_dir / current NoteFeed を持つ
  // (application service が application state から解決する)
}
```

トリガー:

- **アプリ起動時**: load-settings 完了後に 1 回呼ぶ (S12 と整合)
- **手動再読込**: 将来の "Refresh" UI ボタン (現 MVP には未配線、binding は用意する)

## Output {#output}

- `NoteFeed`（`source` を hydrate し、現在 filter/sort を適用した状態）
- 派生 read DTO として `visible_notes: Vec<NoteView>` を計算
- domain event: **なし**（read 側、揮発、I-F1〜I-F7 の write 側 invariant に変更なし）

## Errors {#errors}

- **なし**（spec の core 動作: 起動時の hydration はユーザーの作業を妨げない）
  - `NoteRepository::list_all()` 内の I/O エラー / `.md` の parse 失敗は
    **個別 Note 単位で skip** する。「全部 or 何も読まない」ではなく
    「読めるものだけ読む」が UX 上の原則 (load-settings の partial fallback と同型)
  - I/O 失敗自体は port 実装側で log に残し、aggregate には到達しない

## Steps {#steps}

1. `loadAllNotes: StorageDir → Vec<Note>`
   - `NoteRepository::list_all()` で `storage_dir/*.md` を全件 parse
   - parse 失敗 / I/O 失敗は skip (上記 Errors 節)
2. `hydrateFeedSource: (NoteFeed, Vec<Note>) → NoteFeed`
   - `NoteFeed.source` を差し替える (move semantics)
3. `applyFilter: (Vec<Note>, FeedFilter) → Vec<&Note>`
   - I-F4: AND 合成 (date_range ∧ tag ∧ query)
   - I-F5: マッチング対象は `body` 全文 + `tags[*].name` のみ
   - I-F1: query は NFKC (compatibility normalization) + lowercase 済 (filter 構築時に確立済)
   - I-F7: 削除 (trash) された Note は除外 (本 slice では `source` から既に除外されている前提)
4. `applySort: (Vec<&Note>, SortOrder) → Vec<&Note>`
   - I-F3: sort key が同値の場合は `id` で tiebreak (決定論性)
5. `projectVisibleNotes: Vec<&Note> → Vec<NoteView>`
   - UI 層に渡す read DTO（`NoteSummary` 相当）に投影

## Dependencies {#dependencies}

- `NoteRepository` — `list_all() -> Vec<Note>` を新規追加 (本 workflow が初出)
- `Settings` — `storage_dir` 解決のため (load-settings の後段で呼ぶ前提)

## Notes {#notes}

- **`source` の型表現**: `NoteFeed.source = Vec<Note>` を採用 (aggregates.md と整合)
  - 候補 `Vec<NoteId>` は read pipeline で都度 `load_by_id` を呼ぶ O(N) IO になり MVP に過剰
  - `Vec<Note>` 一括 hydration は memory に乗り切るサイズ感 (1k Note 程度を想定)
- **冪等性**: 同じ `storage_dir` に対して何度呼んでも同じ `Vec<Note>` を返す
  (Note の identity が `.md` ファイル名と 1:1、I-N2)
- **NoteFeed 揮発性との関係**: `source` は揮発で OK (`filter` も揮発、`sort` のみ Settings から復元)。
  `list-feed` を起動時に必ず呼ぶ前提なら hydration が無いまま filter / sort を触る状況は起こらない
- **trash との関係**: 削除済 Note は `NoteRepository::list_all` の対象外
  (delete-note slice が OS ゴミ箱に移すため `storage_dir` から消える)。
  I-F7 は本 workflow では port 契約に委譲する形で satisfy される
- **手動 Refresh の位置づけ**: MVP では UI トリガーを持たない。本 workflow は
  pure な再 hydration として書き、UI 側がいつでも再呼出できる shape にする
