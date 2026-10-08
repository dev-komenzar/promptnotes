---
paths:
  - ".ori/scenarios/**/tests/*.{spec,test}.{ts,tsx}"
---

## テストランナー {#test-runner}

scenario の UI 駆動 runner は **runner matrix 3 種**から選択される。derive phase が優先チェーン（`scenario.instructions.md` の runner chain 参照）で解決し、spec.md に記録する:

| runner | 用途 | 主な駆動対象 |
|---|---|---|
| **Playwright** | compose-service 系 web app の E2E | ブラウザ操作 + API 呼び出し + DB 状態確認 |
| **WDIO**（`@wdio/tauri-service`） | local 系 app（tauri 等）の E2E | ビルド済み binary（native window） |
| **Vitest** | API-only scenario（UI app 非参加） | API 呼び出し中心の統合テスト |

- **「1 scenario = 1 UI runner」制約**: scenario の UI 駆動面は単一 runner でカバーできること。detox は web を駆動不可、playwright は tauri binary を駆動不可、という runner 能力差に起因する制約
- tauri app の runner 実体は `@wdio/tauri-service`（`driverProvider: 'external'` = tauri-driver）。ori の wdio scenario は **Linux のみ正式サポート**（storage の XDG 隔離が Linux 前提。`#test-readiness` 参照）
- RN + web の同時 UI 駆動が必要な高度ケースは WDIO multiremote（Appium + chromedriver）を文書上の escape とする（通常は不要）

## テスト構造 {#test-structure}

テストコードは以下の構造に従う（Playwright 例。WDIO は `describe` / `it`、Vitest は `describe` / `test` を使用）:

```typescript
// @ori-generated scenario:<scenario-id>
import { test, expect } from '@playwright/test';

test.describe('scenario:<scenario-id>', () => {
  test('step 1 — validation#<id>', async ({ page, request }) => {
    // Given: 事前条件
    // When: 操作
    // Then: 検証
  });

  test('step 2 — validation#<id>', async ({ page, request }) => {
    // ...
  });
});
```

### 命名規則 {#naming-conventions}

- **describe**: `scenario:<scenario-id>`（例: `scenario:order-flow-e2e`）
- **test / it**: `step N — validation#<id>`（例: `step 1 — validation#order-create`）
  - `N`: シナリオステップ番号（1-based）
  - `validation#<id>`: validation.md の Gherkin シナリオ ID

### 生成コードマーカー {#generated-code-marker}

テストファイルの先頭に以下のマーカーを配置する:

```typescript
// @ori-generated scenario:<scenario-id>
```

このマーカーは `/ori-sync` が派生ファイルを識別するために使用する。直接編集せず、source（manifest / ドメイン文書）を編集して `/ori-sync` → `/ori-flow` で再生成する（`scenario.instructions.md` §caveats）。

## test-points 網羅対応表 {#test-points-map}

`/ori-generate` は `spec.md#test-points` の全項目を `.ori/scenarios/<id>/test-points-map.md` に対応表として出力する（`@ori-generated`）。`/ori-review` はこの表と spec を突合して欠落を検出する。

```markdown
<!-- @ori-generated scenario:<scenario-id> -->
| TP | test-point (spec.md#test-points) | テストケース (file:line) | 代替担保 (file:line) | 状態 |
|---|---|---|---|---|
| TP-1 | 発火タイミング (debounce 500ms) | tests/x.spec.ts:42 | — | COVERED |
| TP-2 | event 発行 (NoOpBus) | — | apps/n/src/.../bus.test.ts:10 | N/A(代替担保) |
| TP-3 | sort 順 | — | — | **UNCOVERED** |
```

