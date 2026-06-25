---
coherence:
  source: derived
  last_derived: 2026-06-25
  upstream:
    - domain/workflows/auto-save-note.md#auto-save-note
    - domain/aggregates.md#note-aggregate
    - domain/bounded-contexts.md#note-capture
    - domain/domain-events.md#note-body-edited
    - domain/validation.md#s2-autosave-debounce
    - domain/validation.md#s9-idempotent-autosave
  hash:
    domain/workflows/auto-save-note.md#.*: 1a3b1789524f
    domain/aggregates.md#.*: 94b27e21aade
    domain/bounded-contexts.md#.*: 4d579125a513
    domain/domain-events.md#.*: 8abdfac78084
    domain/validation.md#.*: 5294b0c32f1b
ori:
  schema:
    propagation_level: file
---

# auto-save-note spec {#auto-save-note-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive auto-save-note`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

EDITING ブロックでのキー入力後、500ms debounce が成立した時に Note 本文を永続化する slice。Note Capture BC の AutoSave 経路（domain/bounded-contexts.md#note-capture-ubiquitous-language の "AutoSave" 用語に対応）を実装する。

> domain/workflows/auto-save-note.md#auto-save-note より：「EDITING ブロックでのキー入力後 500ms debounce が成立した時に、Note 本文を永続化する」
>
> domain/bounded-contexts.md#note-capture-subdomain-type より：本 BC は **core subdomain**。`.md` ファイル所有性と frontmatter 整合性がここに集約される。

本 slice は **application service レベルで冪等性ガード**を行う。同一 body の連続発火は早期 return で event 非発行（S9）。`Note::edit_body` 自体は呼ばれれば I-N4 に従い `updatedAt` を更新するため、guard は use case 側の責務である。

> domain/validation.md#s9-idempotent-autosave より：「『同一 body 編集』を application service レベルで弾く方針。Note Aggregate 自体は呼ばれれば `updatedAt` を更新する」

debounce timer 本体（500ms の集約・cancel）は UI / application 層の責務で、本 slice の use case は **debounce 成立後の単一発火を 1 回処理する** stateless API である。Flush（focus 喪失 / blur / quit）は別 slice (`flush-note`) の責務。

## 入出力 {#io}

### Input {#io-input}

> domain/workflows/auto-save-note.md#input より：

```rust
struct AutoSaveNoteCommand {
  note_id: NoteId,
  new_body: String,        // 編集中の現在 body（未 parse の生文字列）
}
```

依存（外部から注入される interface）:

- `NoteRepository` — `.md` ファイルの **読み出し** (`load_by_id` 等) と書き出し (`write`) を提供
- `Clock` — `now()` 取得（テスト時 injectable）
- `EventBus` — domain event の **同期** 発行（in-process）

`DebounceTimer` は本 slice の use case には注入されない（UI / composition root の責務）。

> domain/workflows/auto-save-note.md#dependencies は `DebounceTimer` を列挙するが、これは「workflow 全体の依存」であり、use case API のシグネチャ依存ではない。本 slice の use case (`execute`) は debounce 成立後の単発呼び出し。

### Output {#io-output}

戻り値: `Result<Option<Note>, AutoSaveError>`

- `Ok(Some(note))` — body が変化したケース。永続化 + event 発行
- `Ok(None)` — **no-op**（S9 冪等性ガードで早期 return、event 非発行）
- `Err(_)` — `NoteNotFound` / `PersistError`

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

`body` 本文は載せない（Shared Kernel 経由で `&Note` を参照すれば取得可能）。

### Errors {#io-errors}

> domain/workflows/auto-save-note.md#errors より：

```rust
enum AutoSaveError {
  NoteNotFound { id: NoteId },
  PersistError { path: PathBuf, cause: io::Error },
}
```

`InvalidBody` 系のエラーは存在しない：`NoteBody` は「任意の UTF-8 文字列（空文字も許容）」と定義されており (`aggregates.md#note-aggregate-elements`)、parse 失敗ケースがない。

## 不変条件 {#invariants}

slice 完了時に成立すべき条件。括弧内は domain での出典。

### Note Aggregate 由来 {#invariants-note-aggregate}

> domain/aggregates.md#note-aggregate-invariants より引用：

