---
name: ori-generate
description: /ori-flow phase 2。scenario spec からテストコード・runner config・docker-compose.yml を生成する (run-mode 対応)
---

ユーザが `/ori-generate <scenario-id>` を呼んだ、または `/ori-flow` 内部から phase 2 として起動した際に、**該当 scenario のテストコードと runner config と docker-compose.yml を生成**します。

## 引数

- `scenario-id`：対象 scenario の id（`.ori/scenarios/<id>/` が存在する事を前提）

## 役割

- **テストコード生成器**：`manifest.yaml` + `spec.md`（runner 解決済み）+ `validation.md` (Gherkin) から、**runner 別**のテストコードを生成
- **runner config 生成器**：runner の種類に応じた config（`playwright.config.ts` / `wdio.conf.ts`、vitest は config なし）。**compose / driver の lifecycle は config が所有**する
- **docker-compose 生成器**：参加者のうち **compose-service 系 app + infra のみ**から `docker-compose.yml` を生成（決定的部分は `scripts/generate-docker-compose.sh` が担当）
- **記録係**：生成物は `.ori/scenarios/<id>/` に出力（scenario ディレクトリは self-contained）

## 入力 / 出力

- 入力：
  - `.ori/scenarios/<id>/manifest.yaml`（必須。`infrastructure.services` 参加者リスト + `infrastructure.overrides`）
  - `.ori/scenarios/<id>/spec.md`（必須。runner 解決済み — derive が記録した `runner:` を確認）
  - `.ori/scenarios/<id>/validation.md`（必須。Gherkin 形式の検証シナリオ）
  - `.ori/architecture.md`（必須。`workspace.apps[].runtime` blocks + `scenario_test_runner`）
- 出力：
  - `.ori/scenarios/<id>/tests/<scenario-id>.spec.ts`（テストコード、`@ori-generated`）
  - `.ori/scenarios/<id>/playwright.config.ts` + `teardown.mjs` + `tsconfig.json`、または `wdio.conf.ts`（runner 別。vitest は config なし）
  - `.ori/scenarios/<id>/docker-compose.yml`（compose-service 系 app + infra のみ。**参加者ゼロなら省略**）

## 手順

1. **scenario 存在確認**：
   ```bash
   bash ./scripts/check-scenario-exists.sh <scenario-id>
   ```
   - exit 0: 存在 → 次のステップへ
   - exit 2: 類似候補あり → ユーザに「これですか？」と確認、Yes なら正しい id で再開
   - exit 3: 未 scaffold だが validation.md に一致する section anchor がある（scenario id = anchor、1:1）→ ユーザ確認の上 `new-scenario.js <id>`（ori-flow skill bundle）で scaffold するか確認
   - exit 1: 未発見 → 新規 scenario 作成を**ユーザに確認**してから進める

2. **manifest.yaml の読み込み**：`.ori/scenarios/<id>/manifest.yaml` を Read。`infrastructure.services` が空ならエラー停止し、「先に manifest に infrastructure.services を追記してください」と案内

3. **spec.md の読み込み**：`.ori/scenarios/<id>/spec.md` を Read。実装ノートの `runner:` 記録（derive が優先チェーンで解決）を確認。未解決なら `/ori-derive` に差し戻す

4. **validation.md の読み込み**：`.ori/scenarios/<id>/validation.md` を Read。Gherkin 形式の検証シナリオを理解

5. **`.ori/architecture.md` の読み込み**：`workspace.apps[].runtime` blocks と `scenario_test_runner` を確認

