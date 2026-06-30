---
name: ori-derive
description: /ori-flow phase 1。manifest の derives_from とドメイン文書から slice spec.md を合成する
---

ユーザが `/ori-derive <slice-id>` を呼ぶ、または `/ori-flow` 内部から phase 1 として起動した際に、**該当 slice の `spec.md` をドメイン文書から派生**します。**派生のみ。実装は phase 4 の責務**。

## 引数

- `slice-id`：対象 slice の id（`.ori/slices/<id>/` が存在する事を前提）

## 役割

- **派生器**：`manifest.yaml` の `derives_from:` に列挙されたドメイン section を読み、slice 単位の spec に再構成
- **整合性チェッカー**：複数 upstream に矛盾があれば停止し、`/ori-propose` を促す
- **記録係**：spec.md は **derived** ファイル。`coherence.source: derived` で書き、人間が直接編集すると `/ori-sync --force` が要求される

## 入力 / 出力

- 入力：
  - `.ori/slices/<id>/manifest.yaml`（必須。`derives_from:` を持つ）
  - `manifest.derives_from` に列挙されたドメイン section（例：`domain/aggregates.md#note-aggregate`）
- 出力：`.ori/slices/<id>/spec.md`
  - frontmatter: `coherence.source: derived`、`upstream:` に派生元 section を列挙、`hash:` に派生元のスナップショットハッシュ
  - 必須 H2 セクション（**`.apm/instructions/feature-spec.instructions` 準拠** — instructions ファイル名は legacy のまま）

## 必須セクション（`.apm/instructions/feature-spec.instructions.md` 準拠）

| H2 | id | 内容 |
|----|----|------|
| `## 概要 {#overview}` | overview | この slice が解く問題、対応する workflow / UI |
| `## 入出力 {#io}` | io | 入力（command / form）と出力（event / view）の型 |
| `## 不変条件 {#invariants}` | invariants | slice 完了時に成り立つ条件（domain 不変条件 + slice 固有制約） |
| `## 境界契約 {#boundary-contract}` | boundary-contract | Slice DoD rule 2 を満たす external boundary 宣言 (kind / contact point / public_entry / 禁止 import) |
| `## テスト観点 {#test-points}` | test-points | テストで検証すべきシナリオ列挙（phase 3 で test 化）。**boundary test 経由項目を必ず含める** |
| `## 実装ノート {#impl-notes}` | impl-notes | アーキ層への落とし込みヒント（依存 interface 等）。Slice DoD rule 1/3/4 由来の項目を必ず含める |

各 H2 は `{#id}` 必須。H3 を追加する場合も `{#id}` 必須。**section 雛形 / 記述例の SSoT は `.apm/instructions/feature-spec.instructions.md`** — このスキル内に template を duplicate しない (DoD 拡張時の sync 漏れを防ぐ)。

## 手順

1. **slice 存在確認**：
   ```bash
   bash ./scripts/check-slice-exists.sh <slice-id>
   ```
   - exit 0: 存在 → 次のステップへ
   - exit 2: 類似候補あり → ユーザに「これですか？」と確認、Yes なら正しい id で再開
   - exit 1: 未発見 → 新規 slice 作成を**ユーザに確認**してから進める
2. **manifest.yaml の読み込み**：`.ori/slices/<id>/manifest.yaml` を Read。`derives_from:` が空ならエラー停止し「先に DDD phase で domain を整備するか、manifest に upstream を追記してください」と案内
3. **upstream section の取得**：
   ```bash
   bash ./scripts/resolve-upstream.sh <slice-id>
   ```
   派生元 section のパスとハッシュを取得
4. **矛盾検出**：複数 upstream が同じ概念について異なる規定を持つ場合、停止して `/ori-propose` を促す（自動マージしない）
5. **spec.md の synthesis**：
   - 上記 6 セクションを必須として埋める (記述形式は `feature-spec.instructions.md` を SSoT として参照)
   - 不明な事項は **推測で埋めず** `**TBD**` マーカーを残し、後段で人間に問う
   - 上流 section の文言を引用する際は `> domain/aggregates.md#note-aggregate より:` の出典を残す
