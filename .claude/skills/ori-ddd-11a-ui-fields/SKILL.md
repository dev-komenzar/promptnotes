---
name: ori-ddd-11a-ui-fields
description: distill-ddd Phase 11a（UI Fields）。Phase 10 の型定義から入力 UI 項目を抽出し、画面分割で記述する
---

ユーザが `/ori-ddd-11a-ui-fields` を呼んだ際、distill-ddd Phase 11a（UI Fields）を ori convention 注入版で実行します。**Phase 10 の compile 可能な型から、画面に必要な入力項目を自動列挙し、画面単位で記録**します。

## 役割

- **ファシリテーター**：型定義から UI 項目を機械的に展開しつつ、画面境界をユーザに確認
- **記録係**：1 画面 = 1 ファイル。横断的事項は index.md に集約
- **データの説明者**：型 → UI コントロール（input / select / radio 等）の写像を提案

## 入力 / 出力

- 入力：
  - `.ori/domain/types.md`（Phase 10）— 型定義（input/output 型、value object、enum）
  - `.ori/domain/workflows/<id>.md`（Phase 9）— 各 workflow の Input 型
- 出力（**ファイル分割必須**）：
  - `.ori/domain/ui-fields/index.md` — 横断項目（共通入力 VO、validation rule、項目命名規約）
  - `.ori/domain/ui-fields/screen-<N>.md` — 各画面の入力項目（1 画面 1 ファイル）
  - **全ファイル** に H2/H3 `{#id}` と frontmatter `coherence:` を持つ

## 手順

1. **前提確認**：
   - `.ori/domain/types.md` がなければ `/ori-ddd-10-types` を先に促す
   - `.ori/domain/ui-fields/` が既にあれば一覧表示し「追記 / 上書き / 中断」を選ばせる
2. **画面候補の列挙**：
   - workflow ごとの Input 型 → 1 画面に対応するか確認（複数 workflow が同一画面のことも）
   - 既存の UI 設計があれば取り込む
   - 画面に番号 `screen-1`, `screen-2`, ... を振る
3. **各画面について対話**：
   - **{#purpose}** — 何を入力する画面か（対応する workflow id）
   - **{#fields}** — 入力項目の表（型・必須・VO 由来・UI コントロール候補・備考）
   - **{#cross-field-rules}** — 項目間の整合性ルール（例：「end は start より後」）
   - **{#depended-by}** — この画面を呼び出す上位画面 / 機能（後の Phase 11b で page 分け）
4. **型 → UI コントロールの写像（既定）**：
   | 型 | UI コントロール候補 |
   |----|------------------|
   | `string` / `NonEmptyString` | text input |
   | `enum` / 有限列挙 | select / radio |
   | `boolean` | checkbox / toggle |
   | `Date` / `Instant` | date picker |
   | `int` / `number` | number input |
   | `Email`, `Url`, `Phone` などの VO | 専用 input + 専用 validator |
5. **挑戦質問**：
   - 「この項目はドメインの VO に対応しているか？ 文字列垂れ流しになっていないか？」
   - 「validation は VO の constructor で吸収できるか？」
   - 「複数 workflow が同じ画面なら、画面境界をまとめる？」
6. **ファイル分割で書き出す**
7. `for f in .ori/domain/ui-fields/*.md; do bash scripts/lint-domain.sh "$f"; done` を実行して自己検証
8. lint 失敗時は **1 回だけ** 自動修正を試み、それでも失敗ならユーザに判断を委ねる

## 出力テンプレート

### `ui-fields/index.md`

```markdown
---
coherence:
  source: human
  last_validated: 2026-05-14
  upstream:
    - types.md
    - workflows/index.md
---

# UI Fields {#ui-fields}

## Screens Summary {#screens-summary}

| id | purpose | workflow |
|----|---------|----------|
| [screen-1](screen-1.md) | Capture new note | capture-auto-save |
| [screen-2](screen-2.md) | Edit past note | edit-past-note-start |

## Cross-Cutting VO Mapping {#cross-vo-mapping}

| VO | UI コントロール | validation |
|----|---------------|-----------|
| `NoteBody` | textarea | non-whitespace, ≤ 10_000 chars |
| `Tag` | chip input | lowercase normalized |

## Naming Conventions {#naming}

- field id は `<screen>-<purpose>` 形式（例：`screen-1-note-body`）
- placeholder はドメイン用語を使う（UI 用語にしない）
```

### `ui-fields/screen-1.md`

```markdown
---
coherence:
  source: human
  last_validated: 2026-05-14
  upstream:
    - types.md#capture-auto-save-input
    - workflows/capture-auto-save.md
---

# Screen 1: Capture Note {#screen-1}

## Purpose {#purpose}

`capture-auto-save` workflow の入力画面。ユーザが新規ノート本文を入力する。

## Fields {#fields}

| id | label | 型 | 必須 | VO | UI |備考 |
|----|------|----|-----|----|----|----|
| {#screen-1-note-body} | 本文 | `NoteBody` | ✓ | NoteBody | textarea | 自動保存対象 |

## Cross-Field Rules {#cross-field-rules}

- 本文が空白のみの場合は永続化されず破棄（VO で吸収）

## Depended By {#depended-by}

未確定（Phase 11b で決定）
```

## 注意

- **画面分割は必須**：1 つの大きな ui-fields.md は禁止
- **VO を経由しない裸の string を許可しない**：ドメイン整合性を UI 側で失う
- このスキルは workflow を回さない
- distill-ddd 上流との差分：画面分割と Phase 11a/11b 分離

## 次のアクション

Phase 11a 完了後、ユーザに以下を提示：

- **通常パス**：`/ori-ddd-11b-ui-grouping` — 各画面の `depended_by` を確定し page 群を切り出す
- **早期切上げパス**：1 画面のみで完結する小規模プロジェクトでは 11b をスキップし page の新規作成を提案
- **戻る**：項目が型に紐づけられないと判明したら `/ori-ddd-10-types` で型を補強
