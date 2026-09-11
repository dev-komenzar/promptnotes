---
paths:
  - ".ori/scenarios/**"
---

## scenario とは {#what-is-scenario}

**scenario** は、複数サービス（フロントエンド・バックエンド・ワーカー等）の境界をまたぐ E2E 検証単位。slice / page と並ぶ ori の第一級概念。

| 概念 | 定義 | 所属 | derives_from | phase 数 |
|---|---|---|---|---|
| slice | 1 use case = 1 handler | 単一 service | workflow | 7 |
| page | UI composition unit | 単一 service | ui-fields | 7 |
| scenario | サービス横断 E2E 検証 | 複数 service | workflows + validation | 4 |

### いつ使うか {#when-to-use}

- フロントエンドの操作がバックエンドの複数 API を経由し、最終的に DB やイベントに反映されることを検証したい場合
- 複数マイクロサービス間の通信（HTTP / メッセージキュー / gRPC）を含むビジネスフローを検証したい場合
- ブラウザ操作 + API 呼び出し + DB 状態確認を組み合わせた E2E 検証が必要な場合

### slice / page との違い {#difference-from-slice-page}

- **slice**: 単一サービス内の 1 ユースケース。境界契約（boundary）を経由したテストが中心
- **page**: 単一サービス内の UI 構成単位。複数 slice の UI fragment を合成
- **scenario**: 複数サービス横断。参加 app を run-mode 別（後述）に起動し、E2E テストを実行

## run-mode と起動知識 {#run-modes}

scenario の参加者（app）は 2 つの run-mode のいずれかで起動される。**混在も可能**:

| mode | 対象 | 起動手段 | 例 |
|---|---|---|---|
| `compose-service` | web / backend 等 server 型 app | docker-compose（`docker-compose.yml` 生成対象） | Next.js frontend、hono backend |
| `local` | host process または simulator/emulator 上の app | ビルド済み binary を runner が直接起動 | Tauri desktop app、将来の RN/Expo app |

- `local` は実行基盤を `target: host | ios-simulator | android-emulator` で区別する
- **build-then-test**: E2E はビルド済み artifact に対して実行する。compose / driver の起動・停止は **runner config が所有**する（テストコード内で service を起動しない。詳細は `scenario-test.instructions.md`）
- **起動知識の SSoT は `.ori/architecture.md` の `workspace.apps[].runtime`**。mode ごとのフィールド:

  ```yaml
  runtime:
    mode: compose-service   # or local
    # compose-service: image / install / build / run / ports / healthcheck / cache_volumes
    # local:          build / binary / target (host | ios-simulator | android-emulator) / runner
  ```

- infra（postgres / redis 等）は app ではないため run-mode を持たない。`/ori-generate` の **infra catalog**（skill bundle 内 `scripts/infra-catalog.yaml`）で解決する
- **「1 scenario = 1 UI runner」制約**: scenario の UI 駆動面は単一 runner（playwright / wdio / vitest）でカバーできること

## ディレクトリ構造 {#directory-structure}

```
.ori/scenarios/<scenario-id>/
  manifest.yaml          # SSoT（人間が書く）
  spec.md                # 派生（/ori-derive が生成）
  validation.md          # 派生（Gherkin 形式、/ori-derive が生成）
  tests/
    <scenario-id>.spec.ts  # 生成テストコード（/ori-generate が生成）
  docker-compose.yml     # 自動生成（/ori-generate が生成。compose-service 系 app + infra のみ、対象ゼロなら省略）
  playwright.config.ts | wdio.conf.ts  # 自動生成（runner 別。vitest は config なし）
  status.yaml            # dirty 管理（/ori-sync が更新）
  review.md              # レビューログ（/ori-review が生成）
```

scenario ディレクトリは **self-contained**（テスト実行に必要な生成物をすべて格納）。

## manifest.yaml スキーマ {#manifest-schema}

scenario の manifest.yaml は以下のフィールドを持つ:

### 必須フィールド {#required-fields}

- **`scenario_id`**: kebab-case。**`.ori/domain/validation.md` の H2 section anchor と 1:1**（§id-convention 参照）。ファイルパス・beads issue ID と連動するため **rename 禁止**
- **`type`**: `scenario` 固定
- **`derives_from`**: ドメイン文書の `path` または `path#section-id` のリスト。**`domain/validation.md#<scenario-id>` を必ず含める**（1:1 anchor）。任意で workflow section 等を追加できる

### id 規約 {#id-convention}

- **source**: `.ori/domain/validation.md` の H2 section anchor（例: `## Scenario S1: ... {#s1-note-created-happy}` → scenario id = `s1-note-created-happy`）。**validation.md が scenario id の registry** であり、id を創作する場合は先に validation.md へ section を追加する
- **1:1 対応**: 1 scenario = 1 validation section。複数の validation section を 1 scenario で cover したい場合は、先に validation.md 側で section を統合する
- **列挙**: `new-scenario.js --list-validation`（ori-flow skill bundle の `scripts/`）で anchor 一覧と scaffold 済み coverage を確認できる
- **機械 guard**: `new-scenario.js <id>` は id が validation.md の anchor と一致しない場合・validation.md が存在しない場合にエラー停止する（推測で id を作らせない）

### オプションフィールド {#optional-fields}

- **`pages`**: 参照する page ID の配列（例: `[order-page, payment-page]`）
- **`contracts`**: サービス間契約の宣言
  - `http`: HTTP エンドポイントのリスト（例: `["POST /api/orders", "GET /api/orders/:id"]`）
  - `events`: イベント名のリスト（例: `["OrderCreated", "PaymentCompleted"]`）
  - `slices`: 参照する slice ID のリスト（例: `["create-order", "process-payment"]`）
