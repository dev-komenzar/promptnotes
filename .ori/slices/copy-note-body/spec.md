---
coherence:
  source: derived
  last_derived: 2026-06-30
  hash:
    domain/workflows/copy-note-body.md#.*: 5ec956893834
    domain/aggregates.md#.*: 82947dbfd3f6
    domain/bounded-contexts.md#.*: 7ebfcda8743b
ori:
  schema:
    propagation_level: file
---

# copy-note-body spec {#copy-note-body-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive copy-note-body`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

ホバー時のコピーボタン押下で、Note の **本文のみ**（YAML frontmatter / タグ情報を除外）を OS クリップボードへ書き出す slice。Note Capture BC の core 経路であり、spec の差別化点 3 件のうち「frontmatter/タグ除外コピー」を実装する。

> domain/bounded-contexts.md#note-capture より：
> > discovery の Core Domain 3 点（プロンプト・ブロックのストック / 座標ズレのない Markdown 編集 / **本文のみのクリップボード出力**）はすべてこの BC に集約される。

> domain/workflows/copy-note-body.md より：
> > spec の core: 「本文のみ」をコピーすることがプロダクトの差別化点

Note aggregate の state を変えないため domain event を発行しない（read-only path）。UI 層は成功時に「コピーしました」トーストを表示してよいが、それは UI レベルの副作用であり本 slice の責務外。

## 入出力 {#io}

### Input {#io-input}

```rust
struct CopyNoteBodyCommand {
  note_id: NoteId,
}
```

> domain/workflows/copy-note-body.md#input より引用。`NoteId` は `^\d{14}$` の文字列（domain/aggregates.md#note-aggregate-elements）。

### Output {#io-output}

- 成功: `Result<(), CopyNoteBodyError>` の Ok variant。副作用として OS クリップボードに Note 本文文字列が書き込まれる
- domain event: **発行しない**（domain state が変わらないため。domain/workflows/copy-note-body.md#notes に明文化）

### Errors {#io-errors}

- `CopyNoteBodyError::NoteNotFound { id: NoteId }` — 指定 `note_id` の Note が `NoteRepository` 上に存在しない、または `NoteRepository::load_by_id` が `io::Err` を返した（後者は I-CNB5 により本 variant へ collapse する）
- `CopyNoteBodyError::ClipboardError { cause: ClipboardErrorKind }` — OS クリップボード API 呼び出しが失敗
  - `ClipboardErrorKind` の variant 集合（最小）: `Unavailable` / `Io(String)`。phase 7 で採用する Tauri clipboard adapter の error 集合に応じて拡張可。拡張時は本 spec と test を同時更新する

domain/workflows/copy-note-body.md#errors は `NoteNotFound` / `ClipboardError` の 2 variant を列挙しており、本 slice はそれを維持。`load_by_id` の I/O 失敗を独立 variant にしないのは I-CNB5 の意図的選択（slice 固有 invariant 参照）。

## 不変条件 {#invariants}

### Note Aggregate 由来 {#invariants-note-aggregate}

- **I-N8（domain/aggregates.md#note-aggregate-invariants）**: `NoteBody` は frontmatter delimiter 行（`---`、末尾空白許容）を含まない。`Note::body_for_clipboard()` の戻り値も同条件を満たす（aggregate construction で既に enforce 済み）
- **read-only**: 本 slice は `Note::body_for_clipboard()` のみを呼び、aggregate の mutating commands（`edit_body` / `assign_tag` / `remove_tag` / `delete_to_trash` 等）を呼ばない。よって `updatedAt` を含む aggregate state は変化しない（I-N3 / I-N4 を逸脱する経路を作らない）

### slice 固有制約 {#invariants-slice-specific}

- **I-CNB1（差別化 invariant）**: OS クリップボードへ書き込む文字列は `Note::body_for_clipboard()` の戻り値**のみ**であり、YAML frontmatter 文字列・タグ表現・`---` delimiter 行・`createdAt` / `updatedAt` の文字列表現を含まない。enforcement は **test-time + 構造**で行う：(a) test `tp-exclude-frontmatter` / `tp-uses-body-for-clipboard` で clipboard 内容を assert、(b) slice 内 application 層は `note.body_for_clipboard()` のみを呼び `note.body()` 直接アクセスは行わない（コードレビューおよび本 invariant の test pin で担保）。compile-time 強制（aggregate の `body()` getter 非公開化等）は他 slice への副作用が大きいため採用しない
- **I-CNB2（空 body 許容）**: NoteBody が空文字（domain/aggregates.md#note-aggregate-elements: 「任意の UTF-8 文字列（空文字も許容）」）の場合も、空文字を clipboard に書き込み Ok を返す（エラー扱いしない）
- **I-CNB3（副作用順序）**: `NoteNotFound` の場合、`ClipboardService::write_text` は呼び出されない（先に load が失敗するため。clipboard 状態を変えない契約）。I-CNB5 による `io::Err` 経路も同様に clipboard 未呼出
- **I-CNB4（domain event 非発行）**: 本 slice は domain event bus に何も publish しない（domain/workflows/copy-note-body.md#notes 明示）。application 層の `CopyNoteBodyUseCase::new` が `EventBus` 引数を取らない構造により compile-time に保証
- **I-CNB5（LoadError collapse — 意図的な情報損失）**: `NoteRepository::load_by_id` の `io::Err`（disk read failure 等）は `NoteNotFound { id }` に collapse する。user-observable には「note 不在」と区別できない（どちらも clipboard 書き込みに到達しない結果は同一）ため、error 表面を `NoteNotFound` / `ClipboardError` の 2 variant に保ち単純化する。trade-off: 永続化層の debug ログでは collapse 前の `io::ErrorKind` が見えなくなる。production で診断要求が出たら spec を見直し variant を 3 つに拡張する（domain workflow / slice spec / test を同時更新）

