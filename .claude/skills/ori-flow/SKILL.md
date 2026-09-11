---
name: ori-flow
description: 1 slice / page / scenario を 7 phase (slice/page) または 4 phase (scenario) で連続実行する薄い chain。各 phase の本体ロジック (self-fix / GREEN-on-first 検出 / reviewer spawn / dirty 解除) は対応する phase skill に閉じており、ori-flow はそれを順次呼ぶだけ。手動で /ori-derive → /ori-plan → ... と順次実行するのと完全等価
---

ユーザが `/ori-flow <id>` を呼んだ際、該当 slice / page / scenario の workflow を順次実行します。manifest の type により実行する phase が分岐します：

- **type: slice / page** → 7 phase (derive, plan, test-red, impl-green, refactor, review, finalize)
- **type: scenario** → 4 phase (derive, generate, review, finalize)

## 設計原則 — 「手動順次呼出と差ゼロ」

`/ori-flow <id>` の動作は、ユーザが手で以下を順次打ったのと**完全に等価**でなければなりません：

```
/ori-derive <id>
/ori-plan <id>
/ori-test-red <id>
/ori-impl-green <id>
/ori-refactor <id>
/ori-review <id>
/ori-finalize <id>
```

この原則を守るため、以下を厳守する：

- **各 phase の本体ロジックは対応する phase skill が責任を持つ**：
  - self-fix 1 回 / 失敗時停止 → 各 phase skill 内で完結
  - GREEN-on-first 検出と強制停止 → `/ori-test-red` 内
  - reviewer agent の fresh-context spawn と差し戻し判断 → `/ori-review` 内
  - dirty 解除・proposal 浮上 → `/ori-finalize` 内
- **ori-flow は orchestration を抱え込まない**：各 phase skill が `bd close` したかどうかだけを見て次へ進む。state を持たない・log を別の場所に書かない・review verdict を独自 parse しない。
- **bd issue が状態の SSoT**：`bd show ori-<phase>-<id>` の status が closed なら成功、open のままなら失敗・停止。

## スクリプト — Scaffold

slice / page / scenario の新規 scaffold は CLI ではなく以下のスクリプトで行う：

```bash
# slice scaffold
node ./scripts/new-slice.js <id> [--type=command|query]

# page scaffold
node ./scripts/new-page.js <id>

# scenario scaffold
node ./scripts/new-scenario.js <id>
```

manifest テンプレートは skill bundle 内の `./templates/slice-manifest.yaml.tpl` / `./templates/page-manifest.yaml.tpl` / `./templates/scenario-manifest.yaml.tpl` から読み込まれる（SSoT）。bundle 隣接 (`scripts/` の sibling) にあるため install 場所に依存せず解決される。

## 引数

- `id`：実装する slice / page / scenario の id（`.ori/slices/<id>/`、`.ori/pages/<id>/`、または `.ori/scenarios/<id>/` に存在するもの）

## 手順

1. **前提確認**：
   - `ls .ori/slices/<id>/`、`ls .ori/pages/<id>/`、または `ls .ori/scenarios/<id>/` でディレクトリの存在を確認。**存在しない場合は自動 scaffold しない**
     - 候補ある → ユーザに「これですか？」確認
     - 候補なし → 新規作成をユーザに確認してから進める
   - `bd show ori-slice-<id>` (または `ori-page-<id>`、`ori-scenario-<id>`) で epic 存在確認。なければユーザに作成を促す。
2. **type 判定**：
   - 対象ディレクトリの `manifest.yaml` を読み、`type` フィールドを確認
   - `type: scenario` の場合 → §「scenario workflow」へ進む
   - `type: slice` または `type: page` の場合 → §「slice/page workflow」へ進む
   - `type` フィールドが存在しない場合 → エラーで停止し、ユーザに確認を求める

### slice/page workflow (7 phase)