- 行数は spec の test-points 項目数と一致させる（1 項目 = 1 行。省略・統合禁止）
- **項目の数え方（SSoT）**: `spec.md#test-points` 節内の **インデントなしの `- ` 行のみ**が 1 項目（sub-bullet・fenced code 内は数えない）。generate / review は同じ次の awk を使う:
  ```bash
  awk '/^```/{c=!c} /^## .*\{#test-points\}/{f=1;next} /^## /{f=0} f && !c && /^- /' .ori/scenarios/<id>/spec.md
  ```
- 状態は `COVERED` / `N/A(代替担保)` / `UNCOVERED` のいずれか
- **`N/A(代替担保)` は「代替担保」列に `file:line` 形式（`path:数字`）が必須**（E2E 不能の理由 + unit test 等）。`file:line` が無ければ（空 / `—` / `-` / `なし` 等）`UNCOVERED` として扱う
- `UNCOVERED` は review で HIGH / NEEDS_FIX（LOW 不可）。`/ori-generate` が未カバーを残す場合も隠さず `UNCOVERED` と書く

## 事前条件 {#preconditions}

### サービス起動は runner config が所有する {#lifecycle-ownership}

テストコードは **service の起動・停止・healthcheck 待機を行わない**。mode にかかわらず、lifecycle は同じ scenario ディレクトリに生成される runner config（`playwright.config.ts` / `wdio.conf.ts`）が所有する:

- **compose-service 系 app + infra**: config が `docker-compose.yml` を起動・停止する（Playwright は `webServer`、WDIO は `onPrepare`）
- **local 系 app**: **build-then-test** — ビルド済み binary を config が起動する（WDIO `onPrepare` + `tauri:options.application` で binary 指定。tauri 例: 事前に `tauri build --debug --no-bundle`）
- **`local` 系 app は docker-compose に含めない**（compose は compose-service 系 app + infra のみ）

旧「`test.beforeAll` で `docker-compose up` + healthcheck 待機」スケッチは **廃止**。テストコードに起動処理を書かないこと。

### app 側の前提条件 {#test-readiness}

runner config が所有する起動に加え、`runner=wdio`(local Tauri)では app 側の次の前提が必要(SSoT: `scenario.instructions.md#test-readiness`)。テストコードはこれらを書かない:

- **build-then-test**: `runtime.build` が `runtime.binary` を生成する。Tauri の `cargo build` 単体(devUrl 参照の dev binary)は不可
- **plugin**: `tauri-plugin-wdio` が app に配線されていること(未導入時は focus 系コマンドごとに 5 秒待機)
- **storage 隔離**: runner config の `onPrepare` が per-run temp root を作り `XDG_CONFIG_HOME` / `XDG_DATA_HOME` / `XDG_CACHE_HOME` / `XDG_STATE_HOME` をその配下に向ける（app 側 override 不要）。temp root は `process.env.ORI_SCENARIO_TMP` で参照できる（例: 保存先変更 scenario の新 dir は `${ORI_SCENARIO_TMP}/new-dir`）。Linux 以外では config が fail fast する
- **fixture seed**: 既存データ前提の scenario は `onPrepare` で seed する

### 実行環境の前提 {#runtime-environment}

`runner=wdio` の実行（`npx wdio run wdio.conf.ts`）は次を前提とする:

- **`LD_LIBRARY_PATH` を外して実行する**: nix devShell 等で `LD_LIBRARY_PATH` が残っていると WebKitWebDriver が `GLIBC_2.xx not found` で起動できず、tauri-driver の準備が終わらないまま wdio が待ち続ける。`env -u LD_LIBRARY_PATH npx wdio run wdio.conf.ts` で実行する（config 側では unset しない — app が LD_LIBRARY_PATH に依存する環境を壊さないため）
- **:4444 が空いていること**: 失敗した実行の tauri-driver が残ると :4444 を占有し続ける（service は空き port に逃げるため残留に気付けない）。生成 config の `onPrepare` が占有を検出して fail fast（`SevereServiceError`）するので、`pkill tauri-driver` で解放してから再実行する（config は kill しない — 別 session の driver を巻き込まないため）

### 成功基準: plugin 警告 {#plugin-warnings}

app log の `Failed to get window states` / `Tauri plugin not available` は `tauri-plugin-wdio` の配線漏れを示す。判定は次のとおり:

- **app 起動〜最初の reload / 再起動までの間に 0 件**であること（配線漏れは初回起動から全コマンドで継続的に出る）
- **reload / 再起動直後の一過性警告は許容**する（frontend の `@wdio/tauri-plugin` 再ロード前に service が問い合わせるため `window.wdioTauri is undefined` が各回 1 件程度出る。配線漏れではない）
- reload 後に `window.wdioTauri` を待つ helper はテストに書かない（helper 自身の `browser.execute` が同じ問い合わせを誘発し得るため）

### healthcheck 待機 {#healthcheck-wait}

起動待機の戦略も config / generate 側の責務。テストコード内に待機 loop を書かない:

- **default は TCP probe**（port 疎通。image ファミリ別に `/ori-generate` が翻訳）
- `runtime.healthcheck: {http: /health}` 宣言時のみ HTTP 待機（`/health` endpoint の提供は app 側の任意 opt-in）
- DB service: TCP probe + 接続テスト、メッセージブローカー: port 疎通で判定

