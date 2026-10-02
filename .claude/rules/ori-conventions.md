---
paths:
  - ".ori/**/*.md"
---

- **見出しID必須**: 全 H2/H3 に `## Heading text {#kebab-case-id}` の形でアンカーを付与。順序番号を含めない意味的命名（`note-aggregate` ✅、`1-aggregate` ❌）
- **派生文書の保護**: frontmatter に `coherence.derives_from:` がある場合、この文書は派生。原則として派生元 (domain doc) を編集する。`/ori-sync --force` による proposal 自動生成は未実装 (MVP stub) のため、現状の手動手順は [feature-spec.md](feature-spec.md) 参照
- **編集後の同期**: 編集を終えたら `/ori-sync` を実行（ただし現状は検知・伝播が未実装の stub で、dirty/hash は手動メンテ。[feature-spec.md](feature-spec.md) 参照）
- **frontmatter は YAML**: `---` で囲み、ファイル最上部に置く
- **言語**: 日本語/英語の混在を許容。用語は `.ori/domain/glossary.md` の定義に従う
- **同一ファイル内で section id をユニークに**: 重複があるとエラーとして扱う
- **`.ori/` 第一級概念**: `.ori/` ディレクトリには slice / page / scenario の 3 つの第一級概念が存在する
  - `.ori/slices/<id>/`: 1 use case = 1 handler（単一サービス内）
  - `.ori/pages/<id>/`: UI composition unit（単一サービス内）
  - `.ori/scenarios/<id>/`: サービス横断 E2E 検証単位（複数サービス横断）
