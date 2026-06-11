---
paths:
  - ".ori/features/*/spec.md"
---

- **このファイルは派生文書**: `manifest.yaml` の `derives_from` が source
- **直接編集を原則禁止**:
  1. spec を変えたい場合は **派生元（domain doc）を編集** → `/ori-sync` で spec.md を再生成
  2. どうしてもここで編集する場合は **`/ori-sync --force <path>`** を実行。ori が `.ori/proposals/` に上流提案を自動生成する
- **構成（推奨）**: `## 概要 {#overview}`, `## 入出力 {#io}`, `## 不変条件 {#invariants}`, `## テスト観点 {#test-points}`, `## 実装ノート {#impl-notes}`
- **glossary 参照**: 用語は `[Note](#note)` 形式で glossary 内アンカーへ
- **コードスニペットは最小限**: 詳細実装は notes.md または src/ 配下に