### 経路境界 {#invariants-boundary}

- **UI 副作用は責務外**: 成功時の toast 表示・ボタンの一時状態変化等は UI 層の責務であり、本 slice の output 契約は `Result<(), CopyNoteBodyError>` のみ
- **Tauri 境界**: Rust 側 command として expose し、tauri-specta で TS bindings を自動生成する（`.ori/architecture.md` cross_root 参照）

## テスト観点 {#test-perspectives}

### happy path: 通常 body のコピー {#tp-happy}

`Note::create("hello world", tags=[], now)` で生成した Note を `note_id` で指定 → clipboard に `"hello world"` 文字列が書き込まれ、Ok を返す。

### frontmatter / tag 除外（差別化 invariant） {#tp-exclude-frontmatter}

タグ付き Note（例：`tags = [Tag("rust"), Tag("memo")]`、body = `"line1\nline2"`）に対して実行 → clipboard 内容は `"line1\nline2"` のみで、`---` / `tags:` / `rust` / `memo` / `createdAt` 等の文字列を**含まない**。I-CNB1 の検証。

### empty body {#tp-empty-body}

NoteBody が空文字の Note → 空文字 `""` が clipboard に書かれ、Ok を返す（I-CNB2）。

### NoteNotFound {#tp-not-found}

存在しない `note_id` を渡す → `CopyNoteBodyError::NoteNotFound { id }` が返り、`ClipboardService::write_text` が呼ばれていない（I-CNB3）。

### repository io::Err collapse {#tp-repo-io-err-collapse}

I-CNB5 の意図的選択を pin する観点。`NoteRepository::load_by_id` が `io::Err`（例：`io::ErrorKind::PermissionDenied`）を返す → `CopyNoteBodyError::NoteNotFound { id }` に collapse し、`ClipboardService::write_text` は呼ばれない（I-CNB3 と整合）。impl 側で error variant を増やすと spec / test 同時更新が必要になる契約を test で固定する。

### ClipboardError {#tp-clipboard-err}

`NoteRepository::load_by_id` は成功するが `ClipboardService::write_text` が `ClipboardErrorKind::*` を返す → `CopyNoteBodyError::ClipboardError { cause }` で伝播し、cause の variant が保存される。

### no state mutation {#tp-no-mutation}

実行前後で Note aggregate の `updatedAt` / `body` / `tags` が変化していない（`NoteRepository` は read のみ呼ばれ、write 系 method は呼ばれない）。

### no domain event {#tp-no-event}

実行後、domain event bus（テスト用 spy）に何も publish されていない（I-CNB4）。

### body_for_clipboard 経路の強制 {#tp-uses-body-for-clipboard}

clipboard に書かれる文字列が `note.body_for_clipboard()` の戻り値と byte-for-byte 一致する（直接 `note.body` を露出させるショートカットを slice 内で作らないことの test 担保）。I-CNB1 の enforcement strategy は test-time + 構造で確定済み（spec.md#invariants-slice-specific 参照）。test ファイル内で `tp_bc1` は seed body との byte 一致、`tp_bc2` は `note.body_for_clipboard()` 戻り値との byte 一致を assert することで、将来 `body_for_clipboard()` に normalization が入った時の regression を検出できる構造にする。

## 実装ノート {#impl-notes}

### 依存 interface（port） {#impl-ports}

- `NoteRepository::load_by_id(&NoteId) -> Result<Note, NoteRepositoryError>` — read only。auto-save-note slice で既に Rust 側に存在するものを再利用
- `ClipboardService::write_text(&str) -> Result<(), ClipboardErrorKind>` — **新規 port**。`ClipboardErrorKind` は最小集合 `Unavailable` / `Io(String)` の 2 variant。Tauri adapter の選定（標準 `ClipboardManager` か `tauri-plugin-clipboard-manager` か）は phase 7 finalize で `commands.rs`（Tauri 境界）と併せて実装する。slice 単体テストは fake clipboard で完結する設計

### slice layout（DDD-VSA-Hex） {#impl-layout}

- Rust（primary、Tauri command 層）: `apps/promptnotes/src-tauri/src/note_capture/slices/copy_note_body/`
  - `commands.rs` — Tauri command として `#[tauri::command]` で expose
  - `handler.rs` — application service（CopyNoteBodyCommand → Result）
  - `ports.rs` — `ClipboardService` trait 定義
- TS（UI 連携）: `apps/promptnotes/src/lib/note-capture/slices/copy-note-body/`
  - tauri-specta 生成 bindings 経由でホバーボタン onClick から呼ぶ

### 既存 slice との関係 {#impl-related-slices}

- `auto-save-note` slice の `NoteRepository::load_by_id` を read-only で再利用
- aggregate の `Note::body_for_clipboard()` query API が未実装なら phase 4 で追加（domain/aggregates.md#note-aggregate-queries に既に定義済み）

### 非責務 {#impl-non-responsibility}

- 成功 toast 表示・ホバーボタン UI 状態管理は UI 層で実装し、本 slice は呼び出されない
- clipboard 内容の format 変換（HTML / RTF 等）は本 slice の対象外（plain text のみ）
