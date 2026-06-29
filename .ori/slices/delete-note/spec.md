---
coherence:
  source: derived
  last_derived: 2026-06-26
  upstream:
    - domain/workflows/delete-note.md#delete-note
    - domain/aggregates.md#note-aggregate
    - domain/bounded-contexts.md#note-capture
    - domain/domain-events.md#note-deleted-to-trash
    - domain/validation.md#s5-delete-undo-in-window
    - domain/validation.md#s6-delete-replace
  hash:
    domain/workflows/delete-note.md#.*: b727f18ad614
    domain/aggregates.md#.*: 9f9048f5816b
    domain/bounded-contexts.md#.*: 4d579125a513
    domain/domain-events.md#.*: 8abdfac78084
    domain/validation.md#.*: 5294b0c32f1b
    domain/validation.md#.*: 5294b0c32f1b
ori:
  schema:
    propagation_level: file
---

# delete-note spec {#delete-note-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive delete-note`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

ホバー時の削除ボタン押下で、Note を OS のゴミ箱へ移動し、Undo 用の `DeletedNote` を application service の Undo スタックへ push する slice。Note Capture BC の write path で、`Note::delete_to_trash()` と `DeletedNote` ハンドルを介して S5（削除 → トースト中に Undo）と S6（連続削除でスタック）の前段を担う。

> domain/bounded-contexts.md#note-capture より：
> > ユーザの起案・編集・タグ付け・コピー・削除/復元といった **Note の write side** を司る。

> domain/workflows/delete-note.md より：
> > Note を OS のゴミ箱へ移動し、Undo 用 `DeletedNote` を application service に保持する。

Undo スタックは Note Aggregate の責務外であり、Note Capture BC の application service 層が `Vec<DeletedNote>` として保持する（domain/aggregates.md#notes-undo）。本 slice は (a) Note の load、(b) trash 移動、(c) Undo スタックへの push、(d) `NoteDeletedToTrash` event 発行を一連の同期パイプラインとして繋ぐ。Toast の TTL タイマー起動と消失時の stack 除去は UI 層 + application service の責務であり、本 slice の単体テスト境界には含めない（境界線は `UndoStack::push` まで）。

## 入出力 {#io}

### Input {#io-input}

```rust
struct DeleteNoteCommand {
  note_id: NoteId,
}
```

> domain/workflows/delete-note.md#input より引用。`NoteId` は `^\d{14}$`（domain/aggregates.md#note-aggregate-elements）。

### Output {#io-output}

- 成功: `Result<DeletedNote, DeleteNoteError>` の Ok variant
  - `DeletedNote { id: NoteId, original_path: PathBuf }`（domain/workflows/delete-note.md#output）
  - 同 Ok variant の返却前に副作用として下記が確定する：
    - OS ゴミ箱への移動（`TrashService::move_to_trash`）成功
    - application service の Undo スタックに `DeletedNote` を **push**
    - domain event `NoteDeletedToTrash { note_id, original_path, deleted_at }` を `EventBus` に発行
- 失敗: `Result<..>::Err(DeleteNoteError)`（副作用は I-DN3 / I-DN4 に従い未実施）

### Errors {#io-errors}

- `DeleteNoteError::NoteNotFound { id: NoteId }` — 指定 `note_id` の Note が `NoteRepository` 上に存在しない、または `NoteRepository::load_by_id` が `io::Err` を返した（後者は I-DN6 により本 variant へ collapse）
- `DeleteNoteError::TrashError { path: PathBuf, cause: TrashErrorKind }` — `TrashService::move_to_trash` が失敗
  - `TrashErrorKind` の variant 集合（最小）: `PermissionDenied` / `Io(String)` / `Unsupported`。phase 7 finalize で採用する OS 依存 adapter（macOS: `NSWorkspace` / Linux: XDG trash / Windows: `SHFileOperation`）の error 集合に応じて拡張可。拡張時は本 spec と test を同時更新する

domain/workflows/delete-note.md#errors は `NoteNotFound` / `TrashError` の 2 variant を列挙しており、本 slice はそれを維持。`load_by_id` の I/O 失敗を独立 variant にしないのは copy-note-body slice の I-CNB5 と同型の意図的選択（slice 固有 invariant 参照）。

