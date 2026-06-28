---
coherence:
  source: derived
  last_derived: 2026-06-28
  upstream:
    - domain/workflows/flush-note.md#flush-note
    - domain/aggregates.md#note-aggregate
    - domain/bounded-contexts.md#note-capture
    - domain/domain-events.md#note-body-edited
    - domain/validation.md#s3-flush-on-blur
    - domain/validation.md#s13-quit-flush
  hash:
    domain/workflows/flush-note.md#.*: 06ace0dff2ff
    domain/aggregates.md#.*: 9f9048f5816b
    domain/bounded-contexts.md#.*: 4d579125a513
    domain/domain-events.md#.*: 8abdfac78084
    domain/validation.md#.*: 5294b0c32f1b
ori:
  schema:
    propagation_level: file
---

# flush-note spec {#flush-note-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive flush-note`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

EDITING ブロックが debounce timer 待ちの状態で **focus 喪失 / window blur / app quit** の 3 トリガーいずれかが発火した時、debounce を待たず **即時** Note 本文を永続化する slice。Note Capture BC の Flush 経路（domain/bounded-contexts.md#note-capture-ubiquitous-language の "Flush" 用語に対応）を実装する。

> domain/workflows/flush-note.md#flush-note より：「debounce timer を待たず即時永続化を行う。トリガーは Q4 決定の 3 種：(1) ブロック focus 喪失、(2) ウィンドウ blur、(3) アプリ quit」
>
> domain/bounded-contexts.md#note-capture-ubiquitous-language より："Flush — focus 喪失・ウィンドウ blur・アプリ quit による即時永続化"

本 slice は `auto-save-note` slice と **同じ `Note::edit_body` 永続化 + `NoteBodyEdited` 発行契約**を共有する。違いは以下の 3 点：

1. **debounce timer の cancel が責務に含まれる**（auto-save では UI 層、本 slice では明示的にキャンセル）
2. **トリガー識別子 `FlushTrigger` を Input に持つ**（観測ログ / quit 時の連続発火順序を区別するため）
3. **quit 経路で複数 Note への連続 Flush を許容する**（S13、application service レベルで順序処理）

冪等性ガード（同一 body の早期 return）は auto-save-note と同じロジック（domain/workflows/flush-note.md#notes 末尾「冪等ガード（compareBody）は auto-save-note と同じロジック」）。

> domain/validation.md#s3-flush-on-blur より："debounce timer をキャンセル → 即時 `Note::edit_body(now=t2)` を実行（Flush） → event NoteBodyEdited 発行"
>
> domain/validation.md#s13-quit-flush より："quit シグナル受信 → 全 EDITING ブロックを Flush → `Note::edit_body` を A, B, C の順に同期実行 → event が A, B, C 順に連続発行 → quit 完了まで永続化を待つ（最大欠損 500ms を許容）"

## 入出力 {#io}

### Input {#io-input}

> domain/workflows/flush-note.md#input より：

```rust
struct FlushNoteCommand {
  note_id: NoteId,
  pending_body: String,   // debounce 中だった編集中 body（未 parse の生文字列）
  trigger: FlushTrigger,  // BlockBlur | WindowBlur | AppQuit
}

enum FlushTrigger {
  BlockBlur,
  WindowBlur,
  AppQuit,
}
```

依存（外部から注入される interface）:

- `NoteRepository` — `.md` ファイルの **読み出し** (`load_by_id`) と書き出し (`write`)
- `Clock` — `now()` 取得（テスト時 injectable）
- `EventBus` — domain event の **同期** 発行（in-process）
- `DebounceTimer` — `cancel(note_id)` を呼ぶための handle（`auto-save-note` slice の use case には注入されないが、本 slice では cancel 責務のため必須）

> domain/workflows/flush-note.md#dependencies より：`NoteRepository`, `Clock`, `EventBus`, `DebounceTimer`（キャンセル用 handle）

