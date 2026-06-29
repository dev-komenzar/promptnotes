---
target_files:
  - domain/bounded-contexts.md#note-feed-ubiquitous-language
  - domain/bounded-contexts.md#notes
  - domain/glossary.md#glossary-normalized-query
  - domain/glossary.md#cross-context-note-diff
  - domain/event-storming.md#note-feed-hot-spots
  - domain/event-storming.md#open-questions
  - domain/types.md#dependencies
  - domain/types.md#coverage-aggregates
  - domain/workflows/list-feed.md#steps
  - domain/ui-fields/index.md#mapping
by: slices/update-feed-filter
reason: accepted だった 2026-06-26-update-feed-filter-aggregates-nfc-vs-nfkc.md proposal の target_files が list-feed 工作前の時点で確定していたため、同 proposal の coverage から漏れた 6 domain ファイル（bounded-contexts / glossary / event-storming / types / workflows/list-feed / ui-fields/index）に NFC 記述が残留。aggregates.md I-F1 (NFKC) / validation.md S8 walkthrough (NFKC) と同-性を担保するための一括 terminology 整理。
created: 2026-06-29
status: accepted
accepted_at: 2026-06-29
accepted_by: human (takuya.komenzar@gmail.com)
applied_to:
  - domain/bounded-contexts.md#note-feed-ubiquitous-language (行63)
  - domain/bounded-contexts.md#notes (行126)
  - domain/glossary.md#glossary-normalized-query (行91)
  - domain/glossary.md#cross-context-note-diff (行271)
  - domain/event-storming.md#note-feed-hot-spots (行104)
  - domain/event-storming.md#open-questions (行134)
  - domain/types.md#dependencies (行84)
  - domain/types.md#coverage-aggregates (行101)
  - domain/workflows/list-feed.md#steps (行58)
  - domain/ui-fields/index.md#mapping (行31)
related_beads:
  - ori-64x
  - ori-64x.8
  - ori-84i
  - ori-84i.finalize
supersedes_partial:
  proposal: 2026-06-26-update-feed-filter-aggregates-nfc-vs-nfkc.md
  note: 同 proposal の rationale (NFKC 必須、NFC では半角化せず S8 不成立) を継承し、未 cover だった 6 ファイルに同 decision を拡張適用する
---

# Proposal: domain 6 ファイルに残留する "NFC" terminology を NFKC に一括整理

## 発見の経緯 {#context}

- 検出元：`slices/update-feed-filter` の `/ori-derive` 再実行（ori-64x.8 follow-up 対応）
- 何を試みていたか：accepted となった `2026-06-26-update-feed-filter-aggregates-nfc-vs-nfkc.md` proposal の「受理時の追加作業」に従い、`/ori-derive update-feed-filter` で spec.md を再生成しようとした
- 何が想定と違ったか：manifest.derives_from に列挙された 4 upstream のうち `bounded-contexts.md#note-feed` のみ NFC 記述が残っており、他3 upstream（aggregates.md / validation.md / workflows/update-feed-filter.md）の NFKC 表記と矛盾。ori-derive skill step 4「矛盾検出」の停止条件に合致
- 追加調査で、`bounded-contexts.md` 以外にも 5 ファイル（glossary / event-storming / types / workflows/list-feed / ui-fields/index）に同一の NFC 記述が残留していることを grep で確認。これらは全て list-feed slice 工作(commit `cf6b472`)前後で編集されたか、あるいは元 proposal 発足時点から存在していたが target_files から漏れていた
- ルート原因：元 proposal は `aggregates.md` / `validation.md` / `workflows/update-feed-filter.md` の 3 ファイルを target として発足したが、その時点で ubiquitous language や依存 crate 表、ui-fields mapping 等、同一 terminology を使う domain 全体を走査していなかった。また list-feed 工作で別途 `bounded-contexts.md` / `workflows/list-feed.md` が編集された際にも NFC→NFKC 整合性チェックが走らなかった

## 現状仕様 {#current}

10 箇所の NFC 記述が残留。各引用：

### 1. domain/bounded-contexts.md 行63 (`#note-feed-ubiquitous-language`)

> - **Query** — 検索バーの文字列。case-insensitive substring + NFC 正規化で `body` と `tags` にマッチ

### 2. domain/bounded-contexts.md 行126 (`#notes`)

> Note Feed は in-memory index / NFC 正規化 / 検索文字列マッチの世界。

### 3. domain/glossary.md 行91 (`#glossary-normalized-query`)

> - **定義**: 検索バー入力を NFC 正規化 + lowercase 化した文字列

### 4. domain/glossary.md 行271 (`#cross-context-note-diff`)

> Note Feed: in-memory index / NFC 正規化 / 検索文字列マッチの世界。

