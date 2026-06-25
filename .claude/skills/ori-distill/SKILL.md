---
name: ori-distill
description: distill-ddd phase 1-11 + ori 独自 phase 11b（Page grouping）を対話的に進める
---

ユーザが `/ori-distill [phase=<name>]` を呼んだ際、distill-ddd の対話フローを ori convention 注入版で実行します。

## phase 一覧

1. **discovery** — Core domain & business drivers
2. **storming** — Event Storming
3. **contexts** — Bounded Contexts & Subdomains
4. **mapping** — Context Map
5. **aggregates** — Aggregate design（H2 = Aggregate、`{#id}` 必須）
6. **events** — Domain events
7. **validate** — Use case scenarios
8. **glossary** — Ubiquitous language
9. **workflows** — DMMF pipelines（**ori はファイル分割：`workflows/{index.md, <id>.md}`**）
10. **types** — Compilable type definitions
11. **simulate** — Type-level scenario verification + ui-fields
11b. **ui-grouping** *(ori 独自)* — `ui-fields/screen-*.md` の depended_by を決定し page 群を確定

## 各 phase の出力

- `.ori/domain/<phase-name>.md` (Phase 1-8、10)
- `.ori/domain/workflows/<id>.md` (Phase 9、ファイル分割)
- `.ori/domain/ui-fields/screen-N.md` (Phase 11a、画面分割)
- 全ファイルに `{#id}` 必須 + `ori:` frontmatter（design.md §5: `node_id` / `type` / `depends_on`）

## 手順

1. **`.ori/domain/` の既存文書を確認**：`for f in .ori/domain/*.md; do bash scripts/lint-domain.sh "$f"; done` を実行
2. **指定 phase（または最も近い未完 phase）から開始**
3. 対話的に質問を投げ、ユーザの返答を文書に反映
4. `bash scripts/lint-domain.sh .ori/domain/<generated-file>.md` を実行して生成ファイルを自己検証
5. **Phase 9 完了時**: workflow ごとに新規 slice 作成を提案（手動 OK / 自動 OK の選択）
6. **Phase 11b 完了時**: 各 page について同様に新規 page 作成を提案

## 注意

- distill-ddd 本家のアップストリーム変更があれば手動で取り込む（fork 形態）
- このスキルは **`ori-flow` を呼ばない**。slice / page 単位の実装は別途