`FlushTrigger` は use case 内では分岐に使わない（3 種いずれも同じ pipeline）。観測ログ / domain event payload には現状載せない（[#oq-trigger-in-event](#oq-trigger-in-event) で議論）。

### Output {#io-output}

戻り値: `Result<Option<Note>, FlushError>`

- `Ok(Some(note))` — body が変化したケース。永続化 + event 発行
- `Ok(None)` — **no-op**（冪等性ガードで早期 return、event 非発行）
- `Err(_)` — 4 variant のいずれか（[#io-errors](#io-errors) 参照）

> 戻り値型は `auto-save-note` slice の `Result<Option<Note>, AutoSaveError>` と同形。Flush と AutoSave が同じ「`Note::edit_body` 永続化 + `NoteBodyEdited` 発行」契約を共有する（auto-save-note spec C-AS9）ことから、戻り値も同形にする。

成功時 (`Ok(Some(note))`)：

- 戻り値: 更新後の Note Aggregate
- 発行 event: [`NoteBodyEdited`](../../domain/domain-events.md#note-body-edited)

> domain/domain-events.md#note-body-edited-payload より：

```rust
struct NoteBodyEdited {
  note_id: NoteId,
  updated_at: Timestamp,
}
```

`body` 本文は載せない（Shared Kernel 経由で `&Note` を参照すれば取得可能）。`FlushTrigger` も載せない（[#oq-trigger-in-event](#oq-trigger-in-event)）。

### Errors {#io-errors}

> domain/workflows/flush-note.md#errors は 2 variant（`NoteNotFound`, `PersistError`）のみ記載。本 slice は **`auto-save-note` と同形の 4 variant** に整形する（[#oq-error-variant-alignment](#oq-error-variant-alignment) で根拠を議論）。

```rust
enum FlushError {
  NoteNotFound { id: NoteId },
  InvalidBody { source: NoteBodyError },
  LoadError { path: PathBuf, source: io::Error },
  PersistError { path: PathBuf, source: io::Error },
}
```

- **`NoteNotFound`** — `NoteRepository::load_by_id` が `Ok(None)` を返した場合
- **`InvalidBody`** — `NoteBody::new(pending_body)` が失敗した場合（aggregates.md#note-aggregate-invariants の **I-N8** 違反、典型的には `---` 行を含む body）
- **`LoadError`** — `load_by_id` の read I/O 失敗 / 既存 `.md` ファイルの parse 失敗 (`io::ErrorKind::InvalidData`)
- **`PersistError`** — `NoteRepository::write` の I/O 失敗（**write 経路専用**に意味を絞る）

> auto-save-note proposal accepted (.ori/proposals/accepted/2026-06-25-auto-save-note-workflows-auto-save-note-errors.md) followup より：「flush-note workflow にも同形を将来適用 (本 proposal で同時改訂はしない)」。本 slice は spec レベルで先取りし、upstream への propagate は phase 7 で proposal 化する（[#oq-error-variant-alignment](#oq-error-variant-alignment)）。

## 不変条件 {#invariants}

slice 完了時に成立すべき条件。括弧内は domain での出典。

### Note Aggregate 由来 {#invariants-note-aggregate}

> domain/aggregates.md#note-aggregate-invariants より引用：

- **I-N1**: `id` は immutable。本 slice は `Note::edit_body` のみを呼び `id` には触れない
- **I-N3**: `updatedAt >= createdAt` を常に満たす。`Note::edit_body` 経由で `now` を渡すため、`now >= note.created_at` が `Clock` 契約として前提
- **I-N4**: `body` 変更時に `updatedAt = now`（秒精度）。`Note::edit_body` 内で保証される
  - 同一秒内の連続編集では `updatedAt` は同じ値に留まる（時計の解像度）
- **I-N8**: `body` の構築は `NoteBody::new` 経由でのみ可能であり、frontmatter delimiter 行 (`---`) を含まない。本 slice は `pending_body: String` を `NoteBody::new` で smart construct するため、違反時は `FlushError::InvalidBody` で表面化させる

### slice 固有制約 {#invariants-slice-specific}

- **C-FL1**: 入口で **`DebounceTimer::cancel(note_id)` を必ず最初に呼ぶ**（domain/workflows/flush-note.md#steps step 1、S3 「debounce timer をキャンセル」）。
  - cancel は **idempotent** で、timer が無くてもエラーを返さない（spurious cancel 許容）
  - cancel と persist の順序は固定: **cancel → load → ... → persist**
  - **race 防止の責務分担**: 「AutoSave 並走防止」自体は **composition root（UI 層の debounce timer）の責務**である。Rust use case 側の C-FL1 はあくまで「`DebounceTimer` port を最初に同期呼び出しする」契約に留まり、port 実装が何を cancel するか（JS timer / Rust timer / no-op）には依存しない。production の `NoOpDebounceTimer` 採択根拠は [#oq-debounce-cancel-composition](#oq-debounce-cancel-composition) を参照
- **C-FL2**: `note_id` で `NoteRepository::load_by_id` を呼ぶ。
  - 戻り値 `Ok(None)` → `FlushError::NoteNotFound { id }`
  - 戻り値 `Err(io)` → `FlushError::LoadError { path, source }`（read I/O 失敗、別 variant に分離）
- **C-FL3**: `pending_body` 文字列を `NoteBody::new(String) -> Result<NoteBody, NoteBodyError>` で構築する（aggregate I-N8 由来の **fallible smart constructor**）。失敗時は `FlushError::InvalidBody { source }` で表面化
- **C-FL4**: `existing_note.body() == pending_body` の場合は **何もせず `Ok(None)` を返す**（冪等性ガード、event 非発行）
  - 比較は `NoteBody` 値の `PartialEq` で行う（バイト等価）
  - `auto-save-note` C-AS3 と同形
- **C-FL5**: body が変化した場合のみ `Note::edit_body(new_body, now)` を呼ぶ
- **C-FL6**: `NoteRepository::write(&updated_note)` で永続化する。失敗時は `FlushError::PersistError { path, cause }` を返し、event は **発行しない**
  - 失敗時 Note は in-memory には更新済みだが、永続化されていないため event を発行すべきでない（at-least-once 永続 → event の順）
- **C-FL7**: 永続化成功後、`EventBus::publish(DomainEvent::NoteBodyEdited { note_id, updated_at })` を **1 回だけ** 同期発行する
- **C-FL8**: use case は **stateless**。`FlushTrigger` の値は use case 内部状態に影響しない（observability の hint のみ）
- **C-FL9**: 本 slice は `tags` を変更しない

### 経路境界 {#invariants-boundary}

- **C-FL10**: Flush **と** AutoSave は同じ「`Note::edit_body` 永続化 + `NoteBodyEdited` 発行」契約を共有するが、本 slice は **Flush 経路のみ**を実装する。差分は (a) `DebounceTimer::cancel` の追加、(b) `FlushTrigger` の追加。AutoSave (`auto-save-note` slice) は cancel 責務を持たない
- **C-FL11**: S13 連続 Flush（quit 経路）の **複数 Note 順次処理**は本 slice の外部（application service / Tauri command 層）の責務。本 slice の use case は **1 回の `FlushNoteCommand` を処理する** stateless API である
  > domain/validation.md#s13-quit-flush より：「quit シグナル受信 → 全 EDITING ブロックを Flush → `Note::edit_body` を A, B, C の順に同期実行」。順序保証は呼び出し側 (Tauri command + frontend hook) の責務

## テスト観点 {#test-perspectives}

phase 3 で failing test に展開する観点を列挙。各観点は 1 つ以上の test に対応する想定。

### happy path: body 変化あり {#tp-happy}

- **TP-H1**: 既存 Note A (`body="hello"`, `updated_at=t0`) に対し `FlushNoteCommand { note_id: A.id, pending_body: "hello world", trigger: BlockBlur }` を発火 → `Ok(Some(updated))`、`updated.body == "hello world"`、`updated.updated_at == t1` (`Clock::now()` の戻り値)
- **TP-H2**: TP-H1 のケースで `DebounceTimer::cancel(A.id)` が **1 回** 呼ばれ、その後に `NoteRepository::write(&updated)` が呼ばれる（C-FL1 順序）
- **TP-H3**: TP-H1 のケースで `EventBus::publish(NoteBodyEdited { note_id: A.id, updated_at: t1 })` が **1 回だけ** 呼ばれる
- **TP-H4**: `updated.tags == A.tags`、`updated.created_at == A.created_at`、`updated.id == A.id`（C-FL9、I-N1）
- **TP-H5**: `trigger` を `WindowBlur` / `AppQuit` に変えても TP-H1〜H4 と同じ振る舞い（C-FL8、3 トリガー等価）

### S3: focus 喪失で即時 Flush {#tp-s3-blur}

- **TP-S3-1**: S3 シナリオを再現：`trigger=BlockBlur`、debounce timer が active な状態で発火 → `cancel` が呼ばれ、その後 `Ok(Some(updated))` 返却
- **TP-S3-2**: TP-S3-1 で `EventBus::publish(NoteBodyEdited { updated_at: t2 })` が発行される（domain/validation.md#s3-then「event NoteBodyEdited 発行」）

### S13: quit 時の連続 Flush（順序）{#tp-s13-quit}

- **TP-S13-1**: `trigger=AppQuit` で 1 Note を Flush する基本ケース → `Ok(Some(updated))`、`cancel` → `write` → `publish` の順
- **TP-S13-2**: use case は **1 件の Flush しか責任を持たない**ことを type-level pin で確認（C-FL11、複数 Note 順序保証は外部）

### 冪等性ガード（body 不変） {#tp-idempotent}

- **TP-I1**: Note A (`body="hello"`) に対し `pending_body: "hello"`、`trigger=BlockBlur` を発火 → `Ok(None)`
- **TP-I2**: TP-I1 のケースで `DebounceTimer::cancel` は **呼ばれる**（C-FL1、cancel は body 比較より先）、`NoteRepository::write` は **呼ばれない**（C-FL4）
- **TP-I3**: TP-I1 のケースで `EventBus::publish` が **呼ばれない**

### NoteNotFound {#tp-not-found}

- **TP-NF1**: 存在しない `note_id` で発火 → `Err(FlushError::NoteNotFound { id })`
- **TP-NF2**: TP-NF1 のケースで `DebounceTimer::cancel` は **呼ばれる**（cancel は load より先）、`NoteRepository::write` も `EventBus::publish` も呼ばれない
- **TP-NF3**: TP-NF1 で `id` フィールドは入力の `note_id` をそのまま返す

### LoadError {#tp-load-err}

- **TP-LE1**: `load_by_id` が `Err(PermissionDenied)` を返す → `Err(FlushError::LoadError { path, source })`、`path == <storage_dir>/<id>.md`、`source.kind() == PermissionDenied`
- **TP-LE2**: TP-LE1 のケースで `write` も `publish` も呼ばれない（`cancel` は呼ばれている）
- **TP-LE3**: read 失敗は **`PersistError` には化けない**（C-FL2 / workflow#errors 同形化）

### InvalidBody {#tp-invalid-body}

- **TP-IB1**: `pending_body` が `---`（frontmatter delimiter 行）を含む → `Err(FlushError::InvalidBody { source: ContainsFrontmatterDelimiter })`（I-N8）
- **TP-IB2**: TP-IB1 のケースで `write` も `publish` も呼ばれない

### PersistError {#tp-persist-err}

- **TP-PE1**: 既存 Note A に body 変化ありで発火、`NoteRepository::write` が `io::Error::PermissionDenied` を返す → `Err(FlushError::PersistError { path, cause })`
- **TP-PE2**: TP-PE1 で `cause.kind() == ErrorKind::PermissionDenied`
- **TP-PE3**: TP-PE1 で `EventBus::publish` が **呼ばれない**（C-FL6: 永続化失敗時は event 非発行）
- **TP-PE4**: TP-PE1 後に同じ command を再発火（fs が回復済）→ `Ok(Some(updated))` 返る。use case の stateless 性確認

### cancel ↔ persist の順序 {#tp-cancel-order}

- **TP-CO1**: 全 happy path / error path で `DebounceTimer::cancel(note_id)` が `NoteRepository::load_by_id` より **前** に呼ばれる（C-FL1、AutoSave 並走防止）
- **TP-CO2**: `cancel` が **idempotent**（timer なしでも `Ok(())`）であることを mock で確認

### body 比較の詳細 {#tp-body-compare}

- **TP-BC1**: `body=""`（空文字）の Note に `pending_body=""` → `Ok(None)`（空文字も冪等性対象）
- **TP-BC2**: `body="hello"` に `pending_body="hello "` (末尾スペース) → `Ok(Some(updated))`（バイト等価ではない、I-N4 経路）
- **TP-BC3**: `body="hello"` に `pending_body="Hello"` (case 違い) → `Ok(Some(updated))`（case-sensitive 比較）

### 不変条件チェック {#tp-invariants}

- **TP-INV1**: 任意の TP-H* の戻り値で `updated.id == input.note_id`（I-N1）
- **TP-INV2**: 任意の TP-H* の戻り値で `updated.updated_at >= updated.created_at`（I-N3）

### no-op 契約 {#tp-api-shape}

- **TP-AS1**: `FlushNoteUseCase::execute(&self, cmd) -> Result<Option<Note>, FlushError>` のシグネチャを **type-level pin**（compile-time assertion）
- **TP-AS2**: `tags` フィールドが副作用で変化しないことを TP-H4 の延長で確認

## 実装ノート {#impl-notes}

### アーキ層への落とし込み {#impl-layers}

DDD-VSA-Hex / typescript-tauri の階層に従い、本 slice は Rust 側で実装する（`implementation.language: rust`）。

```
apps/promptnotes/src-tauri/src/note_capture/
├── mod.rs                  # pub use slices::*; shared::*
├── shared/                 # 既存：types/, events.rs, ports.rs, result.rs
│   ├── events.rs           # DomainEvent::NoteBodyEdited は auto-save-note slice で追加済み
│   └── ports.rs            # NoteRepository::load_by_id は auto-save-note slice で追加済み
└── slices/
    ├── create_note/        # 既存
    ├── auto_save_note/     # 既存
    └── flush_note/         # 新規
        ├── mod.rs          # pub use commands::*
        ├── domain.rs       # FlushNoteCommand, FlushTrigger, FlushError (4 variant)
        ├── application.rs  # FlushNoteUseCase: cancel → load → parse → compare → edit → persist → emit
        ├── commands.rs     # #[tauri::command] flush_note → tauri-specta surface
        └── tests.rs        # unit tests for TP-* (in-memory NoteRepository mock + DebounceTimer mock)
```

### 既存 port / event の再利用 {#impl-reuse}

`auto-save-note` slice で追加された以下は **再利用** する（拡張不要）：

- **`NoteRepository::load_by_id(&self, id: &NoteId) -> std::io::Result<Option<Note>>`** — auto-save-note で追加済み
- **`DomainEvent::NoteBodyEdited { note_id, updated_at }`** — auto-save-note で追加済み
- **`Note::from_persisted(...)`** — aggregates.md に追加済み（auto-save-note proposal で正式化）

本 slice では新規に追加するのは：

- **`DebounceTimer` port**（trait）— `cancel(note_id: &NoteId)` メソッドを持つ薄い trait。実装は composition root（Tauri 起動部）または frontend hook 側で注入される。テストは mock で完結
- **`FlushTrigger` enum**（slice domain layer）

### Pipeline ステップ {#impl-pipeline}

> domain/workflows/flush-note.md#steps の DMMF pipeline を採用：

1. `cancel_debounce: NoteId → ()` — `DebounceTimer::cancel(&note_id)` を呼ぶ。idempotent
2. `load_note: NoteId → Result<Note, FlushError>` — `NoteRepository::load_by_id` を呼び、`Ok(None)` を `NoteNotFound`、`Err(io)` を `LoadError` に変換
3. `parse_body: String → Result<NoteBody, FlushError>` — `NoteBody::new(String)` (**aggregate I-N8 由来の fallible smart constructor**)、失敗時は `InvalidBody`
4. `compare_body: (&Note, &NoteBody) → BodyDiff` — `note.body() == pending_body` で `Unchanged | Changed`
5. `branch_on_diff`:
   - `Unchanged` → `Ok(None)` 早期 return
   - `Changed` → step 6 へ
6. `update_body: (Note, NoteBody, Timestamp) → Note` — `note.edit_body(new_body, now)` (`Clock::now()` を `now` に注入)
7. `persist: &Note → Result<(), FlushError>` — `NoteRepository::write(&updated)`、失敗時は `PersistError`
8. `emit: &Note → ()` — `EventBus::publish(DomainEvent::NoteBodyEdited { note_id, updated_at })`

ステップ 2, 3, 7 で fallible（4 variant の error 表面化）、1, 7, 8 で副作用が走る。ステップ 4-6 は副作用なし。

**順序ロジック**: step 1 (`cancel`) は **load より前** に置く（C-FL1）。AutoSave と Flush が同時に走る race condition を防ぐため、cancel は無条件 / 同期 / 最初に実行。

### BodyDiff の表現 {#impl-body-diff}

`auto-save-note` slice 内 `application.rs` に `BodyDiff` enum がある（private impl detail）。本 slice では **同じ表現を再宣言** する（slice 間の domain 漏れを避けるため、private 再宣言が DDD-VSA 的に正しい）。

```rust
enum BodyDiff {
  Unchanged,
  Changed(NoteBody),
}
```

ステップ 5 の分岐を型で表現することで、`Unchanged` 経路と `Changed` 経路の混線（例: `Unchanged` でも誤って `write` を呼ぶ等）を compile-time に防ぐ。

### Tauri command surface {#impl-tauri}

- `#[tauri::command] async fn flush_note(state: State<AppState>, note_id: String, pending_body: String, trigger: FlushTriggerDto) -> Result<FlushOutcome, FlushErrorDto>`
- 戻り値は **DTO 経由**で frontend に渡す（4 variant を serde で安定化）
- `trigger: FlushTriggerDto` は frontend からの enum（`"block_blur" | "window_blur" | "app_quit"` を serde で受ける）
- `note_id: String` は frontend からの raw 文字列。`NoteId::try_from` 失敗時の扱いは `auto-save-note` の [#oq-invalid-note-id](../auto-save-note/spec.md#oq-invalid-note-id) と共通方針（sentinel epoch + NoteNotFound 降格）を継承
- tauri-specta で TS bindings 生成

### DebounceTimer port の表現 {#impl-debounce-port}

`DebounceTimer` は **port (trait)** として `note_capture/shared/ports.rs` に追加する（auto-save-note slice / 他 slice からも将来参照する可能性）：

```rust
pub trait DebounceTimer {
  fn cancel(&self, note_id: &NoteId);
}
```

- 戻り値は `()`（idempotent、失敗は無視）
- 実装は composition root（Tauri 起動部）で frontend と橋渡しする。本 slice は **trait 経由でしか触らない**
- テスト用 mock は `tests.rs` 内に `MockDebounceTimer { cancel_calls: RefCell<Vec<NoteId>> }` 等で実装

### S13 連続 Flush の責務分離 {#impl-quit-orchestration}

domain/validation.md#s13-quit-flush の「A, B, C 順に同期実行」「quit 完了まで永続化を待つ」は **本 slice の use case の外部** で実装する：

- **Tauri 側**: `app.on_window_event(WindowEvent::CloseRequested)` で全 EDITING Note の `FlushNoteCommand` を順次呼び、最後の結果を待ってから `app.exit(0)` する
- **frontend 側**: window close hook が `flush_note` Tauri command を await し、複数 EDITING Note を順次処理

本 slice の `FlushNoteUseCase::execute` は **1 件しか責任を持たない**（C-FL11）。テスト TP-S13-2 はこの境界を type-level で pin する。

### Out of scope {#out-of-scope}

- AutoSave 経路（500ms debounce 集約）— `auto-save-note` slice の責務
- DebounceTimer の実装（UI 層 / composition root）
- S13 の「複数 EDITING Note の順次処理」 — Tauri 起動部の責務（[#impl-quit-orchestration](#impl-quit-orchestration)）
- tag の変更（`assign-tag` / `remove-tag` slice）
- create-note 経路（既存 `create-note` slice）
- `NoteRepository` の永続化フォーマット詳細（既存 `FsNoteRepository` を再利用、auto-save-note で `load_by_id` 拡張済み）
- frontend の EDITING ↔ IDLE ステートマシン（Note Capture の UI 層、本 slice の外）
- `NoteBodyEdited` event に `FlushTrigger` を載せるか（[#oq-trigger-in-event](#oq-trigger-in-event)）

## Open Questions {#open-questions}

### Errors 4 variant 化の upstream propagate {#oq-error-variant-alignment}

- **status**: open
- **問題**: domain/workflows/flush-note.md#errors は 2 variant（`NoteNotFound`, `PersistError`）のみ。本 spec は `auto-save-note` と同形の 4 variant（`InvalidBody`, `LoadError` を追加）に **先取り** で整形している。
- **根拠**:
  - auto-save-note proposal accepted (.ori/proposals/accepted/2026-06-25-auto-save-note-workflows-auto-save-note-errors.md) followup: 「flush-note workflow にも同形を将来適用」
  - 同じ `NoteRepository::load_by_id` + `NoteBody::new` 経路を踏むため、4 variant は構造的に必要
- **対応**: phase 7 finalize で `/ori-propose` を作成し、domain/workflows/flush-note.md#errors を 4 variant に更新する proposal を上げる
- **影響**: domain workflow 上の文言差分。実装には影響なし（spec が先取りで決定済み）

### NoteBodyEdited event に FlushTrigger を載せるか {#oq-trigger-in-event}

- **status**: open
- **問題**: 観測ログや subscriber 側の挙動分岐のため `FlushTrigger` を event payload に載せるかどうか。
- **trade-off**:
  - 載せない（現状）: payload 最小化方針（domain-events.md#notes-payload-minimization）と一致。subscriber は経路を区別できない
  - 載せる: 観測しやすいが Shared Kernel の `&Note` で取得できない情報を event に持たせることになる
- **暫定**: 載せない。NoteBodyEdited は AutoSave / Flush 共通の event であり、Flush 経路の識別は **観測ログ層** （application service 側）で行う

### Debounce cancel の composition root {#oq-debounce-cancel-composition}

- **status**: open（review HIGH-1 由来）
- **問題**: production composition root (`flush_note/commands.rs::flush_note`) は `NoOpDebounceTimer` を注入している。すなわち Rust 側で `DebounceTimer::cancel` は何も行わない。「AutoSave 並走防止」契約は **frontend の debounce timer** が `flush_note` invoke 前に同期 cancel する前提に依存する
- **理由**: 実体の 500ms debounce timer は frontend (TypeScript hook) 側にある。Rust 側に shadow timer を持つと double-source-of-truth になり、`auto-save-note` slice との重複も発生する
- **trade-off**:
  - 現方針 (NoOp + JS 側 cancel): 設計は単純、ただし backend からは race が観測不能
  - Rust 側に shared cancel notifier（`tokio::sync::Notify` 等）: race を backend で防げるが、UI 層が cancel をスキップした場合のみ意味があり、複雑化する割に収益が薄い
  - `auto_save_note` Tauri command 側に "ignore if flushed within Nms" guard: 別 slice 側に責務がはみ出す
- **暫定**: NoOp 採用。frontend hook 側の責務として「`flush_note` invoke 前に必ず `clearTimeout` を同期実行する」ルールを `apps/promptnotes/src/lib/note-capture/` に明文化する（本 slice の out-of-scope だが follow-up）。本 OQ を accept ratifying するまで、C-FL1 文言は port 呼び出し契約に限定する
- **影響**: 万一 frontend が cancel を skip した場合、AutoSave debounce 完了タイミング次第で `auto_save_note` Tauri command が `flush_note` の直後に走り、同一 NoteBody で `write` を 2 回踏みうる。冪等性ガード (C-FL4 / C-AS3) で event は重複しないが、disk I/O は 2 回発生する

### NoteId parse 失敗時の variant {#oq-invalid-note-id}

- **status**: open（auto-save-note と共通）
- **問題**: Tauri boundary で `note_id: String` を受け取り `NoteId::try_from` に失敗した場合の error variant 選択
- **暫定**: `auto-save-note` の [#oq-invalid-note-id](../auto-save-note/spec.md#oq-invalid-note-id) と同じ方針（sentinel epoch + `NoteNotFound` 降格）を採用
- **影響**: 上流 (`NoteId::try_from`) で smart constructor が明文化されたら両 slice 同時に更新
