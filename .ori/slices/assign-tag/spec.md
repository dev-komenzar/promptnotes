---
coherence:
  source: derived
  last_derived: 2026-06-28
  upstream:
    - domain/workflows/assign-tag.md#assign-tag
    - domain/aggregates.md#note-aggregate
    - domain/bounded-contexts.md#note-capture
    - domain/domain-events.md#note-tags-changed
    - domain/validation.md#s4-tag-assign-normalize
    - domain/validation.md#s10-tag-invalid-char
  hash:
    domain/workflows/assign-tag.md#.*: 4efe2dfe63c4
    domain/aggregates.md#.*: 991ebe2e34f1
    domain/bounded-contexts.md#.*: 4d579125a513
    domain/domain-events.md#.*: 8abdfac78084
    domain/validation.md#.*: 5294b0c32f1b
ori:
  schema:
    propagation_level: file
---

# assign-tag spec {#assign-tag-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive assign-tag`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

Note にタグを付与する slice。ユーザが入力した raw tag 文字列を `Tag::new` で正規化 (lowercase + trim) し、禁止文字 (` `, `\t`, `\n`, `,`, `[`, `]`) を含む場合は構築時点で reject する。TagSet に既存タグが含まれていれば no-op、新規追加された場合のみ永続化と `NoteTagsChanged` event 発行を行う。Note Capture BC の Tag 編集経路（domain/bounded-contexts.md#note-capture-ubiquitous-language の "Tag" 用語に対応）を実装する。

> domain/workflows/assign-tag.md#assign-tag より：「Note にタグを付与する。正規化後の重複は no-op、禁止文字は reject。」
>
> domain/bounded-contexts.md#note-capture-subdomain-type より：本 BC は **core subdomain**。`.md` ファイル所有性と frontmatter 整合性がここに集約される。

本 slice は **application service レベルで TagDiff 判定** を行う。`Tag::new` で正規化された tag が既に TagSet に存在する場合（同一 `name` の Tag が居る場合）は no-op で event を発行しない（S4 重複排除）。`Note::assign_tag` 自体は呼ばれれば I-N5 に従い後勝ち無視で no-op となるが、application service 側で「Added か Unchanged か」を判定して永続化／event 発行を分岐させる。

> domain/validation.md#s4-tag-assign-normalize より：「TagSet に同一 `name` が既存（I-N5）→ assign は **no-op**、event NoteTagsChanged は発行しない（変化がないため）」

`parseTag` は Note を load する前にバリデーション可能（workflow#notes）。禁止文字検出は Note 不在でも早期 reject できる。本 slice は AutoSave / Flush とは独立の write 経路で、`tags` のみを変更する（C-AT8）。

## 入出力 {#io}

### Input {#io-input}

> domain/workflows/assign-tag.md#input より：

```rust
struct AssignTagCommand {
  note_id: NoteId,
  raw_tag: String,    // ユーザ入力（未正規化）
}
```

依存（外部から注入される interface）:

- `NoteRepository` — `.md` ファイルの **読み出し** (`load_by_id`) と書き出し (`write`) を提供。`storage_dir()` も持つ（auto-save-note で拡張済み）
- `Clock` — `now()` 取得（テスト時 injectable、`Note::assign_tag` の `updated_at` 更新で使用）
- `EventBus` — domain event の **同期** 発行（in-process）

### Output {#io-output}

戻り値: `Result<Option<Note>, AssignTagError>`

