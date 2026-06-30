---
coherence:
  source: derived
  last_derived: 2026-06-24
  upstream:
    - domain/workflows/create-note.md#create-note
    - domain/aggregates.md#note-aggregate
    - domain/domain-events.md#note-created
    - domain/bounded-contexts.md#note-capture
    - domain/ui-fields/screen-1.md#fields
  hash:
    domain/workflows/create-note.md#.*: 33a51fe91246
    domain/aggregates.md#.*: 37fa7433eab4
    domain/domain-events.md#.*: 8abdfac78084
    domain/bounded-contexts.md#.*: 4d579125a513
    domain/ui-fields/screen-1.md#.*: ddbf06d54f9a
ori:
  schema:
    propagation_level: file
---

# create-note spec {#create-note-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive create-note`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

Draft 入力欄に書かれた本文を Cmd+Enter で確定し、新規 `Note` として `.md` ファイルに永続化する slice。Note Capture BC の **唯一の起案経路**であり、Note Aggregate のコンストラクタ `Note::create` を呼び出す入口。

> domain/workflows/create-note.md#create-note より：「Draft 入力欄に入力された本文を Cmd+Enter で確定し、新規 Note として永続化する」
>
> domain/bounded-contexts.md#note-capture より：本 BC は **core subdomain**。PromptNotes でなければ実現できない差別化領域（`.md` ファイル所有性、frontmatter/タグ除外コピー）の起点。

対応 UI（screen-1.md#fields-draft）:

- `#screen-1-draft-body` — CodeMirror 6 の Draft 本文入力欄（`Cmd+N` focus / `Cmd+Enter` 確定）
- `#screen-1-draft-submit` — `Cmd+Enter` と等価の `＋追加` ボタン