## 不変条件 {#invariants}

### Note Aggregate 由来 {#invariants-note-aggregate}

- **I-N7（domain/aggregates.md#note-aggregate-invariants）**: 削除された Note の identity は application service の `Vec<DeletedNote>` Undo スタックに push され、対応する Toast の有効期間中のみ復元可能。本 slice は **push** までを責務とし、有効期間管理（TTL / 消失時の pop）は別 slice（restore-deleted-note）+ UI 層が担う
- **I-N1 / I-N2 / I-N3 / I-N4 の non-violation**: 本 slice は Note を `delete_to_trash` で consume するだけで `body` / `tags` / `updatedAt` を更新する経路は持たない。よって updatedAt monotonic（I-N3 / I-N4）に違反しない

### slice 固有制約 {#invariants-slice-specific}

- **I-DN1（path resolution）**: `original_path` は `storage_dir / <note_id>.md` で決定論的に導出される（domain/workflows/delete-note.md#steps step 2）。slice 内で別経路（symlink 解決・絶対パス再構築 等）を通さない。`storage_dir` は `NoteRepository::storage_dir()` 経由で取得する（review Pass 1 M-2 解消: `SettingsReader` port は廃止し、auto-save-note / create-note / copy-note-body slice と一貫して `NoteRepository::storage_dir()` を再利用。`NoteRepository` への `storage_dir` 注入は Tauri command 境界で Settings Aggregate から hydrate される）
- **I-DN2（trash 経由のみ）**: ファイル削除は必ず `TrashService::move_to_trash` 経由で行い、`std::fs::remove_file` 等の unlink API を slice 内で呼ばない（domain/bounded-contexts.md#note-capture-ubiquitous-language の DeleteToTrash 定義）。これは「OS のゴミ箱から復帰」可能性を保証するための構造的不変条件
- **I-DN3（副作用順序: load 失敗時）**: `NoteNotFound`（および I-DN6 で collapse された load io::Err）の場合、`TrashService::move_to_trash` / `UndoStack::push` / `EventBus::publish` は呼ばれない（read 段階で短絡）
- **I-DN4（副作用順序: trash 失敗時）**: `TrashError` の場合、`UndoStack::push` と `EventBus::publish` は呼ばれない（trash 移動が成功しないと Undo 不可能 + state 不整合のため）
- **I-DN5（event 発行は trash + push 成功後）**: `NoteDeletedToTrash` event は (a) `TrashService::move_to_trash` 成功 (b) `UndoStack::push` 完了 の両方が確定してから publish する。順序逆転すると subscriber（Note Feed の表示除外）が Undo 不能状態を観測する可能性があり、I-N7 の「Undo 可能性」を破る
- **I-DN6（LoadError collapse — 意図的な情報損失）**: `NoteRepository::load_by_id` の `io::Err`（disk read failure 等）は `NoteNotFound { id }` に collapse する。user-observable には「note 不在」と区別できない（どちらも trash 移動に到達しない結果は同一）ため、error 表面を `NoteNotFound` / `TrashError` の 2 variant に保ち単純化する。trade-off: 永続化層の debug ログでは collapse 前の `io::ErrorKind` が見えなくなる。production で診断要求が出たら spec を見直し variant を 3 つに拡張する（domain workflow / slice spec / test を同時更新）。本 slice は copy-note-body slice の I-CNB5 と同じ方針を採用
- **I-DN7（DeletedNote 内容）**: Undo スタックに push する `DeletedNote` の `id` は load した Note の `id` と一致し、`original_path` は I-DN1 で導出した path と byte 一致する。これにより restore-deleted-note slice が同 path に書き戻す前提（domain/workflows/delete-note.md#notes「ファイル名は id から決定論的に導出可能」）を pin する
- **I-DN8（独立 TTL — 既存スタックは破壊しない）**: 本 slice は Undo スタックに新 `DeletedNote` を push するだけで、既存要素（他の delete から残っている DeletedNote）は破棄しない（domain/validation.md#s6-delete-replace「各 DeletedNote は独立に保持される」）。`UndoStack::push` の契約として保証する

### 経路境界 {#invariants-boundary}

