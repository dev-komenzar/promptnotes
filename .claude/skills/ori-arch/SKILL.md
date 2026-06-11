---
name: ori-arch
description: pattern (DDD-VSA-Hex 等) と stack (typescript / typescript-tauri 等) を決定し、upstream framework init をユーザに案内した上で `.ori/architecture.md` を生成する。`/ori-init` の next step。
---

`/ori-init` で `.ori/` skeleton が作られた後の **次のステップ**。pattern / stack を対話で決め、
upstream の framework init (`pnpm create vite@latest` 等) はユーザ自身に走ってもらい、
最後に `.apm/skills/ori-arch/patterns/<pattern>/stacks/<stack>/architecture.md.tpl` を render して
target の `.ori/architecture.md` を書き出します。

## 設計原則 — 「decide → upstream init guide → ori artifact 追加」三段構え

このスキルの責務は 3 つだけ (design.md §17 確定 / 2026-06-07):

1. **decide**：ユーザに pattern / stack を聞き、placeholder 値 (app 名, BC 名) を確定する
2. **upstream framework init**：各 stack ごとの bash 手順 (`pnpm create vite@latest`, `pnpm create tauri-app` 等) を **ユーザに案内** する。bootstrap 系ファイル (`package.json`, `tsconfig.json`, `eslint.config.js`, `vitest.config.ts`, `.gitignore`, `README.md` 等) はここで生まれる。skill は自動実行しない (network / 対話 / 既存ファイル削除リスクを避ける)
3. **ori artifact 追加**：`scripts/render-architecture.js` を呼んで `architecture.md.tpl` を render し、target の `.ori/architecture.md` を 1 ファイルだけ書き出す。これ以外 ori は target にファイルを足さない

`example-slice/` (`.apm/skills/ori-arch/patterns/<pattern>/stacks/<stack>/example-slice/`) は
**AI 専用の study material** であり target にはコピーしない。`/ori-flow new-slice <id>`
等で初回 slice を生成する際に AI が **on-demand で参照** し、ユーザの実ドメインに沿った
slice を直接生成する。「他人の `task-management` example を消して自分のものを書く」工数は発生しない。

## 手順

1. **前提確認**：
   - `ls .ori/config.yaml` が存在することを確認。なければ `/ori-init` を先に実行するよう案内
   - `apps/` 配下に既存の app があるか確認 (あれば overwrite するか聞く)

2. **pattern を選んでもらう**：
   - **ddd-vsa-hex** (default)：DDD 文脈、Vertical Slice Architecture、Hexagonal port-adapter
   - 将来：`hex` / `layered` などを追加予定 (現状は ddd-vsa-hex のみ実装)

3. **stack を選んでもらう**：
   - **typescript** (default)：Vite/Node 等で動く pure TypeScript
   - **typescript-tauri**：上記 + Tauri v2 (Rust 側 IPC bindings 付き)
   - **pattern × stack のマトリクス**：`.apm/skills/ori-arch/patterns/<pattern>/stacks/<stack>/architecture.md.tpl` が存在する組合せのみ有効。`render-architecture.js` は未知組合せで exit 2 + 利用可能な値を列挙する

4. **bounded context 名 (`{{BC_NAME}}`) を決める**：
   - default は `task-management` (kebab-case)
   - Rust 側 (`{{BC_NAME_RS}}`) は kebab→snake で自動導出 (`task-management` → `task_management`)。明示したい場合のみ `--bc-rs` で指定

5. **upstream framework init をユーザに案内する** (skill は自動実行しない)：

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