- **I-N1**: `id` は immutable。本 slice は `Note::edit_body` のみを呼び `id` には触れない
- **I-N3**: `updatedAt >= createdAt` を常に満たす。`Note::edit_body` 経由で `now` を渡すため、`now >= note.created_at` が `Clock` 契約として前提
- **I-N4**: `body` 変更時に `updatedAt = now`（秒精度）。`Note::edit_body` 内で保証される
  - 同一秒内の連続編集では `updatedAt` は同じ値に留まる（時計の解像度）

### slice 固有制約 {#invariants-slice-specific}

- **C-AS1**: `note_id` で `NoteRepository::load_by_id` を呼び、結果が `None` の場合は `AutoSaveError::NoteNotFound { id }` を返す（S9 と異なり、ノートそのものが消えていた場合）
- **C-AS2**: `new_body` 文字列を `NoteBody::from(String)` で構築する（infallible、空文字も valid）
- **C-AS3**: `existing_note.body() == new_body` の場合は **何もせず `Ok(None)` を返す**（S9 冪等性ガード、event 非発行）
  - 比較は `NoteBody` 値の `PartialEq` で行う（バイト等価）
  - body 以外（tags / timestamps）の差分は本 slice の判定対象外（タグ変更は `assign-tag` / `remove-tag` slice の責務）
- **C-AS4**: body が変化した場合のみ `Note::edit_body(new_body, now)` を呼ぶ
- **C-AS5**: `NoteRepository::write(&updated_note)` で永続化する。失敗時は `AutoSaveError::PersistError { path, cause }` を返し、event は **発行しない**
  - 失敗時 Note は in-memory には更新済みだが、永続化されていないため event を発行すべきでない（at-least-once 永続 → event の順）
- **C-AS6**: 永続化成功後、`EventBus::publish(DomainEvent::NoteBodyEdited { note_id, updated_at })` を **1 回だけ** 同期発行する（C-LS7 と対照的）
- **C-AS7**: use case は **stateless**。`AutoSaveTrigger` を時系列で de-duplicate する責務は持たない（debounce 自体は UI 層、冪等性ガードは body 差分で実現）
- **C-AS8**: 本 slice は `tags` を変更しない（C-AS3 の対偶として、tags 変更経路は別 slice）

### 経路境界 {#invariants-boundary}

- **C-AS9**: AutoSave **と** Flush は同じ「`Note::edit_body` 永続化 + `NoteBodyEdited` 発行」契約を共有するが、本 slice は **AutoSave 経路のみ**を実装する。Flush は別 slice (`flush-note`) で実装され、debounce timer の cancel は flush 側の責務
  > domain/domain-events.md#note-body-edited-trigger より：「発行経路は AutoSave (debounce) または Flush」

## テスト観点 {#test-perspectives}

phase 3 で failing test に展開する観点を列挙。各観点は 1 つ以上の test に対応する想定。

### happy path: body 変化あり {#tp-happy}

- **TP-H1**: 既存 Note A (`body="hello"`, `updated_at=t0`) に対し `AutoSaveNoteCommand { note_id: A.id, new_body: "hello world" }` を発火 → `Ok(Some(updated))`、`updated.body == "hello world"`、`updated.updated_at == t1` (`Clock::now()` の戻り値)
- **TP-H2**: TP-H1 のケースで `NoteRepository::write(&updated)` が 1 回呼ばれる
- **TP-H3**: TP-H1 のケースで `EventBus::publish(NoteBodyEdited { note_id: A.id, updated_at: t1 })` が **1 回だけ** 呼ばれる
- **TP-H4**: `updated.tags == A.tags`、`updated.created_at == A.created_at`、`updated.id == A.id`（C-AS8、I-N1）

### S9: 冪等性ガード（body 不変） {#tp-idempotent}

- **TP-I1**: Note A (`body="hello"`) に対し `new_body: "hello"` を発火 → `Ok(None)`
- **TP-I2**: TP-I1 のケースで `NoteRepository::write` が **呼ばれない**（C-AS3）
- **TP-I3**: TP-I1 のケースで `EventBus::publish` が **呼ばれない**（S9）
- **TP-I4**: TP-I1 のケースで `Clock::now` を **呼んでもよい**が、戻り値は `updated_at` に伝播しない（呼ばれないことを assert する必要はない）

### NoteNotFound {#tp-not-found}

- **TP-NF1**: 存在しない `note_id` で発火 → `Err(AutoSaveError::NoteNotFound { id })`
- **TP-NF2**: TP-NF1 のケースで `NoteRepository::write` も `EventBus::publish` も呼ばれない
- **TP-NF3**: TP-NF1 で `id` フィールドは入力の `note_id` をそのまま返す

