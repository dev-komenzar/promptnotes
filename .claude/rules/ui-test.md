---
paths:
  - "**/*.{spec,test}.tsx"
  - "**/e2e/**/*.{spec,test}.{ts,tsx}"
  - "**/playwright/**/*.{spec,test}.{ts,tsx}"
  - "**/__tests__/**/*.{spec,test}.{ts,tsx}"
---

# UI テスト規約（ポインタ）

> UI selector / testid 命名の **canonic source** を pattern/stacks 正典へ一本化済み。
> 本ファイルはポインタを持ち、concretion を二重管理しない。

## 正典ポインタ

| 関心事 | 正典 |
| --- | --- |
| 層別 selector 優先順位 (Component: role / E2E: testid) | `.apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md` "Test conventions" → "UI selector / testid 規約" |
| testid 命名 (VSA namespace, `.` separator, `<elem>` 機能名) | 同上 |
| production fixture (`setupProductionBuilder()`) | `.apm/skills/ori-arch/patterns/ddd-vsa-hex/stacks/typescript-tauri/test.md` "#setup-production-builder" |
| 実装側規約 (Smart Constructor / Result / VSA 配置) | `ddd-typescript.instructions.md` |

## 責務分離

- 本ファイル = UI テスト層に効く rules の **glue**（`applyTo` で TS テストファイルに適用）。
- selector / testid / fixture の規範本文は pattern/stacks 正典が担う。
- domain test のメタルールは `ddd-test.instructions.md`、シナリオ E2E は `scenario-test.instructions.md`。

UI コンポーネントの単体テスト (`*.test.tsx`) は `ddd-test`（メタルール）と本ファイル
（UI 層）が補完的に効く。`ddd-test` は「spec トレース / adapter 境界 mock の心構え」、
本ファイルは「DOM query 時に getByRole / data-testid を VSA 命名で使う」と直交する。