- **UI 副作用は責務外**: Toast 表示・縦パイル管理・per-Toast TTL タイマー開始は UI 層 + application service の責務であり、本 slice の output 契約は `Result<DeletedNote, DeleteNoteError>` と 3 副作用（trash / push / event）のみ
- **restore は別 slice**: `DeletedNote::restore()` / restore-deleted-note workflow は別 slice の責務。本 slice は restore 経路に直接踏み込まない
- **Tauri 境界**: Rust 側 command として expose し、tauri-specta で TS bindings を自動生成する（`.ori/architecture.md` cross_root 参照）

## テスト観点 {#test-perspectives}

### happy path: 通常 note の削除 {#tp-happy}

`Note::create("body", tags=[], now)` で生成した Note を `note_id` で指定 → (a) `TrashService::move_to_trash(storage_dir/<id>.md)` が 1 回呼ばれ、(b) `UndoStack::push(DeletedNote { id, original_path })` が呼ばれ、(c) `EventBus::publish(NoteDeletedToTrash { note_id, original_path, deleted_at })` が 1 回 emit され、(d) Ok(DeletedNote) を返す。

### tags 付き note の削除 {#tp-with-tags}

`tags = [Tag("rust"), Tag("memo")]` の Note を削除 → tags の有無は結果に影響せず、happy path と同じ副作用順序が観測される。I-DN1 で `original_path` が `<id>.md` のみで決まることを確認。

### NoteNotFound {#tp-not-found}

存在しない `note_id` を渡す → `DeleteNoteError::NoteNotFound { id }` が返り、`TrashService::move_to_trash` / `UndoStack::push` / `EventBus::publish` のいずれも呼ばれない（I-DN3）。

### repository io::Err collapse {#tp-repo-io-err-collapse}

I-DN6 の意図的選択を pin する観点。`NoteRepository::load_by_id` が `io::Err`（例：`io::ErrorKind::PermissionDenied`）を返す → `DeleteNoteError::NoteNotFound { id }` に collapse し、`TrashService::move_to_trash` / `UndoStack::push` / `EventBus::publish` は呼ばれない（I-DN3 と整合）。impl 側で error variant を増やすと spec / test 同時更新が必要になる契約を test で固定する。

### TrashError 伝播 {#tp-trash-err}

`NoteRepository::load_by_id` は成功するが `TrashService::move_to_trash` が `TrashErrorKind::*` を返す → `DeleteNoteError::TrashError { path, cause }` が返り、`UndoStack::push` / `EventBus::publish` は呼ばれない（I-DN4）。path は I-DN1 通り `storage_dir / <id>.md` と一致。

### 副作用順序: trash → push → event {#tp-side-effect-order}

happy path で 3 副作用の **呼び出し順序** を spy で観測 → `TrashService::move_to_trash` が最初、次に `UndoStack::push`、最後に `EventBus::publish` の順で並ぶ（I-DN5）。途中で順序逆転していないことを assert。

### event payload の整合性 {#tp-event-payload}

happy path の `NoteDeletedToTrash` event の payload について：(a) `note_id` が input と一致、(b) `original_path` が I-DN1 通り `storage_dir / <id>.md`、(c) `deleted_at` が `Clock` から取得した時刻と一致する（domain/domain-events.md#note-deleted-to-trash-payload）。

### Undo スタックの累積（既存要素を破壊しない） {#tp-stack-accumulate}

事前に Undo スタックへ別 `DeletedNote(A)` が積まれている状態で、本 slice で Note B を削除 → スタックは `[DeletedNote(A), DeletedNote(B)]` となり、A は破棄されない（I-DN8、domain/validation.md#s6-delete-replace）。

### DeletedNote の id / path 一貫性 {#tp-deleted-note-shape}

happy path で push される `DeletedNote.id` が input の `note_id` と一致、`DeletedNote.original_path` が I-DN1 の path と byte 一致（I-DN7）。

### trash 経路のみ（unlink 不使用） {#tp-trash-only}

本 slice の impl コードに `std::fs::remove_file` / `std::fs::remove_dir_all` 等の unlink API への直接依存が出現しないことを構造的に確認（I-DN2）。確認は 2 段構成:

