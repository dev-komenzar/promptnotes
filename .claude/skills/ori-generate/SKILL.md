---
name: ori-generate
description: /ori-flow phase 2。scenario spec からテストコード・runner config・docker-compose.yml を生成する (run-mode 対応)
---

ユーザが `/ori-generate <scenario-id>` を呼んだ、または `/ori-flow` 内部から phase 2 として起動した際に、**該当 scenario のテストコードと runner config と docker-compose.yml を生成**します。

## 引数

- `scenario-id`：対象 scenario の id（`.ori/scenarios/<id>/` が存在する事を前提）

## 役割

- **テストコード生成器**：`manifest.yaml` + `spec.md`（runner 解決済み。Gherkin は `#scenario-steps`）から、**runner 別**のテストコードを生成
- **runner config 生成器**：runner の種類に応じた config（`playwright.config.ts` / `wdio.conf.ts`、vitest は config なし）。**compose / driver の lifecycle は config が所有**する
- **docker-compose 生成器**：参加者のうち **compose-service 系 app + infra のみ**から `docker-compose.yml` を生成（決定的部分は `scripts/generate-docker-compose.sh` が担当）
- **記録係**：生成物は `.ori/scenarios/<id>/` に出力（scenario ディレクトリは self-contained）

## 入力 / 出力

- 入力：
  - `.ori/scenarios/<id>/manifest.yaml`（必須。`infrastructure.services` 参加者リスト + `infrastructure.overrides`）
  - `.ori/scenarios/<id>/spec.md`（必須。runner 解決済み — derive が記録した `runner:` を確認）
  - `.ori/scenarios/<id>/spec.md#scenario-steps`（Gherkin 形式のシナリオ。原典は `.ori/domain/validation.md#<id>`）
  - `.ori/architecture.md`（必須。`workspace.apps[].runtime` blocks + `scenario_test_runner`）
  - `.ori/pages/<page-id>/testids.yaml`（UI 駆動 scenario のみ。testid 契約 — step 7 で `scripts/testids.js sync` してから読む）
- 出力：
  - `.ori/scenarios/<id>/tests/<scenario-id>.spec.ts`（テストコード、`@ori-generated`）
  - `.ori/scenarios/<id>/playwright.config.ts` + `teardown.mjs` + `tsconfig.json`、または `wdio.conf.ts`（runner 別。vitest は config なし）
  - `.ori/scenarios/<id>/test-points-map.md`（`spec.md#test-points` ↔ テストケースの網羅対応表、`@ori-generated`）
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

