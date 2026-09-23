---
name: ori-arch
description: `/ori-distill` の次のステップ。upstream framework init をユーザに案内し、`/ori-architect` スキル (要件対話) に委譲して `.ori/architecture.md` を動的生成する。pattern / stack の決定は委譲先で実施。
---

`/ori-distill` で `.ori/domain/` が作られた後の **次のステップ** (ori の大フロー
「1. DDD ドキュメント作成 = ori-init → ori-distill → ori-arch / 2. slice・page ごとの
ori-flow」のうち、**architecture.md 生成**に相当)。upstream の framework init
(`pnpm create vite@latest` 等) はユーザ自身に走ってもらい、最後に
**`/ori-architect` スキル**が要件対話 (platforms / os_integration / ui_native 等) から
`.ori/architecture.md` を 1 ファイル生成します (手順 3)。

> ori-c79 で固定 stack テンプレート (`stacks/<stack>/architecture.md.tpl`) の
> cartesian product 方式を撤廃。DDD + vsa-hex の核 (invariants) は不変、
> ビルド/配信/OS 統合の差は decision_points として対話で確定する。
> 旧 tpl の期待出力は golden test (`packages/skills/ori-arch/tests/golden-agent-vs-tpl.test.ts`)
> の GOLDEN 定数に引き継がれた。

## 設計原則 — 「upstream init guide → ori artifact 追加」二段構え

このスキルの責務は 2 つだけ (design.md §17 確定 / ori-c79 で agent 生成に転換)。
要件の決定 (pattern / stack / BC 名 = decision_points) は `/ori-architect` に一元化しており、
このスキルでは行わない:

1. **upstream framework init**：各 stack ごとの bash 手順 (`pnpm create vite@latest`, `pnpm create tauri-app` 等) を **ユーザに案内** する。bootstrap 系ファイル (`package.json`, `tsconfig.json`, `eslint.config.js`, `vitest.config.ts`, `.gitignore`, `README.md` 等) はここで生まれる。skill は自動実行しない (network / 対話 / 既存ファイル削除リスクを避ける)
2. **ori artifact 追加**：**`/ori-architect` スキルに委譲**して、
   `generation_procedure` (compose → generate → self-check) に従って
   `.ori/architecture.md` を 1 ファイルだけ生成させる (手順 3)。
   要件対話 (elicit → decide) は ori-architect がメイン session で実施し、生成結果は doctor の
   guardrails 検証 (g-1..g-8、lint.js) で機械チェックする。

`example-slice/` (`.apm/skills/ori-arch/patterns/<pattern>/stacks/<stack>/example-slice/`) は
**AI 専用の study material** であり target にはコピーしない。`/ori-flow new-slice <id>`
等で初回 slice を生成する際に AI が **on-demand で参照** し、ユーザの実ドメインに沿った
slice を直接生成する。「他人の `task-management` example を消して自分のものを書く」工数は発生しない。

## 手順

1. **前提確認**：
   - `ls .ori/config.yaml` が存在することを確認。なければ `/ori-init` を先に実行するよう案内
   - `apps/` 配下に既存の app があるか確認 (あれば overwrite するか聞く)

2. **upstream framework init をユーザに案内する** (skill は自動実行しない)：

   ```bash
   # ddd-vsa-hex / typescript の場合
   cd apps/<app-name> && pnpm create vite@latest . --template vanilla-ts
   ```

   ```bash
   # ddd-vsa-hex / typescript-tauri の場合
   cd apps/<app-name> && pnpm create vite@latest . --template vanilla-ts
   pnpm tauri init   # cwd: apps/<app-name>
   ```

   上記の bash 手順を README に貼って提示し、ユーザに実行してもらう。bootstrap 系
   (`package.json`, `tsconfig.json`, `eslint.config.js`, `vitest.config.ts`, `.gitignore`,
   `README.md` 等) はこの段階で揃う。

