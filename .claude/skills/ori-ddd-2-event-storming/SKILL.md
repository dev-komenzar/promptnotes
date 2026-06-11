---
name: ori-ddd-2-event-storming
description: distill-ddd Phase 2（Event Storming）。domain events / commands / actor / aggregate 候補を時系列で洗い出す
---

ユーザが `/ori-ddd-2-event-storming` を呼んだ際、distill-ddd Phase 2（Event Storming）を ori convention 注入版で実行します。**ビジネスで実際に起きるイベントを時系列で並べ、command / actor / aggregate 候補を可視化**します。

## 役割

- **ファシリテーター**：「次に何が起きるか」を質問で引き出す
- **時系列整理者**：イベントを時間軸に沿って並べる（前後関係を明示）
- **記録係**：合意した event / command / actor / aggregate 候補のみ `.ori/domain/event-storming.md` に書く

## 入力 / 出力

- 入力：`.ori/domain/discovery.md`（Phase 1）。なければ「Phase 1 を先にやるか確認」と促す
- 出力：`.ori/domain/event-storming.md`
  - frontmatter: `coherence: { source: human, upstream: [discovery.md] }`
  - H2/H3 すべて `{#id}` 必須

## 手順

1. **前提確認**：Phase 1 の Core Domain / Business Drivers を読み返す
2. **イベント列挙（過去形・受動）**：
   - 「ユーザが X した結果、何が起きた／何が記録された？」を時系列で
   - 1 イベント = 1 行。具体名詞を使う（「Note が保存された」「Tag が付与された」）
3. **command の同定**：各 event を引き起こす command（命令形）を 1 つ書く
4. **actor の同定**：誰が command を発行するか（user / system / external service）
5. **aggregate 候補の抽出**：event に登場する名詞をクラスタリング → 集約候補
6. **挑戦質問**：
   - 「この event は本当にビジネス的意味がある？ それとも UI イベント？」
   - 「同じ aggregate に属する event がどれか」
   - 「actor が複数いる event は分割できるか」
7. **文書生成**：合意した内容のみ Markdown で記述
8. `bash scripts/lint-domain.sh .ori/domain/event-storming.md` を実行して自己検証
9. lint 失敗時は **1 回だけ** 自動修正、それでもダメなら人間に判断委ねる

## 出力テンプレート

```markdown
---
coherence:
  source: human
  last_validated: 2026-05-14
  upstream:
    - discovery.md
---

# Event Storming {#event-storming}

## Timeline {#timeline}

| # | event (past tense) | command | actor | aggregate 候補 |
|---|---|---|---|---|
| 1 | Note が下書きされた | DraftNote | user | Note |
| 2 | Note が自動保存された | AutoSaveNote | user | Note |
| 3 | Tag が付与された | AssignTag | user | Note + Tag |

## Aggregate Candidates {#aggregate-candidates}

- **Note** — body, updatedAt, tags
- **Tag** — name (lowercase 正規化要検討)

## Open Questions {#open-questions}

- 「自動保存」の throttle 単位は分？秒？
- Tag は Note の child？ 独立 aggregate？

## Notes {#notes}

...
```

## 注意

- **イベントは過去形・具体名詞**：「保存」ではなく「Note が保存された」
- **UI イベントを混ぜない**：「ボタンが押された」は禁止
- **このスキルは workflow を回さない**

## 次のアクション

Phase 2 完了後、ユーザに以下を提示：

- **通常パス**：`/ori-ddd-3-bounded-contexts` — event クラスターから bounded context を切る
- **スキップパス**：単一 context が明らかな場合 `/ori-ddd-5-aggregates` へ直行（Phase 3/4 省略）
  - リスク：後で context を切る必要が出たら遡る
- **戻る**：core domain が不明瞭と判明したら `/ori-ddd-1-discovery` へ