## サービス横断検証 {#cross-service-verification}

scenario テストは以下の組み合わせで検証する:

### ブラウザ / app 操作 {#browser-operations}

- **Playwright**: `page` オブジェクトを使用（フォーム入力、ボタンクリック、ページ遷移、UI の状態確認）
- **WDIO**: `browser` オブジェクトを使用（tauri app の window を駆動）
  - **`browser.execute` の戻り値判定は `!= null`**: WebDriver は JS の `undefined` を `null` で返す。`waitUntil` で window 上の結果を poll する場合に `!== undefined` で判定すると即座に成立し、`null.ok` 等で TypeError になる

    ```typescript
    // window 上の結果は型を明示して読む（Window に無い property は strict tsc で TS2339）
    const readResult = () => browser.execute(() => (window as unknown as { __result?: unknown }).__result);
    // NG: undefined は null で返るため即成立
    await browser.waitUntil(async () => (await readResult()) !== undefined);
    // OK
    await browser.waitUntil(async () => (await readResult()) != null);
    ```

### セレクタ / testid {#selectors}

E2E は `data-testid` を第一推奨とする（SSoT: `ddd-vsa-hex/pattern.md` §UI selector / testid 規約 → page / widget の testid 契約）。page / widget 上の要素の testid は **`.ori/pages/<id>/testids.yaml` の契約値だけ**を使い、規則から自前で導出しない。

- 参加 page ごとに `scripts/testids.js sync <page-id>`（`/ori-generate` skill bundle の script） で契約を最新化してから読む（ui-field 由来は `derived:`、それ以外は `extra:`）
- ui-fields の field id（`screen-<N>-*`）をそのまま testid にしない
- 契約に無い要素が必要なら `scripts/testids.js add-extra <page-id> --testid <kind>.<page-id>.<elem> --purpose <...> --source scenario:<scenario-id>` で追記してから使う（推測した testid をテストに直書きしない）
- 実装が既にある page の違反は `scripts/check-page-testids.sh --emit-issues --implemented-only <page-id>...`（`/ori-generate` skill bundle の script）で bd issue (`testid-violation` + `page:<id>`) にし、issue id を spec.md 実装ノートに記録する（生成は止めない。実装が契約に追いつくまで RED になる旨を明示）。実装の移行手順は [`ui-test.instructions.md#testid-migration`](../../apm_modules/dev-komenzar/ori/.apm/instructions/ui-test.instructions.md#testid-migration)

### API 呼び出し {#api-calls}

- Playwright の `request` オブジェクト、または runner から利用可能な HTTP client を使用
- REST API の呼び出し（GET / POST / PUT / DELETE）
- レスポンスの検証（ステータスコード、ボディ）

### DB 状態確認 {#db-state-verification}

- 直接 DB 接続して状態を確認
- テスト後のデータ整合性検証
- 注意: テストごとに独立データを使用（後述）

### イベント検証 {#event-verification}

- メッセージキュー（Redis / RabbitMQ 等）からのイベント受信確認
- イベントの内容検証
- タイムアウト設定（イベント到着待機）

## データ管理 {#data-management}

### テストごと独立データ {#test-isolated-data}

各テストは独立したデータを使用する。テスト間でデータを共有しない:

```typescript
test('step 1 — validation#order-create', async ({ request }) => {
  // テスト固有のデータを生成
  const orderId = `order-${Date.now()}`;

  // テスト実行
  const response = await request.post('/api/orders', {
    data: { id: orderId, /* ... */ }
  });

  // 検証
  expect(response.status()).toBe(201);
});
```

### 後始末 {#cleanup}

テスト後にデータをクリーンアップする:

```typescript
test.afterEach(async () => {
  // テストデータの削除
  // DB のロールバック
  // キューのクリア
});
```

## 注意 {#caveats}

- **生成コードマーカー必須**: 全テストファイルに `// @ori-generated scenario:<scenario-id>` を配置
- **サービス起動をテストコードに書かない**: compose / driver の起動・停止・待機は runner config の責務（`#lifecycle-ownership` 参照）
- **テスト間独立**: 各テストは独立して実行可能であること（順序依存禁止）
- **タイムアウト設定**: サービス間通信のタイムアウトを適切に設定
- **リトライ戦略**: ネットワーク不安定を考慮したリトライ設定
- **ログ出力**: テスト失敗時のデバッグ用ログを出力
