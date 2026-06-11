---
name: ori-bug
description: 人間が見つけたバグを README の 4 ケース（domain / impl / spec / cross-slice）に triage し、対応する動線を案内する。実行はしない、ルーティングのみ
---

ユーザが `/ori-bug` を呼んだ際、**バグの所在を ori graph 上のどのノードに帰属させるか**を 4 つの triage 質問で診断し、対応する recovery flow を提案します。**自動で fix は実行しない**——README のケース分類に基づいてユーザを正しい動線へ送るだけ。

## 役割

- **triage 担当**：4 つの質問でバグを 4 ケース（domain / impl / spec / cross-slice）に分類
- **動線案内人**：各ケースに対応するスキル呼び出しを提示
- **実行禁止者**：fix は他スキル / 人間の責務。このスキルは類型化と案内のみ

## バグの 4 ケース（README より）

| # | バグの所在 | 症状 | 起点 |
|---|----------|------|------|
| **1** | **ドメインモデル**が誤り | 不変条件抜け、概念の境界違い | `.ori/domain/` 編集 |
| **2** | **spec は正しいが impl が誤り**（テスト網羅漏れ） | impl が edge case で落ちる | 失敗テスト追加 |
| **3** | **spec 自体に欠陥**（domain は正しいが derive が悪い） | 派生時に取りこぼし／曲解 | `--force` で spec 編集 → proposal |
| **4** | **複数 slice の統合バグ** | 単体は OK だが組み合わせで破綻 | 新規 slice 作成 |

## triage 質問（順番に問う）

1. **「ドメインモデルが捉え損ねている事象か？」**
   - Yes → **ケース 1（domain bug）**
   - No → 次へ
2. **「spec.md にこの動作の規定があるか？」**
   - No → **ケース 3（spec bug）**
   - Yes → 次へ
3. **「spec の規定通り impl が動かない？」**
   - Yes → **ケース 2（impl bug）**
   - No → 次へ
4. **「単一 slice の範囲を超える？」**
   - Yes → **ケース 4（cross-slice bug）**
   - No → ユーザに「症状をもう少し詳しく」と問い返す

## 手順

1. **症状ヒアリング**：
   - 何が起きるべきで、実際は何が起きたか
   - 再現手順
   - 関係する slice id（推定で OK）
2. **関連文書の参照**（必要に応じて Read）：
   - `.ori/slices/<id>/spec.md`
   - `.ori/domain/aggregates.md` の該当 section
3. **4 つの triage 質問を順に問う**：上記フロー
4. **分類結果と動線を提示**（実行はしない）
5. **ユーザが動線を実行する宣言をしたら**、対応するスキル（/ori-distill / /ori-flow / /ori-propose）へ promot を促す

## ケース別の動線（提示のみ、実行しない）

### ケース 1（domain bug）

```
分類：ドメインバグ。.ori/domain/ の編集が必要。

推奨動線：
  1. Read / Edit .ori/domain/aggregates.md  ← 不変条件を追加 / 修正
  2. `/ori-sync`                         ← dirty 化された slice を一覧表示
  3. `/ori-flow <dirty-slice-1>`        ← 影響 slice を順次再走

ヒント：複数 slice が dirty 化される可能性が高い。最も影響の大きい
slice から /ori-flow で再 derive することを推奨。
```

### ケース 2（impl bug）

```
分類：実装バグ。spec.md は正しい想定。

推奨動線：
  1. .ori/slices/<id>/tests/ に失敗テストを追加（whitespace / unicode 等）
  2. pnpm test                      ← RED 確認
  3. `/ori-impl-green <id> --reason "manual bug: <概要>"`
  4. `/ori-review <id>` (推奨)

ヒント：ドメインには触らない。テストを先に書いて修正範囲を局所化する。
```

### ケース 3（spec bug）

```
分類：spec バグ。domain は正しいが派生が悪い。

推奨動線：
  1. Read / Edit .ori/slices/<id>/spec.md
  2. `/ori-sync`                       ← guardrail がブロックする
      エラー：spec.md is derived. Edit blocked.
        [1] Edit domain source
        [2] Force edit + upstream proposal: `/ori-sync --force <path>`
  3. オプション [2] を選んだ場合：proposal が .ori/proposals/ に生成される
  4. `/ori-review-proposals`          ← 人間レビューで accept/reject

ヒント：多くの場合 domain を直すのが正解（ケース 1 への昇格）。
spec で局所決定したい時のみ --force を使う。
```

### ケース 4（cross-slice bug）

```
分類：統合バグ。複数 slice にまたがる。

推奨動線：
  1. `/ori-distill phase=workflows`   ← Phase 9 に戻り欠落シナリオを追加
  2. 新規 slice を作成
  3. `/ori-flow <integration-slice-id>`

ヒント：既存 slice を編集せず、新規 slice として境界を明確にする。
「ドメインモデルにシナリオが欠けていた」とほぼ同義。
```

## 注意

- **fix を実行しない**：このスキルはルーティングのみ。実際の修正は対象スキルに渡す
- **ケース 1 ↔ ケース 3 は紙一重**：迷ったら domain を直す方向を優先（ケース 1 へ昇格）
- **アンチパターン回避**：「impl だけパッチ」「spec を `--force` なし編集」「review skip」は禁止（README 参照）

## 次のアクション

triage 結果に応じて以下を案内（実行はしない）：

- **ケース 1 と分類された場合**：`.ori/domain/<file>.md` の編集 → `/ori-sync` → `/ori-flow <dirty-id>`
- **ケース 2 と分類された場合**：失敗テスト追加 → `/ori-impl-green <id> --reason "bug fix"` → `/ori-review <id>`
- **ケース 3 と分類された場合**：`/ori-sync --force <spec>` で proposal 生成 → `/ori-review-proposals`
- **ケース 4 と分類された場合**：`/ori-ddd-9-workflows` 再走 → 新規 slice 作成 → `/ori-flow`
- **どれにも分類できないパス**：症状情報が不足。ユーザにヒアリング継続、難しければ `bd human` で人間判断 flag
