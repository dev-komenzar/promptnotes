---
paths:
  - ".ori/**/*.md"
---

- **見出しID必須**: 全 H2/H3 に `## Heading text {#kebab-case-id}` の形でアンカーを付与。順序番号を含めない意味的命名（`note-aggregate` ✅、`1-aggregate` ❌）
- **派生文書の保護**: frontmatter に `coherence.derives_from:` がある場合、この文書は派生。直接編集しない（`/ori-sync --force` は廃止済）。派生元を編集して `/ori-sync` → `/ori-flow` で再生成し、上流の変更が要る場合は `/ori-propose` を使うこと
- **編集後の同期**: 編集を終えたら `/ori-sync` を実行（APM hook が自動起動するが、確実性のため明示呼出推奨）
- **frontmatter は YAML**: `---` で囲み、ファイル最上部に置く
- **言語**: 日本語/英語の混在を許容。用語は `.ori/domain/glossary.md` の定義に従う
- **同一ファイル内で section id をユニークに**: 重複があるとエラーとして扱う
- **`.ori/` 第一級概念**: `.ori/` ディレクトリには slice / page / scenario の 3 つの第一級概念が存在する
  - `.ori/slices/<id>/`: 1 use case = 1 handler（単一サービス内）
  - `.ori/pages/<id>/`: UI composition unit（単一サービス内）
  - `.ori/scenarios/<id>/`: サービス横断 E2E 検証単位（複数サービス横断）。**scenario id は validation.md の section anchor と 1:1。複数 section を 1 つの scenario にまとめないこと**（section 統合は validation.md 側で先に行う）
- **skill / instructions からの参照 (ori 本体の執筆規約)**: APM は skill folder を `.claude/skills/<name>/` / `.agents/skills/<name>/` へ copytree するだけで、consumer に `.apm/` は存在しない。`.apm/skills/` 配下の md と `.apm/instructions/` に `.apm/` 始まりの path を書かない
  - 実行する script は skill root 相対 (`node scripts/x.js`)。他 skill の script は `scripts/build-skills.mjs` の `SHARED_ENTRIES` で自 skill の `scripts/` に複製してから参照する (script が自分の位置から読む template 等も `assets` で複製する)
  - 読むだけの参照 (instructions / 他 skill の asset / SKILL.md / agent) は、書いているファイルからの相対 markdown link `[text](../../instructions/x.instructions.md#id)` で書く。repo ではそのまま解決し、consumer では APM が install 時に `apm_modules/` 内の実体を指す path へ書き換える (APM #1147。code span 内も対象)
  - APM が書き換えるのは実在ファイルへの link だけ。dir への link は書き換えないので、兄弟 skill 内の dir (`../ori-architect/patterns/.../example-slice/`) に限る (skill は兄弟 dir に配置されるので解決する)
  - 検査: `packages/skills-shared/tests/skill-references.test.ts`
