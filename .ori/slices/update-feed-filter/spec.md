---
coherence:
  source: derived
  last_derived: 2026-06-26
  upstream:
    - domain/workflows/update-feed-filter.md#update-feed-filter
    - domain/aggregates.md#note-feed-aggregate
    - domain/bounded-contexts.md#note-feed
    - domain/validation.md#s8-query-normalize
  hash:
    domain/workflows/update-feed-filter.md#.*: 79e229cbcd37
    domain/aggregates.md#.*: 9f9048f5816b
    domain/bounded-contexts.md#.*: 4d579125a513
    domain/validation.md#.*: 5294b0c32f1b
ori:
  schema:
    propagation_level: file
---

# update-feed-filter spec {#update-feed-filter-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive update-feed-filter`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

`NoteFeed` aggregate（read model、揮発）の **filter** を更新する command slice。Note Feed BC の最初の slice であり、本 slice で `NoteFeed` aggregate root と `FeedFilter` / `NormalizedQuery` / `DateRangeFilter` VO を新規定義する。

> domain/workflows/update-feed-filter.md より：「NoteFeed の filter（query / date_range / tag）を更新する。read model のため domain event を発行しない（揮発状態）」
>
> domain/bounded-contexts.md#note-feed-purpose より：「Note 集合に対する read side を司る。検索文字列・期間・タグでの絞り込みと createdAt / updatedAt × 昇降順での並べ替えを提供する」

入力は `UpdateFeedFilterCommand` の sum type で 4 分岐（`SetQuery` / `SetDateRange` / `SetTag` / `ClearAll`）。出力は更新後 `NoteFeed`。`Tag` VO は Note Capture BC の shared kernel として既存（`assign-tag` slice 経由で main 到達済）を再利用する。

本 slice の scope は **filter 操作のみ**。`sort` / `visible_notes` / Note source 取得は別 slice（`change-sort-order`、`list-feed`）の責務。

## 入出力 {#io}

### Input {#io-input}

> domain/workflows/update-feed-filter.md#input より：

```rust
enum UpdateFeedFilterCommand {
  SetQuery { raw: String },                     // 検索バー入力
  SetDateRange { range: DateRangeFilter },      // 期間プリセット選択
  SetTag { tag: Option<Tag> },                  // タグチップクリック / 解除
  ClearAll,                                     // 全リセット
}
```

`Tag` は `note_capture::shared::types::Tag`（Shared Kernel として再利用）。`raw` は UI 層から受け取る検証前の文字列で、本 slice 内で `NormalizedQuery` 構築（NFC + lowercase + 空文字 → None）を行う。

### Output {#io-output}

戻り値: `NoteFeed`（更新後、move semantics）

> domain/workflows/update-feed-filter.md#output より：「NoteFeed（filter 更新後）／ domain event: **なし**（揮発）」

- 各 command 適用後の `NoteFeed` を返す。current state を base に diff 適用する pure 関数
- 副作用なし（C-UF6: I/O 無し、event 発行無し）

### Errors {#io-errors}

> domain/workflows/update-feed-filter.md#errors より：「**なし**（無効入力は VO 構築側で吸収するか、UI 層で reject）」

戻り型に `Result` を露出しない。

