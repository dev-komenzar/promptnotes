---
paths:
  - ".ori/features/*/manifest.yaml"
---

- **必須トップレベルフィールド**: `id`, `type`, `derives_from`, `implementation`
- **`id`**: lower-kebab-case。ファイルパス・beads issue ID と連動するため **rename 禁止**
- **`type`**: `workflow` または `ui` のみ
- **`derives_from`**: ドメイン文書の `path` または `path#section-id` のリスト。例: `domain/aggregates.md#note-aggregate`、`domain/workflows/app-startup.md`
- **`relations`** (任意): `{ target, type }` のリスト。`type` は `derives_from` か `references` のみ（MVP）
- **`implementation`**: `language`, `primary_bc`, `generates` (生成先ファイル一覧)
- **不明な top-level キー禁止**: typo 検出のためスキーマは strict mode
- **編集後**: 必須キーとスキーマの自己検証必須