確定後の UI 副作用は別 layer の責務（[#out-of-scope](#out-of-scope)）。

## 入出力 {#io}

### Input {#io-input}

> domain/workflows/create-note.md#input より：

```rust
struct CreateNoteCommand {
  raw_body: String,        // Draft 入力欄の生テキスト
  raw_tags: Vec<String>,   // 初期タグ（Draft 入力時に指定可能）
}
```

依存（外部から注入される interface）:

- `NoteRepository` — `.md` ファイルの書き出し（frontmatter + body）
- `Clock` — `now()` 取得（テスト時 injectable）
- `EventBus` — domain event の **同期** 発行（in-process）

### Output {#io-output}

戻り値: `Result<Option<Note>, Error>`

- `Ok(Some(note))` — 通常成功（`.md` 書き出し + event 発行）
- `Ok(None)` — **no-op**（空 body のため何も起こさない、C-CN3）
- `Err(_)` — `InvalidTag` / `PersistError`

成功時 (`Ok(Some(note))`)：

- 戻り値: 生成された Note Aggregate
- 発行 event: [`NoteCreated`](../../domain/domain-events.md#note-created)

> domain/domain-events.md#note-created-payload より：

```rust
struct NoteCreated {
  note_id: NoteId,
  created_at: Timestamp,
  initial_tags: TagSet,
}
```

> domain/domain-events.md#note-created-timing より：同期発行。ordering 要件なし。

### Errors {#io-errors}

> domain/workflows/create-note.md#errors より：

- `InvalidTag { raw: String, source: TagError }` — タグ正規化 / 禁止文字違反（I-N6）
- `InvalidBody { source: NoteBodyError }` — `raw_body` が NoteBody の domain 不変条件に違反（現状は `---` 単独行の禁止のみ。domain/aggregates.md#note-aggregate-elements より「frontmatter 由来の `---` を含まない」）
- `PersistError { path: PathBuf, source: io::Error }` — `.md` 書き出し失敗

> 上記 3 種は domain/workflows/create-note.md#errors の `InvalidTag` / `PersistError` に加え、Pass 1 review で発覚した NoteBody validation の error path（`spec.md#open-questions#oq-notebody-validation-surface`）を spec として明示化したもの。

## 不変条件 {#invariants}

slice 完了時に成立すべき条件。括弧内は domain での出典。

### Note Aggregate 由来 {#invariants-note-aggregate}

> domain/aggregates.md#note-aggregate-invariants より引用 / 関連項のみ列挙：

- **I-N1**: `id` は immutable。`Note::create` 後に書き換え不可
- **I-N2**: `createdAt` は `id` の `YYYYMMDDhhmmss` parse 結果と一致
- **I-N3**: `updatedAt >= createdAt`（create 直後は等号成立）
- **I-N5**: `tags` 内に同一 `Tag::name` は 1 件のみ
- **I-N6**: `Tag::name` は正規化規則（lowercase + trim、禁止文字 ` \t\n,[]` 排除）を必ず満たす

I-N4（edit_body の updatedAt 更新）と I-N7（DeletedNote stack）は本 slice の範囲外。

### slice 固有制約 {#invariants-slice-specific}

- **C-CN1**: `id = now.format("YYYYMMDDhhmmss")`。秒精度。`createdAt = updatedAt = now`
- **C-CN2**: `Note::create` の確定経路は **本 slice のみ**。AutoSave / Flush 経路では新規作成しない（domain/workflows/create-note.md#notes より）
- **C-CN3**: **空 / whitespace のみの `raw_body` は no-op として扱い、Note を作成しない**（戻り値 `Ok(None)`、event 非発行、副作用なし）。Cmd+Enter 連打時の重複防止と Empty Note 防止を兼ねる。
  - **判定規則**: `raw_body.trim().is_empty()` （前後の空白除去後に空文字列なら no-op）
  - **責務**: presentation 層 (UI) と command layer の defense-in-depth で両方判定
  - **upstream との関係**: domain/workflows/create-note.md#notes は「空 raw_body でも作成を許容」と記述するが、これは aggregate が拒否しないという意味であり、本 slice の command boundary では reject (no-op)。要 upstream 修正提案（[#open-questions](#open-questions)）
- **C-CN4**: 永続化（`.md` 書き出し）が成功して初めて `NoteCreated` を発行する（domain/domain-events.md#note-created-trigger より「永続化成功時」）。Persist 失敗時は event 非発行 + `PersistError` を返す
- **C-CN5**: 永続化先ファイル名は `<note_id>.md`（NoteId が basename = filename と 1:1。domain/aggregates.md#note-aggregate-elements より）
- **C-CN6**: 同一秒内 id 衝突は **構造的に発生しない**（C-CN3 により Cmd+Enter 連打の 2 回目以降は raw_body が空のため no-op になる、domain/ui-fields/screen-1.md#cross-draft-submit「入力欄を即時クリア」より）

## テスト観点 {#test-perspectives}

phase 3 で failing test に展開する観点を列挙。各観点は 1 つ以上の test に対応する想定。

### happy path {#tp-happy}

- **TP-H1**: `raw_body="hello"`, `raw_tags=[]`, `now=2026-06-24T10:00:00` で実行
  - 戻り値 `Note.id == "20260624100000"`
  - `Note.body == NoteBody("hello")`
  - `Note.tags` は空 TagSet
  - `Note.createdAt == Note.updatedAt == 2026-06-24T10:00:00`
  - `NoteRepository::write` が `<storage>/20260624100000.md` を 1 回呼ばれる
  - `NoteCreated { note_id: "20260624100000", created_at: ..., initial_tags: empty }` が発行される

### 空 body は no-op {#tp-empty-body}

- **TP-E1**: `raw_body=""` → `Ok(None)` （Note 非生成、event 非発行、`NoteRepository::write` も呼ばれない）
- **TP-E2**: `raw_body="   "` (whitespace only) → `Ok(None)` （C-CN3 の trim 判定）
- **TP-E3**: `raw_body="\n\t  \n"` (改行 + tab + space) → `Ok(None)`
- **TP-E4**: `raw_body="a"` (1 文字でも非空) → `Ok(Some(note))` の通常パス
- **TP-E5**: `raw_body="\u{3000}\u{3000}"` (全角空白のみ) → `Ok(None)` （Rust `str::trim` は Unicode `White_Space` を網羅）

### タグ初期付与 {#tp-with-tags}

- **TP-T1**: `raw_tags=["GPT", "Coding"]` → 正規化されて `["gpt", "coding"]` を持つ Note（I-N6、順序保持）
- **TP-T2**: `raw_tags=["gpt", "gpt"]` → 重複排除後 1 件（I-N5、先勝ち）
- **TP-T3**: `raw_tags=["GPT", "gpt", "Gpt"]` → 全件 lowercase 正規化後 1 件 `["gpt"]`（I-N5 × I-N6 cross-case dedupe）

### タグ正規化失敗 {#tp-invalid-tag}

- **TP-IT1**: `raw_tags=["bad,tag"]` → `InvalidTag { raw: "bad,tag", source: InvalidChar }`（I-N6）
- **TP-IT2**: `raw_tags=["a b"]` → `InvalidTag`（半角空白は内部禁止文字）
- **TP-IT3**: `InvalidTag` 発生時、`NoteRepository::write` は **呼ばれない**（C-CN4 の対偶: 書き出し前に reject）
- **TP-IT4**: `InvalidTag` 発生時、`NoteCreated` event は発行されない
- **TP-IT5**: `raw_tags=[""]` → `InvalidTag { raw: "", source: TagError::Empty }`（trim 後に empty なため）

### NoteBody validation 失敗 {#tp-invalid-body}

- **TP-IB1**: `raw_body="---"` (単独行) → `InvalidBody { source: ContainsFrontmatterDelimiter }`（aggregates.md#note-aggregate-elements）
- **TP-IB2**: `raw_body="hello\n---\nworld"` (中間行 `---`) → `InvalidBody`（frontmatter delimiter line に該当）
- **TP-IB3**: `InvalidBody` 発生時、`NoteRepository::write` は呼ばれない / `NoteCreated` event は発行されない

### 永続化失敗 {#tp-persist-error}

- **TP-PE1**: `NoteRepository::write` が `io::Error` を返す → `PersistError { path: <storage>/20260624100000.md, cause: ... }` を返す
- **TP-PE2**: `PersistError` 時、`NoteCreated` event は発行されない（C-CN4）

### 不変条件チェック {#tp-invariants}

- **TP-I1**: 任意の入力に対して `Note.createdAt.format("%Y%m%d%H%M%S") == Note.id` を満たす（I-N2）
- **TP-I2**: 任意の入力に対して `Note.updatedAt >= Note.createdAt`（I-N3、create 直後は等号）
- **TP-I3**: **trim 後の** Tag content に禁止文字 ` \t\n,[]` が **内部** 出現する raw_tag は常に reject される（I-N6 property test）。strategy: leading/trailing whitespace、prefix/suffix は ASCII 英字大小 + ひらがな (`\u{3040}-\u{309F}`) を混在させた範囲で生成 (uppercase 正規化と CJK 許容を proptest level で固定)。trim で除去される前後 whitespace のみのケースは reject ではなく **正規化対象**（lowercase + trim 後の name が valid なら OK、Empty なら `TagError::Empty`）

### 同一秒内重複は構造的に発生しない {#tp-collision}

- **TP-C1**: 同一 `now` で `CreateNoteCommand { raw_body: "x", ... }` の直後に `CreateNoteCommand { raw_body: "", ... }` を発火 → 2 回目は `Ok(None)` （C-CN3 + C-CN6、id 衝突は起こらない）
- **TP-C2**: 同一 `now` で `CreateNoteCommand { raw_body: "x", ... }` を 2 回連続で発火する経路は本 slice には存在しない（UI 層が Cmd+Enter 後に Draft をクリアするため）。テストでは記述しない

## 実装ノート {#impl-notes}

### アーキ層への落とし込み {#impl-layers}

DDD-VSA-Hex / typescript-tauri の階層に従い、本 slice は Rust 側で実装する（`implementation.language: rust`）。

```
apps/promptnotes/src-tauri/src/note_capture/slices/create_note/
├── mod.rs              # public API: pub use commands::*
├── domain.rs           # CreateNoteCommand, error 型
├── application.rs      # CreateNoteUseCase: orchestrate parse → build → persist → emit
├── infrastructure.rs   # NoteRepository impl (filesystem の `.md` write)
└── commands.rs         # #[tauri::command] create_note → tauri-specta surface
```

TS 側は tauri-specta-generated bindings (`apps/promptnotes/src/lib/note-capture/shared/ipc/bindings.ts`) を経由して呼び出す。本 slice では TS 側のロジック層は作らない（presentation は別 widget / page slice の責務）。

### 依存 interface {#impl-deps}

- `NoteRepository::write(&Note) -> io::Result<()>` — infrastructure 層で実装、`<storage_dir>/<id>.md` に frontmatter + body を書く
- `Clock::now() -> Timestamp` — テストで injectable な秒精度 clock
- `EventBus::publish(NoteCreated)` — in-process 同期 bus

`StorageDir` は **Settings Aggregate から解決済みの絶対パス**を `NoteRepository` 構築時に注入する想定（cross-BC 結合は composition root の責務、create-note slice 内では具体パス解決ロジックを持たない）。

### Domain 内の VO 構築 {#impl-vos}

- `NoteId::from_timestamp(now)` — `YYYYMMDDhhmmss` フォーマット
- `NoteBody::new(raw_body)` — frontmatter 記号 `---` 単独行を含まない検証（domain/workflows/create-note.md#steps 1 より）
- `Tag::new(raw_tag)` — 正規化 + 禁止文字検証（garde derive 推奨。bd memory promptnotes-1-invariant-... 参照）
- `TagSet::from_iter(tags)` — 順序保持 + 重複排除

### Pipeline ステップ {#impl-pipeline}

> domain/workflows/create-note.md#steps の DMMF pipeline をベースに、本 slice の C-CN3 を加味：

0. **empty-body guard** (C-CN3): `raw_body.trim().is_empty()` なら **即 `Ok(None)` を返して終了** （以降のステップは実行しない）
1. `parseBody: String → NoteBody`
2. `parseTags: Vec<String> → Result<TagSet, InvalidTag>`（first error short-circuit）
3. `assignId: Clock → NoteId`
4. `build: (NoteId, NoteBody, TagSet, Timestamp) → Note`
5. `persist: Note → Result<(), PersistError>`
6. `emit: Note → NoteCreated`

ステップ 0-4 は副作用なし。5 で初めて I/O が走り、5 成功後にのみ 6 を実行する（C-CN4）。

### Frontmatter フォーマット {#impl-frontmatter}

`.md` ファイルの内容は infrastructure 層の `NoteRepository` の責務。spec レベルでは以下のみ規定：

- frontmatter は YAML、トップに `---\n...\n---\n` で囲む
- 含む key: `createdAt`, `updatedAt`, `tags`（YAML inline list、順序保持）
- `body` は frontmatter 直後に改行を挟んで書き出す
- 詳細フォーマット（日時表現、escape 規則）は **infrastructure テスト** で固定する
  - 現状の固定値（`fs_note_repo_writes_frontmatter_and_body_at_id_path` ほか）:
    ```
    ---
    createdAt: YYYYMMDDhhmmss
    updatedAt: YYYYMMDDhhmmss
    tags: [name1, name2]
    ---
    <body>
    ```
  - `tags` は inline list, `Tag::name` (lowercase + trim 済み) を順序保持で `", "` 区切り。Tag::new の禁止文字制約により YAML を破壊する文字は混入しない

### Out of scope {#out-of-scope}

本 slice は **command (write side)** のみを扱う。以下は別 slice / layer の責務：

- Draft 入力欄のクリア（UI 層が `NoteCreated` を subscribe）
- 新規 Block の挿入とフォーカス遷移（screen-1 widget / page の responsibilities、cross-draft-submit ルール）
- Note Feed 側の表示更新（Note Feed BC が Shared Kernel 経由で再描画）
- `.md` フォーマットの厳密化（infrastructure テスト or 別 decision）

## Open Questions {#open-questions}

### C-CN3 と domain workflow notes の整合 (RESOLVED 2026-06-24) {#oq-empty-body-domain-drift}

- **状況** (解決前): domain/workflows/create-note.md#notes は「空 raw_body でも作成を許容（spec では明示禁止していない、I-N1〜I-N7 に違反しない）」と記述
- **slice 側決定**: 本 spec C-CN3 は「空 / whitespace のみは no-op」と規定（2026-06-24 ユーザ判断）
- **解決** (2026-06-24): domain/workflows/create-note.md#notes を「空 / whitespace のみの `raw_body` は presentation/command layer で no-op (Note 非生成、event 非発行)」に更新。aggregate は依然空 body を受理する。proposal: ori-bw7 (closed)

### NoteBody validation の error surface (RESOLVED 2026-06-24) {#oq-notebody-validation-surface}

- **状況** (解決前): domain/workflows/create-note.md#errors は `InvalidTag` / `PersistError` のみを列挙し、`NoteBody::new` の domain 不変条件違反（`---` 単独行を含む body）の error path を明示していない
- **Pass 1 review 発覚** (2026-06-24): 実装は `NoteBody::new(raw_body).expect(...)` で panic していた → 合法 Markdown (水平線 `---`) で Tauri command が crash
- **slice 側決定**: 本 spec #io-errors に `InvalidBody { source: NoteBodyError }` variant を明示追加（review 受けて 2026-06-24）
- **解決** (2026-06-24): domain/workflows/create-note.md#errors に `InvalidBody { source: NoteBodyError }` variant 追加。proposal: ori-9mh (closed)