3. **`/ori-architect` に委譲して `.ori/architecture.md` を生成する**：

   `.apm/skills/ori-architect/SKILL.md` を起動する (メイン session で実行。
   要件対話が必要なため subagent にはしない — ori-8gz)。architect-expert の
   agent 定義 (ori-c79) はこのスキルへ書き直され、対話 → 生成 → self-check を
   一貫して担う。

   ```text
   /ori-architect (メイン session)
   ├── elicit   — questions: を順に提示し回答を確定 (推奨 + 上書き可)
   ├── decide   — decision_points 確定 (roots / layer_sets)
   ├── compose  — invariants から IR を組み立て
   ├── generate — .ori/architecture.md を書く (上書き可否を先に確認)
   ├── self-check — doctor: node .apm/skills/ori-doctor/scripts/lint.js .ori
   └── confirm  — ユーザ確定
   ```

   起動時、以下を ori-architect に渡す:
   - app 名 (`.ori/config.yaml` の `workspace.apps[0].name`)
   - upstream framework init 済みであること (手順 2)

   self-check で g-1..g-8 に fail した場合は ori-architect 内で修正を繰り返す
   (往復は ori-architect の confirm まで。ユーザ報告は ori-arch が担う)。

   > 旧 tpl render コマンドは tpl 廃止後 guidance のみを返す:
   > ```bash
   > node ./scripts/render-architecture.js --pattern ddd-vsa-hex --stack typescript
   > # → exit 2: "ori-architect スキルが要件対話から生成します"
   > ```
   > 参照用に `architecture.md.tpl` を保持している bundle がある場合のみ
   > `--patterns-dir <dir>` 付きで render を実行できる。

4. **runner deps をプロジェクト root package.json に追加する** (ori-bc9 / D5):

   生成された `architecture.md` の `scenario_test_runner.runner` に応じて、
   scenario 実行に必要な test runner deps を **root** package.json に追加する
   (ori-init の `install-tauri-scaffold.sh` が Cargo.toml へ `cargo add` するのと同型 precedent。
   `.ori/scenarios/` 配下の test は root node_modules から解決するため置き場所は root 固定)：

   | runner | 追加コマンド (project root で実行) |
   |---|---|
   | `playwright` | `pnpm add -D @playwright/test` |
   | `wdio` | `pnpm add -D @wdio/cli @wdio/local-runner webdriverio @wdio/tauri-service` |
   | `vitest` | `pnpm add -D vitest` (unit test で導入済みなら skip) |

   - root package.json が無い場合は `pnpm init` で作ってから追加する
   - network 等で失敗した場合はコマンドを提示して先へ進む (fail-safe。blocking しない)
   - runner の実行要件がある場合は追加で案内する (wdio + tauri: WebKitGTK / tauri-driver
     等の環境手順は nix devShell 化を推奨 — 詳細は ori-architect SKILL.md の runtime recipes)

5. **example-slice/ への参照導線を AI に思い出させる**：
   - 初回 slice を作るとき `/ori-flow new-slice <id>` で必ず
     `.apm/skills/ori-arch/patterns/<pattern>/stacks/<stack>/example-slice/` を **読んでから**
     ユーザ固有 domain の slice を生成すること (target にコピーしない)
   - 構造規約 / public_entry / cross-slice 禁止のような不変則は `architecture.md` 由来、
     具体的な実装スタイル (Result 型のシグネチャ等) は `example-slice/` 由来

## 注意

- **上書きは確認してから**：既存 `.ori/architecture.md` がある場合は agent の
  `generation_procedure` (confirm step) で上書き可否を確認する
- **.ori/ skeleton は壊さない**：書き出すのは `.ori/architecture.md` 1 ファイルのみ。`/ori-init` が作る `.ori/config.yaml` 等とは衝突しない
- **`apps/` は生成しない**：upstream framework init の責務
- **patterns/ 探索順** (`render-architecture.js`、Phase K2 / R2 で簡略化)：
  1. `--patterns-dir <dir>` 引数
  2. skill bundle 隣接 (`.apm/skills/ori-arch/patterns/` — bundle が住む場所がどこであっても `patterns/` は sibling)
