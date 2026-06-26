---
coherence:
  source: derived
  last_derived: 2026-06-26
  upstream:
    - domain/workflows/restore-deleted-note.md#restore-deleted-note
    - domain/aggregates.md#note-aggregate
    - domain/bounded-contexts.md#note-capture
    - domain/domain-events.md#note-restored-from-trash
    - domain/validation.md#s5-delete-undo-in-window
    - domain/validation.md#s7-undo-after-toast
  hash:
    domain/workflows/restore-deleted-note.md#.*: e32a07cd279b
    domain/aggregates.md#.*: 9f9048f5816b
    domain/bounded-contexts.md#.*: 4d579125a513
    domain/domain-events.md#.*: 8abdfac78084
    domain/validation.md#.*: 5294b0c32f1b
    domain/validation.md#.*: 5294b0c32f1b
ori:
  schema:
    propagation_level: file
---

# restore-deleted-note spec {#restore-deleted-note-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive restore-deleted-note`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

delete-note slice (PR #8 マージ済) と対称な write 側 slice。Toast 表示中の「元に戻す」ボタン押下から起動され、(a) Undo スタックから指定 `NoteId` の `DeletedNote` を取り出し、(b) OS ゴミ箱から原 path にファイルを復帰させ、(c) `.md` を読み直して Note aggregate を再構築し、(d) Undo スタックから当該要素のみ pop し、(e) `NoteRestoredFromTrash` event を発行する。S5 (削除 → Toast 中 Undo) を成功経路、S7 (Toast 消失後 Undo は no-op) を `NoUndoAvailable` 経路として pin する。

> domain/bounded-contexts.md#note-capture より：
> > ユーザの起案・編集・タグ付け・コピー・削除/復元といった **Note の write side** を司る。

> domain/workflows/restore-deleted-note.md より：
> > Toast 表示中の「元に戻す」ボタン押下で **特定の** `DeletedNote` を OS ゴミ箱から復帰する。Toast は複数並列表示されるため、どの DeletedNote を復元するかを `NoteId` で指定する。

domain/aggregates.md#notes-undo の per-toast 独立性 (1 つの Toast の Undo は他の Toast / DeletedNote に影響を与えない、S6 改訂の核) を本 slice の `UndoStack::remove_by_id` で構造的に担保する。

## 入出力 {#io}

### Input {#io-input}

```rust
struct RestoreDeletedNoteCommand {
  note_id: NoteId,
}
```

> domain/workflows/restore-deleted-note.md#input より引用。`NoteId` は `^\d{14}$` (domain/aggregates.md#note-aggregate-elements)。どの Toast (= DeletedNote) を復元するかを 1 件ずつ指定する。

### Output {#io-output}

- 成功: `Result<Note, RestoreDeletedNoteError>` の Ok variant
  - 復帰した `Note` (NoteRepository::load_by_id で reload した結果)
  - 同 Ok variant の返却前に副作用として下記が確定する：
    - OS ゴミ箱から原 path にファイル復帰 (`TrashService::restore_from_trash` 成功)
    - Undo スタックから当該 `DeletedNote` のみ pop (`UndoStack::remove_by_id`、I-RDN8: per-toast 独立性)
    - domain event `NoteRestoredFromTrash { note_id, restored_at }` を `EventBus` に発行
- domain event: `NoteRestoredFromTrash { note_id, restored_at }` (domain/domain-events.md#note-restored-from-trash-payload)

### Errors {#io-errors}

- `RestoreDeletedNoteError::NoUndoAvailable { id: NoteId }` — 指定 `NoteId` の `DeletedNote` が Undo スタックに存在しない (Toast 消失後 / 既に Undo 済み / そもそも未削除、S7)
- `RestoreDeletedNoteError::TrashRestoreError { path: PathBuf, cause: TrashErrorKind }` — `TrashService::restore_from_trash` が失敗
  - `TrashErrorKind` は delete-note slice で既に定義済みの enum を再利用 (PermissionDenied / Io(String) / Unsupported)
- `RestoreDeletedNoteError::ReadError { path: PathBuf, source: io::Error }` — `NoteRepository::load_by_id` が `io::Err` を返した、または復帰後の `.md` が見つからない (`Ok(None)`)
  - delete-note の I-DN6 (collapse to NoteNotFound) とは異なる方針: 本 slice では「ゴミ箱から復帰させた直後の reload」なので `Ok(None)` も「予期せぬ消失」として ReadError 扱いにする (操作の事後不整合を表面化)

domain/workflows/restore-deleted-note.md#errors と一致する 3 variant 構成。

## 不変条件 {#invariants}

### Note Aggregate 由来 {#invariants-note-aggregate}

- **I-N1 / I-N2 / I-N3 / I-N4 の non-violation**: 本 slice は `Note::from_persisted` 経由でファイルから再構築するだけで、aggregate の field を書き換える経路を持たない。`id` / `created_at` / `updated_at` は永続化された frontmatter から復元される (domain/workflows/restore-deleted-note.md#notes「復元後の Note は元と同じ `NoteId` / `body` / `tags` / `createdAt`。`updatedAt` は OS ゴミ箱の API 仕様に依存」)
- **I-N7 の継続**: 復元成功時に当該 `DeletedNote` だけが Undo スタックから除去される (`UndoStack::remove_by_id`)。他の Toast / DeletedNote は影響を受けない (S6 改訂の per-toast 独立性、本 slice 側で remove-by-id 経路により担保)

### slice 固有制約 {#invariants-slice-specific}

- **I-RDN1（NoUndoAvailable は read-only）**: `UndoStack::find_by_id` で空振りした場合、`TrashService::restore_from_trash` / `NoteRepository::load_by_id` / `UndoStack::remove_by_id` / `EventBus::publish` のいずれも呼ばれない。S7 シナリオ (Toast 消失後の Undo 試行) の安全側挙動を担保
- **I-RDN2（path resolution は DeletedNote 由来）**: 復元先 path は `DeletedNote::original_path()` を信頼する。`storage_dir / <id>.md` で再計算しない (I-RDN7 の冪等性確保のため、delete 時の path をそのまま採用する)
- **I-RDN3（副作用順序: trash 失敗時）**: `TrashRestoreError` の場合、`NoteRepository::load_by_id` / `UndoStack::remove_by_id` / `EventBus::publish` は呼ばれない。ゴミ箱に残ったままで「あたかも Undo 失敗」を表現
- **I-RDN4（副作用順序: reload 失敗時）**: `ReadError` の場合、`UndoStack::remove_by_id` / `EventBus::publish` は呼ばれない。Undo スタックには `DeletedNote` を残し、次の Undo ボタン押下で再試行可能にする (note ファイルが復帰しているか分からないため、user に再操作の余地を残す側に倒す pragmatic choice)
- **I-RDN5（副作用順序: 成功時）**: 成功経路は (a) `UndoStack::find_by_id` → (b) `TrashService::restore_from_trash` → (c) `NoteRepository::load_by_id` → (d) `UndoStack::remove_by_id` → (e) `EventBus::publish` の順。order log spy で pin する
- **I-RDN6（event 発行は副作用全成功後）**: `NoteRestoredFromTrash` event は (a)〜(d) の 4 副作用が全て成功してから publish する。順序逆転すると subscriber (Note Feed) が再登場前に観測する可能性があり、UI 表示と aggregate 実体が乖離する
- **I-RDN7（per-toast 独立性）**: `UndoStack::remove_by_id(note_id)` は指定 `NoteId` の DeletedNote 1 件のみを除去する。他の Toast に対応する DeletedNote (異なる NoteId) は破棄しない (domain/validation.md#s6-delete-replace「各 Toast / DeletedNote は独立した有効期間を持ち、互いに干渉しない」と整合)
- **I-RDN8（restored_at は Clock 由来）**: `NoteRestoredFromTrash.restored_at` は `Clock::now()` から取得する。OS ファイルシステムの metadata (atime / mtime) ではなく application 時計を使う (テスト容易性 + Clock injection の一貫性)
- **I-RDN9（DeletedNote 検索は等価性ベース）**: `UndoStack::find_by_id(note_id)` は `DeletedNote::id() == note_id` の最初の要素を返す。aggregates.md#notes-undo の「各 DeletedNote は独立した有効期間を持つ」前提で、同じ `NoteId` の DeletedNote が複数同時に存在する事は無い (delete-note slice が同じ id を 2 度連続で push する経路が存在しないため。重複時の挙動は未定義として OQ で記録)

### 経路境界 {#invariants-boundary}

- **UI 副作用は責務外**: Toast の閉鎖 animation・Note Feed の再描画は UI 層 / subscriber の責務であり、本 slice の output 契約は `Result<Note, RestoreDeletedNoteError>` と 5 副作用 (find / trash / reload / remove / event) のみ
- **Tauri 境界**: Rust 側 command として expose し、tauri-specta で TS bindings を自動生成する (`.ori/architecture.md` cross_root 参照)
- **delete-note 経路への非干渉**: 本 slice は `Note::delete_to_trash` を呼ばない。`UndoStack` を共有するだけで、delete-note 側の push 経路を逆向きに辿る pop / restore を実装する

## テスト観点 {#test-perspectives}

### happy path: トースト中の Undo {#tp-happy}

事前に Undo スタックへ `DeletedNote(id=X, path=/dir/X.md)` を seed → `restore_from_trash(/dir/X.md)` が 1 回呼ばれる → `load_by_id(X)` で復帰した Note が返る → スタックから X が pop される → `NoteRestoredFromTrash { note_id=X, restored_at=clock.now() }` が emit される → Ok(restored_note) を返す。S5 (delete → undo) の Undo 側経路を pin。

### NoUndoAvailable: 空スタック (S7 二重防御) {#tp-no-undo-empty}

Undo スタックが空の状態で実行 → `RestoreDeletedNoteError::NoUndoAvailable { id }`、trash / load / remove / event のいずれも呼ばれない (I-RDN1)。Toast 消失後の Undo 試行を想定。

### NoUndoAvailable: 別 NoteId は残っている (per-toast 独立性、S6/S7) {#tp-no-undo-different-id}

Undo スタックに `DeletedNote(A)` のみ → `note_id=B` で実行 → `NoUndoAvailable { id=B }`、A は変更されずスタックに残る、副作用ゼロ (I-RDN1 + I-RDN7)。

### per-toast 独立: スタックから 1 件のみ pop (S6 改訂) {#tp-stack-targeted-pop}

事前に `[DeletedNote(A), DeletedNote(B), DeletedNote(C)]` を seed → `note_id=B` で restore → 成功後スタックは `[DeletedNote(A), DeletedNote(C)]` (B のみ pop、A / C は順序保持で残存、I-RDN7)。

### TrashRestoreError 伝播 {#tp-trash-restore-err}

事前に `DeletedNote(X)` を seed → `TrashService::restore_from_trash` が `TrashErrorKind::PermissionDenied` を返す → `RestoreDeletedNoteError::TrashRestoreError { path=/dir/X.md, cause }`、load / remove / event 未呼出、X はスタックに残る (I-RDN3, I-RDN4 と整合)。

### ReadError: io::Err 経路 {#tp-read-err-io}

事前に `DeletedNote(X)` を seed → `restore_from_trash` 成功 → `load_by_id(X)` が `io::Err(InvalidData)` を返す → `RestoreDeletedNoteError::ReadError { path, source }`、remove / event 未呼出、X はスタックに残る (I-RDN4)。

### ReadError: Ok(None) も ReadError 扱い (delete-note の collapse 方針と差別化) {#tp-read-err-ok-none}

事前に `DeletedNote(X)` を seed → `restore_from_trash` 成功 → `load_by_id(X)` が `Ok(None)` を返す (復帰したはずなのに見つからない予期せぬ状態) → `RestoreDeletedNoteError::ReadError { path, source: io::ErrorKind::NotFound }` に collapse、remove / event 未呼出。delete-note slice I-DN6 の「`Ok(None)` を NoteNotFound に collapse」とは異なる方針 (本 slice では「事後不整合の表面化」を優先) を pin。

### 副作用順序: find → trash → load → remove → event {#tp-side-effect-order}

happy path で 5 副作用の呼び出し順を OrderLog spy で確認 (I-RDN5)。

### event payload の整合性 {#tp-event-payload}

happy path で発行された `NoteRestoredFromTrash` の payload について：(a) `note_id` が input と一致、(b) `restored_at == Clock::now()` (I-RDN8)。

### restored note が永続化形状を保持 {#tp-restored-note-shape}

happy path で返る `Note` の `id` / `body` / `tags` / `created_at` が seed した `DeletedNote.original_path` の .md frontmatter から構築された値と一致する (domain workflow#notes「復元後の Note は元と同じ NoteId / body / tags / createdAt」)。

### path は DeletedNote 由来 (storage_dir/<id>.md を再計算しない) {#tp-path-from-deleted-note}

事前に `DeletedNote(id=X, path=/custom/dir/X.md)` を seed (storage_dir とは異なる任意 path) → restore → `TrashService::restore_from_trash` 呼出 path が `/custom/dir/X.md` (I-RDN2)。

### NoUndoAvailable は read-only (I-RDN1 強化) {#tp-no-undo-noop}

空スタックでの NoUndoAvailable 経路で `TrashService` / `NoteRepository::load_by_id` / `UndoStack::remove_by_id` / `EventBus::publish` の spy が全て 0 回呼出のままである事を assert。

## 実装ノート {#impl-notes}

### 依存 interface（port） {#impl-ports}

- `TrashService::restore_from_trash(&Path) -> Result<(), TrashErrorKind>` — **delete-note の TrashService trait を拡張**。既存 `move_to_trash` と並ぶ inverse method。`TrashErrorKind` は再利用 (PermissionDenied / Io(String) / Unsupported)
- `UndoStack::find_by_id(&NoteId) -> Option<DeletedNote>` — **新規 method**。既存 `push` と並べて拡張
- `UndoStack::remove_by_id(&NoteId) -> Option<DeletedNote>` — **新規 method**。成功時のみ呼ぶ。戻り値は将来の audit / 検証用 (現状は discard)
- `NoteRepository::load_by_id(&NoteId) -> io::Result<Option<Note>>` — 既存 (auto-save-note / copy-note-body / delete-note と共有)
- `Clock::now() -> Timestamp` — `restored_at` 取得用 (I-RDN8)
- `EventBus::publish(NoteRestoredFromTrash) -> ()` — 既存 event bus port を再利用

### shared 拡張 {#impl-shared-extension}

- `note_capture/shared/events.rs`: `DomainEvent::NoteRestoredFromTrash { note_id, restored_at }` variant を追加 (domain/domain-events.md#note-restored-from-trash-payload に準拠)
- `note_capture/slices/delete_note/ports.rs`: `TrashService` trait に `restore_from_trash` method を追加、`UndoStack` trait に `find_by_id` + `remove_by_id` を追加

### slice layout（DDD-VSA-Hex） {#impl-layout}

- Rust: `apps/promptnotes/src-tauri/src/note_capture/slices/restore_deleted_note/`
  - `mod.rs` — module 宣言 + re-export
  - `domain.rs` — `RestoreDeletedNoteCommand` / `RestoreDeletedNoteError` (NoUndoAvailable / TrashRestoreError / ReadError 3 variant)
  - `application.rs` — `RestoreDeletedNoteUseCase` の 5-step pipeline (find → trash → load → remove → emit)
  - `commands.rs` — `#[tauri::command]` で expose (delete-note と並列)
  - `tests.rs` — 上記テスト観点を実装
- 既存 slices/mod.rs に `pub mod restore_deleted_note;` を追加
- `lib.rs` に `restore_deleted_note::commands::restore_deleted_note` を register

### 既存 slice との関係 {#impl-related-slices}

- `delete-note` slice の `TrashService` trait を import 経由で拡張 (本 slice からは `restore_from_trash` を呼ぶ。逆向きの inverse)
- `delete-note` の `UndoStack::push` と本 slice の `UndoStack::find_by_id / remove_by_id` は同一 trait の異なる method
- `DeletedNote` VO は `shared/types/deleted_note.rs` の既存定義をそのまま使う
- `Note::from_persisted` (既存 aggregate command) を NoteRepository 実装が内部で使う (本 slice は load_by_id 経由でのみ呼ぶ)

### 非責務 {#impl-non-responsibility}

- Toast UI の閉鎖 animation / Note Feed の再描画 (subscriber 層)
- DeletedNote の TTL タイマー管理 (UI 層 + delete-note slice の push 経路)
- 復帰した Note の updated_at 更新 (本 slice では aggregate 状態を変えない方針、I-N1〜I-N4)

## Open Questions {#open-questions}

### oq-duplicate-deleted-note-by-id {#oq-duplicate-deleted-note-by-id}

`UndoStack` に同じ `NoteId` の DeletedNote が複数同時に push される経路は現状の delete-note slice 設計には存在しない (同じ id の Note を 2 回連続で削除しても 1 回目で trash 移動 → 2 回目は load 段階で NoteNotFound)。しかし future-proof として、`find_by_id` / `remove_by_id` の挙動 (FIFO vs LIFO vs 全件) を明示するか検討する。

- 理由: 現状は実質「id 一意」前提で `find_by_id` が `Option<DeletedNote>` を返す
- 検討: 重複時の挙動を spec で明示するか、aggregate-level invariant で「同 id push 禁止」を入れるか
- 現状: spec I-RDN9 で「単一前提」を明文化、impl は最初に見つけた要素 (Vec の前から) を返す

### oq-read-error-ok-none-policy {#oq-read-error-ok-none-policy}

delete-note slice I-DN6 は `Ok(None)` を NoteNotFound に collapse するが、本 slice は I-io_errors と TP-RE2 で `Ok(None)` も ReadError 扱いとする。

- 理由: 本 slice は restore 直後の reload なので `Ok(None)` は「予期せぬ消失」= 事後不整合の徴候
- 検討: 上流 workflow#errors は `ReadError { path, cause: io::Error }` のみで `Ok(None)` ケースに言及がない。spec で本 slice の collapse 方針を明文化 (I-io_errors)
- 現状: ReadError へ collapse (source は io::ErrorKind::NotFound 相当)。今後 production 運用で別ルートが必要になれば 4 variant 化検討

### oq-trash-service-extension {#oq-trash-service-extension}

delete-note の `TrashService` trait に `restore_from_trash` を追加する設計を選択したが、別 trait (`TrashRestoreService`) として分離する案もある。

- 理由: SRP 観点では `move_to_trash` と `restore_from_trash` は inverse op として同 trait に収まる方が UI 契約として自然
- 検討: 別 trait にすると delete-note slice が restore service の存在を知る必要が無くなる
- 現状: 同一 trait として拡張。delete-note slice の tests/ では `restore_from_trash` を default impl で `unimplemented!()` させて影響を抑える
