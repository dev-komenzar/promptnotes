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

**2026-06-26 proposal accepted**: `2026-06-26-update-feed-filter-aggregates-nfc-vs-nfkc.md` が accept され、`aggregates.md` / `validation.md` / `workflows/update-feed-filter.md` の 3 upstream に適用済み。

## 2026-06-29 proposal 作成（comprehensive: 6 file × 10 箇所）

- target_files: 6 domain ファイル × 10 箇所
  - domain/bounded-contexts.md (行63, 126)
  - domain/glossary.md (行91, 271)
  - domain/event-storming.md (行104, 134)
  - domain/types.md (行84, 101)
  - domain/workflows/list-feed.md (行58)
  - domain/ui-fields/index.md (行31)
- file: .ori/proposals/2026-06-29-update-feed-filter-nfkc-spread.md
- reason: accepted だった 2026-06-26 proposal の target_files が list-feed 工作前の時点で確定していたため 6 ファイルが coverage から漏れ、NFC 記述が残留。aggregates.md I-F1 (NFKC) / validation.md S8 (NFKC) との同-性を担保するため一括 terminology 整理。元 proposal の rationale (NFKC 必須、NFC では半角化せず S8 不成立) を継承。
- 影響 slice: update-feed-filter (re-derive 必須, ori-64x.8 close 条件) + list-feed (bounded-contexts.md + workflows/list-feed.md の 2 upstream が dirty 化, 再 derive + review 必要)
- supersedes_partial: 2026-06-26-update-feed-filter-aggregates-nfc-vs-nfkc.md (rationale 継承, 未 cover だった 6 ファイルに同 decision を拡張適用)
- status: pending — `/ori-review-proposals` で人間 ratification 待ち
