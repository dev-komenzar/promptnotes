---
name: ori-architect
description: /ori-arch から委譲され、要件対話 (platforms / os_integration / ui_native 等) から `.ori/architecture.md` を動的生成する。DDD + vsa-hex の核 (invariants) は不変、ビルド/配信/OS 統合の差は decision_points としてメイン session で対話確定する。/ori-init → /ori-arch の次に呼ばれる。
---

`/ori-arch` の「ori artifact 追加」step として動作する。**このスキルはメイン session で
実行される** (要件対話 = ヒアリングが必要なため。subagent は headless でユーザと対話できない —
ori-c79 で agent として定義したが ori-8gz でスキルへ書き直した)。

## 入力 / 出力

- 入力：
  - ユーザ要件（platforms / os_integration / ui_native / language / BC 名）— 対話で引き出す
  - app 名（`.ori/config.yaml` の `workspace.apps[0].name`）
  - upstream framework init 済みの `apps/<app>/`（`/ori-arch` の手順 5 で案内済み）
- 出力：`.ori/architecture.md` 1 ファイルのみ。それ以外 ori は target にファイルを足さない
  - frontmatter: ArchitectureSpec (`version: 1`、root/roots、layer_sets、slice_internal、
    cross_slice、cross_bc、cross_root、`phase_hooks:`)
  - 本文: `## Decisions` に decision_points の回答を記録

## 手順

1. **elicit** — `questions:` を順に提示し、ユーザの回答を得る（推奨 + 上書き可。ハイブリッド UI 対応）
2. **decide** — decision_points を確定し、roots（id / language / adapter / slice_root /
   public_entry）と layer_sets を決める
3. **compose** — `invariants:` から layer graph / slice_internal / boundaries を選択・結合して
   frontmatter を組み立てる
4. **generate** — `.ori/architecture.md` を書く（前回生成物がある場合は上書き可否を先に確認）
5. **self-check** — guardrails g-1..g-8 を検証する：`node .apm/skills/ori-doctor/scripts/lint.js .ori`
   が全 pass すること（fail したら修正して再生成）
6. **confirm** — 生成物をユーザに提示し確定を得る

## ロール

あなたは ori workflow の **architecture expert** です。利用者の要件を対話で引き出し、
`.ori/architecture.md`（依存グラフの SSoT）を 1 ファイル動的生成します。
DDD + vsa-hex パターンの核（`invariants`）は常に維持し、言語 / 配信先 / OS 統合の差は
スタック側の変数として要件対話から確定します。テンプレートの cartesian product では
なく、「要件 → architecture.md」の 1 つの生成手順で CLI（server のみ）から
web + ios + android + desktop までの multiplatform をカバーします。

固定ガイド（`docs/start/typescript-web.md` / `docs/start/tauri-v2.md`）の選択肢は
**参考実装**であり、要件が一致すれば同型の出力を再現できますが、要件が異なれば
`questions` / `generation_procedure` に従って自由に組み合わせます。

## invariants

DDD + vsa-hex の**不変部分**。`pattern.md` / 旧 `stacks/*/architecture.md.tpl` から
抽出した共通項で、どの stack を選んでも維持しなければならない。doctor がこの YAML を
機械 parse して生成結果を検証する。

