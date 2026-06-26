---
proposal:
  id: 2026-06-26-update-feed-filter-aggregates-nfc-vs-nfkc
  status: pending
  source: slice:update-feed-filter
  created_at: 2026-06-26
  target_files:
    - .ori/domain/aggregates.md#note-feed-aggregate-elements
    - .ori/domain/aggregates.md#note-feed-aggregate-invariants
    - .ori/domain/aggregates.md#note-feed-aggregate-operations
    - .ori/domain/validation.md#s8-query-normalize
    - .ori/domain/workflows/update-feed-filter.md#steps-set-query
  related_beads:
    - ori-64x
---

# Domain 修正提案: "NFC" → "NFKC" の terminology 整理

## 背景

`update-feed-filter` slice 実装中、S8 シナリオの walkthrough（全角 `Ｇｐｔ` → 半角 `gpt` matching）を満たすには Unicode **NFKC (Normalization Form Compatibility Composition)** が必要であり、**NFC (Canonical Composition)** だけでは半角化は起きない。

確認:

```
"Ｇｐｔ".chars().nfc().collect::<String>()  // → "Ｇｐｔ" (変化なし)
"Ｇｐｔ".chars().nfkc().collect::<String>() // → "Gpt"
```

Unicode 上、半角 Latin と全角 Latin は **canonically equivalent ではない**（互換等価のみ）。

## 現状の domain 記述

以下の箇所が全て **"NFC + lowercase"** と書いているが、意図する挙動は NFKC:

- `aggregates.md` 行 111: `query: Option<NormalizedQuery>` — NFC + lowercase 化済み
- `aggregates.md` 行 115: NormalizedQuery — NFC 正規化 + lowercase 化した結果
- `aggregates.md` 行 124: **I-F1**: query は常に NFC 正規化済み
- `aggregates.md` 行 139: filter_by_query — NFC + lowercase に正規化
- `validation.md` 行 200, 202-203: S8 walkthrough で NFC + lowercase: `gpt`（全角 → 半角化）
- `workflows/update-feed-filter.md` 行 40: NFC 正規化 + lowercase 化

## 提案

上記全箇所の **"NFC"** を **"NFKC (compatibility normalization)"** に置き換え、I-F1 を以下に改訂:

> **I-F1**: `query` は常に **NFKC 正規化済み + lowercase 済み**（マッチング時に再正規化しない）。NFKC を使う理由: 全角 Latin / 半角 Latin、半角カナ / 全角カナ等の互換等価文字を同一視するため。canonical decomposition のみの NFC では半角化が起きず、S8 シナリオが成立しない。

## 影響範囲

- `update-feed-filter` slice: 既に NFKC 実装済（`note_feed/shared/types/normalized_query.rs:17` `raw.nfkc()`）→ 影響なし
- 将来の `list-feed` / body matching slice: domain 修正後に body 側も同じ NFKC 関数を使う事で I-F1 (single canonical form) が一貫保証される

## 受理時の追加作業

- spec.md#impl-normalized-query (update-feed-filter slice) の "raw.nfc()" 記述を "raw.nfkc()" に更新（`/ori-derive` 再実行で自動）
- domain の hash 再計算 → update-feed-filter slice の `coherence.hash:` も再生成

## 却下時の代替

代替案: NFC のまま据え置く場合、S8 シナリオの "全角 → 半角化" 文言を削除し、半角入力前提に walkthrough を書き換える必要がある。ただし PromptNotes の検索 UX として全角入力時の挙動は重要なため、却下は推奨しない。
