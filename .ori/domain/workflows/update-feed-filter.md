---
coherence:
  source: human
  last_validated: 2026-06-20
  upstream:
    - aggregates.md#note-feed-aggregate
    - validation.md#s8-query-normalize
---

# update-feed-filter {#update-feed-filter}

NoteFeed の filter（query / date_range / tag）を更新する。
read model のため domain event を発行しない（揮発状態）。

## Input {#input}

```rust
enum UpdateFeedFilterCommand {
  SetQuery { raw: String },                     // 検索バー入力
  SetDateRange { range: DateRangeFilter },      // 期間プリセット選択
  SetTag { tag: Option<Tag> },                  // タグチップクリック / 解除
  ClearAll,                                     // 全リセット
}
```

## Output {#output}

- `NoteFeed`（filter 更新後）
- domain event: **なし**（揮発）

## Errors {#errors}

- なし（無効入力は VO 構築側で吸収するか、UI 層で reject）

## Steps {#steps}

### SetQuery 分岐 {#steps-set-query}

1. `normalizeQuery: String → Option<NormalizedQuery>`
   - NFC 正規化 + lowercase 化
   - 空文字なら `None`、それ以外は `Some(query)`
2. `applyQuery: (NoteFeed, Option<NormalizedQuery>) → NoteFeed`
   - filter.query を更新

### SetDateRange 分岐 {#steps-set-date-range}

1. `applyDateRange: (NoteFeed, DateRangeFilter) → NoteFeed`

### SetTag 分岐 {#steps-set-tag}

1. `applyTag: (NoteFeed, Option<Tag>) → NoteFeed`

### ClearAll 分岐 {#steps-clear-all}

1. `resetFilters: NoteFeed → NoteFeed`
   - `query=None, date_range=All, tag=None`

## Dependencies {#dependencies}

- なし（in-memory のみ、IO なし）

## Notes {#notes}

- 1 文字入力ごとに即時 filter 適用（Q7 決定: debounce なし）
- マッチングは `NoteFeed::visible_notes` の query 側で実行（I-F5: body + tags のみ）
- 起動時は ClearAll 相当の状態で初期化（Q3 決定、S12）
