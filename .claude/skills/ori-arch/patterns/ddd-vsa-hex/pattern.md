---
ori:
  node_id: pattern:ddd-vsa-hex
  type: pattern
  version: 1.0.0
  applicable_when:
    - "domain complexity: medium-high"
    - "bounded contexts: multiple"
    - "test-first development desired"
  not_applicable_when:
    - "domain complexity: trivial (CRUD only)"
    - "single-developer prototype"
  default_layer_set: ddd-vsa-hex-ts
  alternate_layer_sets: [ddd-vsa-hex-rs]
  cross_cutting_concerns: [auth, error-handling, logging]
---

# Pattern: DDD-VSA-Hex

DDD (Domain-Driven Design) + VSA (Vertical Slice Architecture) + Hexagonal を 1
パターンに統合したアーキテクチャ。BC ごとに slice 群を縦に積み、各 slice 内は
hexagonal の薄い 4 + 1 サブレイヤで pipeline 化する。

## Summary

- **BC = top-level folder**: 1 BC = 1 ディレクトリ。BC 間は contracts / events 経由でのみ
  協調し直接 import 禁止。
- **Slice = use case 単位の vertical cut**: BC 内に `slices/<slice-id>/` を並べ、それぞれが
  domain / application / infrastructure / presentation / tests を自前で持つ。
- **Public entry per slice**: slice の対外 API は `<public_entry>`(言語別)1 ファイル
  のみ。slice 内部に外から直接 import するのはアーキ違反。
- **UI layer は別ピラミッド**: 1 ピラミッドの外側に `ui-widget` / `ui-page` (ddd-vsa-hex
  固有の ui-layer)を置き、slice の public entry 経由でのみ domain に触る。
- **SSoT は architecture.md frontmatter**: 依存ルールは全て `.ori/architecture.md` の
  frontmatter に declarative に書かれ、adapter (eslint / rust 等)が build-time に
  enforce する。

## When to use

- 中〜高複雑度のドメインで、ロジックがビジネスルール起点に育つ見込みがある。
- 複数の BC が同居し、横断的な汚染 (ある BC の事情が別 BC に染み出す)を物理的に防ぎたい。
- テスト先行 / 静的依存検査ベースで品質を底上げしたい (ユニットの粒度を slice-internal
  に押し込みたい)。
- 同一 project 内で frontend / backend (or TS / Rust)を 2 root として平行に育てる必要がある。

## When NOT to use

- CRUD だけで済むトリビアルなアプリ / プロトタイプ。レイヤ階層がオーバーヘッドになる。
- ドメインモデルが 1 個に閉じ、かつ team 規模が 1 人だけ。VSA の slice 分離コストを
  正当化しにくい。
- ライブラリパッケージ / SDK 開発。BC + slice の概念が当てはまらない。

## Tradeoffs

| 得るもの | 払うコスト |
| --- | --- |
| BC / slice 境界が物理的に lint で守られる | ファイル数 / ディレクトリ階層が深くなる |
| slice 単位の追加・差し替え・削除が安全 | 共通化したい code に thin layer をいくつか足す必要が出る |
| domain / application / infrastructure の分離が test しやすい | "1 file の方が読みやすい" 局面では分散感が出る |
| TS / Rust 両 root 同居が可能 (`cross_root` で繋ぐ) | 2 言語を維持する手間が増える |

## Conceptual structure (stack-agnostic)

```
<root.path>/
├── <slice_root>/                       # BC = 1 ディレクトリ (e.g. task-management)
│   ├── shared/                         # BC-internal shared (kind: shared)
│   │   ├── types/                      # Result / branded VOs / 共通型
│   │   ├── events/                     # base DomainEvent shape
│   │   ├── contracts/                  # cross-slice 契約 (default empty)
│   │   └── <ipc/>                      # (stack 依存) 例: tauri-specta bindings
│   └── slices/                         # slice_subdir
│       └── <slice-id>/                 # 1 use case = 1 slice
│           ├── <public_entry>          # 対外 PUBLIC API — 唯一の出入口
│           ├── domain/                 # aggregates / VO / events (pure)
│           ├── application/            # use case orchestration
│           ├── infrastructure/         # adapters / I/O
│           ├── presentation/           # view model / pure render (or commands)
│           └── tests/                  # slice-local tests
├── <ui-widget>/                        # ddd-vsa-hex ui-layer (order 1, 任意)
└── <ui-page>/                          # ddd-vsa-hex ui-layer (order 2)
```

- BC を複数同居させる場合は `<slice_root>` を横に並べる(`task-management/` と
  `billing/` 等)。
- frontend / backend など複数 root を同居させる場合は `architecture.md` の
  `roots:` で複数宣言し、`cross_root:` で生成物 (例: type-bridge bindings)を declare。

## Layer responsibilities

### Top-level layers

