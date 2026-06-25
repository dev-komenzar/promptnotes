---
target: domain/workflows/auto-save-note.md#errors
by: slices/auto-save-note
reason: impl phase で AutoSaveError に `InvalidBody`（NoteBody 不正）と `LoadError`（read I/O 失敗）の 2 variant が必要と判明し、現在の workflow 規定 (2 variant) と乖離している
created: 2026-06-25
status: accepted
accepted_at: 2026-06-25
accepted_by: human (takuya.kometan@gmail.com)
applied_to: domain/workflows/auto-save-note.md#errors (4 variant に拡張) + #notes (NoteBody / load/persist 分離の補足追加)
followup: flush-note workflow にも同形を将来適用 (本 proposal で同時改訂はしない)
---

# Proposal: auto-save-note workflow の Errors を 4 variant に拡張する

## 発見の経緯 {#context}

- 検出元：`slices/auto-save-note` の phase 4 (impl-green) と phase 6 (review)
- 試みていたこと：spec.md の 2 variant 規定 (`NoteNotFound`, `PersistError`) でパイプラインを実装
- 想定との差:
  - **InvalidBody 経路の必要性**: `NoteBody::new(new_body)` は実態として `---` 行を reject する fallible constructor（create-note BC の I-N6 由来）。workflow 規定通り「Errors なし」では parse 失敗ケースを握り潰すしかない
  - **LoadError 経路の必要性**: `NoteRepository::load_by_id(id)` は `Ok(None)`（不在 → `NoteNotFound`）と `Err(io::Error)`（read I/O 失敗 / 既存 `.md` ファイルの parse 失敗）の 2 系統があり、後者を `PersistError` に丸めると意味的に「write 失敗」と区別不能（Pass 1 review HIGH-2）

## 現状仕様 {#current}

> domain/workflows/auto-save-note.md#errors より：

```
## Errors {#errors}

- `NoteNotFound { id: NoteId }`
- `PersistError { path: PathBuf, cause: io::Error }`
```

> domain/aggregates.md#note-aggregate-elements より（**`NoteBody` 定義**）：

```
- **NoteBody** (VO)
  - 任意の UTF-8 文字列（空文字も許容、frontmatter 由来の `---` を含まない）
```

aggregate 側は既に「`---` を含まない」と書いてあるが、workflow#errors 側にはこの不変条件違反に対応する error 経路が無い。

## 矛盾／欠落 {#gap}

派生側 (`slices/auto-save-note/spec.md`, impl) が必要とする条件:

- AutoSaveError の variant 集合に少なくとも以下が必要：
  1. `NoteNotFound { id }` — 既存
  2. `InvalidBody { source: NoteBodyError }` — **新規必要**：`NoteBody::new` 失敗を呼び出し元に伝える
  3. `LoadError { path, source: io::Error }` — **新規必要**：read 失敗を意味的に分離
  4. `PersistError { path, source: io::Error }` — 既存（write 失敗専用に意味を絞る）

現状ドメインで満たせない理由:

- workflow#errors が 2 variant 固定なので、impl 側が増やすと spec.md (derived) を直編集することになり、CoDD の SSoT (frontmatter `coherence.source: derived`) と矛盾する。Pass 2 review HIGH-3 で指摘済

## 提案する変更 {#proposal}

### domain/workflows/auto-save-note.md#errors を以下に置換

```
## Errors {#errors}

- `NoteNotFound { id: NoteId }` — load_by_id が `Ok(None)` を返した場合
- `InvalidBody { source: NoteBodyError }` — `NoteBody::new(new_body)` が失敗した場合（frontmatter delimiter line `---` を含む等）
- `LoadError { path: PathBuf, source: io::Error }` — load_by_id の read I/O 失敗 / 既存 `.md` ファイルの parse 失敗
- `PersistError { path: PathBuf, source: io::Error }` — `NoteRepository::write` の I/O 失敗 (write 経路専用)
```

### domain/workflows/auto-save-note.md#notes に補足を追記

```
- `NoteBody` 不変条件（frontmatter delimiter `---` を含まない）は aggregate 由来。AutoSave は新規 body を受け取り構築するため、aggregate と同じ不変条件チェックを通る
- read 失敗と write 失敗は意味的に異なる経路として error variant を分離する。前者は UI 側「ノートが壊れている」フィードバック、後者は「保存できない」フィードバックを別に出せる
```

### 影響範囲

- **直接影響**: `slices/auto-save-note/spec.md` を `/ori-derive auto-save-note` で再生成すると frontmatter の hash と本文（io-errors, C-AS1）が upstream に揃う → Pass 2 HIGH-3 / HIGH-4 が同時解消
- **波及候補**: `flush-note` slice も同じ NoteBody / NoteRepository 経路を踏むので、`flush-note` workflow の Errors 規定も同形に揃える必要がある（**TBD: flush-note workflow も同時改訂するか、本提案 accept 後に flush-note 派生時に個別判断するかをユーザ確認**）

## 代替案 {#alternatives}

1. **spec/impl 分離維持**: workflow.md は 2 variant のまま、slice spec.md に open question として「impl は防衛で 4 variant」を明記。`oq-invalid-body-variant` で部分的にやっている運用。コスト最小だが CoDD の「spec is source of truth」(memory: feedback_spec_is_source_of_truth) に反する
2. **aggregate 側の NoteBody を緩める**: `NoteBody` constructor を infallible に変更し、`---` 行を許容する。永続化層で encode/escape する。create-note BC の I-N6 (frontmatter 区切り防護) と衝突するため、Note Capture BC 横断の議論が必要 → 大改修