4. **シナリオの読み込み**：`.ori/scenarios/<id>/spec.md#scenario-steps` を Read。Gherkin 形式のシナリオを理解（原典確認が要る場合は `.ori/domain/validation.md#<id>`）

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
   - `spec.md#scenario-steps` の Gherkin シナリオからテストコードを生成
   - runner は spec.md の `runner:` 記録に従う（playwright / wdio / vitest）
   - テストコード内に **service の起動・停止・healthcheck 待機を書かない**（lifecycle は runner config が所有 — `scenario-test.instructions.md` 参照）
   - **wdio: `browser.execute` の戻り値は `!= null` で判定する**（WebDriver は `undefined` を `null` で返すため、`!== undefined` の poll は即成立する。ori-oan.9）。例: `await browser.waitUntil(async () => (await browser.execute(() => (window as unknown as { __result?: unknown }).__result)) != null)`
   - **wdio: temp path は `process.env.ORI_SCENARIO_TMP` 配下を使う**（例: 保存先変更 scenario の新 dir。runner config が per-run temp root を公開する）
   - **selector は testid 契約の値だけを使う（G5 / ori-oan.7）**: E2E は `data-testid` を第一推奨（SSoT: `ddd-vsa-hex/pattern.md` "page / widget の testid 契約"、手順の詳細は `scenario-test.instructions.md#selectors`）。scenario が操作・検証する page / widget ごとに:
     1. `node scripts/testids.js sync <page-id>` で `.ori/pages/<page-id>/testids.yaml` を最新化して読む（exit 1 = page 未 scaffold / `<elem>` 衝突。推測で埋めず停止し、上流 (11b / page scaffold) の未完了としてユーザに提示）
     2. ui-field 由来の要素は `derived:`、それ以外は `extra:` の `testid` をそのまま使う。ui-fields の field id（`screen-<N>-*`）や規則から testid を自前で導出しない
     3. 契約に無い要素（エラー表示等）が必要なら、テストに直書きせず先に追記する:
        ```bash
        node scripts/testids.js add-extra <page-id> --testid <kind>.<page-id>.<elem> --purpose "<役割>" --source scenario:<scenario-id>
        ```
        追記のみ（既存行の変更・削除はしない）。実装が後から追従する（`/ori-impl-green` / `/ori-doctor` が検出）
   - **実装との乖離は起票して生成を続ける（非停止、ori-oan.13）**: 参加 page / widget の id をすべて渡して実行する:
     ```bash
     bash scripts/check-page-testids.sh --emit-issues --implemented-only <page-id>...
     ```
     - 実装の無い page は飛ばす（scenario-first では未実装が正常。実装は `/ori-impl-green` が契約どおりに付ける）
     - 実装のある page に違反（契約 testid の実装不在・その page の動的 testid / 形式違反）があれば、page ごとに `testid-violation` + `page:<id>` の bd issue を起票する。open の issue があれば起票せず、その id を出す
     - 出力された issue id と違反の要約を spec.md `#impl-notes` に記録し、「実装が契約に追従するまで RED。移行手順は [`ui-test.instructions.md#testid-migration`](../../../apm_modules/dev-komenzar/ori/.apm/instructions/ui-test.instructions.md#testid-migration)」と書く
     - exit code は違反件数 (非 0 でも失敗ではない)。ただし stderr に `ERROR:` が出たら検査も起票もされていない (page id の誤り・未知 option)。引数を直して再実行する。生成は止めない。実装の testid に合わせてテストを捏造しない。契約も書き換えない
     - `SKIP` 行の page は記録不要 (未実装、または契約を導出できない — 後者は手順 1 の sync で既に停止しているはず)。bd が無く issue id が出ない場合は、違反の要約だけを impl-notes に書き「`/ori-doctor --testid-sweep` で起票」と添える
     - 実装はあるのに契約 testid も field id も使っていない page は「実装なし」に見える。生成した scenario が RED なのに起票が無ければ `/ori-doctor --testid-sweep` を案内する
   - テストファイルは `.ori/scenarios/<id>/tests/<scenario-id>.spec.ts` に出力、先頭に `// @ori-generated scenario:<scenario-id>` マーカー
   - **test-points 網羅対応表を生成する（R5）**: `spec.md#test-points` の全項目（項目の数え方は `scenario-test.instructions.md#test-points-map` の awk が SSoT。インデントなしの `- ` 行のみが項目で、sub-bullet は項目に含めず親項目の説明として扱う）を `.ori/scenarios/<id>/test-points-map.md` に写す。形式・規則の SSoT は `scenario-test.instructions.md#test-points-map`。項目を省略・統合しない。E2E で原理的に検証不能な項目は `N/A(代替担保)` とし、代替担保（unit test の file:line）を必ず併記する。代替が無ければ `UNCOVERED` のまま残す（推測で COVERED にしない）

8. **runner config の生成**（runner 別、`.ori/scenarios/<id>/` 直下）:
   - **playwright**（compose-service web 駆動）: `playwright.config.ts`。`webServer` で compose を起動（`docker compose up -d --wait` — healthy 後に CLI が終了するため停止は `teardown.mjs` の globalTeardown で明示 `down`）、`url` / `port` で TCP 起動待機。あわせて `tsconfig.json` も出力
   - **wdio**（local tauri 駆動）: `wdio.conf.ts`。`services: [['@wdio/tauri-service', { driverProvider: 'external' }]]`、`tauri:options.application` に `runtime.binary`（絶対パス解決）を指定。`onPrepare` / `onComplete` が以下を所有する:
     - `.ori/scenarios/node_modules` → `apps/<app>/node_modules` の symlink を冪等に作成（G3。無いと spec の `@wdio/globals` 等が ESM 解決できない）
     - **platform / port の fail fast**: `process.platform !== 'linux'` なら storage 隔離を保証できない旨で throw（wdio scenario は Linux のみ正式サポート）。tauri-driver port（`:4444`）が占有済みなら残留 tauri-driver の可能性と `pkill tauri-driver` を案内して throw（kill はしない。ori-oan.10）。**throw は `SevereServiceError`（`webdriverio`）を使う** — 普通の `Error` は wdio launcher がログに出すだけで実行を続行する
     - `runtime.test_env` の各 env を `process.env` に設定（固定文字列。app 固有 env）
     - **storage 隔離（G4、ori-oan.8）**: `mkdtempSync` で per-run temp root を採番し、`XDG_CONFIG_HOME` / `XDG_DATA_HOME` / `XDG_CACHE_HOME` / `XDG_STATE_HOME` を root 配下に設定する（app 側 override 不要）。root は `ORI_SCENARIO_TMP` として公開する。test_env の **後** に設定し、test_env による上書きを許さない。`TAURI_TEST_STORAGE_DIR` は注入しない（ori 標準ではない）
     - **`maxInstances: 1` 固定**: `@wdio/tauri-service` は `maxInstances > 1` / multiremote で `XDG_DATA_HOME` を `<tmpdir>/tauri-worker-<cid>` に上書きし、per-run 隔離と seed が崩れる
     - 既存データ前提の scenario は fixture を seed（G6。`.md` frontmatter 形式は app / domain の SSoT に従う）
     - compose-service 系参加時は `docker compose up -d --wait`、`onComplete` で `down -v`
     - `onComplete` で temp dir を削除
   - **wdio の app 前提（G1/G2）**: 実行前に以下を満たす（SSoT: [`scenario.instructions.md#test-readiness`](../../../apm_modules/dev-komenzar/ori/.apm/instructions/scenario.instructions.md#test-readiness)）。Rust 配線は §「wdio の app 前提 patch」で **検出して冪等に patch** する:
     - `runtime.build` が `runtime.binary` を生成する（Tauri の `cargo build` 単体は devUrl 参照の dev binary なので不可）
     - `tauri-plugin-wdio` の Rust 配線（Cargo dep / capabilities `wdio:default` / lib.rs の `#[cfg(debug_assertions)]` 登録）
     - frontend の動的 import と test build script は framework / 言語固有のため **生成せず impl-notes の要求として記録** する
   - **vitest**（API-only）: config なし。compose-service 系参加時は起動を globalSetup で行うか、scenario 単独実行を前提とする（`local` 系 app は不参加のはず）
   - 起動待機は **TCP probe が default**（`runtime.healthcheck: {http: /health}` 宣言時のみ HTTP 待機）

9. **生成物の検証**:
   - テストコード: `npx tsc --noEmit` で構文検証（wdio の `tauri:options` は template の型付き capability 定数で型通過済みのため、既知エラー許容なしで exit 0 が期待値）
   - runner config: `npx tsc --noEmit` で構文検証
   - docker-compose.yml: script が `docker compose config -q` 済み（失敗時のみここで確認）
   - 検証失敗時は **1 回だけ** 自動修正を試み、それでも失敗ならユーザに判断を委ねる

10. **beads issue 更新**：
    ```bash
    bd update ori-generate-<scenario-id> --status=closed --notes="test code + runner config + docker-compose.yml generated (runner=<name>)"
    ```
11. **phase 台帳の更新（scenario）** — 決定的 writer で記録する（R1）:
    ```bash
    node scripts/scenario-status.js set <scenario-id> generate done
    ```
    - `phases.generate` と `beads.completion` が更新される（冪等）。実行失敗時は停止しユーザに委ねる

## 出力フォーマット

### テストコード

テストコードは `spec.md#scenario-steps` の Gherkin シナリオに基づき、runner 別の記法（playwright / wdio / vitest）で生成します。構造・命名規約の SSoT は [`scenario-test.instructions.md`](../../../apm_modules/dev-komenzar/ori/.apm/instructions/scenario-test.instructions.md)。

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
import { execSync } from 'node:child_process'; // compose-service 参加時のみ使用
import { existsSync, mkdirSync, mkdtempSync, rmSync, symlinkSync } from 'node:fs';
import { createConnection } from 'node:net';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
// onPrepare で普通の Error を throw しても wdio launcher はログに出すだけで続行する。停止には SevereServiceError
import { SevereServiceError } from 'webdriverio';

const __dirname = dirname(fileURLToPath(import.meta.url));
const APP_DIR = resolve(__dirname, '../../../apps/<app>');
// runtime.binary (build-then-test) を絶対パスで解決
const BINARY = resolve(APP_DIR, 'src-tauri/target/debug/<app>');

// tauri-driver の待受 port（@wdio/tauri-service の既定値。tauriDriverPort を指定する場合は揃える）
const DRIVER_PORT = 4444;

let tmpDir = '';

// tauri-driver の待受 port が既に使われているか（残留 tauri-driver 検出用）
const isPortInUse = (port: number) =>
  new Promise<boolean>((done) => {
    const socket = createConnection({ port, host: '127.0.0.1' });
    socket.setTimeout(1000, () => { socket.destroy(); done(false); });
    socket.once('connect', () => { socket.destroy(); done(true); });
    socket.once('error', () => done(false));
  });

// 'tauri:options' は WebdriverIO.Capabilities の型に無い。`config: WebdriverIO.Config` 注釈下で
// capability を直書きすると tsc --noEmit が TS2353 ("'tauri:options' does not exist in type
// 'RequestedStandaloneCapabilities'") で失敗するため、型を明示した定数に切り出す（ori-7jm.7）。
// 注意: 行頭が「@ts-expect-error」のコメントは tsc にディレクティブとして解釈され TS2578 になる
const tauriCapability: WebdriverIO.Capabilities & { 'tauri:options': { application: string } } = {
  browserName: 'tauri', // 'wry' も可（同一扱い）
  'tauri:options': { application: BINARY },
};

export const config: WebdriverIO.Config = {
  runner: 'local',
  specs: ['./tests/**/*.spec.ts'],
  // 1 固定: >1 / multiremote では tauri-service が XDG_DATA_HOME を tauri-worker-<cid> に上書きし隔離が崩れる
  maxInstances: 1,
  // external = tauri-driver (intermediary) + WebKitWebDriver (native)。v1.4.0 の default は
  // 'embedded' で、これは tauri-plugin-wdio-webdriver を app に必要とするため external を明示。
  services: [['@wdio/tauri-service', { driverProvider: 'external' }]],
  capabilities: [tauriCapability],
  framework: 'mocha',
  mochaOpts: { ui: 'bdd', timeout: 60000 },
  reporters: ['spec'],

  onPrepare: async () => {
    // XDG 隔離は Linux でしか効かない。他 platform では実ユーザ領域を汚すため実行しない
    if (process.platform !== 'linux') {
      throw new SevereServiceError(`ori wdio scenario は Linux のみサポート (storage 隔離を保証できない: ${process.platform})`);
    }
    // 失敗した実行の tauri-driver が残っていないか（残留 driver は黙って別 port に逃げられると気付けない）
    if (await isPortInUse(DRIVER_PORT)) {
      throw new SevereServiceError(`:${DRIVER_PORT} が使用中です。残留 tauri-driver の可能性があります (pkill tauri-driver で解放)`);
    }
    // G3: scenario から app の node_modules を ESM 解決できるようにする
    // symlink は .ori/scenarios/ 直下（全 scenario 共有）に置く
    const link = resolve(__dirname, '..', 'node_modules');
    if (!existsSync(link)) symlinkSync(resolve(APP_DIR, 'node_modules'), link, 'dir');
    // runtime.test_env がある場合はここで設定（例: process.env.FOO = 'bar'）。XDG_* より先に置き、上書きさせない
    // G4: XDG を per-run temp に向けて storage / settings / WebView data を隔離（app 側 override 不要）
    tmpDir = mkdtempSync(join(tmpdir(), 'ori-scenario-<id>-'));
    for (const [env, sub] of [
      ['XDG_CONFIG_HOME', 'config'],
      ['XDG_DATA_HOME', 'data'],
      ['XDG_CACHE_HOME', 'cache'],
      ['XDG_STATE_HOME', 'state'],
    ]) {
      process.env[env] = resolve(tmpDir, sub);
      mkdirSync(process.env[env]!, { recursive: true });
    }
    // test code / seed から temp root を参照できるよう公開（worker は onPrepare 後に起動され env を継承）
    process.env.ORI_SCENARIO_TMP = tmpDir;
    // G6: 既存データ前提ならここで fixture seed（frontmatter 形式は app/domain SSoT に従う。
    //     保存先は XDG 配下: 例 resolve(tmpDir, 'data', '<tauri identifier>', ...)）
    // compose-service 系参加時のみ:
    // execSync('docker compose -f docker-compose.yml up -d --wait', { cwd: __dirname, stdio: 'inherit' });
  },
  onComplete: async () => {
    // execSync('docker compose -f docker-compose.yml down -v --remove-orphans', { cwd: __dirname, stdio: 'inherit' });
    if (tmpDir) rmSync(tmpDir, { recursive: true, force: true });
  },
};
```

### wdio の app 前提 patch（G1/G2、ori-oan.3）

`runner=wdio` の生成時、参加 local Tauri app に対して以下を **存在チェックして冪等に patch** する（施済みなら skip）。これらは **オリジナル app ファイルへの patch** であり、`.ori/` の派生ファイルではない点に注意:

1. **node_modules symlink（G3）**: `.ori/scenarios/node_modules` → `../../apps/<app>/node_modules` を作成。`.ori/.gitignore` に `scenarios/node_modules` を追記（未記載時のみ）
2. **Cargo deps（G1）**: `apps/<app>/src-tauri/Cargo.toml` の `[dependencies]` に `tauri-plugin-wdio = "1"` が無ければ追加（ACL 解決のため無条件）
3. **capabilities（G1）**: `apps/<app>/src-tauri/capabilities/*.json` の `permissions` に `"wdio:default"` が無ければ追加
4. **lib.rs 登録（G1）**: `builder.plugin(tauri_plugin_wdio::init())` を `#[cfg(debug_assertions)]` 下で登録する。既存 chain を `let builder = ...` に括り出して追記し、log plugin の登録を chain 先頭へ寄せる（wdio plugin の logger との二重 set panic 回避）。既に `tauri_plugin_wdio::init()` があれば skip
5. **impl-notes に記録（生成しない。framework / 言語固有のため）**:
   - **frontend**: `@wdio/tauri-plugin` を `VITE_WDIO_TEST` 時のみ動的 import する（SvelteKit 例: `src/lib/wdio-test-setup.ts` に `if (browser && import.meta.env.VITE_WDIO_TEST) void import('@wdio/tauri-plugin')` を置き、entry (`+layout.svelte` 等) から import）
   - **test build script**: `runtime.build` が参照する command は `VITE_WDIO_TEST=1` を伴う debug build（例: package.json に `"build:test": "VITE_WDIO_TEST=1 bun run tauri build --debug --no-bundle"`）であること
   - **production 非混入**: Rust は `debug_assertions`、frontend は `VITE_WDIO_TEST` gate。release build に plugin 参照 0 を検証する

**冪等性**: 各 patch は該当文字列/キーの存在チェックで skip する。再実行で重複追加しない。**検証**: `npx tsc --noEmit` に加え、可能なら `build:test` 後に app log の `Failed to get window states` / `Tauri plugin not available` が **app 起動〜最初の reload / 再起動まで 0** であることを確認する（reload / 再起動直後の一過性警告は許容。基準の SSoT は `scenario-test.instructions.md#plugin-warnings`。ori-oan.11）。

### docker-compose.yml

`scripts/generate-docker-compose.sh` が生成（参加 app は runtime block、infra は catalog から）。手で書かない。

## 注意

- **自動 scaffold は禁止**：scenario が存在しなくても勝手に新規作成を呼ばない（ユーザ確認必須）
- **生成物は派生ファイル**：直接編集は不可（テストコード / runner config / docker-compose.yml すべて）。更新手順は [`scenario.instructions.md`](../../../apm_modules/dev-komenzar/ori/.apm/instructions/scenario.instructions.md) §caveats に従う
- **app 前提 patch は派生ファイルではない**：§「wdio の app 前提 patch」の Rust 配線（Cargo.toml / capabilities / lib.rs）と node_modules symlink は **app オリジナルへの冪等 patch**（派生ファイルではないため再生成対象外）。frontend import / test build script は生成せず impl-notes の要求として残す
- **前提条件を満たす**：`runner=wdio` では [`scenario.instructions.md#test-readiness`](../../../apm_modules/dev-komenzar/ori/.apm/instructions/scenario.instructions.md#test-readiness) の前提（G1〜G6）を満たす。満たせない場合は `TBD` を残して人間判断に委ねる
- **推測で埋めない**：`TBD` を残し、人間判断に委ねる箇所を明示（app↔infra 接続 env の app 固有値等）
- **lifecycle は config 所有**：テストコード内に compose up / healthcheck 待機を書かない
- このスキルは spec を書かない。**phase 2 = 生成のみ**
- **SSoT 参照原則**：生成物の仕様は常に `manifest.yaml` + `spec.md`（`#scenario-steps`）+ `.ori/architecture.md` を参照する

## 次のアクション

phase 2 完了後、`/ori-flow` 内部なら自動的に phase 3 へ。単独呼び出しの場合：

- **メインパス**：`/ori-review <scenario-id>` — phase 3。scenario の adversarial review
- **生成物を修正するパス**：source（manifest.yaml / architecture.md）を編集 → `/ori-sync` → `/ori-flow`（[`scenario.instructions.md`](../../../apm_modules/dev-komenzar/ori/.apm/instructions/scenario.instructions.md) §caveats）