```yaml
invariants:
  # ── 1. layer graph (layer_sets + rules) ──────────────────────────────
  # tpl の layer_sets から抽出。UI を持つ stack は ddd-vsa-hex-ts を、
  # backend のみ / 2 言語同居は ddd-vsa-hex-rs を組み合わせる。
  layer_graph:
    ddd-vsa-hex-ts:
      layers:
        - { id: shared, kind: shared }
        - { id: domain, kind: slice, slice_internal: slice-internal-ts }
        - { id: ui-widget, kind: ui-layer, order: 1 }
        - { id: ui-page, kind: ui-layer, order: 2 }
      rules:
        cross_layer:
          - { from: ui-page, allow: [ui-widget, shared, domain] }
          - { from: ui-widget, allow: [shared, domain] }
          - { from: domain, allow: [shared] }
          - { from: shared, allow: [] }
        same_layer: prohibited
        public_entry_required: true
    ddd-vsa-hex-rs:
      layers:
        - { id: shared, kind: shared }
        - { id: domain, kind: slice, slice_internal: slice-internal-rs }
      rules:
        cross_layer:
          - { from: domain, allow: [shared] }
          - { from: shared, allow: [] }
        same_layer: prohibited
        public_entry_required: true

  # ── 2. slice-internal sub-layers (one-way pipeline) ───────────────────
  # slice 内部は常に一方向。tests のみ任意の sub-layer に到達できる。
  # (注) ddd-vsa-hex-rs の application は infrastructure への到達を許す
  #       (tpl 由来: Rust の application が port 実装を呼ぶ経路)。
  slice_internal:
    slice-internal-ts:
      sub_layers: [domain, application, infrastructure, presentation, tests]
      rules:
        - { from: presentation, allow: [application, domain] }
        - { from: application, allow: [domain] }
        - { from: infrastructure, allow: [domain] }
        - { from: domain, allow: [] }
        - { from: tests, allow: [domain, application, infrastructure, presentation] }
    slice-internal-rs:
      sub_layers: [domain, application, infrastructure, presentation]
      rules:
        - { from: presentation, allow: [application, domain] }
        - { from: application, allow: [domain, infrastructure] }
        - { from: infrastructure, allow: [domain] }
        - { from: domain, allow: [] }

  # ── 3. boundaries (import 境界) ──────────────────────────────────────
  boundaries:
    # 各 slice の対外 API は public entry 1 ファイルのみ
    # (TS: index.ts / Rust: mod.rs)。slice 内部への直 import は違反。
    public_entry_required: true
    cross_slice:
      prohibited_direct: true
      via: [shared/contracts, shared/events]
    # BC をまたぐ場合は app-level の shared/ 経由。同 app 内は event bus を 1 つ共有。
    cross_bc:
      via_pattern: <app-level shared/contracts + shared/events>
      same_event_bus: true
    # 複数 root (例: TS + Rust) 同居時のみ。生成物は手書き禁止。
    cross_root:
      - from: { root: <id>, path: <generator-source> }
        to: { root: <id>, path: <generated-binding> }
        generator: <generator-name>
        auto_generated: true
```

## guardrails

生成される `.ori/architecture.md` が満たすべき**検証ルール**。doctor がこの
YAML を機械 parse し、生成結果に各 `check` を適用して適合判定する。guardrail 違反は
「単なる好み」ではなく、SSoT としての architecture.md が不正であることを意味する。

```yaml
guardrails:
  - id: g-1
    target: frontmatter
    check: parseArchitectureSpec() が pass する (version=1、root か roots[] が必須)
    failure: 依存グラフ SSoT として解釈できない frontmatter
  - id: g-2
    target: layer_sets
    check: 使用する layer_set id は全て invariants.layer_graph に存在し、layers / cross_layer / same_layer / public_entry_required が一致する
    failure: 未知の layer_set または invariants を書き換えた dependency rule
  - id: g-3
    target: slice_internal
    check: 宣言された slice_internal id (root の layer.slice_internal) は全て invariants.slice_internal に存在し、sub_layers / rules が一致する
    failure: 一方向 pipeline を破る sub-layer 定義
  - id: g-4
    target: cross_slice
    check: prohibited_direct が true、via が [shared/contracts, shared/events] を含む
    failure: slice 直 import が lint で reject されない設定
  - id: g-5
    target: cross_bc
    check: BC 間は app-level shared/contracts + shared/events 経由、same_event_bus が true
    failure: BC をまたぐ直接依存の温床
  - id: g-6
    target: public_entry
    check: 全 root で public_entry_required が true、public_entry が 1 ファイルに解決される
    failure: slice 内部への直 import が可能な設定
  - id: g-7
    target: cross_root
    check: "auto_generated: true の生成物は手書き対象外で、generator が明示されている"
    failure: 生成物と手書きの境界が消える
  - id: g-8
    target: decision_points
    check: platforms / os_integration / ui_native 等の decision_point が全て確定済み (未回答のまま生成しない)
    failure: 要件未確定のままの architecture.md (後で大きな書き換えが必要)
```

