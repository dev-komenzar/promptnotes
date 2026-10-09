---
name: ori-bootstrap
description: `/ori-architect` の次のステップ。`.ori/architecture.md` から stack を確定して upstream framework init を案内し、runner deps の追加と tauri specta scaffold の apply を行い、app が build 可能かを readiness verify (静的 + build) で PASS/FAIL 判定する。app の bootstrap ファイルは生成しない。
---

ori の大フロー **DDD (`/ori-init` → `/ori-distill`) → `/ori-architect` → `/ori-bootstrap` → `/ori-flow`**
のうち、**codebase init** に相当する step。`/ori-architect` が確定した `.ori/architecture.md`
(roots / workspace.apps[].runtime / scenario_test_runner) を入力に、app を
「scenario / slice を書き始められる = build できる」状態まで guide + verify する。

## 役割 {#role}

- stack の確定 (roots の language 構成 → `typescript` / `typescript-tauri`) と upstream framework init の **案内** (自動実行しない)
- runner deps を project root `package.json` に追加 (`scenario_test_runner.runner` 由来)
- stack=typescript-tauri の specta scaffold を apply (`ori-init` bundle の `install-tauri-scaffold.sh`)
- readiness verify: 静的チェック + `runtime.build` 実走 → PASS / FAIL と修正動線を返す

ori-5er の原則「ori は `apps/` 配下に bootstrap を書かない」を維持する。`package.json` /
`tsconfig.json` / `Cargo.toml` 等は upstream init (vite / tauri CLI) に委譲する。

## 入力 / 出力 {#io}

- 入力：
  - `.ori/architecture.md` (`/ori-architect` 生成。無ければ FAIL → `/ori-architect` へ戻す)
  - `apps/<app>/` (未初期化でよい。upstream init はこの skill が案内する)
- 出力：
  - root `package.json` の devDependencies (runner deps) — `pnpm add -D` 経由
  - typescript-tauri のみ: specta scaffold 配置物 (`install-tauri-scaffold.sh` の表を参照)
  - readiness verify の結果 (PASS / FAIL)。ファイルには書かない

## 手順 {#steps}

script はすべて project root で実行する (`./scripts/bootstrap.js` は skill bundle 相対)。

1. **前提確認**：`.ori/architecture.md` が無ければ `/ori-architect` を先に実行するよう案内して終了
2. **guide を取得**：

   ```bash
   node ./scripts/bootstrap.js guide          # 人間向け
   node ./scripts/bootstrap.js guide --json   # 機械向け (apps[].stack / upstream_init / tauri_scaffold / runner_deps)
   ```

   stack → 開始ガイドの対応 (`guide` が `doc` として返す):

   | stack | 判定 (roots の language) | 開始ガイド |
   |---|---|---|
   | `typescript` | typescript のみ | `docs/start/typescript-web.md` |
   | `typescript-tauri` | typescript + rust | `docs/start/tauri-v2.md` |

   上記以外は未対応 stack として `docs/start/index.md` を案内して終了する。

3. **upstream framework init を案内する** (skill は **自動実行しない** — network / 対話 /
   既存ファイル削除リスクを避ける)：`guide` の `upstream_init` をそのまま提示し、ユーザに
   実行してもらう。`apps/<app>/package.json` が既にある app はスキップしてよい (上書きを促さない)。

   ```bash
   # typescript
   (mkdir -p apps/<app> && cd apps/<app> && pnpm create vite@latest . --template vanilla-ts && pnpm install)
   # typescript-tauri
   (mkdir -p apps/<app> && cd apps/<app> && pnpm create vite@latest . --template vanilla-ts \
     && pnpm add -D @tauri-apps/cli && pnpm tauri init && pnpm install)
   ```

4. **runner deps を root package.json に追加する** (旧 `/ori-arch` 手順 4 の移設、ori-bc9 / D5)：
   `guide` の `runner_deps` を project root で実行する。`.ori/scenarios/` 配下の test は root
   node_modules から解決するため置き場所は root 固定。runner と deps の対応は
   `ori-architect` SKILL.md の runtime recipes (`runner_deps`) が SSoT。

   - root `package.json` が無い場合は `pnpm init` で作ってから追加する
   - network 等で失敗した場合はコマンドを提示して先へ進む (fail-safe。blocking しない)

