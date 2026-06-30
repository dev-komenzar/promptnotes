---
ori:
  node_id: type-definitions:index
  type: type-definitions
  depends_on:
    - aggregate:collection
    - event:collection
    - workflow:index
---

# Types {#types}

Phase 5/6/9 の概念を **Rust の compile 可能な型** に落とした参照実装。
`.ori/domain/code/rust/` 配下に置き、`src/` には Phase 11 以降で写経・参照する。

## Language Choice {#language-choice}

**Rust** を採用。理由：

- Tauri v2 の backend は Rust（標準ルート）
- Phase 9 workflows.md が既に Rust 風 struct/enum で記述されている
- newtype pattern による smart constructor が型安全に書ける
- `thiserror` / `garde` / `unicode-normalization` を bd memory にて確定済み

TypeScript 等の追加言語は MVP では生成しない（必要になれば後追い）。

## Files {#files}

| topic | file | aggregate / 概念 |
|--|--|--|
| Note Aggregate | [code/rust/note.rs](code/rust/note.rs) | Note / NoteId / NoteBody / BodyHash / Tag / TagSet / Timestamp / TagDiff / BodyDiff / DeletedNote |
| Note Feed Aggregate | [code/rust/note_feed.rs](code/rust/note_feed.rs) | NoteFeed / FeedFilter / NormalizedQuery / DateRangeFilter / SortField / SortDirection / SortOrder |
| Settings Aggregate | [code/rust/settings.rs](code/rust/settings.rs) | Settings / StorageDir / Theme / SettingsDiff |
| Update Channel Aggregate | [code/rust/update_channel.rs](code/rust/update_channel.rs) | UpdateChannel / Version / Release / VersionComparison |
| Domain Events | [code/rust/events.rs](code/rust/events.rs) | DomainEvent enum + 12 event structs + EventBus trait |
| Domain Errors | [code/rust/errors.rs](code/rust/errors.rs) | TagError / NoteIdError / NoteBodyError / PersistError / ReadError / TrashError / ClipboardError / InvalidPath / PathError / NoUndoAvailable / UpdateError / VersionError |
| Workflow Signatures | [code/rust/workflows.rs](code/rust/workflows.rs) | 13 workflow traits + Command / Error 型 + dependency traits |
| Module Tree | [code/rust/lib.rs](code/rust/lib.rs) | re-export root |
| Cargo Manifest | [code/rust/Cargo.toml](code/rust/Cargo.toml) | 参照用の依存記述 |

## Patterns Used {#patterns}

### Newtype + Smart Constructor {#patterns-newtype}

各 VO は `pub struct Foo(InternalType);` の newtype として表現。
内部フィールドを **private** に保ち、構築は `Foo::try_from_*` または
`Foo::from_*` を経由させる。

例: `Tag::try_from_string("  GPT  ")` → 正規化 + 禁止文字チェック →
`Result<Tag, TagError>` を返す。`Tag(String)` を外部から構築できない。

### Result + thiserror {#patterns-result}

すべての error type は `thiserror::Error` を derive し、`Display` と
`std::error::Error` を自動実装。workflow の戻り値は `Result<Output, *Error>`。

### Discriminated Union via enum {#patterns-enum}

- `DomainEvent` は 9 種類の event を 1 つの enum で sum type 化
- `TagDiff = Unchanged | Added(Tag) | Removed(Tag)` で「変化があったか」を型で表現
- `BodyDiff = Unchanged | Changed(NoteBody)` で AutoSave/Flush の冪等性ガード

### Ownership-based State Transition {#patterns-ownership}

`Note::edit_body(self, ...) -> Note` のように **self を消費して新 Note を返す**
形式。古い `Note` 値を保持できないため、不変条件違反（古い状態への書き戻し）
を構造的に防ぐ。

### Dependency Injection via Trait Objects {#patterns-di}

`NoteRepository`, `Clock`, `EventBus`, `TrashService`, `ClipboardService`,
`SettingsRepository`, `OsDirs`, `UpdaterPlugin`, `UndoStack` を trait として定義。
workflow 実装は `&dyn Trait` または generic 経由で受け取り、production / test
で実装を差し替え可能。

## Dependencies Rationale {#dependencies-rationale}

bd memory `promptnotes-1-invariant-2-tag-normalizedquery-storagedir-3` で確定済みの選定：

