---
coherence:
  source: derived
  last_derived: 2026-06-27
  upstream:
    - domain/workflows/list-feed.md#list-feed
    - domain/aggregates.md#note-feed-aggregate
    - domain/bounded-contexts.md#note-feed
    - domain/validation.md#s12-startup-state
  hash:
    domain/workflows/list-feed.md#.*: c5b5e65e566a
    domain/aggregates.md#.*: 5beb07a42810
    domain/bounded-contexts.md#.*: 2dc895d7be08
ori:
  schema:
    propagation_level: file
---

# list-feed spec {#list-feed-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive list-feed`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

`storage_dir` 配下の Note `.md` を全件読み込んで NoteFeed の `source: Vec<Note>` を hydrate し、現在の `filter` / `sort` を適用した `visible_notes` を返す **read pipeline** slice。

> domain/workflows/list-feed.md より：「アプリ起動時に `storage_dir/*.md` を全件 parse し、`NoteFeed.source` を hydrate して `filter` / `sort` を適用した `visible_notes` を返す。書き込みは伴わず domain event を発行しない（揮発 read model）」
>
> domain/bounded-contexts.md#note-feed-purpose より：「Note 集合に対する read side を司る」

トリガーは (1) アプリ起動時 (load-settings 完了直後)、(2) 手動 Refresh (本 MVP では UI 未配線、binding のみ用意)。本 slice によって NoteFeed BC の read query が初めて end-to-end で繋がる (update-feed-filter / change-sort-order は state mutation のみで `source` を持たなかった)。

oq-source-shape (`Vec<Note>` vs `Vec<NoteId>`) は `Vec<Note>` で確定 (update-feed-filter / change-sort-order の deferred follow-up を本 slice で吸収)。理由は workflows/list-feed.md#notes / aggregates.md#note-feed-aggregate-elements に明記済。

## 入出力 {#io}

### Input {#io-input}

```rust
struct ListFeedCommand {
  // 引数なし。application service が NoteRepository / 現 NoteFeed を解決する
}
```

Tauri command 層では引数なしの `list_notes()` として export する。`storage_dir` は load-settings 経由で解決済の Settings から取り、`NoteFeed` は process-local の `InMemoryNoteFeedState` から snapshot を取る。

### Output {#io-output}

```rust
struct NoteFeedDto {
  notes: Vec<NoteSummary>,   // sort 適用済の visible_notes
}

struct NoteSummary {
  id: String,                // YYYYMMDDhhmmss
  body: String,
  tags: Vec<String>,
  created_at: String,        // YYYYMMDDhhmmss
  updated_at: String,
}
```

- 副作用は **state replace のみ** (read pipeline は pure)
- domain event 発行なし (C-LF6)

### Errors {#io-errors}

> domain/workflows/list-feed.md#errors より：「**なし**。port 実装側で個別 Note 単位で skip + log」

戻り型に `Result` を露出しない (`Result<NoteFeedDto, String>` でラップはするが Err variant は使わない、UI 側 catch も silent fallback)。

## 不変条件 {#invariants}

### NoteFeed Aggregate 由来 {#invariants-note-feed-aggregate}

> domain/aggregates.md#note-feed-aggregate-invariants より引用：

- **I-F2**: filter が空のとき `source` 全件を sort 順で返す
- **I-F3**: sort tiebreak は `id` (タイムスタンプ秒精度) — 決定論性
- **I-F4**: filter は AND 合成 (date_range ∧ tag ∧ query)
- **I-F5**: マッチング対象は `body` 全文 + `tags[*].name` のみ
- **I-F7**: 削除 (trash) された Note は除外 (本 slice は `list_all` の port 契約に委譲)

I-F1 / I-F6 は filter の構築側 (`update-feed-filter` slice) で確立済。

### slice 固有制約 {#invariants-slice-specific}

