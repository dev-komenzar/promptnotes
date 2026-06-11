---
paths:
  - "**/*.{spec,test}.tsx, **/e2e/**/*.{spec,test}.{ts,tsx}, **/playwright/**/*.{spec,test}.{ts,tsx}, **/__tests__/**/*.{spec,test}.{ts,tsx}"
---

UI 要素の selector 規約。`ddd-test.instructions.md` (domain 層: fast-check + spec.md トレース) と責務を分離し、本ファイルは **プレゼンテーション層から UI 要素を引く際のルール** に限定する。ori-adopting プロジェクトが UI framework を採用した場合に適用される。

## 1. 層別デフォルト

- **Component test 層** (Vitest + Testing Library 等) — `getByRole` / `getByLabelText` を **第一推奨**。`data-testid` (`getByTestId`) は role 不能時の fallback のみ
  - Why: Testing Library 公式の優先順位。アクセシビリティ tree に近い query は a11y 回帰も同時に検出できる
  - 例: ボタンは `screen.getByRole('button', { name: /complete/i })`、フォームは `screen.getByLabelText('Task title')`
- **E2E 層** (Playwright 等) — `data-testid` を **第一推奨**。`testIdAttribute: 'data-testid'` を Playwright config に設定し、`page.getByTestId(...)` で引く
  - Why: Playwright 公式の優先順位。E2E は実 DOM・実 CSS の組合せで role 安定性が落ちるため testid が破綻しにくい

## 2. testid 命名規則 (VSA 構造を反映)

VSA + ddd-vsa-hex 構造 (`apps/<app>/src/<bc>/slices/<slice-id>/`, `apps/<app>/src/ui-{widget,page}/<id>/`) を testid namespace に直接マップする:

| 配置 | testid pattern | 例 |
| --- | --- | --- |
| slice presentation の集約要素 | `<slice-id>` | `data-testid="complete-task"` (slice の root container) |
| slice presentation の子要素 | `<slice-id>.<elem>` | `data-testid="complete-task.submit"` |
| ui-widget | `widget.<id>.<elem>` | `data-testid="widget.task-list.row"` |
| ui-page | `page.<id>.<elem>` | `data-testid="page.tasks.header"` |
| shared (BC 共有 UI) | `shared.<area>.<elem>` | `data-testid="shared.toast.message"` |

- **separator は `.`** (kebab-case の中の `-` と衝突しない、grep が安定)
- **BC prefix は collision 時のみ escalation**: 通常は `<slice-id>` で一意になるよう slice ID を選ぶ。複数 BC で同名 slice が出た場合のみ `<bc>.<slice-id>.<elem>` に拡張
- **`<elem>` は機能名** (`submit` / `cancel` / `row` / `header` / `empty-state`) 。実装詳細名 (`button1` / `div-wrapper`) 禁止
- **動的要素は append**: list row なら `data-testid="widget.task-list.row"` (固定) + `data-key={task.id}` (動的)、selector 側で `getByTestId('widget.task-list.row').filter({ has: page.getByText(taskTitle) })` 等で絞る

## 3. 対象範囲

- UI framework (React / Solid / Svelte 等) を採用した **ori-adopting プロジェクト** に適用
- ori 本体の pattern stack (`.apm/skills/ori-arch/patterns/<p>/stacks/<s>/`) と example-slice は UI framework を内包しない方針 — 本規約は **upstream framework init + `/ori-flow` で生成された slice の中で UI を足したときの規約** であり、ori 側に Playwright / Testing Library 等を default dep として持たせない (UI framework は upstream init で選択される)

## 4. prod ビルドでの testid strip

- **デフォルトは残す** (debug / 外部ツール / バグ報告での再現性のため有用)
- bundle size が critical なプロジェクトは downstream で bundler plugin を追加する (例: `@swc/plugin-remove-data-testids`, vite の `define` 経由 strip)。ori 側では同梱しない

## 5. Tauri / native shell 変種

- 命名規則・selector 優先順位は同じ
- E2E ランナー選択 (tauri-driver / Playwright + webview / WebdriverIO / Selenium 等) は本規約の **scope 外**。各プロジェクトが採用する E2E スタックに本ファイルの testid 規約を被せる前提

## 責務分離

- 本ファイル = UI selector / testid namespace の規約
- [`ddd-test.instructions.md`](ddd-test.instructions.md) = domain test (vitest + fast-check + spec.md トレース) の規約
- [`ddd-typescript.instructions.md`](ddd-typescript.instructions.md) = 実装側 (Smart Constructor / Result 型 / VSA 配置) の規約

UI コンポーネントの単体テスト (`*.test.tsx`) は両方 (ddd-test + ui-test) が applyTo にマッチし得る。両者は補完的で、ddd-test は「Result の比較や VO の property test を書け」、ui-test は「DOM を query する時は getByRole / data-testid を VSA 命名で使え」と直交する。
