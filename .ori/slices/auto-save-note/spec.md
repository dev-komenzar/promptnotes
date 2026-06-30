---
coherence:
  source: derived
  last_derived: 2026-06-30
  upstream:
    - domain/workflows/auto-save-note.md#auto-save-note
    - domain/aggregates.md#note-aggregate
    - domain/bounded-contexts.md#note-capture
    - domain/domain-events.md#note-body-edited
    - domain/validation.md#s2-autosave-debounce
    - domain/validation.md#s9-idempotent-autosave
  hash:
    domain/workflows/auto-save-note.md#.*: 642c5094fd1a
    domain/aggregates.md#.*: 82947dbfd3f6
    domain/bounded-contexts.md#.*: 7ebfcda8743b
    domain/domain-events.md#.*: 8abdfac78084
    domain/validation.md#.*: 31244b277867
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

- `NoteRepository` — `.md` ファイルの **読み出し** (`load_by_id`) と書き出し (`write`) を提供。`storage_dir()` も持つ
- `Clock` — `now()` 取得（テスト時 injectable）
- `EventBus` — domain event の **同期** 発行（in-process）

`DebounceTimer` は本 slice の use case には注入されない（UI / composition root の責務）。

> domain/workflows/auto-save-note.md#dependencies は `DebounceTimer` を列挙するが、これは「workflow 全体の依存」であり、use case API のシグネチャ依存ではない。本 slice の use case (`execute`) は debounce 成立後の単発呼び出し。

### Output {#io-output}

戻り値: `Result<Option<Note>, AutoSaveError>`