### PersistError {#tp-persist-err}

- **TP-PE1**: 既存 Note A に body 変化ありで発火、`NoteRepository::write` が `io::Error::PermissionDenied` を返す → `Err(AutoSaveError::PersistError { path, cause })`
- **TP-PE2**: TP-PE1 で `cause.kind() == ErrorKind::PermissionDenied`
- **TP-PE3**: TP-PE1 で `EventBus::publish` が **呼ばれない**（C-AS5: 永続化失敗時は event 非発行）
- **TP-PE4**: TP-PE1 後に同じ command を再発火（fs が回復済）→ `Ok(Some(updated))` 返る。**再発火は AutoSave 経路の責務外**だが、use case の stateless 性確認として有効

### body 比較の詳細 {#tp-body-compare}

- **TP-BC1**: `body=""`（空文字）の Note に `new_body=""` → `Ok(None)`（空文字も冪等性対象）
- **TP-BC2**: `body="hello"` に `new_body="hello "` (末尾スペース) → `Ok(Some(updated))`（バイト等価ではない、I-N4 経路）
- **TP-BC3**: `body="hello"` に `new_body="Hello"` (case 違い) → `Ok(Some(updated))`（case-sensitive 比較）

### 不変条件チェック {#tp-invariants}

- **TP-INV1**: 任意の TP-H* の戻り値で `updated.id == input.note_id`（I-N1）
- **TP-INV2**: 任意の TP-H* の戻り値で `updated.updated_at >= updated.created_at`（I-N3）
- **TP-INV3**: 同一秒内に TP-H1 と同じ command を 2 回発火 → 2 回目も body は新値だが、`Clock::now` が同じ秒を返すなら `updated_at` は同じ値 (I-N4 補足、秒精度)。本 slice は冪等性 guard が C-AS3 で先に効くため、実際にはこのケースは TP-I1 ルートに行く

### no-op 契約 {#tp-api-shape}

- **TP-AS1**: `AutoSaveNoteUseCase::execute(&self, cmd) -> Result<Option<Note>, AutoSaveError>` のシグネチャを **type-level pin**（compile-time assertion）
- **TP-AS2**: `tags` フィールドが副作用で変化しないことを TP-H4 の延長で確認

## 実装ノート {#impl-notes}

### アーキ層への落とし込み {#impl-layers}

DDD-VSA-Hex / typescript-tauri の階層に従い、本 slice は Rust 側で実装する（`implementation.language: rust`）。

```
apps/promptnotes/src-tauri/src/note_capture/
├── mod.rs                  # pub use slices::*; shared::*
├── shared/                 # 既存：types/, events.rs, ports.rs, result.rs
│   ├── events.rs           # DomainEvent::NoteBodyEdited を追加（impl phase）
│   └── ports.rs            # NoteRepository::load_by_id を追加（impl phase）
└── slices/
    ├── create_note/        # 既存
    └── auto_save_note/     # 新規
        ├── mod.rs          # pub use commands::*
        ├── domain.rs       # AutoSaveNoteCommand, AutoSaveError
        ├── application.rs  # AutoSaveNoteUseCase: load → compare → edit → persist → emit
        ├── infrastructure.rs  # (必要なら) FsNoteRepository への load_by_id 拡張
        ├── commands.rs     # #[tauri::command] auto_save_note → tauri-specta surface
        └── tests.rs        # unit tests for TP-* (in-memory NoteRepository mock)
```

### 既存 port / event の拡張 {#impl-port-extension}

本 slice の前提として、`note_capture/shared/` に 2 つの拡張が必要（impl phase で実施）：

- **`NoteRepository::load_by_id(&self, id: &NoteId) -> std::io::Result<Option<Note>>`** — 新規追加。`.md` ファイルから Note を復元する経路。create_note slice は write only だったため未定義
- **`DomainEvent::NoteBodyEdited { note_id, updated_at }`** — 既存 enum に variant 追加

両者とも他 slice (`flush-note`, `assign-tag` 等) でも再利用される shared 拡張。本 slice の作業に含めるが、**shared layer の変更は load-settings / create-note の test 群に影響しない**ことを test-red phase で確認する（既存テスト GREEN 維持）。

### Pipeline ステップ {#impl-pipeline}

> domain/workflows/auto-save-note.md#steps の DMMF pipeline を採用：

