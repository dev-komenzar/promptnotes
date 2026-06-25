---
name: ori-ddd-1-discovery
description: distill-ddd Phase 1（Discovery）。Core Domain と business driver を引き出し .ori/domain/discovery.md を生成する
---

ユーザが `/ori-ddd-1-discovery` を呼んだ際、distill-ddd Phase 1（Discovery）を ori convention 注入版で実行します。**Core Domain と business driver の言語化**が目的です。

## 役割

- **ファシリテーター**：ユーザの事業文脈・解きたい課題を質問で引き出す
- **ドメインエキスパート挑戦者**：曖昧な答えに「なぜそれが core なのか」を問い返す
- **記録係**：合意できた事項のみ `.ori/domain/discovery.md` に書き込む（推測で埋めない）

## 入力 / 出力

- 入力：ユーザとの対話（事業ドメイン、ターゲット利用者、競合差別化要因）
- 出力：`.ori/domain/discovery.md`
  - frontmatter: `ori:` ブロック（design.md §5 Frontmatter 規約に従う）
    - `node_id: discovery:overview`
    - `type: discovery`
    - `depends_on: []`（Phase 1 は pipeline 起点）
  - 個別 `persona:<id>` node は H3 anchor `{#<persona-id>}` から resolve される
  - H2/H3 すべてに `{#kebab-id}` アンカー（後段 phase の参照アンカーとして必須）

## 手順

1. **前提確認**：
   - `.ori/domain/discovery.md` が既にあれば内容を要約してユーザに提示し、「追記 / 上書き / 中断」を選ばせる
   - `.ori/` 自体がなければ `/ori-init` を先に走らせるよう案内
2. **対話で引き出す項目**（最低限）：
   - **{#problem-space}**：解決したい課題は何か。誰が困っているか
   - **{#core-domain}**：このプロダクトでなければ実現できない中核領域はどこか
   - **{#business-drivers}**：成功指標。何を増やし／減らしたいか
   - **{#non-goals}**：意図的にやらないこと（境界の言語化）
   - **{#stakeholders}**：主要なステークホルダーと関心
3. **挑戦質問**：
   - 「それは core か supporting か generic か」
   - 「競合と差別化する point は core 側にあるか」
   - 「業務指標と実装活動の対応はどうか」
4. **文書生成**：合意した内容のみ `.ori/domain/discovery.md` に Markdown で記述
5. `bash scripts/lint-domain.sh .ori/domain/discovery.md` を実行して自己検証
6. lint 失敗時は **1 回だけ** AI 側で自動修正を試み、それでも失敗ならユーザに判断を委ねる

## 出力テンプレート

```markdown
---
ori:
  node_id: discovery:overview
  type: discovery
  depends_on: []
---

# Discovery {#discovery}

## Problem Space {#problem-space}

...

## Core Domain {#core-domain}

...

## Business Drivers {#business-drivers}

...

## Non-Goals {#non-goals}

...

## Stakeholders {#stakeholders}

...
```

## 注意

- **推測で書かない**：ユーザが言語化していない事項を勝手に補完しない
- **このスキルは workflow を回さない**：実装は `/ori-flow` の責務
- distill-ddd 上流の本家 phase 1 prompt との差分は `ori:` frontmatter（design.md §5）と `{#id}` 必須ルールのみ

## 次のアクション

Phase 1 完了後、ユーザに以下のいずれかを提示：

- **通常パス**：`/ori-ddd-2-event-storming` — イベントストーミングで業務フローを洗い出す
- **スキップパス**：Phase 2-4（戦略設計）を省略して `/ori-ddd-5-aggregates` から始める
  - 適用条件：1 つの bounded context しか想定していない小規模プロジェクト
  - リスク：境界の見落とし。Phase 9 workflows で破綻したら遡って Phase 2 へ
- **中断**：discovery が固まらなければ「もう少し事業の前提を整理してから戻ってくる」も正解
