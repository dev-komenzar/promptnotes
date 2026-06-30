---
paths:
  - "apps/*/src/**/*.ts"
---

## モジュール構造

- code 配置の base は **`apps/<app>/src/`**（`<app>` は `.ori/config.yaml` `workspace.apps[]` で resolve）
- 各 Bounded Context は **`apps/<app>/src/<bc>/`**：
  - `domain/`: BC 共有 aggregate / event 等の Phase 10 types 生成領域
  - `shared/contracts/events/`: Phase 6 events
  - `shared/events/event-bus.ts`: `@ori-generated`
  - `shared/ipc/bindings.ts`: tauri-specta 生成。**slice tests の唯一の contact point**
  - `shared/test-fixtures/`: production wiring fixture (`setupProductionBuilder()`) を置く
  - `slices/<slice-id>/{domain,application,infrastructure,presentation,tests}/`:
    - `domain/`: slice 固有の VO / command DTO（純粋関数のみ）
    - `application/`: ユースケース handler
    - `infrastructure/`: adapter、副作用（Tauri command、ファイル I/O 等）
    - `presentation/`: UI fragment。**slice DoD で必須** (下記参照)
    - `tests/`: impl と co-locate（vitest）。**bindings 経由のみ** (下記参照)

## Value Object

- **Branded type 必須**: `type NoteId = string & { readonly __brand: 'NoteId' }`
- **Smart Constructor**: `tryNewNoteId(raw: string): Result<NoteId, NoteIdError>`
- **同値性は値で判定**：`===` で動くよう primitive ベース

## Aggregate

- Root の不変条件はコンストラクタ/メソッドで保護
- **Command メソッド**: 新 state + 発行 Events の組を返す: `{ state: Note, events: NoteEvent[] }`
- **副作用なし**：`Date.now()` やファイル I/O は引数で受け取る

## Workflow (DMMF style)

- Pipeline 関数: `loadConfig → scanVault → hydrateFeed → initSession`
- **各 stage の中間型を別々の型として定義**：コンパイラが「段階」を強制
- エラーは `Result<Ok, Err>` で表現、throw 禁止
- 副作用は pipeline 境界 (`infrastructure/`) に注入

## 型

- `any` 禁止。`unknown` を narrowing で扱う
- `Result` 型は neverthrow など外部 lib を使う（ori MVP では neverthrow を推奨）

## Slice 完了 (DoD) における必須事項 {#slice-dod-required}

`.apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md` の "Slice Definition of Done"
を TypeScript 側で具体化する規約。

### `presentation/` は slice 完了の必須サブレイヤ {#presentation-required}

- UI を持つ slice (`slice_kind: page` または UI を露出する `command`/`query`) は
  `slices/<slice-id>/presentation/` 配下に **少なくとも 1 つの公開コンポーネント** を
  置くこと。`presentation/` が空 / placeholder のままの slice は DoD rule 1 違反
- presentation 層は同 slice の `application/` を **`application/` の名前付き export
  のみ** から触る。`infrastructure/` / `domain/` を直 import するのは禁止
- `presentation/` の export は slice の `index.ts` から re-export し、外部
  (ui-widget / ui-page / 他 BC) は `index.ts` 経由でのみ参照する (`public_entry`
  単一化)
- UI を持たない純 backend slice (`workflow` type / 内部 query 等) で
  `presentation/` を省略する場合は、`manifest.yaml` の `expected_deliverables.sub_layers`
  からも `presentation` を外し、矛盾を残さないこと

### test は `shared/ipc/bindings` 経由のみ {#test-binding-only}

DoD rule 2 (boundary 経由 test) を TS で具体化する規約。
**slice の DoD test (`slices/<slice-id>/tests/`) は以下の制約を満たす**:

- 唯一許される command surface import は
  `import { commands } from "<bc>/shared/ipc/bindings";`
  (tauri-specta が `commands.rs` から生成、`ddd-rust.instructions.md` 参照)
- **以下は全て DoD 違反**:
  - `application/` の handler を直 import (`import { handleCreateNote } from "../application/..."`)
  - `infrastructure/` の adapter を直 import
  - 他 slice の `application/` / `infrastructure/` を直 import
  - `@tauri-apps/api/core` の `invoke` を直接呼ぶ
- production wiring 強制 (DoD rule 3): tests は
  `setupProductionBuilder()` で fixture を組む (`ui-test.instructions.md` 参照)

DoD rule 2/3 を満たさない test は、対象 slice の sub_layers が埋まっていても
"green" とは見なされない。`/ori-doctor` が import 経路を AST レベルで検査し、
違反を `dod-violation` label 付き issue として起票する
(`task-management.instructions.md` 参照)。

### 内部 fake/mock test の扱い

- `application/` 内に co-locate された fake/mock を使った unit test (orchestration 検証)
  は許容するが、**DoD カウントから除外** する (DoD rule 3)
- DoD カウントに含めたい test は必ず `slices/<slice-id>/tests/` 配下に置き、
  bindings + `setupProductionBuilder()` 経由で動かす

## 禁止事項

- **`.ori/slices/<slice-id>/src/` への code 出力は禁止**。`.ori/slices/<slice-id>/` は SSoT メタ専用（manifest.yaml / spec.md / plan.md / review.md / status.yaml / notes.md のみ）
- code（impl + tests）は必ず `apps/<app>/src/<bc>/slices/<slice-id>/` 配下に置く
- slice tests が `application/` / `infrastructure/` を直 import するのは禁止 (上記 DoD 規約)