1. `load_note: NoteId → Result<Note, NoteNotFound>` — `NoteRepository::load_by_id` を呼び、`Ok(None)` を `NoteNotFound` に変換
2. `parse_body: String → NoteBody` — `NoteBody::from(String)` (infallible)
3. `compare_body: (&Note, &NoteBody) → BodyDiff` — `note.body() == new_body` で `Unchanged | Changed`
4. `branch_on_diff`:
   - `Unchanged` → `Ok(None)` 早期 return
   - `Changed` → step 5 へ
5. `update_body: (Note, NoteBody, Timestamp) → Note` — `note.edit_body(new_body, now)` (`Clock::now()` を `now` に注入)
6. `persist: &Note → Result<(), PersistError>` — `NoteRepository::write(&updated)`
7. `emit: &Note → ()` — `EventBus::publish(DomainEvent::NoteBodyEdited { note_id, updated_at })`

ステップ 1, 6, 7 で I/O が走る。ステップ 2-5 は副作用なし。

### BodyDiff の表現 {#impl-body-diff}

```rust
enum BodyDiff {
  Unchanged,
  Changed(NoteBody),
}
```

step 4 の分岐を型で表現することで、`Unchanged` 経路と `Changed` 経路の混線（例: `Unchanged` でも誤って `write` を呼ぶ等）を compile-time に防ぐ。private impl detail なので `pub` 不要。

### Tauri command surface {#impl-tauri}

- `#[tauri::command] async fn auto_save_note(state: State<AppState>, note_id: String, new_body: String) -> Result<Option<Note>, AutoSaveError>`
- `note_id: String` は frontend からの raw 文字列。`NoteId::try_from` 失敗時は `AutoSaveError::NoteNotFound { id: NoteId(raw) }` 相当に丸める（**TBD**: `InvalidNoteId` を別 variant にするか NoteNotFound に集約するかは impl phase で決める）
- 戻り値は `Result<Option<Note>, AutoSaveError>`。tauri-specta で TS bindings 生成

### debounce timer の境界 {#impl-debounce}

UI 層（frontend）で 500ms の debounce を実装し、debounce 成立後に **単発で** `auto_save_note` Tauri command を呼ぶ。本 slice の Rust 側 use case は debounce 自体を知らない。

- 利点: use case は stateless → test しやすい、Flush (`flush-note`) と use case を共有しやすい
- composition root（frontend hook 等）に `setTimeout` / `cancel` の責務が住む

### Out of scope {#out-of-scope}

- Flush 経路（focus 喪失 / blur / quit）— `flush-note` slice の責務
- DebounceTimer の実装（UI 層）
- tag の変更（`assign-tag` / `remove-tag` slice）
- create-note 経路（既存 `create-note` slice）
- `NoteRepository` の永続化フォーマット詳細（既存 `FsNoteRepository` を再利用）
- frontend の EDITING ↔ IDLE ステートマシン（Note Capture の UI 層、本 slice の外）

## Open Questions {#open-questions}

### NoteId parse 失敗時の variant {#oq-invalid-note-id}

- **status**: open
- **問題**: Tauri boundary で `note_id: String` を受け取り `NoteId::try_from` に失敗した場合、`AutoSaveError::NoteNotFound { id }` に丸めるか、別 variant `InvalidNoteId { raw: String }` を切るか
- **trade-off**:
  - 集約: error 表面が単純 (`{ NoteNotFound, PersistError }` の 2 variant 維持)
  - 分離: UI 側で「id が壊れている」と「存在しない」を区別できる（再試行可否の判断）
- **暫定**: impl phase で「frontend は常に valid な NoteId を送る」前提なら NoteNotFound に集約、不安なら別 variant。test-red で `TP-NF1` の expected error を決めるまでに確定する
- **影響**: `AutoSaveError` 定義 + Tauri command の戻り値型

### compareBody の `==` 比較で改行コード差を吸収するか {#oq-newline-normalization}

- **status**: open
- **問題**: macOS / Windows のクリップボード経由でペーストすると改行が `\r\n` ↔ `\n` に変わるケース。バイト等価では「差分あり」となり毎回 AutoSave が走る
- **trade-off**:
  - 厳格 (バイト等価): S9 の文言通り、シンプル
  - 寛容 (改行正規化): UX 上は「同じ内容」だが冪等性 guard が通る
- **暫定**: バイト等価で実装（domain workflow にも改行正規化の明示なし）。問題が顕在化したら domain 側 (S9 補足) に提案する
- **影響**: TP-I1 / TP-BC* の test 文言
