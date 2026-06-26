---
coherence:
  source: derived
  last_derived: 2026-06-26
  upstream:
    - domain/workflows/remove-tag.md#remove-tag
    - domain/aggregates.md#note-aggregate
    - domain/bounded-contexts.md#note-capture
    - domain/domain-events.md#note-tags-changed
  hash:
    domain/workflows/remove-tag.md#.*: 7a61f8005f29
    domain/aggregates.md#.*: 9f9048f5816b
    domain/bounded-contexts.md#.*: 4d579125a513
    domain/domain-events.md#.*: 8abdfac78084
ori:
  schema:
    propagation_level: file
---

# remove-tag spec {#remove-tag-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive remove-tag`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

Note からタグ 1 件を削除する slice。Note Capture BC の write path で、タグチップの × ボタン押下から起動される。`tag_name` は呼び出し側 (UI 層) で既に正規化済みと仮定し、本 slice 内で再正規化や `Tag::new` parse は行わない。存在しないタグの削除は no-op（永続化も event も発火させない）。assign-tag slice と対称な pipeline で、同じ NoteTagsChanged event を発行する。

> domain/bounded-contexts.md#note-capture より：
> > ユーザの起案・編集・タグ付け・コピー・削除/復元といった **Note の write side** を司る。

> domain/workflows/remove-tag.md より：
> > Note から指定されたタグを削除する。存在しないタグの削除は no-op。

domain/domain-events.md#note-tags-changed-trigger は「`Note::assign_tag(tag)` または `Note::remove_tag(tag_name)` の永続化成功時。個別の付与 / 削除を区別せず、結果としての TagSet 全体を運ぶ」と規定しており、本 slice もこれに従う。

## 入出力 {#io}

### Input {#io-input}

```rust
struct RemoveTagCommand {
  note_id: NoteId,
  tag_name: String,    // 正規化済み (UI からタグチップ表示文字列をそのまま渡す)
}
```

> domain/workflows/remove-tag.md#input より引用。`NoteId` は `^\d{14}$`（domain/aggregates.md#note-aggregate-elements）。`tag_name` は domain workflow#notes 「`tag_name` は UI（タグチップの × ボタン）から既に正規化済みで来る前提」に従う。

### Output {#io-output}

- 成功: `Result<Option<Note>, RemoveTagError>` の Ok variant
  - `Some(Note)`: タグが実際に削除された場合 (updated_at 更新済、永続化済、event 発火済)
  - `None`: 該当 tag_name が TagSet に存在せず no-op だった場合 (永続化・event 共に未実施)