6. **Slice DoD 由来 item の derive**：`manifest.yaml` の `expected_deliverables` block (SSoT: `.apm/instructions/feature-manifest.instructions.md`) と `.ori/architecture.md` の stack 情報を読み、以下を spec.md の所定 section に必ず差し込む。**hardcoded sample をスキルに置かず**、各 instructions の記述例を SSoT として参照する:
   - **`## 境界契約 {#boundary-contract}`**: `expected_deliverables.boundary.kind` と `contact_point` を写し、`production_fixture.location` と `cross_root_contracts[].generator` を併記。`feature-spec.instructions.md` の "境界契約 section 必須化" 記述例に揃える
   - **`## テスト観点 {#test-points}`**:
     - "boundary 経由 boundary test" 項目を 1 つ以上必須 (pattern.md DoD rule 2)。`cross_root` がある stack なら "tauri-specta bindings 経由 invoke" と書く、無ければ "slice の public_entry 経由 import" と書く
     - "production fixture 経由のみで構築" 項目 (pattern.md DoD rule 3) — `setupProductionBuilder()` 雛形を参照
   - **`## 実装ノート {#impl-notes}`**:
     - "stub commands.rs を `Err(\"pending\")` 返す形で先に置く" (RED state b3、`/ori-impl-red` の sub-step。`/ori-impl-red` SKILL.md を参照)
     - "production fixture (`apps/<app>/src/<bc>/shared/test-fixtures/`) を構築 (未設置なら追加)" — DoD rule 3
     - "全 sub_layers (`domain`/`application`/`infrastructure`/`presentation`/`tests`) 埋め込み" — DoD rule 1
     - "`cross_root_contracts` を持つ slice では phase_hooks (architecture.md `phase_hooks.flow-impl-{red-pre,green-post}`) で specta 再生成 — DoD rule 4"
7. **spec.md の自己検証**：
   - 必須 H2 6 種が揃っているか（`## 概要`、`## 入出力`、`## 不変条件`、`## 境界契約`、`## テスト観点`、`## 実装ノート`）
   - 境界契約に boundary kind / contact point が宣言済か
   - テスト観点に boundary test と production fixture の項目が両方あるか
   - 実装ノートに stub commands / production fixture / sub_layers / phase_hooks のいずれかへの言及があるか (slice_kind が `command` で `cross_root` を持つ場合は **全部必須**、それ以外は該当する rule だけ必須)
   - 全 H2/H3 に `{#id}` があるか（grep: `^###? [^{]+$`）
   - frontmatter `coherence.source: derived` と `upstream:` の有無
8. 検証失敗時は **1 回だけ** 自動修正を試み、それでも失敗ならユーザに判断を委ねる
9. **beads issue 更新**：
   ```bash
   bd update ori-derive-<slice-id> --status=closed --notes="spec.md generated from <N> upstream sections"
   ```

## 出力フォーマット

spec.md の構造 (frontmatter / 必須 H2 / 記述例) の SSoT は `.apm/instructions/feature-spec.instructions.md`。**スキル内に template / sample を duplicate しない** (DoD 拡張時の sync 漏れを防ぐため、ori-fzr.6 以降の方針)。各 section の具体的な記述例は instructions 内の "記述例" / "boundary-contract section 必須化" 等を参照すること。

## 注意

- **自動 scaffold は禁止**：slice が存在しなくても勝手に新規作成を呼ばない（ユーザ確認必須）
- **spec.md は派生ファイル**：直接編集には `/ori-sync --force` が必要
- **推測で埋めない**：`TBD` を残し、人間判断に委ねる箇所を明示
- このスキルは test や impl を書かない。**phase 1 = spec 派生のみ**
- **SSoT 参照原則** (ori-fzr.6 以降): spec.md の section 仕様 / 記述例 / DoD 由来 item 雛形は **このスキル内に hardcoded で書かない**。常に `.apm/instructions/feature-spec.instructions.md` / `feature-manifest.instructions.md` / `.apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md` を読み込んで参照する。pattern.md DoD rule の改訂時にスキル更新が漏れて drift するのを防ぐため

## 次のアクション

phase 1 完了後、`/ori-flow` 内部なら自動的に phase 2 へ。単独呼び出しの場合：

- **メインパス**：`/ori-plan <slice-id>` — phase 2。下流 phase（test-red / impl-green / refactor / review）の beads issue description を埋める
- **TBD を解消するパス**：`/ori-distill phase=<関連 phase>` でドメインに戻り合意形成 → 再度 `/ori-derive`
- **矛盾発見パス**：`/ori-propose` で upstream 修正提案を作成 → 人間レビュー
