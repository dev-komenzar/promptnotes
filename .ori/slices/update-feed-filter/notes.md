# update-feed-filter — Implementation notes

## NoteFeed の最小フィールド構成 (sort / source の drop)

spec.md#impl-bc-scope は「`sort` / `source` を default 相当で保持」と書いたが、impl では **保持せず field 自体を落とす** 形を採用した:

- 理由 1: 後続 slice (`change-sort-order` / `list-feed`) で field 形状が確定するまで struct 表面に出さない方が PartialEq/Eq の不意の semantic shift を避けられる
- 理由 2: spec の Open Question (`oq-source-shape`) が示す通り source 型 (`&[Note]` か `Vec<NoteId>` か) は未確定。placeholder を入れると間違った型を borrow しがち
- 後続 slice 着手時に struct layout 変更 + 既存テスト helper の再生成が必要になる (ただし update-feed-filter 側 test は影響なし)

decision: 下流 slice が確定するまで Note Feed BC の minimal scope は `NoteFeed { filter: FeedFilter }` で凍結。

**superseded by `change-sort-order` slice (2026-06-26)**: 後続の `change-sort-order` slice が NoteFeed.sort を必要としたため、`NoteFeed { filter, sort }` 構造へ拡張された。sort 型は spec で議論されていた「shared `user_preferences::SortOrder` か独立か」(ori-64x.9) の決定として **shared 採用**。`source` は依然 out of scope（`list-feed` slice で確定予定）。

## NFKC vs NFC

domain 文書は "NFC + lowercase" と表記しているが、S8 walkthrough (`Ｇｐｔ` → `gpt`) を満たすには NFKC (compatibility normalization) が必要。本 slice は walkthrough の意図を優先して `nfkc()` を採用。terminology のずれは proposal で domain 修正提案する (follow-up: ori-64x.8)。
