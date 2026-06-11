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
  - `slices/<slice-id>/{domain,application,infrastructure,presentation,tests}/`:
    - `domain/`: slice 固有の VO / command DTO（純粋関数のみ）
    - `application/`: ユースケース handler
    - `infrastructure/`: adapter、副作用（Tauri command、ファイル I/O 等）
    - `presentation/`: UI fragment（必要時）
    - `tests/`: impl と co-locate（vitest）

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

## 禁止事項

- **`.ori/slices/<slice-id>/src/` への code 出力は禁止**。`.ori/slices/<slice-id>/` は SSoT メタ専用（manifest.yaml / spec.md / plan.md / review.md / status.yaml / notes.md のみ）
- code（impl + tests）は必ず `apps/<app>/src/<bc>/slices/<slice-id>/` 配下に置く
