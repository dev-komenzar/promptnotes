---
name: ori-feature-status
description: slice / page の進捗を一覧 or 個別で要約表示。status.yaml + beads issue 状態 + dirty マーク を統合した俯瞰ビュー
---

ユーザが `/ori-feature-status [slice-id]` を呼んだ際、**slice / page の現在地を at-a-glance で表示**します。`.ori/slices/` と `.ori/pages/` のファイル一覧、`status.yaml`、`bd show` の結果を統合し、見やすい形に整形。

## 役割

- **状況集約者**：複数の情報源（status.yaml / beads / git）を統合
- **優先順位提案者**：dirty で priority 高 / blocker 多い slice を上位に
- **next-action 提示者**：見た直後に「次に何を打てばよいか」が分かる出力

## 入力 / 出力

- 入力：
  - 引数なし → 全 slice / page 一覧
  - `slice-id` 引数 → 当該 slice の詳細
- 出力：標準出力に整形済みレポート。**ファイル生成しない**

## 表示モード

### 全体モード（引数なし）

```
ori status (全 <N> slice / page)

  id                            kind    phase            dirty   beads          last activity
  ──────────────────────────────────────────────────────────────────────────────────────────────
  capture-auto-save             slice   review           ✓       3 open         2026-05-14 13:20
  edit-past-note-start          slice   derive           ✓✓      5 open         2026-05-14 09:11
  capture-form                  page    done             -       0 open         2026-05-13 21:42
  switch-edit-target            slice   scaffold         -       7 open         (not started)

Legend: in progress / done / blocked / ✓ dirty (1 mark) / ✓✓ dirty (≥2)

Recommended next action:
  - edit-past-note-start: 2 dirty marks. Re-derive via /ori-flow edit-past-note-start
  - capture-auto-save: review pending. /ori-review or continue /ori-flow
```

### 個別モード（引数あり）

```
slice: capture-auto-save

Manifest:
  type:           command
  derives_from:
    - domain/aggregates.md#note-aggregate
    - domain/workflows/capture-auto-save.md

Status (.ori/slices/capture-auto-save/status.yaml):
  phase:          review
  dirty:          1 mark
    - upstream domain/aggregates.md#note-aggregate hash mismatch
  last_derived:   2026-05-14 13:20
  last_validated: 2026-05-14 12:05

Beads (epic ori-slice-capture-auto-save):
  ori-derive-...        closed
  ori-plan-...          closed
  ori-test-red-...      closed (4 tests written)
  ori-impl-green-...    closed
  ori-refactor-...      closed
  ori-review-...        in_progress
  ori-finalize-...      open

Proposals: 0 pending

Files (git diff vs main):
  apps/<app>/src/note-capture/slices/capture-auto-save/  18 files changed (+520 / -12)
  .ori/slices/capture-auto-save/                          4 files changed (manifest/spec/status/review)

Next action:
  Complete /ori-review capture-auto-save (in progress)
  Note: 1 dirty mark — re-derive needed before merge
```

## 手順

1. **引数判定**：
   - 引数なし → 全 slice / page を列挙
   - 引数あり → 個別 slice (or page) を詳細表示
2. **データ収集**：
   ```bash
   bash ./scripts/list-slices.sh
   bash ./scripts/list-pages.sh
   ```
   - `--dirty` オプションで dirty な slice のみ表示も可能
3. **dirty マーク検出**：
   - `status.yaml.dirty[]` の件数
   - 派生元ファイルの hash 不一致を確認
4. **last activity 算出**：
   - `git log -1 --format=%ai .ori/slices/<id>/`
   - or beads issue の最新 updated_at
5. **next-action の決定**（heuristic）：
   - dirty があれば「再 derive」
   - phase が in_progress なら「該当 phase の継続」
   - proposal 残れば「/ori-review-proposals」
   - 全て clean なら「次の slice の `/ori-flow`」
6. **整形して出力**：絵文字 + ANSI color（端末対応時）

## 出力フォーマット

- 全体モード：1 行 1 slice / page の表（id / kind / phase / dirty / beads / last activity）
- 個別モード：セクション分けの詳細ブロック

## 注意

- **read-only**：副作用なし。ファイル変更しない
- **複数情報源の不整合に注意**：`status.yaml` と beads がズレている場合は `/ori-doctor` を案内
- **大量 slice / page の場合**：`--limit` で上位 20 件のみ表示、`--all` で全表示
- **CI 用には `--json` を提案**：将来の dashboard 用

## 次のアクション

レポート内容に応じて以下を案内：

- **dirty slice / page がある場合**：影響の大きい順に `/ori-flow <id>` で再 derive
- **review pending の slice がある場合**：`/ori-review <id>` で adversarial レビュー
- **未着手 slice / page がある場合**：`/ori-flow <id>` で開始
- **proposal がある場合**：`/ori-review-proposals` で人間判断
- **全てクリーンな場合**：`/ori-distill` で次の DDD phase に進む、または休む
- **情報源不整合パス**：`/ori-doctor` で詳細診断
