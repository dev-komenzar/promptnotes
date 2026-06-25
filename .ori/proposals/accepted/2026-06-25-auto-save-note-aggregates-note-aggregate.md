---
target: domain/aggregates.md#note-aggregate
by: slices/auto-save-note
reason: NoteBody 不変条件「frontmatter `---` を含まない」が aggregates.md には記載されているが、construction API の fallibility と再構築 API (from_persisted) の存在が要件化されていない
created: 2026-06-25
status: accepted
accepted_at: 2026-06-25
accepted_by: human (takuya.kometan@gmail.com)
applied_to: domain/aggregates.md#note-aggregate (I-N8 invariant 追加 + NoteBody smart constructor 規約 + Note::from_persisted command 追加)
---

# Proposal: Note Aggregate の NoteBody 不変条件と Note 再構築 API を明示化する

## 発見の経緯 {#context}

- 検出元：`slices/auto-save-note` の phase 4 (impl-green)
- 試みていたこと：FsNoteRepository::load_by_id で `.md` ファイルから Note を再構築 + AutoSave 経路で `NoteBody::new(new_body)` を呼ぶ
- 想定との差:
  - aggregates.md は `NoteBody` を「任意の UTF-8 文字列（空文字も許容、frontmatter 由来の `---` を含まない）」と定義するが、**construction API の表面（fallible / infallible）が aggregate 文書から読めない**
  - aggregates.md の Commands には `Note::create` のみで「永続化済 Note の再構築 (from_persisted)」が公開操作として記載されていない。実態として infrastructure 層で必要（FsNoteRepository::load_by_id 経由）

## 現状仕様 {#current}

> domain/aggregates.md#note-aggregate-elements より（NoteBody 定義）：

```
- **NoteBody** (VO)
  - 任意の UTF-8 文字列（空文字も許容、frontmatter 由来の `---` を含まない）
```

> domain/aggregates.md#note-aggregate-commands より（公開操作の Commands 一覧）：

```
- `Note::create(body: NoteBody, tags: TagSet, now: Timestamp) -> Note`
- `Note::edit_body(self, new_body: NoteBody, now: Timestamp) -> Note`
- `Note::assign_tag(self, tag: Tag) -> Note`
- `Note::remove_tag(self, tag_name: &str) -> Note`
- `Note::delete_to_trash(self) -> DeletedNote`
- `DeletedNote::restore(self) -> Note`
```

## 矛盾／欠落 {#gap}

派生側 (`slices/auto-save-note/spec.md`, impl, FsNoteRepository) が必要とする条件:

1. **NoteBody constructor の fallibility 表明**: aggregates.md の不変条件「`---` を含まない」は **construction 時に enforce される**ことを明示する必要がある（パッシブな記述から smart constructor 規約に格上げ）。これが言語化されないと workflow.md 側（特に auto-save-note）が「Errors なし」と書ける根拠を保てない
2. **Note の再構築 API**: 永続化 round-trip（write → read → reconstruct）が aggregate の責務として明文化されていない。`Note::from_persisted(body, tags, created_at, updated_at) -> Note` 相当が必要であり、I-N1 / I-N2 / I-N3 の不変条件を construction で守ることを併記すべき

## 提案する変更 {#proposal}

### NoteBody 定義に smart constructor 規約を追記

```
- **NoteBody** (VO)
  - 任意の UTF-8 文字列（空文字も許容）
  - **不変条件 (I-N8)**: frontmatter 由来の delimiter 行（行全体が `---`、末尾空白許容）を含まない
  - **construction**: `NoteBody::new(raw: String) -> Result<NoteBody, NoteBodyError>` の smart constructor で I-N8 を enforce。`NoteBodyError::ContainsFrontmatterDelimiter` で表面化
```

### Note Aggregate Commands に Note 再構築を追加

```
- `Note::from_persisted(body: NoteBody, tags: TagSet, created_at: Timestamp, updated_at: Timestamp) -> Note`
  - 永続化済 Note の再構築（FsNoteRepository::load_by_id 経由）
  - `id = NoteId::from_timestamp(created_at)` で I-N2 を construction-time に保証
  - 呼び出し側は文書化された frontmatter format（`.md` ファイルの YAML frontmatter）から各 field を解放してから渡す
  - 再構築失敗（malformed frontmatter / missing key 等）は port (`NoteRepository::load_by_id`) 側で io::ErrorKind::InvalidData として表面化（aggregate には到達しない）
```

### Note Aggregate Invariants に I-N8 を追加

```
- **I-N8**: `body` の構築は `NoteBody::new` 経由でのみ可能であり、frontmatter delimiter 行 (`---`) を含まない。永続化フォーマット保持の前提
```

### 影響範囲

- **直接影響**:
  - `slices/auto-save-note` の workflow 側 Errors 拡張 proposal（`workflows-auto-save-note-errors` proposal）と整合性が取れる（NoteBody 不変条件 → InvalidBody variant）
  - `slices/create-note/spec.md` の I-N6 系記述（`NoteBody::ContainsFrontmatterDelimiter`）を upstream に巻き上げ可能 → 既存 `tp_ib*_*` test 群はそのまま残せる
- **波及候補**:
  - `flush-note` slice も同じ NoteBody 不変条件と LoadError 経路を踏むので、workflow proposal 側と同時 accept が望ましい
  - aggregates.md#note-aggregate-invariants の番号体系（I-N1〜I-N7 が既存）に I-N8 を追加する形式が一貫している

## 代替案 {#alternatives}

1. **port 契約として書く**: aggregate ではなく `NoteRepository` port の契約として「load_by_id は valid な Note 構造のみ Ok に load する」と書く。aggregate 文書を触らないが、smart constructor 規約が言語化されないままになる → 部分的な改善
2. **不変条件は I-N6 (Tag 用) のみ維持**: NoteBody の "---" 制約を aggregate 文書から削除し、infrastructure 層の責務に降ろす。永続化フォーマットと aggregate 不変条件が分離するが、smart constructor が緩むため `create-note` slice の TP-IB* test 群（spec の I-N1 系不変条件）と整合しなくなる → 大改修