6. **`.ori/architecture.md` を render する**：

   ```bash
   # 実行 path は環境により異なる:
   # - ori repo dev (cwd = ori 本体): .apm/skills/ori-arch/scripts/render-architecture.js
   # - apm install consumer (cwd = ユーザ project): apm_modules/dev-komenzar/ori/.apm/skills/ori-arch/scripts/render-architecture.js
   # - Claude Code 統合済 consumer: .claude/skills/ori-arch/scripts/render-architecture.js
   node <render-architecture-path> \
     --pattern ddd-vsa-hex \
     --stack typescript \
     --bc task-management
   ```

   オプション:
   - `--pattern <name>`     pattern (required)
   - `--stack <name>`       stack (required)
   - `--app <name>`         app 名 override (省略時は `.ori/config.yaml` の `workspace.apps[0].name`)
   - `--bc <name>`          BC 名 (default `task-management`)
   - `--bc-rs <name>`       Rust 側 BC 名 (省略時は `--bc` から kebab→snake 自動導出)
   - `--force`              既存 `.ori/architecture.md` を上書き
   - `--dest <dir>`         書き出し先 (default cwd)
   - `--patterns-dir <dir>` patterns root (default は skill bundle 隣接の `.apm/skills/ori-arch/patterns/`。Phase K2 で env var / fallback は廃止)

   exit code: `0` success / `1` IO error / `2` usage error (未知 pattern/stack, app 名解決失敗, rendered spec invalid)

7. **example-slice/ への参照導線を AI に思い出させる**：
   - 初回 slice を作るとき `/ori-flow new-slice <id>` で必ず
     `.apm/skills/ori-arch/patterns/<pattern>/stacks/<stack>/example-slice/` を **読んでから**
     ユーザ固有 domain の slice を生成すること (target にコピーしない)
   - 構造規約 / public_entry / cross-slice 禁止のような不変則は `architecture.md` 由来、
     具体的な実装スタイル (Result 型のシグネチャ等) は `example-slice/` 由来

## 注意

- **生成は idempotent**：既存 `.ori/architecture.md` は default で skip。`--force` でのみ上書き
- **.ori/ skeleton は壊さない**：書き出すのは `.ori/architecture.md` 1 ファイルのみ。`/ori-init` が作る `.ori/config.yaml` 等とは衝突しない
- **`apps/` は生成しない**：upstream framework init の責務
- **patterns/ 探索順** (`render-architecture.js`、Phase K2 / R2 で簡略化)：
  1. `--patterns-dir <dir>` 引数
  2. skill bundle 隣接 (`.apm/skills/ori-arch/patterns/` — bundle が住む場所がどこであっても `patterns/` は sibling)
- **CLI 拡張は禁止** (`ori-execution-model-shift-2026-06-03`)：新機能はこのスキル + scripts/ で実装する

## Architecture Export / Check スクリプト

`scripts/` 配下の JS スクリプトで `.ori/architecture.md` を adapter 経由でコンパイル・検証できます。
スクリプト path は環境により異なります (上記 step 6 と同じ規約):

- ori repo dev: `.apm/skills/ori-arch/scripts/<script>.js`
- apm install consumer: `apm_modules/dev-komenzar/ori/.apm/skills/ori-arch/scripts/<script>.js`
- Claude Code 統合済 consumer: `.claude/skills/ori-arch/scripts/<script>.js`

以下、`<script-base>` を上記いずれかに置き換えて実行してください:

```bash
# eslint.config.js を生成
node <script-base>/export.js --adapter=eslint

# Rust 向け arch test を生成
node <script-base>/export.js --adapter=rust --root=rs

# dry-run (ファイル出力なし)
node <script-base>/export.js --adapter=eslint --dry-run

# adapter の native linter で違反チェック
node <script-base>/check.js --adapter=eslint

# ui-fields から ## Page Map セクションを自動更新
node <script-base>/sync-page-map.js

# dry-run
node <script-base>/sync-page-map.js --dry-run
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
- **domain 起点で進めるパス**：`/ori-distill phase=discovery` で distill-ddd phase 1 から domain を立ち上げる
- **既存 domain がある場合のパス**：`/ori-migrate` で `docs/domain/` 等を `.ori/domain/` に昇格