| layer | kind | 責務 |
| --- | --- | --- |
| `shared` | shared | BC-internal の共通基盤 (Result / branded VO / event base / contracts)。誰からも import されてよいが何も import しない |
| `domain` | slice | 1 child = 1 slice。BC のユースケースを vertical に切り出した単位 |
| `ui-widget` | ui-layer (order 1) | 複数 slice を横断する UI コンポーネント (任意) |
| `ui-page` | ui-layer (order 2) | page = 複数 slice の宿主。ルーティング / page-level state を担う |

### Slice-internal sub-layers

slice 内部は一方向 pipeline:

```
presentation → application → domain
infrastructure → domain
tests → (presentation, application, infrastructure, domain)
```

| sub-layer | 責務 |
| --- | --- |
| `domain` | aggregate / VO / domain event / pure 関数。I/O / framework 依存禁止 |
| `application` | use case orchestration。domain 関数を組み合わせ port (infrastructure)を呼ぶ |
| `infrastructure` | I/O adapter (repository / API client / DB)。domain で宣言された port を実装 |
| `presentation` | UI 向けの薄い変換 (view model / pure render)。Rust では tauri commands 等の external interface |
| `tests` | slice-local test。同じ slice 内の任意の sub-layer に到達可 |

## Dependency rules

### Cross-layer (top-level)

```
ui-page    → [ui-widget, shared, domain]
ui-widget  → [shared, domain]
domain     → [shared]
shared     → []
```

`same_layer: prohibited` — 同じ layer の sibling 同士の import は不可
(例: 1 つの ui-widget が別の ui-widget を直接呼ぶ等)。

### Cross-slice

```
prohibited_direct: true
via: [shared/contracts, shared/events]
```

slice A が slice B の事情を必要とする場合は、`<slice_root>/shared/contracts/` に
型を宣言するか `shared/events/` に domain event を発行し、両者がその contract に
依存する形に倒す。slice 直 import は静的検査で reject される。

### Cross-BC

```
via: [<root.path>/shared/contracts, <root.path>/shared/events]
same_event_bus: true
```

BC をまたぐ場合は app-level の `shared/` を経由する。同 app 内では 1 つの
event bus を共有する想定 (multi-app 化したら別途 `cross_app:` で宣言)。

### Cross-root (任意、複数言語/複数 deploy 単位時)

```
cross_root:
  - from: { root: <id>, path: <generator-source> }
    to:   { root: <id>, path: <generated-binding> }
    generator: <generator-name>
    auto_generated: true
```

例: tauri-specta が Rust の `#[tauri::command]` から TS の bindings.ts を生成する
ケース等。生成物は片方の root から手書きで触らない。

## Naming conventions

- **BC name (TS / kebab-case)**: `task-management`, `billing`, `order-fulfillment` 等。
  ドメイン語彙をそのまま使う。
- **BC name (Rust / snake_case)**: TS の `task-management` ↔ Rust の `task_management`。
  言語識別子規則に従う。
- **Slice id**: 動詞句 + 名詞で use case を表現 (`complete-task`, `create-order`,
  `archive-task` 等)。TS は kebab、Rust は snake で揃える。
- **Public entry**: 言語別 (`index.ts` / `mod.rs` / `lib.rs` 等)で 1 ファイル統一。
- **Slice ファイル**: `domain/<vo-name>.ts` / `application/<verb-noun>.ts` 等、
  PascalCase 型と関数名は中身で命名、ファイル名は kebab/snake で統一。
- **Branded VO**: 型名は PascalCase (`TaskId`、`TaskTitle`)、コンストラクタは
  lowercase verb (`taskId(raw)`, `taskTitle(raw)`)、Error 型は `<VO>Error` 命名。

## Cross-cutting concerns placement

| 関心事 | 配置先 | 理由 |
| --- | --- | --- |
| **Auth / authorization** | `<root.path>/shared/guards/` (生成物)、宣言は `.ori/architecture.md` の `cross_cutting_concerns` | 全 slice 横断で必要、SSoT 1 箇所 |
| **Error 共通型** | `<slice_root>/shared/types/result.rs` (or `.ts`) | BC ごとに `Result<T, AppError>` を共有 |
| **Logging** | `<root.path>/shared/logger.ts` (生成物) | 1 instance を全 slice が import |
| **Domain event base** | `<slice_root>/shared/events/event.ts` | BC ごとに event の基底形を 1 つ |
| **Event bus** | `<root.path>/shared/events/event-bus.ts` (生成物) | BC をまたいだ event 配信が 1 hub |
| **Validation / Result** | `<slice_root>/shared/types/result.ts` | "ok / err" の判定だけは BC 全体で揃える |

cross-cutting を slice 内部に閉じ込めると、slice 間の対称性が崩れて結局
app-level に重複が出る。逆に最初から app-level に寄せると BC ごとの方言が
作りにくくなる。`shared/` の 2 段構成 (BC-internal + app-level)はそのバランスを
取るためのもの。