3. **phase 1: derive** — `/ori-derive <id>` を起動
4. **phase 2: plan** — `/ori-plan <id>` を起動
5. **phase 3: test-red** — `/ori-test-red <id>` を起動
6. **phase 4: impl-green** — `/ori-impl-green <id>` を起動
7. **phase 5: refactor** — `/ori-refactor <id>` を起動
8. **phase 6: review** — `/ori-review <id>` を起動（fresh-context spawn は `/ori-review` 内で処理される）
9. **phase 7: finalize** — `/ori-finalize <id>` を起動

各ステップ後、対応する `bd show ori-<phase>-<id>` を見て status が closed なら次の phase へ。closed でなければ §「停止条件」へ。

### scenario workflow (4 phase)

3. **phase 1: derive** — `/ori-derive <id>` を起動（scenario spec を domain docs から合成）
4. **phase 2: generate** — `/ori-generate <id>` を起動（scenario test code を生成）
5. **phase 3: review** — `/ori-review <id>` を起動（scenario の adversarial review）
6. **phase 4: finalize** — `/ori-finalize <id>` を起動（dirty 解除、spec hash 更新）

各ステップ後、対応する `bd show ori-<phase>-<id>` を見て status が closed なら次の phase へ。closed でなければ §「停止条件」へ。

## 停止条件

以下のいずれかが起きたら、orchestrator は即座にループを止めてユーザに hand-off する：

- 対応する `ori-<phase>-<id>` issue が closed されなかった（phase skill が self-fix 後も失敗 → 停止した）
- 対応する issue に `bd human` フラグが立った（phase skill が人間判断を要求した）
- ユーザが `Ctrl+C` 等で中断した
- manifest の `type` フィールドが存在しない、または未知の値である場合

**勝手に retry しない**：phase skill 側で既に self-fix 1 回試行して失敗している。orchestrator が追加 retry すると self-fix policy が二重適用される。

## 差し戻し（review からの巻き戻し）

`/ori-review` が指摘ありと判断した場合、`/ori-review` 自身が patch 用の phase を呼び、再度 review まで進める（**最大 1 回往復**）。orchestrator はこの差し戻しに介入しない — `bd show ori-review-<id>` が closed になるのを待つだけ。

- **slice/page workflow**: `/ori-test-red` / `/ori-impl-green` / `/ori-refactor` を呼び直し
- **scenario workflow**: `/ori-generate` を呼び直し（scenario test code の再生成）

## 注意

- subtask は beads issue description 内の `- [ ]` checklist を更新（**別 issue にしない**）
- domain 文書を変更したくなった場合は `/ori-sync --force <path>` または `/ori-propose` で proposal 生成
- **slice / page / scenario 不在時に勝手に新規作成しない**：必ずユーザ確認
- orchestrator が runner / bundle / 独自 state file を導入したくなったら、それは「各 phase skill が self-contained でない」サイン — 該当 phase skill を強化するのが正しい修正方向（orchestrator に責務を集約しない）
- **scenario type の検出**：`manifest.yaml` の `type: scenario` で判定。`type` フィールドが存在しない場合はエラーで停止

## 次のアクション

`/ori-flow` 完走後（finalize phase 完了）、ユーザに以下を提示：

- **次 slice / page / scenario パス**：`/ori-flow <next-id>` — 他に dirty な slice / page / scenario や未着手 slice / page / scenario があれば続行
- **proposal review パス**：`/ori-review-proposals` — phase 中に `--force` で生成された proposal を人間と共にレビュー
- **全体俯瞰パス**：`/ori-feature-status` で dirty / blocked / done を一覧
- **session 締めパス**：CLAUDE.md の Session Completion 手順（`bd dolt push` / `git push`）

途中停止した場合：

- **戻りパス**：失敗した phase 単独で再実行（`/ori-derive` / `/ori-test-red` / `/ori-impl-green` / `/ori-generate` 等）
- **domain 修正パス**：`/ori-propose` で upstream 修正提案
- **human flag パス**：`bd human ori-<phase>-<id>` で人間判断待ちにする
