---
name: ori-init
description: ori workspace を初期化し distill-ddd phase 1 にユーザを案内する
---

ユーザが ori を初めて使う際の onboarding を担当します。

## 手順

1. **前提確認**：`bd --version` を Bash で確認。なければ README のインストール手順を案内
2. **app name 確認**：cwd basename をそのまま app 名にするとファイルパスが冗長になる場合があるため、ユーザに確認する (ori-gag)。
   - default は cwd basename を `[a-z0-9-]` に sanitize した値
   - 例：`/tmp/ori-acceptance-h4-greenfield` → default `ori-acceptance-h4-greenfield`
   - ユーザに「app name はこれでよいですか? (default=<sanitized basename>) [Enter=採用 / 別名]」を提示
   - 別名指定時は次の step に `--app-name <別名>` を渡す。default 採用なら flag 省略可
3. **current_agent 確認** (ori-zpy)：cwd を scan して active な harness を検出し、
   `config.yaml` の `current_agent` に反映する。
   - 検出 marker (優先順): `.claude/` → claude / `.opencode/` → opencode /
     `.codex/` → codex / `.gemini/` → gemini / `.cursor/` or `.cursorrules` → cursor /
     `.github/copilot-instructions.md` or `.github/copilot/` → copilot
   - 0 個検出 → default `claude`
   - 1 個検出 → そのまま採用
   - 複数検出 → ユーザに「Detected: X, Y. Which agent? [default=X]」を提示し、
     選んだ値を次の step に `--agent <name>` で渡す
   - 検出結果を無視して別 agent を使いたい場合 (例：marker は claude だが普段は cursor を併用)
     も同様に `--agent <name>` で override
4. **`.ori/` skeleton を作成**：
   ```bash
   # default 採用
   bash ./scripts/create-skeleton.sh
   # app 名を override
   bash ./scripts/create-skeleton.sh --app-name <選んだ名前>
   # current_agent を override
   bash ./scripts/create-skeleton.sh --agent <claude|codex|opencode|gemini|cursor|copilot>
   ```
   既存 `.ori/` を上書きする場合は `--force` を付与。
   このスクリプトは pure bash で自己完結 — CLI も npm ライブラリも経由しない
   (ori-execution-model-shift-2026-06-03 / ori-1ih)。
   テンプレートは `scripts/templates/config.yaml` と
   `scripts/templates/domain-scaffold.md.tpl`。
   `--app-name` は同じ sanitization (`[a-z0-9-]`) を経由するので、sanitize 後に
   空文字になる名前 (例：`'///'`) は exit 2 で reject される (skill 側で再 prompt)。
   `--agent` は supported list 外 (`claude/codex/opencode/gemini/cursor/copilot` 以外)
   なら exit 2 で reject される。
5. **状態確認**：`.ori/` 配下の構造を `ls -la .ori/` で確認し、ユーザに表示
6. **次ステップ提示**：
   - distill-ddd phase 1 を始めるなら `/ori-distill phase=discovery` を呼ぶ
   - pattern 決定 / framework scaffold は `/ori-arch` に委譲
   - 既存 docs があれば手動配置 + 検証
7. **config 確認**：`.apm/agents/` の config を読み、現在の agent / phase 別モデル割当を表示

## 注意

- 初期化は **silent**：`.ori/` skeleton と config 以外、プロジェクトルートには一切ファイルを書かない
- 既存の `.ori/` がある場合は `--force` 相当の上書きをユーザに確認すること
- このスキルは workflow を回さない。実装は `/ori-flow` を使う
- Framework / template scaffold（package.json / src-tauri 等）は `/ori-arch` の framework_init で生成される。例外として stack=typescript-tauri の specta infra (Slice DoD rule 4 を満たすのに必須) は本 skill bundle の `install-tauri-scaffold.sh` 経由で `/ori-arch` が apply する (下記)

## Tauri scaffold extension (specta infra)

Slice DoD (`.apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md` の "Slice Definition of Done" rules 2–4) を typescript-tauri stack で機械的に満たすため、本 skill bundle に specta infra の scaffold templates + install script を同梱する。`/ori-arch` が stack=typescript-tauri を選んだ場合、upstream `pnpm tauri init` 完了後にこの script を呼び出す。

```bash
# /ori-arch から呼ばれる前提
bash ./scripts/install-tauri-scaffold.sh \
  --dest <repo-root> \
  --app-name <app-name> \
  --bc-name <bc-name-kebab>
```

配置物 (target project 側):

| path | role |
| --- | --- |
| `apps/<app>/src-tauri/src/bin/export-types.rs` | tauri-specta bindings 出力 entry (`cargo run --bin export-types`) |
| `apps/<app>/src-tauri/Cargo.toml` (deps 追加) | `tauri-specta` / `specta` / `specta-typescript` / `tauri[features=specta]` |
| `apm-scripts/specta-build.sh` | `phase_hooks.flow-impl-{red-pre,green-post}` から call される build wrapper |
| `apps/<app>/src/<BC>/shared/test-fixtures/setupProductionBuilder.ts` | DoD rule 3 (production wiring) の helper 雛形 |
| `apps/<app>/src/<BC>/shared/ipc/.gitkeep` | bindings.ts 出力先 dir (bindings.ts 自体は phase_hook で生成) |

Templates は `scripts/templates/tauri-stack/` 配下 (skill bundle 相対)。`__APP_NAME__` / `__BC_NAME__` / `__BC_NAME_RS__` / `__APP_NAME_RS__` placeholder を sed で置換。`--force` で上書き。Cargo deps は `cargo add` で append (cargo 不在時は manual merge instructions を stderr に出力)。

## 次のアクション

`/ori-init` 完了後、ユーザに以下を提示：

- **新規プロジェクトのメインパス**：`/ori-ddd-1-discovery` — distill-ddd phase 1 から対話で domain を立ち上げる
- **既存プロジェクト移行パス**：`/ori-migrate` — `docs/domain/` 等を `.ori/domain/` に昇格し、検出済み phase から slice / page を一括 scaffold
- **既存 domain がある場合の検証パス**：`.ori/domain/` の schema 整合性を確認 → 不足分の phase を `/ori-ddd-<N>-*` で補完
- **設定確認パス**：`/ori-model` で agent / phase 別の model 割当を確認・変更（capability-role 設定）