- `Ok(Some(note))` — TagSet が変化したケース。永続化 + event 発行
- `Ok(None)` — **no-op**（S4 重複排除で早期 return、event 非発行）
- `Err(_)` — 4 variant のいずれか（[#io-errors](#io-errors) 参照）

成功時 (`Ok(Some(note))`)：

- 戻り値: 更新後の Note Aggregate
- 発行 event: [`NoteTagsChanged`](../../domain/domain-events.md#note-tags-changed)

> domain/domain-events.md#note-tags-changed-payload より：

```rust
struct NoteTagsChanged {
  note_id: NoteId,
  tags: TagSet,
  updated_at: Timestamp,
}
```

`NoteTagsChanged` は他の event と異なり **TagSet 自体を payload に載せる**（domain-events.md#notes より「TagSet 自体が『変化点』なので payload に含む」）。

### Errors {#io-errors}

> domain/workflows/assign-tag.md#errors より：

```rust
enum AssignTagError {
  NoteNotFound { id: NoteId },
  InvalidTag { name: String, reason: TagError },
  LoadError { path: PathBuf, source: io::Error },
  PersistError { path: PathBuf, source: io::Error },
}
```

- **`NoteNotFound`** — `NoteRepository::load_by_id` が `Ok(None)` を返した場合
- **`InvalidTag`** — `Tag::new(raw_tag)` が失敗した場合（aggregates.md#note-aggregate-invariants の **I-N6** 違反）。`TagError` は workflow#errors では `EmptyAfterTrim | InvalidChar(char)`、impl 既存では `Empty | InvalidChar { raw }` という variant 名ずれがある（[#oq-tag-new-signature](#oq-tag-new-signature)）。`name` フィールドは入力 raw 文字列を返す
- **`LoadError`** — `load_by_id` の read I/O 失敗 / 既存 `.md` ファイルの parse 失敗 (`io::ErrorKind::InvalidData`)
- **`PersistError`** — `NoteRepository::write` の I/O 失敗（**write 経路専用**に意味を絞る）

> auto-save-note slice の error 分離方針（LoadError / PersistError）を本 slice にも同形適用。auto-save-note finalize 時の cross_slice_followups で示唆された方針を踏襲。

## 不変条件 {#invariants}

slice 完了時に成立すべき条件。括弧内は domain での出典。

### Note Aggregate 由来 {#invariants-note-aggregate}

> domain/aggregates.md#note-aggregate-invariants より引用：

- **I-N1**: `id` は immutable。本 slice は `Note::assign_tag` のみを呼び `id` には触れない
- **I-N3**: `updatedAt >= createdAt` を常に満たす。`Note::assign_tag` で `now` を渡すため、`now >= note.created_at` が `Clock` 契約として前提
- **I-N5**: `tags` 内に同一 `Tag::name` は 1 件のみ。本 slice は `Tag::new` 正規化後の `name` で重複検出する
- **I-N6**: `Tag::name` は正規化規則（lowercase + trim、禁止文字排除）を必ず満たす。`Tag::new` が construction-time に enforce、違反時は `AssignTagError::InvalidTag` で表面化

### slice 固有制約 {#invariants-slice-specific}

- **C-AT1**: `raw_tag` 文字列を `Tag::new(&str) -> Result<Tag, TagError>` で構築する（aggregate I-N6 由来の fallible smart constructor）。失敗時は `AssignTagError::InvalidTag { name: raw_tag, reason }` で表面化。**Note load より先に行う**（workflow#notes に従い早期 reject）
- **C-AT2**: `note_id` で `NoteRepository::load_by_id` を呼ぶ。
  - 戻り値 `Ok(None)` → `AssignTagError::NoteNotFound { id }`
  - 戻り値 `Err(io)` → `AssignTagError::LoadError { path, source }`（read I/O 失敗、別 variant に分離）
- **C-AT3**: 既存 `note.tags()` に **`Tag::new` 正規化後の `tag.name()` と一致する Tag が含まれる** 場合は **何もせず `Ok(None)` を返す**（S4 重複排除、event 非発行）
  - 比較は `Tag::name()` 文字列等価（正規化済みの lowercase + trim 結果が比較対象）
  - body / timestamps の差分は本 slice の判定対象外
- **C-AT4**: TagSet が変化する場合のみ `Note::assign_tag(tag, now)` を呼ぶ。注: 現状の `Note` aggregate には `assign_tag` 操作が未実装、phase 4 で aggregate に追加する（[#oq-assign-tag-now-injection](#oq-assign-tag-now-injection)）。`updated_at = now` で更新
- **C-AT5**: `NoteRepository::write(&updated_note)` で永続化する。失敗時は `AssignTagError::PersistError { path, cause }` を返し、event は **発行しない**
  - 永続化 → event の順序を守る（at-least-once 永続）
- **C-AT6**: 永続化成功後、`EventBus::publish(DomainEvent::NoteTagsChanged { note_id, tags, updated_at })` を **1 回だけ** 同期発行する。`tags` には更新後の TagSet 全体を載せる（domain-events.md#note-tags-changed-payload）
- **C-AT7**: use case は **stateless**。連続 assign の de-duplicate は TagSet 自身の集合演算で実現する（debounce や session state を持たない）
- **C-AT8**: 本 slice は `body` を変更しない（AutoSave / Flush 経路の責務）。`updated_at` のみ更新される

### 経路境界 {#invariants-boundary}

- **C-AT9**: assign-tag **と** remove-tag は同じ「`Note` を載せ替え + `NoteTagsChanged` 発行」契約を共有するが、本 slice は **assign 経路のみ**を実装する。remove は別 slice (`remove-tag`) の責務
  > domain/domain-events.md#note-tags-changed-trigger より：「`Note::assign_tag(tag)` または `Note::remove_tag(tag_name)` の永続化成功時」
- **C-AT10**: 本 slice は AutoSave / Flush（`NoteBodyEdited` 経路）と完全に独立。Note の異なる field に作用するため event の混線も起きない

## テスト観点 {#test-perspectives}

phase 3 で failing test に展開する観点を列挙。各観点は 1 つ以上の test に対応する想定。

### happy path: TagSet 変化あり {#tp-happy}

- **TP-H1**: 既存 Note A (`tags=["gpt"]`, `updated_at=t0`) に対し `AssignTagCommand { note_id: A.id, raw_tag: "coding" }` を発火 → `Ok(Some(updated))`、`updated.tags().as_slice()` が `["gpt", "coding"]` 相当、`updated.updated_at == t1` (`Clock::now()` の戻り値)
- **TP-H2**: TP-H1 のケースで `NoteRepository::write(&updated)` が 1 回呼ばれる
- **TP-H3**: TP-H1 のケースで `EventBus::publish(NoteTagsChanged { note_id: A.id, tags: updated.tags(), updated_at: t1 })` が **1 回だけ** 呼ばれる
- **TP-H4**: `updated.body == A.body`、`updated.created_at == A.created_at`、`updated.id == A.id`（C-AT8、I-N1）
- **TP-H5**: 空 TagSet (`tags=[]`) の Note に `raw_tag: "gpt"` を発火 → `Ok(Some(updated))`、`updated.tags` は `["gpt"]`

### S4: 正規化と重複排除（no-op） {#tp-normalize-dedupe}

- **TP-N1**: 既存 Note A (`tags=["gpt"]`) に `raw_tag: "  GPT  "` を発火 → `Tag::new` が `"gpt"` に正規化（trim + lowercase）→ TagSet に既存 → `Ok(None)`
- **TP-N2**: TP-N1 のケースで `NoteRepository::write` が **呼ばれない**（C-AT3）
- **TP-N3**: TP-N1 のケースで `EventBus::publish` が **呼ばれない**（S4）
- **TP-N4**: case 違いの dedupe: `tags=["gpt"]` に `raw_tag: "GPT"` → `Ok(None)`
- **TP-N5**: 前後空白だけの差: `tags=["gpt"]` に `raw_tag: " gpt "` → `Ok(None)`

### S10: 禁止文字 reject（InvalidTag） {#tp-invalid-char}

- **TP-IC1**: `raw_tag: "foo,bar"`（カンマ含む）→ `Err(AssignTagError::InvalidTag { name: "foo,bar", reason: TagError::InvalidChar { raw: "foo,bar" } })`（I-N6、S10）
- **TP-IC2**: TP-IC1 のケースで `NoteRepository::load_by_id` も `write` も `publish` も呼ばれない（C-AT1 が早期 reject）
- **TP-IC3**: 他の禁止文字でも reject される: `raw_tag` がスペース内包 (`"foo bar"`)、改行 (`"foo\nbar"`)、`[`、`]` でも `InvalidChar`
- **TP-IC4**: trim 後に空文字 (`raw_tag: "   "`) → `Err(AssignTagError::InvalidTag { name: "   ", reason: TagError::Empty })`（impl 既存の variant 名）

### NoteNotFound {#tp-not-found}

- **TP-NF1**: 存在しない `note_id` で `raw_tag: "gpt"` を発火 → `Err(AssignTagError::NoteNotFound { id })`
- **TP-NF2**: TP-NF1 のケースで `NoteRepository::write` も `EventBus::publish` も呼ばれない
- **TP-NF3**: TP-NF1 で `id` フィールドは入力の `note_id` をそのまま返す

### LoadError {#tp-load-err}

- **TP-LE1**: `load_by_id` が `Err(PermissionDenied)` を返す → `Err(AssignTagError::LoadError { path, source })`、`path == <storage_dir>/<id>.md`、`source.kind() == PermissionDenied`
- **TP-LE2**: TP-LE1 のケースで `write` も `publish` も呼ばれない
- **TP-LE3**: read 失敗は **`PersistError` には化けない**（C-AT2 / auto-save-note と同形）

### PersistError {#tp-persist-err}

- **TP-PE1**: 既存 Note A に新規タグを assign、`NoteRepository::write` が `io::Error::PermissionDenied` を返す → `Err(AssignTagError::PersistError { path, cause })`
- **TP-PE2**: TP-PE1 で `cause.kind() == ErrorKind::PermissionDenied`
- **TP-PE3**: TP-PE1 で `EventBus::publish` が **呼ばれない**（C-AT5: 永続化失敗時は event 非発行）
- **TP-PE4**: TP-PE1 後に同じ command を再発火（fs が回復済）→ `Ok(Some(updated))` 返る。use case の stateless 性確認

### 不変条件チェック {#tp-invariants}

- **TP-INV1**: 任意の TP-H* の戻り値で `updated.id == input.note_id`（I-N1）
- **TP-INV2**: 任意の TP-H* の戻り値で `updated.updated_at >= updated.created_at`（I-N3）
- **TP-INV3**: 任意の TP-H* の戻り値で `updated.tags()` 内に同一 `name` が 2 件以上存在しない（I-N5）
- **TP-INV4**: 任意の TP-H* の戻り値で TagSet の各要素が正規化済み（lowercase + 禁止文字なし、I-N6）

### no-op 契約 {#tp-api-shape}

- **TP-AS1**: `AssignTagUseCase::execute(&self, cmd) -> Result<Option<Note>, AssignTagError>` のシグネチャを **type-level pin**（compile-time assertion）
- **TP-AS2**: `body` フィールドが副作用で変化しないことを TP-H4 の延長で確認

## 実装ノート {#impl-notes}

### アーキ層への落とし込み {#impl-layers}

DDD-VSA-Hex / typescript-tauri の階層に従い、本 slice は Rust 側で実装する（`implementation.language: rust`）。

```
apps/promptnotes/src-tauri/src/note_capture/
├── mod.rs                  # pub use slices::*; shared::*
├── shared/
│   ├── events.rs           # DomainEvent::NoteTagsChanged を追加（既存 enum に variant 追加）
│   ├── ports.rs            # 既存（NoteRepository::load_by_id は auto-save-note で導入済）
│   └── types/
│       └── note.rs         # Note::assign_tag(self, Tag, Timestamp) -> Note を追加
└── slices/
    ├── auto_save_note/     # 既存
    ├── copy_note_body/     # 既存
    ├── create_note/        # 既存
    └── assign_tag/         # 新規
        ├── mod.rs          # pub use commands::*
        ├── domain.rs       # AssignTagCommand, AssignTagError (4 variant)
        ├── application.rs  # AssignTagUseCase: parse → load → diff → assign → persist → emit
        ├── commands.rs     # #[tauri::command] assign_tag → tauri-specta surface
        └── tests.rs        # unit tests for TP-* (in-memory NoteRepository mock)
```

### 既存 port / event / aggregate の拡張 {#impl-extension}

本 slice の前提として、note_capture/shared/ に 2 つの拡張、aggregate に 1 操作の追加が必要：

- **`Note::assign_tag(self, tag: Tag, now: Timestamp) -> Note`** — 新規追加。既存 TagSet に同一 `name` の Tag があれば後勝ち無視で no-op、なければ insertion-order 末尾追加（domain/aggregates.md#note-aggregate-commands に従う）。`updated_at = now` で更新（I-N5、I-N6 は `Tag` 構築時点で保証済みのため aggregate 側は再検証不要）。**ただし application service 側で TagDiff を判定するため、aggregate の no-op 判定だけでは event 発行制御に不十分**。signature が aggregate doc と異なる点は [#oq-assign-tag-now-injection](#oq-assign-tag-now-injection) で扱う
- **`DomainEvent::NoteTagsChanged { note_id, tags, updated_at }`** — 既存 enum に variant 追加。payload は domain-events.md#note-tags-changed-payload に従い TagSet 自体を載せる

両者とも他 slice (`remove-tag` 等) でも再利用される shared 拡張。本 slice の作業に含めるが、**shared layer の変更は既存 slice の test 群に影響しない**ことを test-red phase で確認する（既存テスト GREEN 維持）。

### Pipeline ステップ {#impl-pipeline}

> domain/workflows/assign-tag.md#steps の DMMF pipeline を採用：

1. `parse_tag: String → Result<Tag, AssignTagError>` — `Tag::new(&raw_tag)` を呼ぶ。失敗時は `InvalidTag { name: raw_tag, reason }` に map（**Note load より先に行う**、workflow#notes 準拠）
2. `load_note: NoteId → Result<Note, AssignTagError>` — `NoteRepository::load_by_id` を呼び、`Ok(None)` を `NoteNotFound`、`Err(io)` を `LoadError` に変換
3. `compute_diff: (&Note, &Tag) → TagDiff` — `note.tags().as_slice().iter().any(|t| t.name() == tag.name())` で判定。既存なら `Unchanged`、なければ `Added`
4. `branch_on_diff`:
   - `Unchanged` → `Ok(None)` 早期 return
   - `Added` → step 5 へ
5. `apply_assign: (Note, Tag, Timestamp) → Note` — `note.assign_tag(tag, now)` を呼ぶ
6. `persist: &Note → Result<(), AssignTagError>` — `NoteRepository::write(&updated)`、失敗時は `PersistError`
7. `emit: &Note → ()` — `EventBus::publish(DomainEvent::NoteTagsChanged { note_id, tags: updated.tags().clone(), updated_at })`

ステップ 1, 2, 6 で fallible（4 variant の error 表面化）、6, 7 で I/O が走る。ステップ 3-5 は副作用なし。

### TagDiff の表現 {#impl-tag-diff}

```rust
enum TagDiff {
  Unchanged,
  Added(Tag),
}
```

step 4 の分岐を型で表現することで、`Unchanged` 経路と `Added` 経路の混線（例: `Unchanged` でも誤って `write` を呼ぶ等）を compile-time に防ぐ。private impl detail なので `pub` 不要。

### Tauri command surface {#impl-tauri}

- `#[tauri::command] async fn assign_tag(state: State<AppState>, note_id: String, raw_tag: String) -> Result<AssignTagOutcome, AssignTagErrorDto>`
- 戻り値は **DTO 経由**で frontend に渡す（4 variant を serde で安定化）
- `note_id: String` は frontend からの raw 文字列。`NoteId::try_from` 失敗時の扱いは auto-save-note と同形（[#oq-invalid-note-id-reuse](#oq-invalid-note-id-reuse)）
- tauri-specta で TS bindings 生成

### UI 境界 {#impl-ui}

タグ入力 UI（メタ行のテキストフィールド）で Enter / blur 確定時に `assign_tag` Tauri command を呼ぶ。本 slice の Rust 側 use case は UI state を知らない。

- 利点: use case は stateless → test しやすい、remove-tag (`remove_tag`) と use case を分離しやすい
- UI 側は `InvalidTag` を受けたらエラーメッセージ表示（S10、domain/ui-fields にて確定済）

### Out of scope {#out-of-scope}

- remove-tag 経路（× クリック）— `remove-tag` slice の責務
- AutoSave / Flush 経路（`NoteBodyEdited`）— 別 slice
- create-note 経路（既存 `create-note` slice）
- `NoteRepository` の永続化フォーマット詳細（既存 `FsNoteRepository` を再利用、frontmatter parser は auto-save-note で導入済）
- frontend の Tag 入力 UI（メタ行ステートマシン、本 slice の外）
- Note Feed の TagFilter 再計算（NoteTagsChanged 購読側、Note Feed BC の責務）

## Open Questions {#open-questions}

### NoteId parse 失敗時の variant 再利用 {#oq-invalid-note-id-reuse}

- **status**: open (auto-save-note の OQ と同根)
- **問題**: Tauri boundary で `note_id: String` を受け取り `NoteId::try_from` に失敗した場合の error variant 選択。`AssignTagError` 自体は 4 variant 確定（workflow upstream 反映済）だが、`NoteId` 構造体に smart constructor が無く、frontend から壊れた id が来た場合の振る舞いが未確定
- **trade-off / 暫定**: auto-save-note と同形で sentinel epoch → `NoteNotFound` ルートに降格。問題顕在化後に上流 proposal を作る（`NoteId::try_from(&str)` を smart constructor として aggregates.md#note-aggregate-elements に追記）
- **影響**: `AssignTagError` の variant 数 + Tauri command boundary のエラーマッピング

### Note::assign_tag の now 注入方式 {#oq-assign-tag-now-injection}

- **status**: open
- **問題**: domain/aggregates.md#note-aggregate-commands の現行記述は `Note::assign_tag(self, tag: Tag) -> Note` で `now` を受け取らない。一方で I-N4 / I-N3 を満たすため `updated_at` の更新には `now` 注入が必須
- **trade-off**:
  - signature を `Note::assign_tag(self, tag: Tag, now: Timestamp) -> Note` に変更: 明示的、`create` / `edit_body` と同形（aggregates 更新 proposal が必要）
  - aggregate 内で `Clock` を保持: aggregate が時計依存を持つことになり pure ではなくなる（不採用）
  - application service 側で touch(now) 相当の別操作を介在: aggregate API が散らかる（不採用）
- **暫定**: phase 4 実装時に `Note::assign_tag(self, tag: Tag, now: Timestamp) -> Note` で実装し、phase 6 review 時に上流 proposal を作成する想定（auto-save-note の `edit_body(new_body, now)` と同形）
- **影響**: aggregate API の signature + workflow#steps の文言（`Note::assign_tag(tag)` → `Note::assign_tag(tag, now)`）

### Tag::new variant 名のずれ {#oq-tag-new-signature}

- **status**: open
- **問題**: domain/workflows/assign-tag.md#errors は `TagError = EmptyAfterTrim | InvalidChar(char)` と書くが、impl 既存の `TagError` は `Empty | InvalidChar { raw: String }` で variant 名と payload 形が異なる
- **trade-off**:
  - domain 側を impl 既存 (`Empty | InvalidChar { raw }`) に揃える: proposal が必要、UX 側に影響なし
  - impl 側を domain 文言 (`EmptyAfterTrim | InvalidChar(char)`) にリネーム: 既存呼び出し全箇所に波及
- **暫定**: phase 4 では既存 impl の variant 名を維持し、phase 6 で domain 側に proposal を作る（変更影響範囲が小さい方向）
- **影響**: `AssignTagError::InvalidTag { reason }` の Display 実装、workflow#errors の文言