- `SetQuery { raw: "" }` / 空白のみ → `NormalizedQuery` が `None` に降格（=ClearAll の query 相当）
- `SetTag { tag: None }` → tag filter を解除（Tag VO 構築は呼出側責務）
- `SetDateRange` の `DateRangeFilter::Custom { from, to }` の `from > to` のような不正組合せは VO 構築側で reject する想定だが、現 domain 文書では明示無し → 本 slice では受理し、`Custom` を そのまま保持する（VO レベルバリデーションは将来の DateRangeFilter VO 強化で扱う、[#oq-date-range-validation](#oq-date-range-validation)）

## 不変条件 {#invariants}

slice 完了時に成立すべき条件。括弧内は domain での出典。

### NoteFeed Aggregate 由来 {#invariants-note-feed-aggregate}

> domain/aggregates.md#note-feed-aggregate-invariants より引用：

- **I-F1**: `filter.query` は常に **NFC 正規化済み + lowercase 済み**（マッチング時に再正規化しない）。本 slice の `SetQuery` 経路が単一のエントリポイントとして I-F1 を確立する
- **I-F4**: filter の合成は AND（query ∧ date_range ∧ tag）。本 slice では filter の値だけを書き換え、合成セマンティクスは `NoteFeed::visible_notes` 側（別 slice）が担保する
- **I-F6**: 起動時、`filter` は常に空状態で初期化（揮発、Q3 決定）。`ClearAll` は I-F6 の初期化状態と等価な遷移を提供する

I-F2 / I-F3 / I-F5 / I-F7 は `visible_notes` / `sort` 側の不変条件のため本 slice の範囲外。

### slice 固有制約 {#invariants-slice-specific}

- **C-UF1**: 戻り値の `NoteFeed.filter.query` は `Option<NormalizedQuery>` であり、`Some(q)` ならば `q` は **NFC + lowercase 済み**（I-F1 を本 slice が施行する）
- **C-UF2**: `SetQuery { raw }` で `raw.trim().is_empty() == true` ならば、`filter.query = None`（空文字は filter 解除と等価、S8 walkthrough と整合）
- **C-UF3**: 同じ `SetQuery { raw: "GPT" }` を 2 回連続適用しても結果が等しい（**冪等**）。`NormalizedQuery::from_raw` が決定的なため自動的に成立する
- **C-UF4**: `ClearAll` 適用後の `filter` は `{ query: None, date_range: All, tag: None }`（I-F6 と整合、I-S3 デフォルトの NoteFeed 初期状態）
- **C-UF5**: 各 command は他 filter 軸に **影響しない**（直交性）。`SetTag` は `query` を保持、`SetQuery` は `tag` / `date_range` を保持、`SetDateRange` は `query` / `tag` を保持。`ClearAll` のみ全リセット
- **C-UF6**: 本 slice は **副作用ゼロ**：I/O なし、event 発行なし、Repository への呼出なし（NoteFeed は揮発 read model のため persist 経路自体が無い）
- **C-UF7**: `SetTag { tag: Some(t) }` で `t` が現在の `filter.tag == Some(t)` と等値の場合は **同値 NoteFeed** を返す（冪等性）。`SetTag { tag: None }` は tag filter 解除
- **C-UF8**: `SetQuery` 経路で `NormalizedQuery::from_raw("Ｇｐｔ")` （全角）は NFC 正規化により `"gpt"` （半角 lowercase）へ変換される（S8 シナリオの本質）。`NormalizedQuery` 内部表現に **生の入力文字列を保持しない**

## テスト観点 {#test-perspectives}

phase 3 で failing test に展開する観点を列挙。各観点は 1 つ以上の test に対応する想定。

### SetQuery: 正規化と filter 反映 {#tp-set-query}

- **TP-Q1**: `SetQuery { raw: "GPT" }` → `filter.query == Some(NormalizedQuery("gpt"))`（lowercase、I-F1）
- **TP-Q2**: `SetQuery { raw: "Ｇｐｔ" }`（全角）→ `filter.query == Some(NormalizedQuery("gpt"))`（S8: NFC + lowercase、C-UF8）
- **TP-Q3**: `SetQuery { raw: "" }` → `filter.query == None`（空文字は解除、C-UF2）
- **TP-Q4**: `SetQuery { raw: "   " }`（空白のみ）→ `filter.query == None`（C-UF2、trim 後 empty）
- **TP-Q5**: 同じ `SetQuery { raw: "gpt" }` を 2 回適用 → 結果同一（C-UF3 冪等）
- **TP-Q6**: `SetQuery` 適用後 `tag` / `date_range` が直前値のまま保持される（C-UF5 直交性）

### SetDateRange {#tp-set-date-range}

- **TP-D1**: `SetDateRange { range: Last7Days }` → `filter.date_range == Last7Days`
- **TP-D2**: `SetDateRange { range: All }` → `filter.date_range == All`
- **TP-D3**: `SetDateRange { range: Custom { from, to } }` → そのまま保持
- **TP-D4**: `SetDateRange` 適用後 `query` / `tag` が直前値のまま保持される（C-UF5）

### SetTag {#tp-set-tag}

- **TP-T1**: `SetTag { tag: Some(Tag::new("coding")) }` → `filter.tag == Some(Tag("coding"))`
- **TP-T2**: `SetTag { tag: None }` → `filter.tag == None`（解除）
- **TP-T3**: 現在 `tag == Some("a")` の状態で `SetTag { tag: Some("a") }` → 同値 NoteFeed（C-UF7 冪等）
- **TP-T4**: `SetTag` 適用後 `query` / `date_range` が直前値のまま保持される（C-UF5）

### ClearAll {#tp-clear-all}

- **TP-C1**: 任意の filter 状態で `ClearAll` → `filter == { query: None, date_range: All, tag: None }`（C-UF4, I-F6）
- **TP-C2**: `ClearAll` を 2 回適用しても結果同一（冪等）

### S8 シナリオ: 検索文字列の NFC + lowercase 正規化 {#tp-s8}

> domain/validation.md#s8-query-normalize を walkthrough：

- **TP-S8-1**: Given `filter = empty`、When `SetQuery { raw: "gpt" }`、Then `filter.query == Some(NormalizedQuery("gpt"))`（半角入力なので変化なし）
- **TP-S8-2**: Given `filter = empty`、When `SetQuery { raw: "Ｇｐｔ" }`（全角）、Then `filter.query == Some(NormalizedQuery("gpt"))`（NFC + lowercase で半角化）
- **TP-S8-3**: 本 slice 内で event は発行されない（NoteFeed は read model、C-UF6）

### 不変条件チェック {#tp-invariants}

- **TP-I1**: 任意の `SetQuery` 入力に対し、戻り値の `filter.query` は `None` または NFC + lowercase 済み（I-F1 / C-UF1）。property test 候補
- **TP-I6**: `ClearAll` 後の `filter` が I-F6 初期状態と等しい

### 副作用 {#tp-side-effects}

- **TP-SE1**: `UpdateFeedFilterUseCase::apply` のシグネチャは `(NoteFeed, UpdateFeedFilterCommand) -> NoteFeed`（type-level: no Repository / Bus 引数、C-UF6 を type で固定）

## 実装ノート {#impl-notes}

### アーキ層への落とし込み {#impl-layers}

DDD-VSA-Hex / typescript-tauri の階層に従い、本 slice は Rust 側で実装する（`implementation.language: rust`）。Note Feed BC は本 slice で新規に切られる。

```
apps/promptnotes/src-tauri/src/note_feed/
├── mod.rs                  # pub use slices::*; shared::*
├── shared/
│   ├── mod.rs
│   ├── types/
│   │   ├── mod.rs
│   │   ├── note_feed.rs    # NoteFeed aggregate root (filter のみ本 slice で扱う)
│   │   ├── feed_filter.rs  # FeedFilter VO (query / date_range / tag の組)
│   │   ├── normalized_query.rs  # NormalizedQuery VO (NFC + lowercase smart constructor、I-F1)
│   │   └── date_range_filter.rs # DateRangeFilter enum
│   └── ports.rs            # (本 slice では空。将来 NoteSource 等の port が住む)
└── slices/
    ├── mod.rs
    └── update_feed_filter/
        ├── mod.rs
        ├── domain.rs       # UpdateFeedFilterCommand
        ├── application.rs  # UpdateFeedFilterUseCase: pure pattern match
        └── tests.rs        # TP-* を網羅
```

### Note Feed BC の最小スコープ {#impl-bc-scope}

本 slice では `NoteFeed` aggregate root を「filter 軸だけが意味を持つ最小構造」として定義する。`sort: SortOrder` と `source: &[Note]` は **後続 slice で本格的に使う前提のため、本 slice では型として `default()` 相当を保持するだけ** とする：

- `sort: SortOrder` — `note_capture` / `user_preferences` の `SortOrder` 型を借りるのではなく、`note_feed` 内で独立に持つ（後続 slice が決定）。本 slice では空コンストラクタ + default で十分
- `source: Vec<NoteId>` 相当 — 本 slice では空 Vec で OK。`visible_notes` は別 slice の担当

### Tag VO の cross-BC import {#impl-tag-import}

`SetTag { tag: Option<Tag> }` の `Tag` は `crate::note_capture::shared::types::Tag` を直接 import する（Shared Kernel 規約、`bounded-contexts.md` の Note Aggregate 注記より）。Note Feed BC は Tag VO を **読むだけ** で構築・mutation はしない。

### NormalizedQuery の実装 {#impl-normalized-query}

- `NormalizedQuery::from_raw(raw: &str) -> Option<Self>`:
  1. `raw.nfc().collect::<String>()` で NFC 正規化（`unicode-normalization` crate）
  2. `.to_lowercase()`
  3. `.trim()` 後 empty なら `None`、そうでなければ `Some(NormalizedQuery(s))`
- `unicode-normalization = "0.1"` を `Cargo.toml` の `[dependencies]` に追加する必要あり
- 内部は `String`、`as_str() -> &str` で読み出し

### Pipeline ステップ {#impl-pipeline}

> domain/workflows/update-feed-filter.md#steps の DMMF pipeline を採用：

`UpdateFeedFilterUseCase::apply(feed, cmd)` は **pure pattern match** で 4 分岐：

```rust
match cmd {
  SetQuery { raw } => feed.with_query(NormalizedQuery::from_raw(&raw)),
  SetDateRange { range } => feed.with_date_range(range),
  SetTag { tag } => feed.with_tag(tag),
  ClearAll => feed.reset_filters(),
}
```

`with_query` / `with_date_range` / `with_tag` / `reset_filters` は `NoteFeed` の inherent method として実装（`aggregates.md#note-feed-aggregate-operations` の `filter_by_*` / `clear_filters` の **slice 内 alias**）。Domain 操作名と完全一致させたい場合は別 follow-up。

### Out of scope {#out-of-scope}

本 slice は **filter 軸の更新だけ** を扱う。以下は別 slice / layer の責務：

- `NoteFeed::visible_notes` の matching ロジック（`change-sort-order` / `list-feed` で別 issue）
- `NoteFeed::change_sort` および `SortPreferenceChanged` event 発行（`change-sort-order` slice）
- Note source（Shared Kernel 経由の `&[Note]`）の引き渡し（composition root + repository layer）
- Tauri command surface（`#[tauri::command] update_feed_filter`）と TS bindings（follow-up）

## Open Questions {#open-questions}

### oq-date-range-validation {#oq-date-range-validation}

- **問**: `DateRangeFilter::Custom { from, to }` で `from > to` の場合の挙動が domain 文書に明示無い
- **暫定方針**: 本 slice では VO を smart constructor 化せず enum variant としてそのまま受理する。`visible_notes` 側で範囲フィルタ計算時に空集合になるだけで実害なし
- **解決方向**: Phase 7 (validation) で S 系シナリオに追加する or follow-up issue で `DateRangeFilter::custom(from, to) -> Result<Self, _>` 化

### oq-source-shape {#oq-source-shape}

- **問**: `NoteFeed.source` の型表現（`&[Note]` か `Vec<NoteId>` か）が domain 文書では「Shared Kernel 経由」とだけあり具体型を確定していない
- **暫定方針**: 本 slice では `source` を `Vec<NoteId>`（空）に固定。`visible_notes` slice 着手時に確定
- **解決方向**: `change-sort-order` slice の design 段階で確定 → 本 slice 側を follow-up で adapt