6. **docker-compose.yml の生成**（決定的生成は script が担当）：
   ```bash
   bash ./scripts/generate-docker-compose.sh <scenario-id>
   ```
   script が実施するのは:
   - **サービス名解決 3 ルール**: ① `workspace.apps` と一致 → app service（runtime block [compose-service] から生成）② infra catalog（`scripts/infra-catalog.yaml`）と一致 → catalog から生成（manifest `infrastructure.overrides` で image / ports / environment 上書き）③ 不一致 → **exit 1 で停止**（推測で埋めない）
   - **mode 分岐**: `local` 系 app（tauri 等）は compose に含めない（NOTE 出力のみ。build-then-test なので binary は runner config 経由で起動）
   - **compose 省略条件**: compose-service 系参加者ゼロなら `docker-compose.yml` を生成しない（既存 file があれば削除）
   - **port 衝突検査**: 同一 scenario 内 host port 衝突は exit 1
   - **検証**: `docker compose config -q`（docker 不在時は WARN で続行）

   script の `app env hints` 出力（catalog 既定値）を参考に、**参加 app service の `environment` に接続 env を追記**する。app 固有値（DB 名等）が導出不能なら **`TBD` マーカー**を残して次へ（推測で埋めない）

7. **テストコードの生成**（AI 生成 — script に頼らない）:
   - `validation.md` の Gherkin シナリオからテストコードを生成
   - runner は spec.md の `runner:` 記録に従う（playwright / wdio / vitest）
   - テストコード内に **service の起動・停止・healthcheck 待機を書かない**（lifecycle は runner config が所有 — `scenario-test.instructions.md` 参照）
   - テストファイルは `.ori/scenarios/<id>/tests/<scenario-id>.spec.ts` に出力、先頭に `// @ori-generated scenario:<scenario-id>` マーカー

8. **runner config の生成**（runner 別、`.ori/scenarios/<id>/` 直下）:
   - **playwright**（compose-service web 駆動）: `playwright.config.ts`。`webServer` で compose を起動（`docker compose up -d --wait` — healthy 後に CLI が終了するため停止は `teardown.mjs` の globalTeardown で明示 `down`）、`url` / `port` で TCP 起動待機。あわせて `tsconfig.json` も出力
   - **wdio**（local tauri 駆動）: `wdio.conf.ts`。`onPrepare` で `docker compose up`（compose-service 系参加時）+ `tauri:options.application` でビルド済み binary を指定（binary path は runtime block の `binary`）。tauri-service（`@wdio/tauri-service`）を使用
   - **vitest**（API-only）: config なし。compose-service 系参加時は起動を globalSetup で行うか、scenario 単独実行を前提とする（`local` 系 app は不参加のはず）
   - 起動待機は **TCP probe が default**（`runtime.healthcheck: {http: /health}` 宣言時のみ HTTP 待機）

9. **生成物の検証**:
   - テストコード: `npx tsc --noEmit` で構文検証
   - runner config: `npx tsc --noEmit` で構文検証
   - docker-compose.yml: script が `docker compose config -q` 済み（失敗時のみここで確認）
   - 検証失敗時は **1 回だけ** 自動修正を試み、それでも失敗ならユーザに判断を委ねる

10. **beads issue 更新**：
    ```bash
    bd update ori-generate-<scenario-id> --status=closed --notes="test code + runner config + docker-compose.yml generated (runner=<name>)"
    ```

## 出力フォーマット

### テストコード

テストコードは `validation.md` の Gherkin シナリオに基づき、runner 別の記法（playwright / wdio / vitest）で生成します。構造・命名規約の SSoT は `.apm/instructions/scenario-test.instructions.md`。

### runner config

**playwright**（compose-service web 駆動の例）:

```typescript
// .ori/scenarios/<id>/playwright.config.ts — @ori-generated
import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  webServer: {
    command: 'docker compose -f docker-compose.yml up -d --wait',
    url: 'http://localhost:5173',   // runtime.ports の TCP 待機 (healthcheck 宣言時のみ HTTP)
    reuseExistingServer: false,
    timeout: 120_000,
  },
  globalTeardown: './teardown.mjs',
});
```

