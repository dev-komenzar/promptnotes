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
- tauri app の runner 実体は `@wdio/tauri-service`（公式推奨、macOS サポートもこれ経由）
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

このマーカーは `/ori-sync` が派生ファイルを識別するために使用する。直接編集する場合は `/ori-sync --force` が必要。

## 事前条件 {#preconditions}

### サービス起動は runner config が所有する {#lifecycle-ownership}

テストコードは **service の起動・停止・healthcheck 待機を行わない**。mode にかかわらず、lifecycle は同じ scenario ディレクトリに生成される runner config（`playwright.config.ts` / `wdio.conf.ts`）が所有する:

- **compose-service 系 app + infra**: config が `docker-compose.yml` を起動・停止する（Playwright は `webServer`、WDIO は `onPrepare`）
- **local 系 app**: **build-then-test** — ビルド済み binary を config が起動する（WDIO `onPrepare` + `tauri:options.application` で binary 指定。tauri 例: 事前に `tauri build --debug --no-bundle`）
- **`local` 系 app は docker-compose に含めない**（compose は compose-service 系 app + infra のみ）

旧「`test.beforeAll` で `docker-compose up` + healthcheck 待機」スケッチは **廃止**。テストコードに起動処理を書かないこと。

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