| crate | 用途 |
|--|--|
| **thiserror** | ドメインエラー型の derive |
| **garde** | derive(Validate) ベースの個別 VO 検証（現状は手書きのみ、Phase 11 で本格採用） |
| **unicode-normalization** | NFKC (compatibility normalization) 正規化（NormalizedQuery, body マッチング、I-F1 / I-F5） |
| **time** | Timestamp の秒精度操作（OffsetDateTime + 秒切り捨て）|
| **serde** | Settings の JSON 永続化 / Note の frontmatter 形成（src 側で実装） |
| **url** | Release.url の型安全な URL 表現 |
| **semver** | Version の semver 比較（I-U2: latest > current 判定） |
| **sha2** | BodyHash の SHA-256 計算（I-N9 競合検出用） |
| **hex** | BodyHash の hex エンコード |

`garde` は Phase 10 段階では手書き smart constructor を採用（VO ごとの
バリデーションが軽量で derive macro より直接的）。Phase 11 で
field-level バリデーションが増えた段階で再評価する。

## Coverage Matrix {#coverage}

### Aggregates の不変条件マッピング {#coverage-aggregates}

| Aggregate | 型定義ファイル | 不変条件の表現 |
|--|--|--|
| Note | note.rs | private fields + 構築/更新 method 経由のみ。`updatedAt` は更新メソッド内で必ず代入（I-N4）。`Tag::try_from_string` で I-N6 を集中管理。`BodyHash` は `body` から決定論的に導出（I-N9）。`Note::is_stale` で競合検出（I-N9） |
| NoteFeed | note_feed.rs | `visible_notes()` 内で filter→sort、`SortOrder` で tiebreak（I-F3）。query は `NormalizedQuery::from_raw` で必ず NFKC (compatibility normalization) + lowercase（I-F1）。`upsert_note` / `remove_note` / `hydrate` で source 操作（I-F8） |
| Settings | settings.rs | `StorageDir::try_from_path` で絶対パス検証（I-S1）。デフォルト値は `Settings::defaults` 経由（I-S3） |
| UpdateChannel | update_channel.rs | `with_release` で `latest > current` のみ Some に保持（I-U2）|

### Events の対応 {#coverage-events}

12 event すべてが `events.rs` に struct として定義され、`DomainEvent` enum に集約。
Phase 6 で追加された 3 つの外部ファイル変更イベント（`NoteFileCreatedExternally` /
`NoteFileModifiedExternally` / `NoteFileDeletedExternally`）を含む。

### Workflows の対応 {#coverage-workflows}

13 workflow すべてに対応する trait を `workflows.rs` に定義。production の
Application Service 層がこれらを impl する想定。

## Open Questions {#open-questions}

Phase 10 時点で未決事項はない。`cargo check` でのコンパイル確認は **2026-06-30 に実施済み**（成功）。

- `garde::Validate` の derive 適用は Phase 11 で段階導入予定

## Notes {#notes}

### 最終更新 {#notes-last-update}

**2026-06-30**: Phase 5/6 拡張を反映。
- `note.rs`: `BodyHash` VO、`Note::from_persisted`、`Note::body_hash()`、`Note::is_stale()` 追加（I-N9 競合検出対応）
- `note_feed.rs`: `hydrate()`、`upsert_note()`、`remove_note()` 追加（I-F8 差分更新対応）。NFC→NFKC 修正。NoteFeed を所有型（`Vec<Note>`）に変更
- `events.rs`: `NoteFileCreatedExternally` / `NoteFileModifiedExternally` / `NoteFileDeletedExternally` 追加（Phase 6 Core Domain 拡張）
- `Cargo.toml`: `sha2` / `hex` 依存追加
- `workflows.rs`: `UpdateFeedFilterWorkflow` の lifetime 除去

### 「これは参照型定義であり production 型ではない」 {#notes-not-production}

- `.ori/domain/code/rust/` は **distill-ddd の参照成果物**
- 実プロダクトの `src/` には Phase 11 以降で写経または参照される
- Phase 11 で UI 観点の補強が入ったり、Tauri command との接続層が追加される

### Ownership ベースのトレードオフ {#notes-ownership-tradeoff}

- `Note::edit_body(self, ...) -> Note` は安全だが、所有権の伝播が煩雑
  になる場面がある（特に `&mut Note` を使いたい UI 反映層）
- application service 層で `Arc<RwLock<HashMap<NoteId, Note>>>` のような
  storage を持ち、そこに新 Note を再代入する形が現実的
- Phase 11 の実装で具体化する

### Edition / Toolchain {#notes-toolchain}

- edition = "2021"（Tauri v2 と整合）
- Rust 1.75+ を想定（trait async, GAT 等は未使用なので 1.70 でも可）
- Tauri v2 が要求する Rust バージョンに従う
