---
name: ori-ddd-6-domain-events
description: distill-ddd Phase 6（Domain Events）。各 aggregate が発行する domain event を H3 単位で記述する
---

ユーザが `/ori-ddd-6-domain-events` を呼んだ際、distill-ddd Phase 6（Domain Events）を ori convention 注入版で実行します。**aggregate ごとに発行する event のスキーマ・トリガー・購読者を確定**します。

## 役割

- **ファシリテーター**：aggregate の不変条件・公開操作から event を導出
- **挑戦者**：「この event はビジネス的意味を持つか UI 通知か」を問う
- **記録係**：1 event = 1 H3、`{#event-id}` 必須

## 入力 / 出力

- 入力：`.ori/domain/aggregates.md`（Phase 5）、`.ori/domain/event-storming.md`（Phase 2）、`.ori/domain/context-map.md`（Phase 4）
- 出力：`.ori/domain/domain-events.md`
  - frontmatter: `ori:` ブロック（design.md §5）
    - `node_id: event:collection`（file-level representative）
    - `type: event`
    - `depends_on: [aggregate:collection, event-storming:timeline, context-map:map]`
  - 個別 event node は H3 anchor から導出（例: `### NoteSaved {#note-saved}` → `event:NoteSaved`）
  - **H2 = aggregate**、**H3 = 個別 event**

## 手順

1. **前提確認**：Phase 5 の aggregate と Phase 2 の event 候補を読み返す
2. **event の正式化**：Phase 2 で candidate として挙がった event を aggregate 別に整理し、正式名（過去形・PascalCase）を付ける
3. **各 event について対話**（必須項目）：
   - **{#trigger}** — どの command が成功すると発行されるか
   - **{#payload}** — event が運ぶデータ（aggregate id + 必要な VO のみ）
   - **{#subscribers}** — どの context / aggregate が購読するか（context-map と整合）
   - **{#timing}** — 同期 / 非同期、ordering 要件
4. **挑戦質問**：
   - 「この event は本当に外部に意味があるか？ aggregate 内部の状態変化に過ぎないなら不要」
   - 「payload が aggregate 全体を含んでいないか？ 必要最小限か」
   - 「subscriber が context map に書いた関係と整合しているか」
5. **文書生成**：H2 = aggregate、H3 = event
6. `bash scripts/lint-domain.sh .ori/domain/domain-events.md` を実行して自己検証
7. lint 失敗時は **1 回だけ** 自動修正、それでもダメなら人間判断

## 出力テンプレート

```markdown
---
ori:
  node_id: event:collection
  type: event
  depends_on:
    - aggregate:collection
    - event-storming:timeline
    - context-map:map
---

# Domain Events {#domain-events}

## Note Aggregate {#note-aggregate-events}

### NoteSaved {#note-saved}

#### Trigger {#note-saved-trigger}

`CaptureAutoSave` / `EditNote` command の成功時

#### Payload {#note-saved-payload}

```ts
{
  noteId: NoteId;
  bodyHash: string;       // body 全文は載せない
  occurredAt: Instant;
  version: int;
}
```

#### Subscribers {#note-saved-subscribers}

- Tag Management context（PL 経由、ACL で正規化）
- Search Indexer（非同期）

#### Timing {#note-saved-timing}

非同期 / at-least-once delivery / ordering by `noteId`

### NoteEmptied {#note-emptied}

...

## Tag Aggregate {#tag-aggregate-events}

...
```

## 注意

- **event は過去形 PascalCase**（NoteSaved、TagAssigned）
- **payload は最小限**：aggregate 全体を載せない（version + id + 変化点のみ）
- **subscribers は context-map と整合**：矛盾したら戻って修正
- **このスキルは workflow を回さない**

## 次のアクション

Phase 6 完了後、ユーザに以下を提示：

- **通常パス**：`/ori-ddd-7-validation` — use case シナリオで event 連鎖を検証
- **早期切上げパス**：MVP では validation を省略し `/ori-ddd-9-workflows` へ
  - リスク：event 連鎖の欠陥は workflow で初めて顕在化
- **戻る**：subscriber が context-map に書かれていない場合は `/ori-ddd-4-context-map` へ
