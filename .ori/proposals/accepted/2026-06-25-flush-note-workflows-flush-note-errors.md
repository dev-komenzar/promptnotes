---
target: domain/workflows/flush-note.md#errors
by: slices/flush-note
reason: phase 1 derive で spec が auto-save-note と同形の 4 variant (NoteNotFound / InvalidBody / LoadError / PersistError) を先取り採用したが、upstream workflow は 2 variant のまま。auto-save-note proposal accepted (2026-06-25) で「flush-note workflow にも同形を将来適用」と followup 記録済みであり、本 slice 完了に伴い propagate する
created: 2026-06-25
status: accepted
accepted_at: 2026-06-28
accepted_by: human (takuya.kometan@gmail.com)
applied_to: domain/workflows/flush-note.md#errors (4 variant に拡張) + #notes (NoteBody / load/persist 分離の補足追加)
---

# Proposal: flush-note workflow の Errors を 4 variant に拡張する

## 発見の経緯 {#context}

- 検出元：`slices/flush-note` の phase 1 (derive) と phase 6 (review Pass 1)
- 試みていたこと：spec.md の構築時、`auto-save-note` slice と同じ `Note::edit_body` + `NoteBodyEdited` 契約 (C-FL10) を踏襲しつつ、Errors を upstream workflow (2 variant) に揃えるべきか先取りで揃えるべきかを判断
- 想定との差:
  - **InvalidBody 経路の必要性**: `pending_body: String` を `NoteBody::new` で smart construct する以上、`---` 行を含むケースを error variant に立てないと握り潰しになる（auto-save-note と同じ I-N8 構造）
  - **LoadError 経路の必要性**: `NoteRepository::load_by_id` の read I/O 失敗を `PersistError` に丸めると、auto-save-note review HIGH-2 と同じ問題（read 失敗と write 失敗が UI 側で区別不能）が再発する

## 現状仕様 {#current}

> domain/workflows/flush-note.md#errors より：

```
## Errors {#errors}

- `NoteNotFound { id: NoteId }`
- `PersistError { path: PathBuf, cause: io::Error }`
```

> .ori/proposals/accepted/2026-06-25-auto-save-note-workflows-auto-save-note-errors.md frontmatter より：

```
followup: flush-note workflow にも同形を将来適用 (本 proposal で同時改訂はしない)
```

## 矛盾／欠落 {#gap}

派生側 (`slices/flush-note/spec.md`, impl) が必要とする条件:

- FlushError の variant 集合に少なくとも以下が必要 (auto-save-note と同形)：
  1. `NoteNotFound { id }` — 既存
  2. `InvalidBody { source: NoteBodyError }` — **新規必要**：`NoteBody::new(pending_body)` 失敗を呼び出し元に伝える
  3. `LoadError { path, source: io::Error }` — **新規必要**：read 失敗を意味的に分離
  4. `PersistError { path, source: io::Error }` — 既存（write 失敗専用に意味を絞る）

現状ドメインで満たせない理由:

- workflow#errors が 2 variant のままだと、auto-save-note と FlushError の variant 数が乖離する。両者は同じ `Note::edit_body` + `NoteBody::new` pipeline を踏むため、error 表面が異なると Tauri command boundary の DTO mapping ロジックが slice 間で非対称になる
- spec.md は phase 1 で 4 variant を先取り採用済 (`oq-error-variant-alignment` で根拠記録)。本 proposal accept まで spec と upstream の hash drift が `/ori-sync` で flag され続ける

## 提案する変更 {#proposal}

### domain/workflows/flush-note.md#errors を以下に置換

```
## Errors {#errors}

- `NoteNotFound { id: NoteId }` — load_by_id が `Ok(None)` を返した場合
- `InvalidBody { source: NoteBodyError }` — `NoteBody::new(pending_body)` が失敗した場合（frontmatter delimiter line `---` を含む等、I-N8 違反）
- `LoadError { path: PathBuf, source: io::Error }` — load_by_id の read I/O 失敗 / 既存 `.md` ファイルの parse 失敗
- `PersistError { path: PathBuf, source: io::Error }` — `NoteRepository::write` の I/O 失敗 (write 経路専用)
```

### domain/workflows/flush-note.md#notes に補足を追記

```
- `NoteBody` 不変条件（frontmatter delimiter `---` を含まない、I-N8）は aggregate 由来。Flush は pending_body を受け取り構築するため、aggregate と同じ smart constructor を通る
- read 失敗 (`LoadError`) と write 失敗 (`PersistError`) は意味的に異なる経路として error variant を分離する（auto-save-note workflow と同形）
- 本 errors 形は `auto-save-note` workflow と shared な「Note::edit_body 経由の永続化契約」を反映している。両 workflow を改訂する際は同時に同形を保つこと
```

### 影響範囲

- **直接影響**: `slices/flush-note/spec.md` を `/ori-derive flush-note` で再生成すると frontmatter の hash と本文（io-errors, C-FL2 / C-FL3）が upstream に揃う → spec.md `#oq-error-variant-alignment` が解消
- **波及候補**: なし。auto-save-note と flush-note の両 workflow が同形に揃った時点で、Note Capture BC の write-side error 規約は閉じる
- **status.yaml**: `dirty[]` に `domain/workflows/flush-note.md` が積まれる → accept 後の `/ori-sync` で spec.md hash を更新

## 代替案 {#alternatives}

1. **spec/impl 分離維持**: workflow.md は 2 variant のまま、slice spec.md に open question として「impl は防衛で 4 variant」を明記し続ける運用。`oq-error-variant-alignment` で部分的にやっている。コスト最小だが CoDD の「spec is source of truth」(memory: feedback_spec_is_source_of_truth) に反する
2. **auto-save-note workflow も 2 variant に戻す**: 両者を upstream 規定に合わせ直す方向の整合化。production code は実装済 4 variant を 2 variant に縮退させる必要があり、回帰リスクが高い。「先取り採用」の意義を否定するため非推奨

## 推奨 {#recommended}

採択: **提案する変更**。auto-save-note proposal の followup として記録済みであり、本 slice 完了 (phase 7 finalize) のタイミングで propagate するのが最も自然。accept 後は次の `/ori-sync` で spec.md の last_derived / hash 更新も同時に行う。