## questions

要件対話で埋める **decision_points**。各質問は「推奨 + 上書き可」で提示し、回答を
`generation_procedure` の入力にする。`affects` は回答が architecture.md のどのフィールド
を決めるかを示す。

```yaml
questions:
  platforms:
    prompt: "配信/実行ターゲットを列挙してください。例: server / web / ios / android / desktop / cli"
    options: [server, web, ios, android, desktop, cli]
    default: [web]
    affects: roots の構成、layer_sets の選択 (UI 有無)
  os_integration:
    prompt: OS 統合 (native API / window / tray / ファイルシステム 等) が必要ですか
    options: [none, tauri, electron, capacitor, react-native]
    default: none
    affects: "cross_root の要否 (例: tauri-specta)、forbidden_imports の有無"
  ui_native:
    prompt: UI は web 技術 (DOM) / ネイティブ widget / ハイブリッド のどれですか
    options: [web, native, hybrid]
    default: web
    affects: ui-widget / ui-page layer の採用、presentation の実装形態
  language:
    prompt: 各プラットフォームの実装言語を確定してください
    options: [typescript, rust, swift, kotlin, ...]
    default: typescript
    affects: roots[].language / adapter、slice_internal の選択 (ts / rs)
  bc_names:
    prompt: "最初の BC 名を決めてください (識別子規則: TS=kebab-case / Rust=snake_case)"
    default: (ユーザ入力)
    affects: roots[].slice_root と public_entry パス
  cross_root_contracts:
    prompt: root 間で共有する生成物 (type bridge 等) があれば宣言してください
    default: []
    affects: cross_root エントリ、phase_hooks の要否
```

## generation_procedure

要件から `.ori/architecture.md` を 1 ファイル生成する手順の機械可読仕様。
(手順の実行はこの SKILL.md 冒頭の「手順」に従う。この YAML は doctor 等の
参照用。)

```yaml
generation_procedure:
  steps:
    - id: elicit
      action: questions を順に提示し回答を得る (推奨を示し上書きを許可する)
    - id: decide
      action: decision_points を確定し roots (id / language / adapter / slice_root / public_entry) と layer_sets を決める
    - id: compose
      action: invariants から layer graph / slice_internal / boundaries を選択・結合して frontmatter を組み立てる
    - id: generate
      action: .ori/architecture.md 本文 (layout / rules / regenerate 手順) を書く。page map 等の auto-generated 領域は marker で保護する
    - id: self-check
      action: 生成結果に guardrails を適用し全 pass を確認する (ori-doctor と同一基準)
    - id: confirm
      action: 生成物をユーザに提示し確定を得る。違反があれば修正して再生成
  output_rules:
    - 生成物は .ori/architecture.md 1 ファイルのみ (bootstrap 系は upstream framework init に委譲)
    - shared / domain は常に invariants の層構造に従う
    - decision_point の回答は生成物のコメントや本文の "Decisions" 節に残す
    - "frontmatter に phase_hooks: block を含める (hook 不要な stack は phase_hooks: {})"
```

## 注意

- **知らない stack を捏造しない**: 既存の実績 (typescript / typescript-tauri) から
  要件差分を設計し、未検証の組み合わせは「実験的」と明記する。
- **guardrails は交渉しない**: ユーザ要望が invariants と衝突する場合、理由を説明して
  不変部分は守ったまま decision_points 側で折衷案を提示する。
- **market 参考実装**: LobeHub (React のみで Web+Mobile+Electron)、Bluesky (Expo RN で
  Web+iOS+Android 一本化 + `*.web` / `*.android` / `*.ios` 分岐) は「1 コードベースで
  複数配信」の参考にできるが、ori の layer vocabulary にそのまま当てはめない。