### 5. domain/event-storming.md 行104 (`#note-feed-hot-spots` 内 Q7 決定)

> - マッチング: case-insensitive substring + NFC 正規化（regex / wildcard は Non-Goal）

### 6. domain/event-storming.md 行134 (`#open-questions` 内 Q7)

> - Q7: 検索の対象範囲 → 本文 + タグのみ、case-insensitive + NFC

### 7. domain/types.md 行84 (`#dependencies` crate 表)

> | **unicode-normalization** | NFC 正規化（NormalizedQuery, body マッチング、I-F1 / I-F5） |

### 8. domain/types.md 行101 (`#coverage-aggregates` NoteFeed 行)

> | NoteFeed | note_feed.rs | `visible_notes()` 内で filter→sort、`SortOrder` で tiebreak（I-F3）。query は `NormalizedQuery::from_raw` で必ず NFC + lowercase（I-F1） |

### 9. domain/workflows/list-feed.md 行58 (`#steps` applyFilter)

>    - I-F1: query は NFC + lowercase 済 (filter 構築時に確立済)

### 10. domain/ui-fields/index.md 行31 (`#mapping` table)

> | `NormalizedQuery` | search input | `NormalizedQuery::from_raw` で NFC + lowercase |

対して、既に accepted & applied 済みの他 upstream は全て NFKC に統一済み：

- `aggregates.md` 行112 / 116 / 125 / 140 — NFKC (compatibility normalization) + lowercase
- `validation.md` 行186 / 200 / 202-203 — NFKC (compatibility normalization) + lowercase
- `workflows/update-feed-filter.md` 行40 — NFKC (compatibility normalization) 正規化 + lowercase 化

## 矛盾／欠落 {#gap}

- accepted proposal `2026-06-26-update-feed-filter-aggregates-nfc-vs-nfkc.md` の `applied_to` / `target_files` に上記 6 ファイルが含まれていなかったため、NFC 記述が未修正のまま残存
- `/ori-derive update-feed-filter` 実行時、upstream 間で terminology 不整合（3 NFKC vs 1 NFC = bounded-contexts.md）が発生し skill step 4 の停止条件に合致。spec.md の再生成ができない
- `/ori-derive list-feed` 実行時にも同様の停止条件に合致する可能性あり（list-feed の derives_from にも `bounded-contexts.md#note-feed` が含まれるため）

## 提案する変更 {#proposal}

6 ファイル 10 箇所の "NFC" を "NFKC (compatibility normalization)" に一括置換。各変更は **terminology の整合のみ** で、規則の意味や適用範囲の変更は伴わない：

### 1. domain/bounded-contexts.md 行63

- before: `- **Query** — 検索バーの文字列。case-insensitive substring + NFC 正規化で \`body\` と \`tags\` にマッチ`
- after:  `- **Query** — 検索バーの文字列。case-insensitive substring + NFKC (compatibility normalization) 正規化で \`body\` と \`tags\` にマッチ`

### 2. domain/bounded-contexts.md 行126

- before: `Note Feed は in-memory index / NFC 正規化 / 検索文字列マッチの世界。`
- after:  `Note Feed は in-memory index / NFKC (compatibility normalization) 正規化 / 検索文字列マッチの世界。`

### 3. domain/glossary.md 行91

- before: `- **定義**: 検索バー入力を NFC 正規化 + lowercase 化した文字列`
- after:  `- **定義**: 検索バー入力を NFKC (compatibility normalization) 正規化 + lowercase 化した文字列`

### 4. domain/glossary.md 行271

- before: `Note Feed: in-memory index / NFC 正規化 / 検索文字列マッチの世界。`
- after:  `Note Feed: in-memory index / NFKC (compatibility normalization) 正規化 / 検索文字列マッチの世界。`

### 5. domain/event-storming.md 行104

- before: `- マッチング: case-insensitive substring + NFC 正規化（regex / wildcard は Non-Goal）`
- after:  `- マッチング: case-insensitive substring + NFKC (compatibility normalization) 正規化（regex / wildcard は Non-Goal）`

### 6. domain/event-storming.md 行134

- before: `- Q7: 検索の対象範囲 → 本文 + タグのみ、case-insensitive + NFC`
- after:  `- Q7: 検索の対象範囲 → 本文 + タグのみ、case-insensitive + NFKC (compatibility normalization)`

### 7. domain/types.md 行84

- before: `| **unicode-normalization** | NFC 正規化（NormalizedQuery, body マッチング、I-F1 / I-F5） |`
- after:  `| **unicode-normalization** | NFKC (compatibility normalization) 正規化（NormalizedQuery, body マッチング、I-F1 / I-F5） |`

### 8. domain/types.md 行101