- domain event: `NoteTagsChanged { note_id, tags, updated_at }` (変化時のみ、domain/domain-events.md#note-tags-changed-payload)

### Errors {#io-errors}

- `RemoveTagError::NoteNotFound { id: NoteId }` — 指定 `note_id` の Note が `NoteRepository` 上に存在しない
- `RemoveTagError::LoadError { path: PathBuf, source: io::Error }` — `NoteRepository::load_by_id` が `io::Err` を返した
  - assign-tag slice と同型の意図的な variant 分離。read 側の I/O 失敗を NoteNotFound (= 「存在しない」) と区別する事で診断容易性を保つ
- `RemoveTagError::PersistError { path: PathBuf, source: io::Error }` — `NoteRepository::write` が失敗

domain/workflows/remove-tag.md#errors は `NoteNotFound` / `PersistError` の 2 variant を列挙しているが、`LoadError` を加える 3 variant 構成とする。これは assign-tag slice の同型決定 (read I/O と write I/O の意味分離) を踏襲し、Note Capture BC の write 系 slice (auto-save-note / assign-tag) との一貫性を優先した実装詳細の選択。`load_by_id` の io::Err を NoteNotFound へ collapse する read-only / one-shot 系 slice (copy-note-body I-CNB5 / delete-note I-DN6) とは異なる方針 → 本 slice は write side のため LoadError を残す。

## 不変条件 {#invariants}

### Note Aggregate 由来 {#invariants-note-aggregate}

- **I-N1（domain/aggregates.md#note-aggregate-invariants）**: `id` は immutable。本 slice の削除経路でも維持
- **I-N3**: `updated_at >= created_at`。`Removed` 経路では `updated_at = clock.now()` に更新する（assign-tag と同様の方針）
- **I-N4**: body 変更時に updated_at 更新。本 slice は body を触らないが、tags 変更も「Note の意味的変更」として `updated_at` を更新する (assign-tag aggregate impl と整合)
- **I-N5**: TagSet 内に同一 `Tag::name` は 1 件のみ。削除後も維持される（元々重複が無いため、1 件取り除いた結果も重複なし）
- **I-N6**: `Tag::name` は正規化規則を満たす。削除は既存 Tag を抜くだけのため、残った Tag も全て I-N6 を満たす（追加経路がないため悪化しない）

### slice 固有制約 {#invariants-slice-specific}

- **I-RT1（tag_name 正規化前提）**: 本 slice は `tag_name: String` を受け取るが、内部で再正規化（trim / lowercase）や `Tag::new` parse を**行わない**。比較は受け取った文字列と既存 `TagSet` 内の `Tag::name` (= 既に正規化済み) の **完全一致 (case-sensitive)** で行う。UI 層がタグチップ表示文字列 (= `Tag::name`) をそのまま渡す契約により、空文字 / 禁止文字を含む文字列が来た場合は「存在しない」として no-op になる（domain/workflows/remove-tag.md#notes「不正な tag_name が来ても『存在しないので no-op』となり安全」と整合）
- **I-RT2（no-op semantics）**: 該当 `tag_name` が TagSet に存在しない場合、Ok(None) を返し `NoteRepository::write` / `EventBus::publish` のいずれも呼ばない。「存在しないタグ削除は安全な no-op」(domain workflow#steps step 3 Unchanged 分岐)
- **I-RT3（副作用順序: 削除成功時）**: `TagDiff::Removed` 経路では (a) `Note::remove_tag` で TagSet 更新 + updated_at 更新 → (b) `NoteRepository::write` → (c) `EventBus::publish` の順。途中で失敗した場合 (c) 以降は実施しない
- **I-RT4（副作用順序: persist 失敗時）**: `PersistError` の場合 `EventBus::publish` は呼ばれない (永続化されていない状態を subscriber に通知すると Note Feed の表示再計算が aggregate 実体と乖離するため)。assign-tag C-AT5 と同型
- **I-RT5（副作用順序: load 失敗時）**: `NoteNotFound` / `LoadError` の場合 `NoteRepository::write` / `EventBus::publish` のいずれも呼ばれない
- **I-RT6（id / created_at / body 不変）**: 本 slice は tags のみを書き換える。`id` / `created_at` / `body` は load した Note と書き戻し時の Note で byte-for-byte 一致する (I-N1 と整合)
- **I-RT7（NoteTagsChanged payload は更新後の TagSet 全体）**: domain/domain-events.md#note-tags-changed「結果としての TagSet 全体を運ぶ」に従い、event の `tags` は削除後の TagSet（削除対象を含まない）。subscriber である Note Feed の TagFilter 再計算が正しく動くための契約
- **I-RT8（LoadError vs PersistError 意味分離）**: assign-tag / auto-save-note と同型。read 側と write 側の I/O 失敗を別 variant で表現し、永続化層の診断（再試行可能性 / 権限問題の切り分け）を容易にする

### 経路境界 {#invariants-boundary}

- **UI 副作用は責務外**: タグチップの DOM 更新・トースト表示等は UI 層の責務であり、本 slice の output 契約は `Result<Option<Note>, RemoveTagError>` のみ
- **Tauri 境界**: Rust 側 command として expose し、tauri-specta で TS bindings を自動生成する（`.ori/architecture.md` cross_root 参照）
- **再正規化禁止**: I-RT1 の通り `Tag::new` parse / trim / lowercase 化を本 slice で**呼ばない**。これは UI 契約を信頼する pragmatic choice であり、UI 側のバグで未正規化文字列が渡された場合は no-op 化される (安全側に倒す)

## テスト観点 {#test-perspectives}

### happy path: 既存タグの削除 {#tp-happy}

`tags = [Tag("rust"), Tag("memo")]` の Note を seed → `tag_name="rust"` で削除 → (a) `Note::remove_tag` で `tags = [Tag("memo")]` になり、(b) `NoteRepository::write` が 1 回呼ばれ、(c) `EventBus::publish(NoteTagsChanged { tags: [Tag("memo")], ... })` が 1 回 emit され、(d) Ok(Some(Note)) を返す。updated_at は clock.now() に更新される。

### 最後のタグ削除 → tags 空 {#tp-last-tag}

`tags = [Tag("rust")]` の Note → `tag_name="rust"` で削除 → tags = [] になる。event payload の tags も空 TagSet。

### 中央のタグ削除で順序保存 {#tp-order-preserved}

`tags = [Tag("a"), Tag("b"), Tag("c")]` → `tag_name="b"` で削除 → `tags = [Tag("a"), Tag("c")]` で順序が保たれる（TagSet は YAML inline list の表示順を保持、domain/aggregates.md#note-aggregate-elements TagSet）。

### no-op: 存在しないタグ {#tp-noop-missing}

`tags = [Tag("rust")]` → `tag_name="python"` で削除 → Ok(None)、`NoteRepository::write` / `EventBus::publish` のいずれも呼ばれない (I-RT2)。

### no-op: 空 TagSet {#tp-noop-empty-tagset}

`tags = []` の Note → 任意の `tag_name` で削除 → Ok(None)、write / event 未呼出 (I-RT2)。

### no-op: 未正規化文字列 {#tp-noop-unnormalized}

`tags = [Tag("rust")]` → `tag_name=" RUST "` (前後空白 + 大文字) で削除 → I-RT1 により再正規化しないので「存在しない」扱いになり Ok(None)、write / event 未呼出。UI 契約違反時の安全側挙動を pin する。

### no-op: case-sensitive {#tp-noop-case-sensitive}

`tags = [Tag("rust")]` → `tag_name="Rust"` で削除 → 一致しないため Ok(None) (I-RT1)。Tag::new は input を lowercase 化するが本 slice は正規化された name の完全一致のみを見る。

### NoteNotFound {#tp-not-found}

存在しない `note_id` を渡す → `RemoveTagError::NoteNotFound { id }`、write / event 未呼出 (I-RT5)。

### LoadError {#tp-load-err}

`NoteRepository::load_by_id` が `io::Err(PermissionDenied)` を返す → `RemoveTagError::LoadError { path, source }`、write / event 未呼出 (I-RT5, I-RT8)。path は `storage_dir / <id>.md`。

### PersistError 伝播 {#tp-persist-err}

happy path 構成で `NoteRepository::write` が `io::Err` を返す → `RemoveTagError::PersistError { path, source }`、`EventBus::publish` 未呼出 (I-RT4)。assign-tag C-AT5 と同型。

### 副作用順序: write → publish {#tp-side-effect-order}

happy path で write spy → publish spy の順を OrderLog で確認 (I-RT3)。

### event payload は更新後の TagSet {#tp-event-payload-after-remove}

`tags = [Tag("a"), Tag("b")]` → `tag_name="a"` 削除 → event の `tags` は `[Tag("b")]` のみ（削除後の集合、I-RT7）。`note_id` と `updated_at` が Note と一致。

### id / body / created_at 不変 {#tp-immutability}

happy path 前後で write された Note の `id` / `body` / `created_at` が seed Note と byte-for-byte 一致する (I-RT6)。`updated_at` のみが変化する。

### no domain event for unchanged {#tp-no-event-unchanged}

no-op 経路で event bus に何も publish されない (I-RT2)。

### Note aggregate I-N5/I-N6 維持 {#tp-aggregate-invariants}

削除後の TagSet を `Tag::name` で集計し、重複なし (I-N5) + 残った Tag が正規化規則を満たす (I-N6) ことを確認。

## 実装ノート {#impl-notes}

### 依存 interface（port） {#impl-ports}

- `NoteRepository::load_by_id(&NoteId) -> io::Result<Option<Note>>` — assign-tag / auto-save-note と同じ既存 port
- `NoteRepository::write(&Note) -> io::Result<()>` — 既存
- `NoteRepository::storage_dir() -> &Path` — LoadError / PersistError の path 解決用 (既存)
- `Clock::now() -> Timestamp` — `updated_at` 更新用 (既存)
- `EventBus::publish(DomainEvent::NoteTagsChanged { ... })` — 既存 (assign-tag が同型に使用)

### Note aggregate 拡張 {#impl-aggregate-extension}

`Note::remove_tag(self, tag_name: &str, now: Timestamp) -> Self` を `apps/promptnotes/src-tauri/src/note_capture/shared/types/note.rs` に追加。`assign_tag` と対称な API:
- TagSet に該当 `Tag::name == tag_name` の Tag が無ければ no-op (self をそのまま返す)
- 存在すれば該当 Tag を除いた新 TagSet を作り、`updated_at = now` で新 Self を返す
- `Tag::name` 比較は I-RT1 通り正規化済み前提の完全一致
- 戻り値が seed と同一 (no-op) かを use case 側で識別するため、application 層では aggregate を呼ぶ前に `compute_diff` を実行する（assign-tag と同じ pattern）。aggregate 自体は防御的に no-op するが、event-emission control は use case 責務

### slice layout（DDD-VSA-Hex） {#impl-layout}

assign-tag slice と並列構造:

- Rust: `apps/promptnotes/src-tauri/src/note_capture/slices/remove_tag/`
  - `mod.rs` — module 宣言 + re-export
  - `domain.rs` — `RemoveTagCommand` / `RemoveTagError` (NoteNotFound / LoadError / PersistError 3 variant)
  - `application.rs` — `RemoveTagUseCase` の 5-step pipeline (load → compute_diff → branch → apply_remove → persist → emit)
  - `commands.rs` — `#[tauri::command]` で expose (assign-tag と同型、tauri-specta 連携)
  - `tests.rs` — 上記テスト観点を実装
- 既存 slices/mod.rs に `pub mod remove_tag;` を追加

### 既存 slice との関係 {#impl-related-slices}

- assign-tag slice の `AssignTagUseCase` と対称: parse_tag step を省き、TagDiff variant を `Removed` に変える以外は同形
- auto-save-note の `BodyDiff` / assign-tag の `TagDiff` と同様、`RemoveTagDiff = Unchanged | Removed` を application 層で定義
- `NoteTagsChanged` event は assign-tag と共有 (domain/domain-events.md#note-tags-changed-trigger が両方を含む)

### 非責務 {#impl-non-responsibility}

- タグチップ UI の DOM 更新・animation・トースト表示は UI 層
- `Tag::new` による parse / 正規化 (I-RT1)
- restore-deleted-note とは別経路 (削除 → trash の delete-note slice とも別。本 slice は note 自体は残し tags のみを変更)

## Open Questions {#open-questions}

派生時点で domain 上流と impl の間に持ち越した発散事項。upstream-first proposal 化するか維持するかは finalize / 次 slice 着手時に判断する。

### oq-remove-tag-error-3variant {#oq-remove-tag-error-3variant}

`domain/workflows/remove-tag.md#errors` は `NoteNotFound` / `PersistError` の 2 variant のみ列挙するが、本 slice は `LoadError` を加える 3 variant 構成を採用 (I-RT8)。

- 理由: Note Capture BC の write 系 slice (`auto-save-note` / `assign-tag`) が同型に LoadError を持ち、read I/O failure と write I/O failure を意味分離する設計が BC 全体で一貫
- 検討: domain workflow#errors に LoadError 追記の proposal を出すか、または slice-level の "impl detail variant" として spec 内 I-RT8 のみで説明留めるか
- 現状: 後者 (spec 内説明) で運用。BC 内 3 slice 共通 pattern として確立してから domain proposal にする案

### oq-remove-tag-now-injection {#oq-remove-tag-now-injection}

`domain/aggregates.md#note-aggregate-commands` の `Note::remove_tag(tag_name)` は `now` 引数を取らない signature で記述されているが、impl は `Note::remove_tag(self, tag_name: &str, now: Timestamp) -> Self` で `Timestamp` を注入する。

- 理由: aggregate を pure に保つため (`Clock` への依存を application 層に押し出す)。assign-tag slice の `oq-assign-tag-now-injection` と同型の決定
- 検討: aggregates.md の commands 項を `Note::remove_tag(tag_name, now)` へ揃える proposal を出す
- 現状: assign-tag と同じく Open Question として記録し、両 slice を束ねた proposal を /ori-propose 経由で作る予定

### oq-tag-name-type-strength {#oq-tag-name-type-strength}

`Note::remove_tag(self, tag_name: &str, now)` の `tag_name` は裸の `&str` を受け取り、`Tag::name` との比較で I-N6 の正規化規則を構造的に強制していない (review Pass 1 MED-D)。

- 理由: I-RT1「UI 側がタグチップ文字列を verbatim で渡す」契約で I-N6 を満たすことを期待
- 検討: `TagName` newtype を導入し `assign_tag(self, Tag, now)` と signature を対称化する。compile-time に「I-N6 を満たす正規化済み文字列のみ受け入れる」を表現できる
- 現状: 本 slice では `&str` のまま採用。`TagName` 導入は他 slice (assign-tag / 将来の rename-tag) と束ねた cross-slice refactor として後送り
