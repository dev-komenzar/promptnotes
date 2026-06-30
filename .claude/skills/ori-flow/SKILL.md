---
name: ori-flow
description: 1 slice / page を 7 phase で連続実行する薄い chain。各 phase の本体ロジック (self-fix / GREEN-on-first 検出 / reviewer spawn / dirty 解除) は対応する phase skill に閉じており、ori-flow はそれを順次呼ぶだけ。手動で /ori-derive → /ori-plan → ... と順次実行するのと完全等価
---

ユーザが `/ori-flow <id>` を呼んだ際、該当 slice / page の 7-phase workflow を順次実行します。

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

slice / page の新規 scaffold は CLI ではなく以下のスクリプトで行う：

```bash
# slice scaffold
node ./scripts/new-slice.js <id> [--type=command|query]

# page scaffold
node ./scripts/new-page.js <id>
```

manifest テンプレートは skill bundle 内の `./templates/slice-manifest.yaml.tpl` / `./templates/page-manifest.yaml.tpl` から読み込まれる（SSoT）。bundle 隣接 (`scripts/` の sibling) にあるため install 場所に依存せず解決される。

## 引数

- `id`：実装する slice / page の id（`.ori/slices/<id>/` または `.ori/pages/<id>/` に存在するもの）

## 手順

1. **前提確認**：
   - `ls .ori/slices/<id>/` または `ls .ori/pages/<id>/` でディレクトリの存在を確認。**存在しない場合は自動 scaffold しない**
     - 候補ある → ユーザに「これですか？」確認
     - 候補なし → 新規作成をユーザに確認してから進める
   - `bd show ori-slice-<id>` (または `ori-page-<id>`) で epic 存在確認。なければユーザに作成を促す。
2. **phase 1: derive** — `/ori-derive <id>` を起動
3. **phase 2: plan** — `/ori-plan <id>` を起動
4. **phase 3: test-red** — `/ori-test-red <id>` を起動
5. **phase 4: impl-green** — `/ori-impl-green <id>` を起動
6. **phase 5: refactor** — `/ori-refactor <id>` を起動
7. **phase 6: review** — `/ori-review <id>` を起動（fresh-context spawn は `/ori-review` 内で処理される）
8. **phase 7: finalize** — `/ori-finalize <id>` を起動

各ステップ後、対応する `bd show ori-<phase>-<id>` を見て status が closed なら次の phase へ。closed でなければ §「停止条件」へ。

## 停止条件

以下のいずれかが起きたら、orchestrator は即座にループを止めてユーザに hand-off する：

- 対応する `ori-<phase>-<id>` issue が closed されなかった（phase skill が self-fix 後も失敗 → 停止した）
- 対応する issue に `bd human` フラグが立った（phase skill が人間判断を要求した）
- ユーザが `Ctrl+C` 等で中断した

**勝手に retry しない**：phase skill 側で既に self-fix 1 回試行して失敗している。orchestrator が追加 retry すると self-fix policy が二重適用される。

## 差し戻し（review からの巻き戻し）

`/ori-review` が指摘ありと判断した場合、`/ori-review` 自身が patch 用の phase（`/ori-test-red` / `/ori-impl-green` / `/ori-refactor`）を呼び、再度 review まで進める（**最大 1 回往復**）。orchestrator はこの差し戻しに介入しない — `bd show ori-review-<id>` が closed になるのを待つだけ。

## 注意

- subtask は beads issue description 内の `- [ ]` checklist を更新（**別 issue にしない**）
- domain 文書を変更したくなった場合は `/ori-sync --force <path>` または `/ori-propose` で proposal 生成
- **slice / page 不在時に勝手に新規作成しない**：必ずユーザ確認
- orchestrator が runner / bundle / 独自 state file を導入したくなったら、それは「各 phase skill が self-contained でない」サイン — 該当 phase skill を強化するのが正しい修正方向（orchestrator に責務を集約しない）

## 次のアクション

`/ori-flow` 完走後（phase 7 finalize 完了）、ユーザに以下を提示：

- **次 slice / page パス**：`/ori-flow <next-id>` — 他に dirty な slice / page や未着手 slice / page があれば続行
- **proposal review パス**：`/ori-review-proposals` — phase 中に `--force` で生成された proposal を人間と共にレビュー
- **全体俯瞰パス**：`/ori-feature-status` で dirty / blocked / done を一覧
- **session 締めパス**：CLAUDE.md の Session Completion 手順（`bd dolt push` / `git push`）

途中停止した場合：

- **戻りパス**：失敗した phase 単独で再実行（`/ori-derive` / `/ori-test-red` / `/ori-impl-green` 等）
- **domain 修正パス**：`/ori-propose` で upstream 修正提案
- **human flag パス**：`bd human ori-<phase>-<id>` で人間判断待ちにする
