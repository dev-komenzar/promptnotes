---
target: domain/workflows/remove-tag.md#errors
by: slices/remove-tag
reason: remove-tag slice の Pass 2 review で、Note Capture BC の write 系 3 slice (auto-save-note / assign-tag / remove-tag) が同型の `LoadError` variant を持つ pattern が確立されたため、upstream workflow#errors (現 2 variant) に `LoadError` を追記して derived spec と整合させる
created: 2026-06-29
status: pending
followup_of: 2026-06-25-auto-save-note-workflows-auto-save-note-errors.md
---

# Proposal: remove-tag workflow の Errors を 3 variant に拡張する (LoadError 追記)

## 発見の経緯 {#context}

- 検出元：`slices/remove-tag` の phase 6 (review Pass 2) — LOW-L (oq-remove-tag-error-3variant)
- 试みていたこと：spec.md `#oq-remove-tag-error-3variant` に「BC 内 3 slice 共通 pattern として確立してから domain proposal にする案」と記録し、spec 内説明で運用
- 想定との差:
  - **LoadError 経路の必要性**: `NoteRepository::load_by_id(id)` は `Ok(None)` (不在 → `NoteNotFound`) と `Err(io::Error)` (read I/O 失敗 / 既存 `.md` ファイルの parse 失敗) の 2 系統があり、後者を `PersistError` に丸めると「write 失敗」と区別不能
  - **3 slice で pattern 確立**: `auto-save-note` / `assign-tag` / `remove-tag` の 3 slice が全て `LoadError` を持つ 3 variant (以上) 構成を採用済。oq-remove-tag-error-3variant の「3 slice で確立」条件が満たされたため、本 proposal で upstream 化する

## 現状仕様 {#current}

> domain/workflows/remove-tag.md#errors より：

```
## Errors {#errors}

- `NoteNotFound { id: NoteId }`
- `PersistError { path: PathBuf, cause: io::Error }`
```

> .ori/slices/remove-tag/spec.md#io-errors (derived) より：

```
- `RemoveTagError::NoteNotFound { id: NoteId }`
- `RemoveTagError::LoadError { path: PathBuf, source: io::Error }`
- `RemoveTagError::PersistError { path: PathBuf, source: io::Error }`
```

derived 側は既に 3 variant で実装済 (I-RT8)。upstream workflow が 2 variant のため、`/ori-sync` で hash drift が指摘され続ける状態。

## 矛盾／欠落 {#gap}

派生側 (`slices/remove-tag/spec.md`, impl) が必要とする条件:

- RemoveTagError の variant 集合に少なくとも以下が必要：
  1. `NoteNotFound { id }` — 既存
  2. `LoadError { path, source: io::Error }` — **新規必要**：read 失敗を意味的に分離 (auto-save-note / assign-tag と同型)
  3. `PersistError { path, source: io::Error }` — 既存（write 失敗専用に意味を絞る）

現状ドメインで満たせない理由:

- workflow#errors が 2 variant 固定なので、impl 側が 3 variant にすると spec.md (derived) と upstream の hash drift が `/ori-sync` で継続指摘される。CoDD の SSoT (frontmatter `coherence.source: derived`) と整合しない

## 提案する変更 {#proposal}

### domain/workflows/remove-tag.md#errors を以下に置換

```
## Errors {#errors}

- `NoteNotFound { id: NoteId }` — load_by_id が `Ok(None)` を返した場合
- `LoadError { path: PathBuf, source: io::Error }` — load_by_id の read I/O 失敗 / 既存 `.md` ファイルの parse 失敗
- `PersistError { path: PathBuf, source: io::Error }` — `NoteRepository::write` の I/O 失敗 (write 経路専用)
```

### domain/workflows/remove-tag.md#notes に補足を追記

```
- read 失敗 (`LoadError`) と write 失敗 (`PersistError`) は意味的に異なる経路として error variant を分離する (auto-save-note / assign-tag workflow と同形)
- 本 errors 形は Note Capture BC の write-side 3 slice (auto-save-note / assign-tag / remove-tag) で共有される「read/write I/O 意味分離」契約を反映している
```

### 影響範囲

- **直接影響**: `slices/remove-tag/spec.md` の `/ori-derive remove-tag` 再生成で frontmatter hash と本文 (io-errors, I-RT8) が upstream に揃う → `#oq-remove-tag-error-3variant` が解消
- **波及候補**: なし。auto-save-note (4 variant) / flush-note (4 variant) / assign-tag (3 variant 相当) / remove-tag (3 variant) で Note Capture BC の write-side error 規約は閉じる
- **status.yaml**: `dirty[]` に `domain/workflows/remove-tag.md` が積まれる → accept 後の `/ori-sync` で spec.md hash を更新

## 代替案 {#alternatives}

1. **spec/impl 分離維持**: workflow.md は 2 variant のまま、slice spec.md に open question として「impl は防衛で 3 variant」を明記し続ける運用。`oq-remove-tag-error-3variant` で部分的にやっている。コスト最小だが CoDD の「spec is source of truth」に反する
2. **`assign-tag` workflow も同時改訂**: assign-tag slice も LoadError を持つため、同時に proposal 化する。本 proposal は remove-tag 単独に留め、assign-tag は別 proposal で扱う (slice の finalize タイミングが異なるため)

## 推奨 {#recommended}

採択: **提案する変更**。3 slice で pattern 確立した時点での upstream 化であり、oq-remove-tag-error-3variant に記録した条件が満たされたため。accept 後は次の `/ori-sync` で spec.md の last_derived / hash 更新も同時に行う。
