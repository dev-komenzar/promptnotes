---
paths:
  - ".ori/**/*.md"
---

- **見出しID必須**: 全 H2/H3 に `## Heading text {#kebab-case-id}` の形でアンカーを付与。順序番号を含めない意味的命名（`note-aggregate` ✅、`1-aggregate` ❌）
- **派生文書の保護**: frontmatter に `coherence.derives_from:` がある場合、この文書は派生。直接編集する前に `/ori-sync --force <path>` を実行して proposal を自動生成すること
- **編集後の同期**: 編集を終えたら `/ori-sync` を実行（APM hook が自動起動するが、確実性のため明示呼出推奨）
- **frontmatter は YAML**: `---` で囲み、ファイル最上部に置く
- **言語**: 日本語/英語の混在を許容。用語は `.ori/domain/glossary.md` の定義に従う
- **同一ファイル内で section id をユニークに**: 重複があるとエラーとして扱う
