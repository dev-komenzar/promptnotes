---
name: ori-ddd-5-aggregates
description: distill-ddd Phase 5（Aggregates）。集約ごとの不変条件と公開操作を確定し .ori/domain/aggregates.md を生成する
---

ユーザが `/ori-ddd-5-aggregates` を呼んだ際、distill-ddd Phase 5（Aggregate Design）を ori convention 注入版で実行します。**集約境界・不変条件・公開操作の言語化**が目的です。

## 役割

- **ファシリテーター**：bounded context（Phase 3）と context map（Phase 4）の合意を踏まえ、各 context 内の集約を設計
- **ドメインエキスパート挑戦者**：「その不変条件は集約内で守られるか」「公開操作は最小か」を問う
- **記録係**：各集約を H2 1 つで表現、必須 H3 セクションを欠かさない

## 入力 / 出力

- 入力：`.ori/domain/bounded-contexts.md`（Phase 3）と `.ori/domain/context-map.md`（Phase 4）
  - 未生成なら「Phase 3 / 4 を先に通すか、簡易モードで進めるか」をユーザに確認
- 出力：`.ori/domain/aggregates.md`
  - frontmatter: `ori:` ブロック（design.md §5）
    - `node_id: aggregate:collection`（file-level representative）
    - `type: aggregate`
    - `depends_on: [bounded-context:collection, context-map:map]`
  - 個別 aggregate node は H2 anchor から導出（例: `## Note Aggregate {#note-aggregate}` → `aggregate:Note`、PascalCase 化）
  - **H2 = 1 つの集約**（`## Note Aggregate {#note-aggregate}` 形式）
  - **必須 H3 サブセクション**：
    - `### 構成要素 {#<aggregate-id>-elements}` — root entity、entity、value object
    - `### ビジネス不変条件 {#<aggregate-id>-invariants}` — 集約境界内で常に true となる条件
    - `### 公開操作 {#<aggregate-id>-operations}` — 外部から呼べるコマンド／クエリ

## 手順

1. **前提確認**：
   - `.ori/domain/aggregates.md` が既にあれば内容を要約し「追記 / 上書き / 中断」を選ばせる
   - `.ori/domain/bounded-contexts.md` 不在の場合、ユーザに「Phase 3 をやるか、暗黙の単一 context として進めるか」を確認
2. **集約候補の列挙**：Phase 2 event storming の noun 群と Phase 3 の context から候補を出す
3. **各集約について対話**（必須 3 観点）：
   - **構成要素**：root entity は何か、内部に持つ entity / VO は何か、境界の理由は何か
   - **ビジネス不変条件**：「常に成り立つべき条件」（例：「Note は body 編集後、必ず `updatedAt` が増分」）
   - **公開操作**：command（状態変更）と query（読み取り）を最小集合で列挙
4. **挑戦質問**：
   - 「この不変条件は集約をまたがず守れるか？ またがるなら集約境界が間違っている」
   - 「公開操作は CRUD ではなく業務に対応するか？」
   - 「root entity の identity はどこで決まるか？」
5. **文書生成**：合意した内容のみ Markdown で記述。**H2 = 集約、H3 = 必須 3 種（+ 任意の "Notes" など）**
6. `bash scripts/lint-domain.sh .ori/domain/aggregates.md` を実行して自己検証
7. lint 失敗時は **1 回だけ** 自動修正を試み、それでも失敗ならユーザに判断を委ねる

## 出力テンプレート

```markdown
---
ori:
  node_id: aggregate:collection
  type: aggregate
  depends_on:
    - bounded-context:collection
    - context-map:map
---

# Aggregates {#aggregates}

## Note Aggregate {#note-aggregate}

### 構成要素 {#note-aggregate-elements}

- **Note**（root entity）：`id: NoteId`、`body: NoteBody`、`createdAt`、`updatedAt`
- **NoteBody**（VO）：空白のみは無効

### ビジネス不変条件 {#note-aggregate-invariants}

- `body` 編集時、`updatedAt` は必ず前回より大きい値に更新される
- 空 `NoteBody`（空白のみを含む）は永続化されず破棄される

### 公開操作 {#note-aggregate-operations}

- `Note.create(body: NoteBody): Note`
- `Note.editBody(newBody: NoteBody): Note`
- `Note.isEmpty(): boolean`

## Tag Aggregate {#tag-aggregate}

...
```

## 注意

- **集約は最小に**：迷ったら分割する方向。1 集約 = 1 トランザクション境界
- **不変条件をまたぐ集約は誤り**：Phase 4 context map と矛盾するなら戻る
- このスキルは workflow を回さない。実装は `/ori-flow` の責務
- distill-ddd 上流との差分：H3 必須 3 種ルール、`{#id}` 強制、`ori:` frontmatter（design.md §5）

## 次のアクション

Phase 5 完了後、ユーザに以下を提示：

- **通常パス**：`/ori-ddd-6-domain-events` — 各集約が発行する domain event を列挙
- **早期切上げパス**：MVP では event を後回しにし `/ori-ddd-9-workflows` から始める
  - 適用条件：CQRS / event-sourcing を採用しない、かつ集約間の async 連携が想定にない
  - リスク：context map の uppdownstream で event が必須化したら戻る
- **戻る**：不変条件が集約をまたいで成立しないと判明したら `/ori-ddd-3-bounded-contexts` へ遡る
