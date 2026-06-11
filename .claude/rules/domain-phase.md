---
paths:
  - ".ori/domain/**/*.md"
---

## Phase 別の構造規約

- **aggregates.md** (Phase 5): H2 = Aggregate（`## 1. Note Aggregate {#note-aggregate}`）。各 H2 配下に必須 H3: `構成要素`, `ビジネス不変条件`, `公開操作`
- **bounded-contexts.md** (Phase 3): H2 = Bounded Context、H3 = 責務 / Aggregate / ユビキタス言語 等
- **domain-events.md** (Phase 6): H2 = Event group、H3 = 個別 Event（H3 に `{#id}` 必須）
- **workflows/<id>.md** (Phase 9): H1 = ワークフロー名、必須 H2: `概要`, `ステージ`, `ステップ`
- **workflows/index.md**: 一覧表 + 未解決の問い。`<!-- ori:auto-table:start -->` ... `<!-- ori:auto-table:end -->` は ori が自動生成
- **ui-fields/screen-N.md** (Phase 11a): H2 = 画面、H3 = UI region（`{#id}` 必須）
- **ui-fields/index.md**: 検証エラー ↔ UI フィールドマッピング + 横断的事項
- **glossary.md** (Phase 8): 用語の一意性を保つ。同じ語に複数定義があれば bounded-contexts.md の言語差分表へ移す

## 共通

- 表の列は phase 間で再利用：`Aggregate / Event / Command / 問題` 等
- Mermaid 図は推奨（特に context-map、bounded-contexts）
- 「未解決の問い」セクションは feature 固有なら個別文書に、横断的なら index.md に
- distill-ddd の phase 順序を尊重：後方 phase が前方 phase を上書きしない
