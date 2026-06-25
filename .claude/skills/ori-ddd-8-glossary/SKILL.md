---
name: ori-ddd-8-glossary
description: distill-ddd Phase 8（Glossary）。ユビキタス言語を整理し、必要なら context 間の意味差分も明示する
---

ユーザが `/ori-ddd-8-glossary` を呼んだ際、distill-ddd Phase 8（Ubiquitous Language Glossary）を ori convention 注入版で実行します。**ドメイン用語を集約・統一**し、bounded context 間で同じ単語が異なる意味を持つ場合は明示します。

## 役割

- **用語整理者**：これまでの phase で登場した名詞・動詞を抽出
- **意味調停者**：context 間で重複する語の意味差を表で示す
- **記録係**：単一ファイル、用語ごとに H3

## 入力 / 出力

- 入力：これまでの全 phase 文書（discovery, event-storming, bounded-contexts, context-map, aggregates, domain-events, validation）
- 出力：`.ori/domain/glossary.md`
  - frontmatter: `ori:` ブロック（design.md §5）
    - `node_id: glossary-term:collection`（file-level representative）
    - `type: glossary-term`
    - `depends_on: [discovery:overview, bounded-context:collection, aggregate:collection, event:collection, scenario:collection]`
  - 個別 term node は H3 anchor から導出（例: `### Note {#glossary-note}` → `glossary-term:note`）
  - H2 = カテゴリ（Aggregates / Events / Commands / Concepts / Cross-Context）
  - H3 = 個別用語

## 手順

1. **前提確認**：上記入力ファイルを読み返す
2. **用語の抽出**：
   - aggregate 名・event 名・command 名
   - VO / entity 名
   - workflow 名
   - business concept（例：「自動保存」「下書き」「公開」）
3. **同義語の統一**：複数語で同じ概念を指している場合、正式名 1 つに統一（旧称は alias として記載）
4. **context 間の意味差検出**：
   - 同じ単語が複数 context で異なる意味を持つ場合 → "Cross-Context" セクションに記載
5. **挑戦質問**：
   - 「この用語はドメイン由来か技術由来か？ 技術用語は glossary に入れない」
   - 「英語と日本語の対応は一意か？」
6. **文書生成**：単一ファイル `.ori/domain/glossary.md`
7. `bash scripts/lint-domain.sh .ori/domain/glossary.md` を実行して自己検証
8. lint 失敗時は **1 回だけ** 自動修正、それでもダメなら人間判断

## 出力テンプレート

```markdown
---
ori:
  node_id: glossary-term:collection
  type: glossary-term
  depends_on:
    - discovery:overview
    - bounded-context:collection
    - aggregate:collection
    - event:collection
    - scenario:collection
---

# Glossary {#glossary}

## Aggregates {#glossary-aggregates}

### Note {#glossary-note}

- **定義**：ユーザが入力する文章単位。body と時刻情報を持つ
- **context**：Note Capture
- **alias**：draft（旧称、非推奨）

### Tag {#glossary-tag}

- **定義**：Note に付与される分類ラベル
- **context**：Tag Management
- **正規化**：小文字に変換、空白除去

## Events {#glossary-events}

### NoteSaved {#glossary-note-saved}

- **定義**：Note の永続化が成功した時に発行
- **emitter**：Note Aggregate

## Concepts {#glossary-concepts}

### 自動保存 (AutoSave) {#glossary-autosave}

- **定義**：ユーザ入力中の継続的永続化。throttle あり
- **context**：Note Capture

## Cross-Context Differences {#cross-context-differences}

| 用語 | Note Capture での意味 | Tag Management での意味 |
|------|---------------------|------------------------|
| **公開状態** | published flag | tag-based visibility filter |
```

## 注意

- **技術用語は入れない**：repository、handler などは glossary 対象外
- **alias は明示する**：「draft = Note (旧称)」のように
- **context 間の意味差は最重要**：同じ単語が違う意味なら必ず Cross-Context セクションに記載
- **このスキルは workflow を回さない**

## 次のアクション

Phase 8 完了後、ユーザに以下を提示：

- **通常パス**：`/ori-ddd-9-workflows` — DMMF pipeline でワークフロー設計に進む
- **早期切上げパス**：glossary を更新せず `/ori-ddd-9-workflows` に進み、phase 9 で必要になったら戻る
- **戻る**：context 間意味差が深刻なら `/ori-ddd-3-bounded-contexts` で境界を見直し