- **C-LF1**: `list_all()` は `Vec<Note>` を返し I/O / parse 失敗は **個別 skip**。「全部 or 何も読まない」ではない
- **C-LF2**: `apply_filter` は `&Vec<Note>` を消費せず `Vec<&Note>` を返す (read pipeline は所有権を奪わない)
- **C-LF3**: `apply_sort` は **stable sort** で I-F3 を satisfy する (`slice::sort_by` は stable)
- **C-LF4**: `apply_filter` は `query` `date_range` `tag` の **3 軸 AND** (I-F4) を **早期 short-circuit** で評価 (どれかが弾けば次軸を見ない)
- **C-LF5**: query 比較は `note.body().contains(q.as_str())` + `tag.name().contains(q.as_str())` の **substring + lowercase-only** (I-F1 で `q` が NFC + lowercase 済なので、`body` / `tag.name` 側も `to_lowercase()` してから比較)
- **C-LF6**: 本 slice は **domain event を発行しない** (read 側、揮発)
- **C-LF7**: `hydrate` 後の `NoteFeed.source` は I-F7 を満たす (port 契約 `list_all` が trash 済を除外する責務)
- **C-LF8**: `date_range` の比較は `Note.created_at` ベース (Q7 補足: filter 軸は created)。`Custom { from, to }` の `from > to` は空集合に降格 (`update-feed-filter` の oq-date-range-validation と整合)
- **C-LF9**: `list_notes` Tauri command は **冪等**: 同じ `storage_dir` 内容で何度呼んでも同じ `visible_notes` を返す

## テスト観点 {#test-perspectives}

### Port `NoteRepository::list_all` {#tp-list-all}

- **TP-LA1**: 空 `storage_dir` → `Vec::new()`
- **TP-LA2**: 2 件の valid `.md` がある → 2 件の `Note` を返す (順序は問わない)
- **TP-LA3**: malformed `.md` が混在 → valid のみ返し、malformed は skip (C-LF1)
- **TP-LA4**: ファイル拡張子 `.md` 以外は無視する (例: `.txt`, README 等)

### Pipeline: filter 適用 {#tp-apply-filter}

- **TP-F1**: filter 空 → `source` 全件 (I-F2)
- **TP-F2**: query "gpt" → body / tags に "gpt" を含む Note のみ (I-F5、C-LF5)
- **TP-F3**: query "Ｇｐｔ" (全角) 入力 → I-F1 が成立した filter なら "gpt" (半角) としてマッチ
- **TP-F4**: tag = Some("coding") → tags に "coding" を含む Note のみ
- **TP-F5**: date_range = Last7Days → 7 日以内 created の Note のみ
- **TP-F6**: query + tag + date_range の **AND 合成** (I-F4)
- **TP-F7**: query "" / None → query 軸無効

### Pipeline: sort 適用 {#tp-apply-sort}

- **TP-S1**: SortField=CreatedAt, Desc → created_at 降順
- **TP-S2**: SortField=CreatedAt, Asc → created_at 昇順
- **TP-S3**: SortField=UpdatedAt, Desc → updated_at 降順
- **TP-S4**: 同 sort key の 2 件 → `id` で tiebreak (I-F3、C-LF3)

### NoteFeed.hydrate / visible_notes {#tp-visible}

- **TP-V1**: hydrate → `source` が hydrate された Notes と等しい
- **TP-V2**: filter 適用 + sort 適用後の visible_notes が pipeline 出力と一致
- **TP-V3**: 同じ Notes で 2 回 hydrate → 結果は等しい (C-LF9 冪等)

### S12 シナリオ walkthrough {#tp-s12}

> domain/validation.md#s12-startup-state を walkthrough：

- **TP-S12-1**: Given storage_dir に 3 件 `.md`、When 起動 (= load-settings → list-feed)、Then visible_notes は 3 件 (filter 空 + sort default で全件)
- **TP-S12-2**: Given Settings.sort_preference = `{ updated_at, asc }`、When 起動、Then visible_notes は updated_at 昇順

### 副作用 {#tp-side-effects}

- **TP-SE1**: `ListFeedUseCase::execute` は `Repository` だけを inject、`EventBus` を取らない (C-LF6 を type-level に固定)

## 実装ノート {#impl-notes}

### アーキ層 {#impl-layers}

DDD-VSA-Hex / typescript-tauri に従い Rust 側で実装する。Note Feed BC の slice ディレクトリに追加：