```javascript
// .ori/scenarios/<id>/teardown.mjs — @ori-generated
// docker compose up --wait は healthy 後に CLI が終了するため、webServer の
// process kill では compose が停止しない。teardown で明示 down する (ori-bc9.5 F-5)。
import { execSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

export default async function globalTeardown() {
  execSync('docker compose -f docker-compose.yml down -v --remove-orphans', {
    cwd: fileURLToPath(new URL('.', import.meta.url)),
    stdio: 'inherit',
  });
}
```

あわせて scenario 直下に `tsconfig.json` を出力する（`/ori-review` の `tsc --noEmit` が
node types / esnext target なしで失敗するため — ori-bc9.5 F-6）。`types` は runner 別に指定する
（playwright は `["node"]` のみ / wdio は `@wdio/globals/types` + `mocha` + `skipLibCheck` —
WDIO v9 の型が TS 7 の lib.dom `URLPattern` と衝突するため。ori-bc9.5 tauri F-2）:

```json
{
  "compilerOptions": {
    "target": "esnext", "module": "esnext", "moduleResolution": "bundler",
    "types": ["node", "@wdio/globals/types", "mocha"],
    "noEmit": true, "strict": true, "skipLibCheck": true
  },
  "include": ["tests/**/*.ts", "wdio.conf.ts"]
}
```

**wdio**（local tauri 駆動の例。`@wdio/tauri-service` v1.4.0 の実 API — ori-bc9.5 tauri F-1）:

```typescript
// .ori/scenarios/<id>/wdio.conf.ts — @ori-generated
import { execSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
// runtime.binary (build-then-test) を絶対パスで解決
const BINARY = resolve(__dirname, '../../../apps/<app>/src-tauri/target/debug/<app>');

export const config = {
  runner: 'local',
  specs: ['./tests/**/*.spec.ts'],
  maxInstances: 1,
  // external = tauri-driver (intermediary) + WebKitWebDriver (native)。v1.4.0 の default は
  // 'embedded' で、これは tauri-plugin-wdio-webdriver を app に必要とするため external を明示。
  services: [['@wdio/tauri-service', { driverProvider: 'external' }]],
  capabilities: [{
    browserName: 'tauri',   // 'wry' も可（同一扱い）
    'tauri:options': { application: BINARY },
  }],
  framework: 'mocha',
  mochaOpts: { ui: 'bdd', timeout: 60000 },
  reporters: ['spec'],
  // compose-service 系参加時のみ onPrepare で docker compose up -d --wait + onComplete で down
  onPrepare: async () => { execSync('docker compose -f docker-compose.yml up -d --wait', { cwd: __dirname, stdio: 'inherit' }); },
  onComplete: async () => { execSync('docker compose -f docker-compose.yml down -v --remove-orphans', { cwd: __dirname, stdio: 'inherit' }); },
};
```

### docker-compose.yml

`scripts/generate-docker-compose.sh` が生成（参加 app は runtime block、infra は catalog から）。手で書かない。

## 注意

- **自動 scaffold は禁止**：scenario が存在しなくても勝手に新規作成を呼ばない（ユーザ確認必須）
- **生成物は派生ファイル**：直接編集には `/ori-sync --force` が必要（テストコード / runner config / docker-compose.yml すべて）
- **推測で埋めない**：`TBD` を残し、人間判断に委ねる箇所を明示（app↔infra 接続 env の app 固有値等）
- **lifecycle は config 所有**：テストコード内に compose up / healthcheck 待機を書かない
- このスキルは spec を書かない。**phase 2 = 生成のみ**
- **SSoT 参照原則**：生成物の仕様は常に `manifest.yaml` + `spec.md` + `validation.md` + `.ori/architecture.md` を参照する

## 次のアクション

phase 2 完了後、`/ori-flow` 内部なら自動的に phase 3 へ。単独呼び出しの場合：

- **メインパス**：`/ori-review <scenario-id>` — phase 3。scenario の adversarial review
- **生成物を修正するパス**：生成物を直接編集 → `/ori-sync --force` → 再度 `/ori-generate`
- **manifest を修正するパス**：`manifest.yaml` を編集 → 再度 `/ori-generate`