- `Ok(Some(note))` — body が変化したケース。永続化 + event 発行
- `Ok(None)` — **no-op**（S9 冪等性ガードで早期 return、event 非発行）
- `Err(_)` — 4 variant のいずれか（[#io-errors](#io-errors) 参照）

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
  InvalidBody { source: NoteBodyError },
  LoadError { path: PathBuf, source: io::Error },
  PersistError { path: PathBuf, source: io::Error },
}
```

- **`NoteNotFound`** — `NoteRepository::load_by_id` が `Ok(None)` を返した場合
- **`InvalidBody`** — `NoteBody::new(new_body)` が失敗した場合（aggregates.md#note-aggregate-invariants の **I-N8** 違反、典型的には `---` 行を含む body）
- **`LoadError`** — `load_by_id` の read I/O 失敗 / 既存 `.md` ファイルの parse 失敗 (`io::ErrorKind::InvalidData`)
- **`PersistError`** — `NoteRepository::write` の I/O 失敗（**write 経路専用**に意味を絞る）

> domain/workflows/auto-save-note.md#notes より：「read 失敗（`LoadError`）と write 失敗（`PersistError`）は意味的に異なる経路として error variant を分離する」

## 不変条件 {#invariants}

slice 完了時に成立すべき条件。括弧内は domain での出典。

### Note Aggregate 由来 {#invariants-note-aggregate}

> domain/aggregates.md#note-aggregate-invariants より引用：

- **I-N1**: `id` は immutable。本 slice は `Note::edit_body` のみを呼び `id` には触れない
- **I-N3**: `updatedAt >= createdAt` を常に満たす。`Note::edit_body` 経由で `now` を渡すため、`now >= note.created_at` が `Clock` 契約として前提
- **I-N4**: `body` 変更時に `updatedAt = now`（秒精度）。`Note::edit_body` 内で保証される
  - 同一秒内の連続編集では `updatedAt` は同じ値に留まる（時計の解像度）
- **I-N8**: `body` の構築は `NoteBody::new` 経由でのみ可能であり、frontmatter delimiter 行 (`---`) を含まない。本 slice は `new_body: String` を `NoteBody::new` で smart construct するため、違反時は `AutoSaveError::InvalidBody` で表面化させる
- **I-N9**: `body_hash` は `body` から決定論的に導出され、`body` 変更時に必ず再計算される。外部プログラムによる `.md` ファイル変更の競合検出に使用される。本 slice は write 側（AutoSave）であり、競合検出（`is_stale` チェック）は application service 層の責務。AutoSave 発火時点ではユーザが編集中のため競合解決は不要（ユーザの編集が正）

### slice 固有制約 {#invariants-slice-specific}

- **C-AS1**: `note_id` で `NoteRepository::load_by_id` を呼ぶ。
  - 戻り値 `Ok(None)` → `AutoSaveError::NoteNotFound { id }`
  - 戻り値 `Err(io)` → `AutoSaveError::LoadError { path, source }`（read I/O 失敗、別 variant に分離）
- **C-AS2**: `new_body` 文字列を `NoteBody::new(String) -> Result<NoteBody, NoteBodyError>` で構築する（aggregate I-N8 由来の **fallible smart constructor**）。失敗時は `AutoSaveError::InvalidBody { source }` で表面化
- **C-AS3**: `existing_note.body() == new_body` の場合は **何もせず `Ok(None)` を返す**（S9 冪等性ガード、event 非発行）
  - 比較は `NoteBody` 値の `PartialEq` で行う（バイト等価）
  - body 以外（tags / timestamps）の差分は本 slice の判定対象外（タグ変更は `assign-tag` / `remove-tag` slice の責務）
- **C-AS4**: body が変化した場合のみ `Note::edit_body(new_body, now)` を呼ぶ
- **C-AS5**: `NoteRepository::write(&updated_note)` で永続化する。失敗時は `AutoSaveError::PersistError { path, cause }` を返し、event は **発行しない**
  - 失敗時 Note は in-memory には更新済みだが、永続化されていないため event を発行すべきでない（at-least-once 永続 → event の順）
- **C-AS6**: 永続化成功後、`EventBus::publish(DomainEvent::NoteBodyEdited { note_id, updated_at })` を **1 回だけ** 同期発行する
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

### NoteNotFound {#tp-not-found}

- **TP-NF1**: 存在しない `note_id` で発火 → `Err(AutoSaveError::NoteNotFound { id })`
- **TP-NF2**: TP-NF1 のケースで `NoteRepository::write` も `EventBus::publish` も呼ばれない
- **TP-NF3**: TP-NF1 で `id` フィールドは入力の `note_id` をそのまま返す

### LoadError {#tp-load-err}

- **TP-LE1**: `load_by_id` が `Err(PermissionDenied)` を返す → `Err(AutoSaveError::LoadError { path, source })`、`path == <storage_dir>/<id>.md`、`source.kind() == PermissionDenied`
- **TP-LE2**: TP-LE1 のケースで `write` も `publish` も呼ばれない
- **TP-LE3**: read 失敗は **`PersistError` には化けない**（C-AS1 / workflow#errors）

### InvalidBody {#tp-invalid-body}

- **TP-IB1**: `new_body` が `---`（frontmatter delimiter 行）を含む → `Err(AutoSaveError::InvalidBody { source: ContainsFrontmatterDelimiter })`（I-N8）
- **TP-IB2**: TP-IB1 のケースで `write` も `publish` も呼ばれない

### PersistError {#tp-persist-err}

- **TP-PE1**: 既存 Note A に body 変化ありで発火、`NoteRepository::write` が `io::Error::PermissionDenied` を返す → `Err(AutoSaveError::PersistError { path, cause })`
- **TP-PE2**: TP-PE1 で `cause.kind() == ErrorKind::PermissionDenied`
- **TP-PE3**: TP-PE1 で `EventBus::publish` が **呼ばれない**（C-AS5: 永続化失敗時は event 非発行）
- **TP-PE4**: TP-PE1 後に同じ command を再発火（fs が回復済）→ `Ok(Some(updated))` 返る。use case の stateless 性確認

### body 比較の詳細 {#tp-body-compare}

- **TP-BC1**: `body=""`（空文字）の Note に `new_body=""` → `Ok(None)`（空文字も冪等性対象）
- **TP-BC2**: `body="hello"` に `new_body="hello "` (末尾スペース) → `Ok(Some(updated))`（バイト等価ではない、I-N4 経路）
- **TP-BC3**: `body="hello"` に `new_body="Hello"` (case 違い) → `Ok(Some(updated))`（case-sensitive 比較）

### 不変条件チェック {#tp-invariants}

- **TP-INV1**: 任意の TP-H* の戻り値で `updated.id == input.note_id`（I-N1）
- **TP-INV2**: 任意の TP-H* の戻り値で `updated.updated_at >= updated.created_at`（I-N3）

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
│   ├── events.rs           # DomainEvent::NoteBodyEdited を追加
│   └── ports.rs            # NoteRepository::load_by_id を追加
└── slices/
    ├── create_note/        # 既存
    └── auto_save_note/     # 新規
        ├── mod.rs          # pub use commands::*
        ├── domain.rs       # AutoSaveNoteCommand, AutoSaveError (4 variant)
        ├── application.rs  # AutoSaveNoteUseCase: load → parse → compare → edit → persist → emit
        ├── infrastructure.rs  # (必要なら) FsNoteRepository への load_by_id 拡張
        ├── commands.rs     # #[tauri::command] auto_save_note → tauri-specta surface
        └── tests.rs        # unit tests for TP-* (in-memory NoteRepository mock)
```

### 既存 port / event の拡張 {#impl-port-extension}

本 slice の前提として、`note_capture/shared/` に 2 つの拡張が必要：