```
apps/promptnotes/src-tauri/src/note_feed/
├── shared/
│   ├── types/
│   │   └── note_feed.rs            # NoteFeed に source: Vec<Note> / hydrate() / visible_notes() を追加
│   └── ports.rs                    # NoteFeedSource trait (= NoteRepository::list_all の wrapper、新規)
└── slices/
    └── list_feed/
        ├── mod.rs
        ├── domain.rs               # ListFeedCommand (引数なし)
        ├── application.rs          # ListFeedUseCase: load_all → hydrate → return feed
        ├── commands.rs             # #[tauri::command] list_notes
        └── tests.rs                # TP-* 網羅
```

- `note_capture::shared::ports::NoteRepository` に `list_all(&self) -> std::io::Result<Vec<Note>>` を default impl (`unimplemented!`) で追加
- `FsNoteRepository::list_all` を実装：`fs::read_dir(storage_dir)` で `.md` のみフィルタ → 既存の `parse_note_md` を再利用 → parse 失敗は log + skip
- `NoteFeed::hydrate(self, notes: Vec<Note>) -> NoteFeed` と `NoteFeed::visible_notes(&self, now: time::OffsetDateTime) -> Vec<&Note>` を追加 (`now` は date_range 計算用、test では fixed time inject)
- 既存 `update-feed-filter` の `InMemoryNoteFeedState` を再利用 (NoteFeed 単一インスタンス)
- 既存 `change-sort-order` の Settings 経路 (config_path / default_storage_dir resolve) を `list_notes` でも再利用

### Tauri command 配線 {#impl-tauri}

```rust
#[tauri::command]
pub async fn list_notes<R: Runtime>(
  app: AppHandle<R>,
  feed_state: State<'_, InMemoryNoteFeedState>,
) -> Result<NoteFeedDto, String> {
  // 1. resolve storage_dir from settings.json (load-settings の経路を再利用)
  // 2. FsNoteRepository::new(storage_dir).list_all()
  // 3. feed_state.snapshot().hydrate(notes)
  // 4. feed.visible_notes(now) → NoteSummary[] に投影
  // 5. feed_state.replace(feed)
}
```

`AppHandle` から `app_data_dir().join("notes")` を default で取り、`settings.json` がある場合はそこから上書きする。`list_settings()` を再呼出する形が一番安全。

### TS bindings {#impl-ts}

```
apps/promptnotes/src/lib/note-feed/slices/list-feed/index.ts
```

`listNotes()` → `NoteFeedDto`。`NoteSummary` の shape は page-main の `feed.svelte.ts` の `NoteSummary` 型と完全一致させる (現状 `id / body / tags / created_at / updated_at` で揃っている)。

### page-main wiring {#impl-page-main}

`PageMain.svelte` の `$effect` で `loadSettingsFn` 完了後に `listNotes()` を呼び、`feedStore.notes = result.notes` 相当を行う。store 側に `hydrateNotes(notes: NoteSummary[])` を追加する (既存 `prependNote` / `applyDelete` と直交)。

### Out of scope {#out-of-scope}

- **手動 Refresh の UI** (binding のみ用意、ボタン配線は別 issue)
- **NoteCreated event 購読での incremental update** (create-note slice が直接 `feedStore.prependNote` を呼ぶ既存挙動を維持)
- **DateRangeFilter VO smart constructor** (`update-feed-filter` の oq-date-range-validation を継承)
- **検索 highlight UI**
- **大量 Note 時の pagination / lazy load** (MVP 規模 ~1k Note では Vec<Note> hydration で十分)

## Open Questions {#open-questions}

### oq-list-feed-now-injection {#oq-list-feed-now-injection}

- **問**: date_range filter (`Last7Days` 等) の評価に `now: OffsetDateTime` が必要。aggregates.md / workflows/list-feed.md には現時点で `now` の依存が明記されていない
- **暫定方針**: `NoteFeed::visible_notes(&self, now)` に `now` を引数で注入。テスト容易性を優先
- **解決方向**: `change-sort-order` の `Timestamp` injection 規約と整合させた upstream proposal を `.ori/proposals/` に出す候補