- before: `query は \`NormalizedQuery::from_raw\` で必ず NFC + lowercase（I-F1）`
- after:  `query は \`NormalizedQuery::from_raw\` で必ず NFKC (compatibility normalization) + lowercase（I-F1）`

### 9. domain/workflows/list-feed.md 行58

- before: `   - I-F1: query は NFC + lowercase 済 (filter 構築時に確立済)`
- after:  `   - I-F1: query は NFKC (compatibility normalization) + lowercase 済 (filter 構築時に確立済)`

### 10. domain/ui-fields/index.md 行31

- before: `| \`NormalizedQuery\` | search input | \`NormalizedQuery::from_raw\` で NFC + lowercase |`
- after:  `| \`NormalizedQuery\` | search input | \`NormalizedQuery::from_raw\` で NFKC (compatibility normalization) + lowercase |`

### Rationale（元 proposal の decision を継承）

`aggregates.md` I-F1 は NFKC を必須とし、その理由を `全角 Latin / 半角 Latin、半角カナ / 全角カナ等の互換等価文字を同一視するため。canonical decomposition のみの NFC では半角化が起きず、S8 シナリオが成立しない` と明記済み（accepted proposal により aggregates.md 行125 に改訂済み）。

実装側（`note_feed/shared/types/normalized_query.rs:17`）は既に `raw.nfkc()` を採用しており、本 proposal は domain terminology を実装事実に合わせる整合作業に過ぎない。規則の意味変更は伴わない。

## 影響範囲 {#impact}

### Slice への波及（dirty mark 分析）

#### `update-feed-filter` (ori-64x, current_phase=null, finalized_at=2026-06-26)

- derives_from:
  - `domain/workflows/update-feed-filter.md#update-feed-filter` — **本 proposal 対象外**（既に NFKC）
  - `domain/aggregates.md#note-feed-aggregate` — **本 proposal 対象外**（既に NFKC）
  - `domain/bounded-contexts.md#note-feed` — **本 proposal 対象** ✅
  - `domain/validation.md#s8-query-normalize` — **本 proposal 対象外**（既に NFKC）
- accept 後: `bounded-contexts.md#note-feed` の hash 変化 → spec.md の `coherence.hash` が不一致 → dirty mark 付与 → `/ori-derive update-feed-filter` 再実行で spec.md 再生成可能（これが ori-64x.8 close の直接条件）
- 現状 status.yaml.dirty[] は空、proposals は accepted 1件のみ。accept 後に dirty 1件 (`domain/bounded-contexts.md#note-feed`) が伝播する

#### `list-feed` (ori-84i, current_phase=finalize, finalized_at=2026-06-27)

- derives_from:
  - `domain/workflows/list-feed.md#list-feed` — **本 proposal 対象** ✅
  - `domain/aggregates.md#note-feed-aggregate` — **本 proposal 対象外**（既に NFKC）
  - `domain/bounded-contexts.md#note-feed` — **本 proposal 対象** ✅
  - `domain/validation.md#s12-startup-state` — **本 proposal 対象外**
- accept 後: 2 upstream hash 変化 → list-feed spec.md の `coherence.hash` が 2件 不一致 → dirty mark 2件伝播 → `/ori-derive list-feed` 再実行が必要
- 現状 status.yaml.dirty[] は空、proposals は空。list-feed は既に finalize 済（ori-84i.finalize closed）だが、本 accept によって再 dirty 化する。list-feed 側の再 derive + review 必要（新規 follow-up issue 検討）

### 他 slice への波及

- `change-sort-order` (ori-64x.9 系): derives_from に `aggregates.md` / `validation.md` を含むが、いずれも既に NFKC のため本 proposal の影響なし
- `assign-tag` / `capture-auto-save` 等 Note Capture 系: NFC/NFKC は query 関係のみのため影響なし

### 実装コードへの影響

- `note_feed/shared/types/normalized_query.rs:17` は既に `raw.nfkc()` を採用済み → 影響なし
- `note_feed/slices/list_feed/` の visible_notes 実装: 既に NormalizedQuery 経由で NFKC 化された query を使用 → 影響なし

## 代替案 {#alternatives}

- **なし**：3 upstream が既に NFKC で accepted 済み、かつ実装コードも `nfkc()` 採用済みのため、本 proposal の却下は domain 全体にわたる terminology 矛盾の恒久化を意味する。`/ori-derive` の度に skill step 4 停止条件を踏むことになり、後続 slice の derive / review workstream が止まる。却下は強く非推奨
- **部分 accept**：bounded-contexts.md のみ accept して他 5 ファイルを却下する案も考えられるが、glossary / event-storming / types / ui-fields にも同一 terminology が残るため、結局 `/ori-derive list-feed` 等で同様の矛盾が再発する。6 ファイル一括 accept が最も整合的