- **`runner`**: runner の明示 override（例: `playwright` / `wdio` / `vitest`）。未指定時は derive phase が優先チェーンで解決（後述）。無効な指定（tauri 参加なのに `playwright` 等）は derive でエラー停止する
- **`infrastructure`**: インフラ構成の宣言
  - `services`: 参加者リスト（app 名 + infra 名）。起動方法は `workspace.apps[].runtime` または infra catalog から解決される
  - `overrides`: infra catalog 既定値の上書き（`overrides.<name>.{image, ports, environment}`）。上書きしない infra は記述不要

manifest は **参加者選択に専念**する。起動方法（image / command / port / healthcheck 等）は manifest に書かない。

### 例 {#example}

```yaml
scenario_id: order-flow-e2e
type: scenario
derives_from:
  - domain/workflows.md#order-workflow
  - domain/validation.md#order-flow-e2e
pages:
  - order-page
  - payment-page
contracts:
  http:
    - "POST /api/orders"
    - "GET /api/orders/:id"
    - "POST /api/payments"
  events:
    - "OrderCreated"
    - "PaymentCompleted"
  slices:
    - create-order
    - process-payment
infrastructure:
  services:
    - web
    - api
    - worker
    - redis
    - postgres
```

## 作成タイミング {#creation-timing}

1. **DDD pipeline 完了**: `/ori-distill` で workflows + validation が整備される
2. **manifest scaffold**: `new-scenario.js <id>`（ori-flow skill bundle の `scripts/`）。id は validation.md の section anchor から選択する（`--list-validation` で anchor 一覧と coverage を確認。§id-convention）。`/ori-arch` 完了時の次アクション・`/ori-feature-status` の coverage 表示が導線になる
3. **beads dep 設定**: 参加 slice の beads issue に `bd depends` が自動設定
4. **全 slice 完了で unblock**: 参加 slice が全て完了したら、scenario の `/ori-flow` が unblock

## 4 phase フロー {#four-phase-flow}

scenario は 4 phase で実装する。詳細は各 SKILL.md に委譲。

### 1. derive (`/ori-derive <scenario-id>`)

- **入力**: manifest.yaml + ドメイン文書（workflows + validation）
- **出力**: spec.md（自然言語 + 参照マッピング）+ validation.md（Gherkin）+ runner 解決結果の記録
- **責務**: ドメイン文書から scenario の仕様を派生。矛盾があれば停止し `/ori-propose` を促す
- **runner chain の解決**（優先チェーン）:
  1. manifest の `runner:`（明示指定）
  2. 参加 UI app からの導出 — `local` 系 app（tauri 等）参加時はその app の `runtime.runner`（= wdio）
  3. global `scenario_test_runner.runner`（`.ori/architecture.md`。UI app 非参加 = API-only 時の default = vitest）
  4. ハードコード default（playwright）

  無効 override はエラー停止（推測で埋めない）。「1 scenario = 1 UI runner」制約を確認する。

### 2. generate (`/ori-generate <scenario-id>`)

- **入力**: manifest.yaml + spec.md（runner 解決済み）+ validation.md（Gherkin）+ `.ori/architecture.md`（runtime blocks）
- **出力**: テストコード + runner config（`playwright.config.ts` / `wdio.conf.ts`、vitest は config なし）+ docker-compose.yml（compose-service 系 app + infra のみ。`local` 系 app は compose に含めない）
- **責務**: Gherkin シナリオからテストコードを生成、`infrastructure.services` を runtime block / infra catalog から解決して docker-compose.yml を生成
- **サービス名解決ルール**: ① `workspace.apps` と一致 → app service（runtime block から生成）② infra catalog と一致 → catalog から生成 ③ 不一致 → 停止してユーザ確認（推測で埋めない）
- **検証**: `docker compose config -q`（self-fix 1 回まで）

### 3. review (`/ori-review <scenario-id>`)

- **入力**: spec.md + テストコード + runner config + docker-compose.yml
- **出力**: review.md（PASS / NEEDS_FIX / REJECT）
- **責務**: spec ↔ テストコードの整合性を review。指摘があれば該当 phase に差し戻し

### 4. finalize (`/ori-finalize <scenario-id>`)

- **入力**: review.md（verdict=PASS）
- **出力**: dirty 解除、spec hash 更新
- **責務**: review PASS を確認し、status.yaml の dirty フラグを解除

## 相互参照 {#cross-references}

- **scenario → page**: オプション、配列（`pages: [id, ...]`）
- **page → scenario**: 参照しない
- **scenario → slice**: contracts.slices で参照
- **scenario → app**: infrastructure.services で app 名を参照（起動方法は `workspace.apps[].runtime` から解決）

## beads 連携 {#beads-integration}

- **EpicKind**: `scenario`
- **issue 名**: `ori-scenario-<scenario-id>`
- **phase issue**: derive / generate / review / finalize
- **依存**: 参加 slice の beads issue に `bd depends` 自動設定
- **dirty 伝播**: `/ori-sync` が `.ori/scenarios/` も走査、finalize で解除

## 注意 {#caveats}

- **spec.md / validation.md は派生ファイル**: 直接編集には `/ori-sync --force` が必要
- **テストコードは派生ファイル**: 直接編集には `/ori-sync --force` が必要
- **runner config（playwright.config.ts / wdio.conf.ts）は派生ファイル**: 直接編集には `/ori-sync --force` が必要
- **docker-compose.yml は派生ファイル**: 直接編集には `/ori-sync --force` が必要
- **推測で埋めない**: `TBD` を残し、人間判断に委ねる箇所を明示
- **自動 scaffold は禁止**: scenario が存在しなくても勝手に新規作成を呼ばない（ユーザ確認必須）
