---
paths:
  - ".ori/features/*/spec.md"
---

- **このファイルは派生文書**: `manifest.yaml` の `derives_from` が source
- **直接編集を原則禁止**:
  1. spec を変えたい場合は **派生元（domain doc）を編集** → `/ori-sync` で spec.md を再生成
  2. どうしてもここで編集する場合は **`/ori-sync --force <path>`** を実行。ori が `.ori/proposals/` に上流提案を自動生成する
- **構成（必須）**: `## 概要 {#overview}`, `## 入出力 {#io}`, `## 不変条件 {#invariants}`,
  `## 境界契約 {#boundary-contract}`, `## テスト観点 {#test-points}`, `## 実装ノート {#impl-notes}`
- **glossary 参照**: 用語は `[Note](#note)` 形式で glossary 内アンカーへ
- **コードスニペットは最小限**: 詳細実装は notes.md または src/ 配下に

## 境界契約 (Boundary contract) section 必須化 {#boundary-contract-section}

`## 境界契約 {#boundary-contract}` section は **必須**。
Slice DoD (`.apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md` の "Slice Definition of Done")
の rule 2「tests は外部境界経由のみ」を spec レベルで明示するための section。

宣言すべき項目:

- **boundary kind**: `tauri_command` / `http_handler` / `direct_public_entry` のいずれか
  (`manifest.yaml` の `expected_deliverables.boundary.kind` と一致させる)
- **contact point (binding)**: tests が import する **唯一の経路** を絶対 path で列挙。
  例:
  - `apps/<app>/src/<bc>/shared/ipc/bindings.ts` (tauri-specta 生成、TS 側 contact)
  - `apps/<app>/src-tauri/src/<bc_rs>/slices/<slice_rs>/commands.rs` (Rust 側 source、
    `#[tauri::command]` の置き場)
- **public_entry**: slice の `mod.rs` / `index.ts` 位置 (DoD rule 2 の代替経路、
  `cross_root` 不在の slice ではこちらが contact point になる)
- **禁止される import 経路**: 「内部 application 直叩き禁止」等、DoD 違反となる import を明示

### 記述例

```markdown
## 境界契約 {#boundary-contract}

- boundary kind: `tauri_command`
- TS 側 contact point: `apps/notes/src/note_taking/shared/ipc/bindings.ts`
  - tests は `import { commands } from "note-taking/shared/ipc/bindings"` のみ可
- Rust 側 source: `apps/notes/src-tauri/src/note_taking/slices/create_note/commands.rs`
  - `#[tauri::command]` + `#[specta::specta]` で公開、`invoke_handler!` に配線
- public_entry: `apps/notes/src/note_taking/slices/create-note/index.ts` (TS) /
  `apps/notes/src-tauri/src/note_taking/slices/create_note/mod.rs` (Rust)
- 禁止: tests が `application/handle_create_note` を直 import するのは DoD 違反
```

`/ori-doctor` はこの section の宣言と実体 boundary file の存在/参照を突合する。
section が欠落している spec は `/ori-derive` 段階で reject される。
