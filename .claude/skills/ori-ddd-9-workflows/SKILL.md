---
name: ori-ddd-9-workflows
description: distill-ddd Phase 9（Workflows）。DMMF pipeline をファイル分割（workflows/index.md + <id>.md）で記述し、slice の scaffold を提案する
---

ユーザが `/ori-ddd-9-workflows` を呼んだ際、distill-ddd Phase 9（Workflows）を ori convention 注入版で実行します。**Domain Modeling Made Functional（DMMF）の pipeline 構造**を、ori 独自のファイル分割形式で記述します。

## 役割

- **ファシリテーター**：use case → input/output → step 列に分解
- **DMMF エキスパート**：各 step を独立した型変換として書き、railway-oriented programming（Result）の流れを意識
- **記録係**：1 workflow = 1 ファイル。集約ファイルを作らずノイズ分離

## 入力 / 出力

- 入力：`.ori/domain/aggregates.md`（Phase 5）、`.ori/domain/domain-events.md`（Phase 6）、`.ori/domain/validate.md`（Phase 7）
- 出力（**ファイル分割必須**）：
  - `.ori/domain/workflows/index.md` — 全 workflow の summary table + open questions
  - `.ori/domain/workflows/<workflow-id>.md` — 各 workflow の pipeline 詳細（1 file = 1 node）
  - **すべて** に H2/H3 `{#id}` と `ori:` frontmatter（design.md §5）を持つ
    - `index.md`: `node_id: workflow:index`, `type: workflow`, `depends_on: [aggregate:collection, event:collection, scenario:collection]`
    - `<id>.md`: `node_id: workflow:<id>`, `type: workflow`, `depends_on: [aggregate:<Name>, event:<Name>, scenario:<id>]`

## 手順

1. **前提確認**：
   - `.ori/domain/aggregates.md` が無ければ `/ori-ddd-5-aggregates` を先に促す
   - `.ori/domain/workflows/` が既に存在する場合、ファイル一覧を表示し「追記 / 上書き / 中断」を選ばせる
2. **workflow 候補の列挙**：
   - Phase 2 event storming の trigger event から workflow 候補を抽出
   - Phase 7 use case scenarios と対応関係を確認
   - 例：`capture-auto-save`, `edit-past-note-start`, `switch-edit-target`
3. **各 workflow について対話**（DMMF pipeline）：
   - **{#input}** — `Input` 型（command + 必要な参照データ）
   - **{#output}** — `Output` 型（成功時の戻り値 + 副作用としての domain event）
   - **{#error}** — `DomainError` 列挙
   - **{#steps}** — `Step1 → Step2 → ...` で関数合成的に列挙。各 step は型変換として記述
   - **{#dependencies}** — 外部依存（Repository、Clock 等）の interface
4. **挑戦質問**：
   - 「この step は副作用フリーか？ I/O は端に追いやれているか？」
   - 「Error 型は集約の不変条件違反と対応しているか？」
   - 「並列に実行可能な workflow と直列依存の workflow は区別されているか？」
5. **ファイル分割で書き出す**：
   - `workflows/index.md`：summary table（`| id | trigger | output | aggregate |`） + open questions
   - `workflows/<id>.md`：1 workflow 1 ファイル
6. `for f in .ori/domain/workflows/*.md; do bash scripts/lint-domain.sh "$f"; done` を実行して自己検証
7. lint 失敗時は **1 回だけ** 自動修正を試み、それでも失敗ならユーザに判断を委ねる

### Phase 完了時：slice の一括 scaffold 提案

Phase 9 完了後、index.md の workflow 一覧を読み上げ、ユーザに以下を確認：

```
以下の workflow を slice として scaffold しますか？

  - capture-auto-save
  - edit-past-note-start
  - switch-edit-target

[1] 一括で作成（各 slice の新規作成を提案）
[2] 個別に選択
[3] スキップ（後から手動で slice を作成）
```

ユーザの選択に応じて slice の新規作成を提案する。

各 slice について beads issue（`ori-slice-<id>` epic + 7 phase tasks）が作成されます。slice の新規作成を提案してください。

## 出力テンプレート

### `workflows/index.md`

```markdown
---
ori:
  node_id: workflow:index
  type: workflow
  depends_on:
    - aggregate:collection
    - event:collection
    - scenario:collection
---

# Workflows {#workflows}

## Summary {#summary}

| id | trigger | output | aggregate |
|----|---------|--------|-----------|
| [capture-auto-save](capture-auto-save.md) | user types | `NoteSaved` event | Note |
| [edit-past-note-start](edit-past-note-start.md) | user opens past note | `EditStarted` event | Note |

## Open Questions {#open-questions}

- 自動保存の throttle 間隔は？（spec で確定 / domain で確定）
```

### `workflows/<id>.md`

```markdown
---
ori:
  node_id: workflow:capture-auto-save
  type: workflow
  depends_on:
    - aggregate:Note
    - event:NoteSaved
    - scenario:first-auto-save-empty
---

# capture-auto-save {#capture-auto-save}

## Input {#input}

`CaptureAutoSaveCommand`：`{ noteId: NoteId, body: NoteBody, occurredAt: Instant }`

## Output {#output}

`NoteSaved` event

## Errors {#error}

- `EmptyBody` — body が空白のみ
- `NoteNotFound` — noteId に対応する Note が存在しない

## Steps {#steps}

1. `validateBody: NoteBody → Result<NoteBody, EmptyBody>`
2. `loadNote: NoteId → Result<Note, NoteNotFound>`
3. `updateBody: (Note, NoteBody) → Note`
4. `persist: Note → IO<NoteSaved>`

## Dependencies {#dependencies}

- `NoteRepository`
- `Clock`
```

## 注意

- **ファイル分割は必須**：1 つの大きな workflows.md は禁止（review 困難・diff ノイズ大）
- **steps は型変換として書く**：「ユーザに通知する」のような UI 動作は別 phase
- このスキルは workflow を回さない。実装は `/ori-flow` の責務
- distill-ddd 上流との差分：ファイル分割 + scaffold 自動提案

## 次のアクション

Phase 9 完了後、ユーザに以下を提示：

- **通常パス**：`/ori-ddd-10-types` — workflows の入出力型を compile 可能な型定義に落とす
- **scaffold パス**：slice の新規作成を提案（上記参照）
  - 作成後すぐ `/ori-flow <slice-id>` で 7-phase 実装を開始できる
  - ただし `.ori/architecture.md` が無ければ先に `/ori-arch` を実行してもらう（pattern / stack を未確定のまま `/ori-flow` を呼ぶと slice render が動かない）
- **UI 観点に進むパス**：`/ori-ddd-11a-ui-fields` — workflow の input/output から UI 項目を抽出
  - 適用条件：workflow が確定し、画面設計に進みたい場合
- **戻る**：steps が集約をまたぐと判明したら `/ori-ddd-5-aggregates` で境界を見直す
