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

## 前提条件(test readiness) {#test-readiness}

scenario の generate は、生成物だけで E2E が走るよう app 側の前提を満たす必要がある。`runner=wdio`(local Tauri)時の確定事項:

- **build-then-test の binary 契約**: `runtime.build` は `runtime.binary` を生成する command でなければならない。Tauri の `cargo build` 単体は devUrl 参照の dev binary になるため不可。`tauri build --debug --no-bundle`(例: `bun run build:test`)を使う
- **plugin 前提**: `@wdio/tauri-service` は `driverProvider` に関係なく `tauri-plugin-wdio` を必須とする。未導入時は focus 系コマンド(`$` / `$$` / `findElement(s)` / `elementClick` / `getTitle`)ごとに 5 秒待機する。配線は Cargo dep + capabilities `wdio:default` + `lib.rs` の `#[cfg(debug_assertions)]` 登録 + frontend 動的 import(`VITE_WDIO_TEST` gate)。production 非混入
- **storage 隔離（XDG temp）**: runner config の `onPrepare` が per-run temp root を `mkdtemp` し、`XDG_CONFIG_HOME` / `XDG_DATA_HOME` / `XDG_CACHE_HOME` / `XDG_STATE_HOME` を root 配下に向ける。Tauri の `app_config_dir` / `app_data_dir`、WebKitGTK の storage、window-state 等はすべて XDG で解決されるため、app 側 override 無しで settings / preferences を含め隔離される。`HOME` は差し替えない（XDG を経由せず `$HOME` を直接読む app 処理は隔離対象外）。temp root は `ORI_SCENARIO_TMP` として test code / seed に公開する
  - app 固有の storage override env（例: 旧 ori 標準の `TAURI_TEST_STORAGE_DIR`）は ori 標準ではなく、generate は注入しない。app は保存先を `app_config_dir` / `app_data_dir` 等の XDG 解決に任せる。`runtime.test_env` は固定文字列のみで per-run temp を指せない（per-run の path が必要な test は `ORI_SCENARIO_TMP` を使う）
- **platform**: wdio scenario は **Linux のみ正式サポート**（XDG 隔離が効くのは Linux のみ。macOS は tauri-driver 非対応）。非 Linux では runner config が隔離を保証できない旨を明示して fail fast する（`SevereServiceError`。普通の `Error` は wdio launcher に握りつぶされる）。runner config は `maxInstances: 1` 固定（>1 / multiremote では `@wdio/tauri-service` が `XDG_DATA_HOME` を上書きする）
- **node_modules 解決**: `.ori/scenarios/node_modules` → `apps/<app>/node_modules` の symlink
- **fixture seed**: 既存データ前提の scenario は `onPrepare` で seed する。frontmatter 形式の SSoT は domain / app 側

`/ori-derive` が spec.md の実装ノートにこれらを記録し、`/ori-generate` が生成物に反映する。frontend への plugin import だけは framework 固有のためコード生成せず、実装ノートの要求として残す。

## ディレクトリ構造 {#directory-structure}

```
.ori/scenarios/<scenario-id>/
  manifest.yaml          # SSoT（人間が書く）
  spec.md                # 派生（/ori-derive が生成）
  tests/
    <scenario-id>.spec.ts  # 生成テストコード（/ori-generate が生成）
  docker-compose.yml     # 自動生成（/ori-generate が生成。compose-service 系 app + infra のみ、対象ゼロなら省略）
  playwright.config.ts | wdio.conf.ts  # 自動生成（runner 別。vitest は config なし）
  status.yaml            # phase 台帳 + dirty 管理（phase skill が scenario-status.js で更新、/ori-sync が dirty を伝播）
  review.md              # レビューログ（/ori-review が生成）
```

scenario ディレクトリは **self-contained**（テスト実行に必要な生成物をすべて格納）。

## manifest.yaml スキーマ {#manifest-schema}

scenario の manifest.yaml は以下のフィールドを持つ:

### 必須フィールド {#required-fields}

- **`scenario_id`**: kebab-case。**`.ori/domain/validation.md` の H2 section anchor と 1:1**（§id-convention 参照）。ファイルパス・beads issue ID と連動するため **rename 禁止**

  > ⚠️ **1 scenario = 1 validation section。複数の validation section を 1 scenario にまとめないこと。**
  > まとめたい場合は先に validation.md 側で section を統合する。`new-scenario.js --list-validation` は
  > 各 section がそれぞれ 1 つの scenario に対応する前提で表示する。
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
  - `slices`: 参照する slice ID のリスト（例: `["create-order", "process-payment"]`）。**任意かつ非要件**: 順序制約・traceability のための情報リンクであり、scenario 検証の成立要件でも slice 実装との対応表でもない。slice 依存ゲートは撤去済みのため blocking しない（未指定・未実装 slice の参照でも scenario は scaffold / RED 可能）
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
2. **manifest scaffold**: `new-scenario.js <id>`（ori-flow skill bundle の `scripts/`）。id は validation.md の section anchor から選択する（`--list-validation` で anchor 一覧と coverage を確認。§id-convention）。`/ori-architect` 完了時の次アクション・`/ori-feature-status` の coverage 表示が導線になる
3. **即 `/ori-flow` 可能（scenario-first 既定）**: scaffold 直後から scenario の `/ori-flow`（derive → generate → review → finalize）を回してよい。参加 slice の完了は待たない（slice が 0 件完了でも 4 phase は通る）