5. **tauri specta scaffold を apply する** (stack=typescript-tauri のみ、旧 `/ori-arch` の呼び出し移設)：
   `apps/<app>/src-tauri/` が存在する (= `pnpm tauri init` 済み) ことを確認してから、`guide` の
   `tauri_scaffold` を実行する。src-tauri が無ければ手順 3 に戻す (script 自身も exit 2 で止まる)。

   ```bash
   bash <ori-init skill>/scripts/install-tauri-scaffold.sh --dest . --app-name <app> --bc-name <bc-kebab>
   ```

   既存 scaffold は `--force` 無しでは上書きされない。`--force` はユーザ確認後のみ付ける。

6. **readiness verify**：

   ```bash
   node ./scripts/bootstrap.js verify                # 静的 + build (exit 0=PASS / 1=FAIL)
   node ./scripts/bootstrap.js verify --skip-build   # 静的のみ (build が重い時の事前確認)
   node ./scripts/bootstrap.js verify --app <name> --json
   ```

   | check id | 内容 | 判定 |
   |---|---|---|
   | `spec.architecture-md` | architecture.md の存在と parse | FAIL |
   | `app.stack` | stack 確定 | FAIL |
   | `app.package-json` / `app.build-script` | upstream init 済み / build 手段あり | FAIL |
   | `app.node-modules` | app の依存インストール済み | FAIL |
   | `tauri.init` | `src-tauri/Cargo.toml` + `tauri.conf.json` | FAIL |
   | `tauri.specta-scaffold` | `export-types.rs` + `apm-scripts/specta-build.sh` | FAIL |
   | `runtime.build-contract` | local runtime の build が binary を生成する契約 (G2) | FAIL |
   | `runner.deps` | root package.json の runner deps | FAIL |
   | `tauri.wdio-plugin` | tauri-plugin-wdio 配線 (G1) | WARN (`/ori-generate` が patch) |
   | `scenarios.node-modules` | `.ori/scenarios/node_modules` symlink (G3) | WARN (`/ori-generate` が作成) |
   | `runtime.block` | scenario 参加 app の runtime block | WARN |
   | `build` | `runtime.build` (無ければ `pnpm build`) を app dir で実走し、local runtime は `runtime.binary` の実在を確認 | FAIL |

   静的チェックに FAIL がある app は build を実行しない (SKIP)。FAIL の各行に `fix:` が付くので、
   それを提示して手順 3〜5 の該当箇所に戻り、PASS まで繰り返す。

## 注意 {#notes}

- **起動 smoke は行わない** (D6)：verify は静的 + build まで。app の起動可否は scenario 実行時に分類する (実行は CI / 手動)
- **Dockerfile / nix devShell は生成しない** (スコープ外)。wdio + tauri の OS 依存 (WebKitGTK / tauri-driver 等) は案内のみ
- **bootstrap ファイルを生成しない**：`apps/<app>/` の `package.json` / `tsconfig.json` / `Cargo.toml` 等は upstream init の責務
- **既存ファイルを確認なしに上書きしない**：upstream init の再実行・scaffold の `--force` はユーザ確認必須
- **G1 / G3 は WARN 止まり**：tauri-plugin-wdio 配線と scenarios symlink は `/ori-generate` が scenario 生成時に冪等 patch する ([`scenario.instructions.md#test-readiness`](../../../apm_modules/dev-komenzar/ori/.apm/instructions/scenario.instructions.md#test-readiness))
- **CLI 拡張は禁止** (`ori-execution-model-shift-2026-06-03`)：機能はこの skill + scripts/ で実装する。script の正典は `packages/skills/ori-bootstrap/src/*.ts` (`pnpm build:skills` で bundle)

## 次のアクション {#next}

`/ori-bootstrap` verify PASS 後、ユーザに以下を提示：

- **scenario-first パス (推奨)**：`node scripts/new-scenario.js --list-validation` で validation.md の未 cover section を確認 → ユーザ確認の上 `new-scenario.js <id>` で scaffold → `/ori-flow <id>`
- **slice パス**：`/ori-flow new-slice <id>` で slice を scaffold → 7-phase TDD
- **FAIL が残る場合**：`fix:` に従って修正 → `node ./scripts/bootstrap.js verify` を再実行
- **architecture を見直すパス**：stack / runtime block を変えたい場合は `/ori-architect` に戻って再生成 → 再度 `/ori-bootstrap`
