# auto-save-note — Implementation notes

## 2026-06-25 proposal 作成

phase 6 Pass 2 review HIGH-3 / HIGH-4 (spec 直編集 + 内部矛盾) を受けて upstream-first で
2 件の proposal を作成。accept 後に `/ori-derive auto-save-note` で spec.md を再生成する。

### Proposal 1: workflow Errors を 4 variant に拡張

- target: `domain/workflows/auto-save-note.md#errors`
- file: `.ori/proposals/2026-06-25-auto-save-note-workflows-auto-save-note-errors.md`
- reason: `AutoSaveError` に `InvalidBody` (NoteBody parse 失敗) と `LoadError` (read I/O 失敗) の 2 variant を追加し、PersistError を write 専用に意味を絞る

### Proposal 2: NoteBody smart constructor と Note 再構築 API の明示化

- target: `domain/aggregates.md#note-aggregate`
- file: `.ori/proposals/2026-06-25-auto-save-note-aggregates-note-aggregate.md`
- reason: NoteBody の不変条件「`---` を含まない」を smart constructor 規約に格上げ + `Note::from_persisted` を公開操作として明文化（FsNoteRepository::load_by_id の前提）

## 直近の状態

- impl + tests は 74/74 GREEN 維持 (review HIGH-2 patch / Timestamp::parse_yyyymmddhhmmss 修正含む)
- spec.md は impl ベースに直編集された状態（upstream propose accept 待ち）
- review (`ori-review-auto-save-note`) は **NEEDS_FIX** で `in_progress` → proposal accept 後に再評価予定