scenario は slice 完了から独立している。ori は scenario を実行しない（実行は CI / 手動）ため、slice 完了は ori にとって検証可能な前提ではない。slice 未実装の状態で生成されたテストは実行時に RED となり、それが未実装を可視化する。参加 slice の beads issue への `bd depends` 設定・slice 完了ゲートは行わない（`contracts.slices` は blocking しない情報リンク。§optional-fields）。

## 4 phase フロー {#four-phase-flow}

scenario は 4 phase で実装する。詳細は各 SKILL.md に委譲。

### 1. derive (`/ori-derive <scenario-id>`)

- **入力**: manifest.yaml + ドメイン文書（workflows + validation）
- **出力**: spec.md（自然言語 + 参照マッピング + `#scenario-steps` は Gherkin（`Scenario:` / `Given` / `When` / `Then`）で書くことが必須。`Then` 件数は domain/validation.md#<id> と一致させる）+ runner 解決結果の記録。scenario dir に `validation.md` は生成しない（Gherkin の原典は `.ori/domain/validation.md`、派生側は spec.md に内包）
- **責務**: ドメイン文書から scenario の仕様を派生。矛盾があれば停止し `/ori-propose` を促す
- **runner chain の解決**（優先チェーン）:
  1. manifest の `runner:`（明示指定）
  2. 参加 UI app からの導出 — `local` 系 app（tauri 等）参加時はその app の `runtime.runner`（= wdio）
  3. global `scenario_test_runner.runner`（`.ori/architecture.md`。UI app 非参加 = API-only 時の default = vitest）
  4. ハードコード default（playwright）

  無効 override はエラー停止（推測で埋めない）。「1 scenario = 1 UI runner」制約を確認する。

### 2. generate (`/ori-generate <scenario-id>`)

- **入力**: manifest.yaml + spec.md（runner 解決済み。Gherkin は `#scenario-steps`）+ `.ori/architecture.md`（runtime blocks）
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
- **scenario → slice**: contracts.slices で参照（任意の情報リンク。1:1 対応は不要）
- **scenario → app**: infrastructure.services で app 名を参照（起動方法は `workspace.apps[].runtime` から解決）

## beads 連携 {#beads-integration}

- **EpicKind**: `scenario`
- **issue 名**: `ori-scenario-<scenario-id>`
- **phase issue**: derive / generate / review / finalize
- **依存**: phase issue 間の順序依存のみ。参加 slice の beads issue への `bd depends` は設定しない（scenario-first。§creation-timing）
- **dirty 伝播**: `/ori-sync` が `.ori/scenarios/` も走査、finalize で解除

### phase 台帳 (status.yaml) と bd の役割 {#phase-ledger}

- **bd issue** (`ori-scenario-<id>` / `ori-<phase>-<scenario-id>`) が **phase 進行の SSoT**（closed かどうかで次 phase へ進む）
- **`.ori/scenarios/<id>/status.yaml`** は **機械可読な成果物台帳**（bd と相補的）。各 phase skill が完了時に決定的 writer で必ず更新する (writer は各 phase skill の `scripts/` に同梱):
  ```bash
  node <phase skill>/scripts/scenario-status.js set <scenario-id> <phase> done
  node <phase skill>/scripts/scenario-status.js show <scenario-id>
  ```
- **手で編集しない**: writer が legacy schema を正規化する（`phases` の文字列値、`beads`/`dirty` 欠落を修復）。`phases` / `beads.completion` が phase 完走の記録

## 注意 {#caveats}

- **派生ファイルの正規更新手順（唯一）**: spec.md・テストコード・runner config・docker-compose.yml はすべて派生ファイル。直接編集しない（`/ori-sync --force` は廃止済）。手順は次の 1 つ:
  1. 変えたい内容の **source を編集**する（`manifest.yaml` / ドメイン文書 / `.ori/architecture.md`）
  2. `/ori-sync` で dirty を伝播 → `/ori-flow <scenario-id>`（該当 phase を再実行）で再生成
  3. source 側に不備があり上流の変更が要る場合は `/ori-propose` で提案を作成する
- **spec.md frontmatter の hash**: `coherence.upstream[].hash` は `/ori-derive` が `resolve-upstream.sh` の出力から書き込む値（upstream **ファイル全体**の sha256 先頭 12 hex。`path#section` 指定でも section 単位ではない）が正典。`<section-id>` や `abc123` 等の placeholder・手書き値は不可。scenario spec.md の hash 更新は `/ori-finalize` では未実装（R3 で扱う）で、現状は `/ori-derive` 実行時点の値
- **推測で埋めない**: `TBD` を残し、人間判断に委ねる箇所を明示
- **scenario = 検証軸、実装は別ワークフロー**: scenario は未充足を RED として示すことに徹する。ori は scenario を実行せず（実行は CI / 手動）、RED の対処も scenario 側では行わない
  - RED の対処は実装軸の別ワークフローで行う: `/ori-flow <slice-id>`（未実装・未 finalize の slice）または `/ori-bug`（case 4: cross-slice bug）
  - scenario に実装を書かない（impl phase を持たない 4 phase 設計を維持し、2 軸の独立性を保つ）
  - scenario と slice を 1:1 対応させる必要はない（1 scenario が複数 slice に跨る / slice を持たない scenario も可）
- **自動 scaffold は禁止**: scenario が存在しなくても勝手に新規作成を呼ばない（ユーザ確認必須）