- **TP-TO1 (behavioral)**: fake `TrashService` がただ一度呼ばれ、`NoteRepository` 上の write 系 method（仮にあれば）が呼ばれないことを assert
- **TP-TO2 (structural, review L-1)**: `include_str!` で slice の production source file（`application.rs` / `commands.rs` / `domain.rs` / `ports.rs` / `mod.rs`）を compile-time に読み込み、`fs::remove_` パターンが出現しないことを assert。将来 contributor が unlink API を直接呼ぶ regression を機械的に検出する

## 実装ノート {#impl-notes}

### 依存 interface（port） {#impl-ports}

- `NoteRepository::load_by_id(&NoteId) -> Result<Note, NoteRepositoryError>` — read only。auto-save-note / copy-note-body slice で既に Rust 側に存在するものを再利用
- `NoteRepository::storage_dir() -> &Path` — `storage_dir` 解決用。`NoteRepository` port の既存 method を再利用（review Pass 1 M-2 解消: `SettingsReader` port は廃止。auto-save-note / create-note / copy-note-body slice と一貫）。`NoteRepository` 実装への `storage_dir` 注入は Tauri command 境界 (`commands.rs`) で Settings Aggregate から hydrate する
- `TrashService::move_to_trash(&Path) -> Result<(), TrashErrorKind>` — **新規 port**。`TrashErrorKind` は最小集合 `PermissionDenied` / `Io(String)` / `Unsupported` の 3 variant。OS 依存実装（macOS: `NSWorkspace.recycleURLs` / Linux: XDG trash / Windows: `SHFileOperation`）は phase 7 finalize で `commands.rs`（Tauri 境界）と併せて実装する。slice 単体テストは fake trash で完結する設計。crate 候補：`trash`（cross-platform）または Tauri plugin
- `UndoStack` — application service 層が保持する `Vec<DeletedNote>`。slice からは `UndoStack::push(DeletedNote)` のみを呼ぶ trait として抽出。restore-deleted-note slice と共有する想定（実装は phase 4 で最小化、本 slice では push のみ）
- `EventBus::publish(NoteDeletedToTrash) -> ()` — 既存 event bus port を再利用（auto-save-note slice の publish 経路と同型）
- `Clock::now() -> Timestamp` — `deleted_at` 取得用。auto-save-note slice の Clock を再利用

### slice layout（DDD-VSA-Hex） {#impl-layout}

- Rust（primary、Tauri command 層）: `apps/promptnotes/src-tauri/src/note_capture/slices/delete_note/`
  - `commands.rs` — Tauri command として `#[tauri::command]` で expose
  - `handler.rs` — application service（`DeleteNoteCommand → Result<DeletedNote, DeleteNoteError>`）
  - `ports.rs` — `TrashService` / `UndoStack` trait 定義（review Pass 1 M-2 解消: `SettingsReader` は廃止）
- TS（UI 連携）: `apps/promptnotes/src/lib/note-capture/slices/delete-note/`
  - tauri-specta 生成 bindings 経由でホバーボタン onClick から呼ぶ
  - Toast 表示・per-Toast TTL タイマー開始は UI 層の責務（本 slice の output を受けて起動）

### 既存 slice との関係 {#impl-related-slices}

- `auto-save-note` slice の `NoteRepository::load_by_id` / `EventBus` / `Clock` port を読み取りで再利用
- `copy-note-body` slice の I-CNB5（io::Err collapse）と同型の方針を I-DN6 で採用
- `restore-deleted-note`（未実装 slice）と `UndoStack` を共有する。本 slice では push のみ実装し、pop / restore は restore-deleted-note 側で実装する
- `Note::delete_to_trash()` aggregate command が未実装なら phase 4 で追加（domain/aggregates.md#note-aggregate-commands に既に定義済み）

### 非責務 {#impl-non-responsibility}

- Toast UI 描画・per-Toast TTL タイマー管理・縦パイル順序制御は UI 層で実装し、本 slice では呼ばれない
- `restore-deleted-note` workflow / `DeletedNote::restore()` の実装は本 slice の対象外
- OS ゴミ箱からの自動削除や TTL ベースの permanent delete は本アプリの責務外（domain/validation.md#s7 の補足）
