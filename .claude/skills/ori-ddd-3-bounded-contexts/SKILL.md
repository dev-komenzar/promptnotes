---
name: ori-ddd-3-bounded-contexts
description: distill-ddd Phase 3（Bounded Contexts）。event cluster から bounded context と subdomain を切り出し H2=BC で記録する
---

ユーザが `/ori-ddd-3-bounded-contexts` を呼んだ際、distill-ddd Phase 3（Bounded Contexts & Subdomains）を ori convention 注入版で実行します。**bounded context（BC）と subdomain（core / supporting / generic）を確定**します。

## 役割

- **ファシリテーター**：event storming の cluster から context 候補を提案
- **挑戦者**：「context 内でユビキタス言語が同じ意味を持つか」を問う
- **記録係**：1 BC = 1 H2、`{#id}` 必須

## 入力 / 出力

- 入力：`.ori/domain/event-storming.md`（Phase 2）と `.ori/domain/discovery.md`（Phase 1）
- 出力：`.ori/domain/bounded-contexts.md`
  - frontmatter: `ori:` ブロック（design.md §5）
    - `node_id: bounded-context:collection`（file-level の representative）
    - `type: bounded-context`
    - `depends_on: [event-storming:timeline, discovery:overview]`
  - 個別 BC の node_id は H2 anchor から導出（例: `## Note Capture {#note-capture}` → `bounded-context:note-capture`）
  - **H2 = 1 BC**、H3 で詳細

## 手順

1. **前提確認**：Phase 1, 2 の文書を読み返す
2. **BC 候補の抽出**：event storming の aggregate cluster をベースに context 候補を列挙
3. **各 BC について対話**（必須 H3）：
   - **{#purpose}** — この context が解く問題
   - **{#subdomain-type}** — core / supporting / generic のいずれか
   - **{#ubiquitous-language}** — この context 固有の用語
   - **{#core-aggregates}** — 主要な aggregate 候補（Phase 5 で正式化）
4. **挑戦質問**：
   - 「同じ単語が context 跨ぎで異なる意味を持っていないか？」
   - 「core / supporting / generic の区別の根拠は？」
   - 「context を分けすぎていないか？ 1 トランザクションで完結する境界か？」
5. **文書生成**：H2 = BC、H3 = 必須 4 種
6. `bash ./scripts/lint-domain.sh .ori/domain/bounded-contexts.md` を実行して自己検証
7. lint 失敗時は **1 回だけ** 自動修正、それでもダメなら人間判断

## 出力テンプレート

```markdown
---
ori:
  node_id: bounded-context:collection
  type: bounded-context
  depends_on:
    - event-storming:timeline
    - discovery:overview
---

# Bounded Contexts {#bounded-contexts}

## Note Capture {#note-capture}

### Purpose {#note-capture-purpose}

ユーザの入力中の自動保存と空 Note 破棄に責任を持つ。

### Subdomain Type {#note-capture-subdomain-type}

**core** — このプロダクトの差別化要因。

### Ubiquitous Language {#note-capture-ubiquitous-language}

- **Note** — body と時刻情報を持つ最小単位
- **AutoSave** — ユーザ入力中の継続的永続化

### Core Aggregates {#note-capture-core-aggregates}

- Note Aggregate

## Tag Management {#tag-management}

### Purpose {#tag-management-purpose}

...
```

## 注意

- **同じ単語が context 跨ぎで違う意味なら別 context**（古典的なユビキタス言語論）
- **core / supporting / generic の区別を必ず付ける**：投資配分の判断材料
- **このスキルは workflow を回さない**

## 次のアクション

Phase 3 完了後、ユーザに以下を提示：

- **通常パス**：`/ori-ddd-4-context-map` — context 間の関係（upstream/downstream、ACL、SharedKernel）を整理
- **早期切上げパス**：単一 context のみなら Phase 4 をスキップし `/ori-ddd-5-aggregates` へ
- **戻る**：context が決まらない場合は `/ori-ddd-2-event-storming` で event cluster を見直し
