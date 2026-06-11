---
name: ori-ddd-7-validation
description: distill-ddd Phase 7（Validation）。use case シナリオを Given/When/Then で記述し、aggregates と events を walkthrough で検証する
---

ユーザが `/ori-ddd-7-validation` を呼んだ際、distill-ddd Phase 7（Validation）を ori convention 注入版で実行します。**use case シナリオの walkthrough で、aggregate / event 設計の整合性を検証**します。

## 役割

- **シナリオ起草者**：典型 / 異常 / edge case を Given/When/Then で書く
- **整合性検証者**：シナリオが既存 aggregate と event で表現できるか確認。できなければ前 phase に戻る
- **記録係**：単一ファイル `.ori/domain/validation.md` に記述（**ファイル分割しない**）

## 入力 / 出力

- 入力：`.ori/domain/aggregates.md`（Phase 5）、`.ori/domain/domain-events.md`（Phase 6）、`.ori/domain/event-storming.md`（Phase 2）
- 出力：`.ori/domain/validation.md`（**単一ファイル**）
  - frontmatter: `coherence: { upstream: [aggregates.md, domain-events.md, event-storming.md] }`
  - シナリオごとに H2、H3 = Given/When/Then

## 手順

1. **前提確認**：Phase 5, 6 の aggregate / event を読み返す
2. **シナリオ候補の列挙**：
   - 典型ケース（happy path） — 各 workflow に 1 つ以上
   - 異常ケース — error / 失敗パターン
   - edge case — 境界値、並行性、空入力
3. **各シナリオを Given/When/Then で記述**：
   - **Given**：初期状態（aggregate のスナップショット）
   - **When**：発行する command 列
   - **Then**：期待される event 列 + 最終 aggregate 状態
4. **挑戦質問**：
   - 「このシナリオを既存の aggregate / event で表現できる？ できないなら Phase 5/6 で抜けがある」
   - 「Then の event が domain-events.md に未定義ではないか」
   - 「並行 command 時の振る舞いは決定的か」
5. **文書生成**：単一ファイル `.ori/domain/validation.md` にシナリオを列挙
6. `bash scripts/lint-domain.sh .ori/domain/validation.md` を実行して自己検証
7. lint 失敗時は **1 回だけ** 自動修正、それでもダメなら人間判断

## 出力テンプレート

```markdown
---
coherence:
  source: human
  last_validated: 2026-05-14
  upstream:
    - aggregates.md
    - domain-events.md
    - event-storming.md
---

# Validation Scenarios {#validation-scenarios}

## Scenario: First auto-save of empty draft {#first-auto-save-empty}

### Given {#first-auto-save-empty-given}

- 新規ユーザ
- 空の Note draft（body = ""）

### When {#first-auto-save-empty-when}

- `CaptureAutoSaveCommand{ body: "   " }`

### Then {#first-auto-save-empty-then}

- event `NoteSaved` は発行されない
- error `EmptyBody`
- aggregate Note は永続化されない

## Scenario: Edit past note (idempotent) {#edit-past-idempotent}

### Given {#edit-past-idempotent-given}

- 既存 Note (id=note-1, body="hello", updatedAt=t0)

### When {#edit-past-idempotent-when}

- `EditNoteBodyCommand{ noteId: note-1, body: "hello" }` を 2 回

### Then {#edit-past-idempotent-then}

- 1 回目：event `NoteSaved` 発行、updatedAt が t1 (> t0)
- 2 回目：event 発行しない（同一 body）、updatedAt = t1 のまま
```

## 注意

- **ファイル分割しない**：validation は workflow と違い網羅性より連続性が重要
- **シナリオで前 phase の欠陥が見つかったら戻る**：Phase 5 / 6 で抜けが発見されることが多い
- **このスキルは workflow を回さない**

## 次のアクション

Phase 7 完了後、ユーザに以下を提示：

- **通常パス**：`/ori-ddd-8-glossary` — ユビキタス言語を整理
- **早期 workflow 化パス**：glossary は後回しにして `/ori-ddd-9-workflows` でも可
- **戻る**：シナリオで表現できないなら `/ori-ddd-5-aggregates` or `/ori-ddd-6-domain-events` を見直す