- **`NoteRepository::load_by_id(&self, id: &NoteId) -> std::io::Result<Option<Note>>`** — 新規追加。`.md` ファイルから Note を復元する経路。create_note slice は write only だったため未定義。FsNoteRepository 側で frontmatter parser を実装し、再構築は **`Note::from_persisted` (aggregates.md#note-aggregate-commands の新規 command)** 経由で行う
- **`DomainEvent::NoteBodyEdited { note_id, updated_at }`** — 既存 enum に variant 追加

両者とも他 slice (`flush-note`, `assign-tag` 等) でも再利用される shared 拡張。本 slice の作業に含めるが、**shared layer の変更は load-settings / create-note の test 群に影響しない**ことを test-red phase で確認する（既存テスト GREEN 維持）。

### Pipeline ステップ {#impl-pipeline}

> domain/workflows/auto-save-note.md#steps の DMMF pipeline を採用：

1. `load_note: NoteId → Result<Note, AutoSaveError>` — `NoteRepository::load_by_id` を呼び、`Ok(None)` を `NoteNotFound`、`Err(io)` を `LoadError` に変換
2. `parse_body: String → Result<NoteBody, AutoSaveError>` — `NoteBody::new(String)` (**aggregate I-N8 由来の fallible smart constructor**)、失敗時は `InvalidBody`
3. `compare_body: (&Note, &NoteBody) → BodyDiff` — `note.body() == new_body` で `Unchanged | Changed`
4. `branch_on_diff`:
   - `Unchanged` → `Ok(None)` 早期 return
   - `Changed` → step 5 へ
5. `update_body: (Note, NoteBody, Timestamp) → Note` — `note.edit_body(new_body, now)` (`Clock::now()` を `now` に注入)
6. `persist: &Note → Result<(), AutoSaveError>` — `NoteRepository::write(&updated)`、失敗時は `PersistError`
7. `emit: &Note → ()` — `EventBus::publish(DomainEvent::NoteBodyEdited { note_id, updated_at })`

ステップ 1, 2, 6 で fallible（4 variant の error 表面化）、6, 7 で I/O が走る。ステップ 3-5 は副作用なし。

### BodyDiff の表現 {#impl-body-diff}

```rust
enum BodyDiff {
  Unchanged,
  Changed(NoteBody),
}
```

step 4 の分岐を型で表現することで、`Unchanged` 経路と `Changed` 経路の混線（例: `Unchanged` でも誤って `write` を呼ぶ等）を compile-time に防ぐ。private impl detail なので `pub` 不要。

### Tauri command surface {#impl-tauri}

- `#[tauri::command] async fn auto_save_note(state: State<AppState>, note_id: String, new_body: String) -> Result<AutoSaveOutcome, AutoSaveErrorDto>`
- 戻り値は **DTO 経由**で frontend に渡す（4 variant を serde で安定化）
- `note_id: String` は frontend からの raw 文字列。`NoteId::try_from` 失敗時の扱いは [#oq-invalid-note-id](#oq-invalid-note-id) で議論中
- tauri-specta で TS bindings 生成

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
- **問題**: Tauri boundary で `note_id: String` を受け取り `NoteId::try_from` に失敗した場合の error variant 選択。`AutoSaveError` 自体は 4 variant 確定（workflow upstream 反映済）だが、`NoteId` 構造体に smart constructor が無く、frontend から壊れた id が来た場合の振る舞いが未確定
- **trade-off**:
  - sentinel epoch + `NoteNotFound`（現 impl）: error 表面は workflow#errors と一致、UI 側で「id 不正」と「存在しない」を区別不能
  - `InvalidNoteId { raw: String }` を別 variant に追加: workflow#errors を 5 variant に再拡張する追加 proposal が必要、UI 側で区別可能
  - aggregates.md#note-aggregate-elements で `NoteId::try_from(&str)` を smart constructor として明文化: 上流から正す王道、追加 proposal が必要
- **暫定**: 現 impl は sentinel epoch で `NoteNotFound` ルートに降格。問題顕在化後に proposal を作る
- **影響**: `AutoSaveError` の variant 数 + Tauri command boundary のエラーマッピング

### compareBody の `==` 比較で改行コード差を吸収するか {#oq-newline-normalization}

- **status**: open
- **問題**: macOS / Windows のクリップボード経由でペーストすると改行が `\r\n` ↔ `\n` に変わるケース。バイト等価では「差分あり」となり毎回 AutoSave が走る
- **trade-off**:
  - 厳格 (バイト等価): S9 の文言通り、シンプル
  - 寛容 (改行正規化): UX 上は「同じ内容」だが冪等性 guard が通る
- **暫定**: バイト等価で実装（domain workflow にも改行正規化の明示なし）。問題が顕在化したら domain 側 (S9 補足) に提案する
- **影響**: TP-I1 / TP-BC* の test 文言
