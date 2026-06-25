---
name: ori-migrate
description: 既存プロジェクト（docs/domain/ がある等）を ori 構造へ移行し、検出済み phase から slice / page を一括 scaffold する
---

ユーザが `/ori-migrate` を呼んだ際、**既存プロジェクトを ori 構造へ昇格**します。例：promptnotes-vcsdd の `docs/domain/` を `.ori/domain/` に移行し、Phase 9（workflows）と Phase 11（ui-fields）の完了度を検出し、対応する slice / page を一括 scaffold します。**dogfooding readiness の要**となるスキル。

## 役割

- **移行計画者**：既存構造を読み、ori 構造との差分を計画として提示
- **検出器**：DDD 各 phase の完了度を文書から推定（Phase 9 / 11 を特に重視）
- **scaffold 提案者**：検出した slice / page の一括 scaffold をユーザに提案

## 入力 / 出力

- 入力：プロジェクトルート（`.git` がある場所）。典型的には：
  - `docs/domain/` 配下の Markdown 群
  - `docs/specs/`、`docs/architecture/` 等の補助文書
  - 既存の `src/`
- 出力：
  - `.ori/` ディレクトリ構造
  - `.ori/domain/` 配下（旧 `docs/domain/` から移行）
  - `.ori/migrate-report.md` — 移行サマリ（検出結果・移動先・未対応事項）
  - 必要に応じて `.ori/slices/<id>/manifest.yaml` / `.ori/pages/<id>/manifest.yaml`（一括 scaffold）

## 手順

1. **前提確認**：
   - `git status` が clean であることを推奨（既存ファイルを移動するため）
   - `.ori/` が既に存在する場合は中断し「`--force` を付けるか手動で merge してください」と案内
2. **ソース構造の調査**：Bash で以下を実行：
   ```bash
   ls -la docs/ 2>/dev/null
   find docs -name '*.md' -type f | head -50
   ```
   - `docs/domain/` の Markdown を全列挙
   - frontmatter があれば既に ori 化されているか判定
3. **phase 完了度の検出**（heuristic）：

   | phase | 検出ヒント |
   |-------|----------|
   | 1 discovery | `discovery.md` / `core-domain.md` / "Problem Space" 見出し |
   | 2 storming | `events.md` / "Domain Event" 多数 / event-storming photo log |
   | 3 contexts | `bounded-contexts.md` / "Bounded Context" 見出し |
   | 4 mapping | `context-map.md` / "Upstream/Downstream" |
   | 5 aggregates | `aggregates.md` / 集約名 H2 が複数 |
   | 6 events | `domain-events.md` |
   | 7 validate | `validate.md` / "Use Case Scenario" |
   | 8 glossary | `glossary.md` / "Ubiquitous Language" |
   | 9 workflows | `workflows/` ディレクトリ or `workflows.md` |
   | 10 types | `types.md` / TypeScript snippet が多い |
   | 11a ui-fields | `ui-fields/` or `screens.md` |
   | 11b ui-grouping | `page-groups.md` or `feature-groups.md` (旧) |

4. **計画の提示**：検出結果を表で表示しユーザに確認：
   ```
   検出された phase 完了度：
     Phase 1 (discovery)        ✓  → .ori/domain/discovery.md
     Phase 5 (aggregates)       ✓  → .ori/domain/aggregates.md
     Phase 9 (workflows)        ✓ split detected → .ori/domain/workflows/
     Phase 10 (types)           △ partial; review needed
     Phase 11a (ui-fields)      ✓ → .ori/domain/ui-fields/
     Phase 11b (ui-grouping)    ✗ missing

   推奨アクション：
     - Phase 11b は `/ori-ddd-11b-ui-grouping` で後追い
     - Phase 10 は移行後に `/ori-ddd-10-types` で補完

   この計画で移行しますか？ [Y/n]
   ```
5. **ファイル移動**：ユーザ承認後、Bash で実行（**`-f` 必須**、対話モードを避ける）：
   ```bash
   mkdir -p .ori/domain
   mv -f docs/domain/* .ori/domain/
   ```
   - `git mv` が使えるなら history を保つため優先：
     ```bash
     git mv docs/domain .ori/domain
     ```
6. **frontmatter 注入**（既に `ori:` がなければ、design.md §5 に従う）：
   - 各 .md を順に読み、frontmatter を upsert
   - `ori.node_id: <type>:<file-stem>` / `ori.type: <controlled-vocabulary>` / `ori.depends_on: []` を最低限挿入
   - controlled vocabulary は design.md §5 表（discovery / persona / event-storming / bounded-context / context-map / relationship / aggregate / event / scenario / glossary-term / workflow / ui-field / page-grouping / type-definitions）
   - H2/H3 の `{#id}` 欠落を検出し、提案として `migrate-report.md` に列挙（**自動修正しない**：domain 文書は人間判断）
7. **`.ori/domain/` の整合性確認**：`for f in .ori/domain/*.md; do bash scripts/lint-domain.sh "$f"; done` を実行。失敗箇所は `migrate-report.md` に記載

## Phase 別 scaffold 案内

   - 新規 slice を作成し bead issue を用意
   - 新規 page を作成し bead issue を用意
     ```
9. **migrate-report.md の書き出し**：
   ```markdown
   # Migration Report {#migrate-report}

   ## Moved {#moved}

   - docs/domain/ → .ori/domain/ (N files via git mv)

   ## Detected Phases {#detected-phases}

   | phase | status | output |
   |-------|--------|--------|
   | 1 discovery | ✓ | .ori/domain/discovery.md |
   | 5 aggregates | ✓ | .ori/domain/aggregates.md |
   | 9 workflows | ✓ split | .ori/domain/workflows/ |
   | 11b ui-grouping | ✗ | needs /ori-ddd-11b-ui-grouping |

   ## Scaffolded Slices / Pages {#scaffolded-slices-pages}

   - capture-auto-save (slice)
   - capture-form (page)

   ## Manual Follow-Ups {#manual-followups}

   - `aggregates.md` の H2 "Note Aggregate" に `{#id}` 欠落（自動修正しない、人間が決める）
   - Phase 10 (types) が部分的。再走推奨
   ```
10. **完了**：
    ```bash
    bd close ori-migrate-<project> --reason="migration done; N slices / pages scaffolded; M manual followups in migrate-report.md"
    ```

## 注意

- **破壊的操作なので git status clean を強く推奨**：移動後の rollback は `git checkout HEAD -- .` で
- **`mv -f` を使う**：AGENTS.md の non-interactive 規約に従う
- **frontmatter の `{#id}` 不足は自動修正しない**：domain 文書は人間判断（CLAUDE.md ケース 1 と整合）
- **既存 src/ は触らない**：移行は domain 層のみ。code 移行は別 issue
- **dogfooding 対象**：promptnotes-vcsdd を初手で通すための skill。失敗時のフィードバックは最優先で反映

## 次のアクション

migration 完了後、ユーザに以下を提示：

- **欠落 phase の補完**：`migrate-report.md` で `✗` がついた phase に対応する `/ori-ddd-*` を順に呼ぶ
- **scaffold 済み slice / page の実装開始**：`/ori-flow <id>` で 7-phase 実装
- **既存テスト / src の取り込み**：別 issue として提案（後続 phase）
- **失敗時の rollback**：`git checkout HEAD -- .` で移行をリセット可能