- **CLI 拡張は禁止** (`ori-execution-model-shift-2026-06-03`)：新機能はこのスキル + scripts/ で実装する
- **`phase_hooks` block は必須出力** (ori-fzr.11 / 2026-06-26)：生成する `.ori/architecture.md` の
  frontmatter に `phase_hooks:` block を含めること (hook 不要な stack は `phase_hooks: {}`)。
  `/ori-flow` / `/ori-doctor` がこの block を読んで Slice DoD rule 4 (`pattern.md`) の
  bindings 再生成を invoke する。schema 詳細は `architecture-md-schema.md` の "phase_hooks" を参照。
  期待値は golden test の `GOLDEN[*].phase_hooks` に固定されている (ori-c79.6)

## Migration — phase_hooks 未保有の既存 architecture.md

ori-fzr.11 (2026-06-26) 以前に `/ori-arch` で生成された `.ori/architecture.md` は frontmatter に `phase_hooks:` block を持たない。`/ori-flow` が phase 終端で binding 再生成 hook を invoke しないため Slice DoD rule 4 が手動運用に退化する (`pattern.md` 参照)。

移行手順:

1. **ori-architect で再生成する場合 (推奨)**: `/ori-architect` スキルに要件対話から
   `.ori/architecture.md` を再生成させる (この SKILL の手順 3)。手で加えていた変更は
   事前に diff を取って merge し直すこと。

2. **手で最小追加で済ませる場合**:
   - typescript-tauri stack: `packages/skills/ori-arch/tests/fixtures/golden-constants.ts`
     の `GOLDEN.typescriptTauri.phase_hooks` 相当の block を frontmatter 末尾に貼る
     (app 名 / BC 名は実値に置換)
   - typescript stack (cross_root 無し): `phase_hooks: {}` を 1 行だけ追加

migration 完了の確認は `node ./scripts/check.js` (adapter check) が pass し、かつ `grep -E '^phase_hooks:' .ori/architecture.md` が hit すること。

## Architecture Export / Check スクリプト

`scripts/` 配下の JS スクリプトで `.ori/architecture.md` を adapter 経由でコンパイル・検証できます。
script path は skill bundle に対する相対 (`./scripts/<x>.js`) で統一します — install 場所
(ori repo dev / apm consumer / Claude Code 統合済 consumer) に依存しません。

```bash
# eslint.config.js を生成
node ./scripts/export.js --adapter=eslint

# Rust 向け arch test を生成
node ./scripts/export.js --adapter=rust --root=rs

# dry-run (ファイル出力なし)
node ./scripts/export.js --adapter=eslint --dry-run

# adapter の native linter で違反チェック
node ./scripts/check.js --adapter=eslint

# ui-fields から ## Page Map セクションを自動更新
node ./scripts/sync-page-map.js

# dry-run
node ./scripts/sync-page-map.js --dry-run
```

オプション (export / check 共通)：
- `--adapter=<name>` — adapter 指定 (省略時は architecture.md の `adapter:` フィールドを使用)
- `--root=<id>` — multi-root 対象 (省略時は `default_root`)
- `--spec=<path>` — spec ファイルパス (省略時: `.ori/architecture.md`)

## 次のアクション

`/ori-arch` 完了後、ユーザに以下を提示：

- **動作確認パス**：upstream init 出力が正しく走るかを smoke チェック
  - `typescript` (vite vanilla-ts)：`pnpm install && pnpm build`（vanilla-ts には `test` script が無いため `build` で代用）
  - `typescript-tauri`：`pnpm install && pnpm build`（Rust 側は `pnpm tauri build` で確認可、初回は時間がかかる）
- **最初の slice 作成パス**：`/ori-flow new-slice <id>` で新 slice を scaffold → 7-phase 開発を回す
- **scenario scaffold パス**: `node .apm/skills/ori-flow/scripts/new-scenario.js --list-validation` で validation.md の未 cover section（scenario 候補）を確認 → ユーザ確認の上 `new-scenario.js <id>` で scaffold → `/ori-flow <id>`（4 phase。scenario id = validation section anchor、1:1）
- **domain 起点で進めるパス**：`/ori-distill phase=discovery` で distill-ddd phase 1 から domain を立ち上げる
- **既存 domain がある場合のパス**：`/ori-migrate` で `docs/domain/` 等を `.ori/domain/` に昇格
