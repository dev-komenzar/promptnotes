---
paths:
  - ".ori/features/*/manifest.yaml"
---

- **必須トップレベルフィールド**: `id`, `type`, `derives_from`, `implementation`, `expected_deliverables`
- **`id`**: lower-kebab-case。ファイルパス・beads issue ID と連動するため **rename 禁止**
- **`type`**: `workflow` または `ui` のみ
- **`derives_from`**: ドメイン文書の `path` または `path#section-id` のリスト。例: `domain/aggregates.md#note-aggregate`、`domain/workflows/app-startup.md`
- **`relations`** (任意): `{ target, type }` のリスト。`type` は `derives_from` か `references` のみ（MVP）
- **`implementation`**: `language`, `primary_bc`, `generates` (生成先ファイル一覧)
- **`expected_deliverables`**: slice 完了 (Slice DoD) の必須成果物宣言。SSoT は
  `.apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md` の "Slice Definition of Done"
  section。`/ori-doctor` はこの宣言と実体生成物を突合して DoD 違反を検出する
- **不明な top-level キー禁止**: typo 検出のためスキーマは strict mode
- **編集後**: 必須キーとスキーマの自己検証必須

## `expected_deliverables` の宣言 {#expected-deliverables}

slice 種別ごとに **required boundary** を明示する。boundary が無い slice は DoD を
満たさないため、生成すべき file path を漏れなく enumerate する:

```yaml
expected_deliverables:
  slice_kind: command          # command | query | page
  sub_layers:                  # pattern.md DoD rule 1: 全埋め必須
    - domain
    - application
    - infrastructure
    - presentation             # ui slice の場合
    - tests
  boundary:                    # pattern.md DoD rule 2: 外部境界の宣言
    kind: tauri_command        # tauri_command | http_handler | direct_public_entry
    contact_points:            # tests が経由する binding / public_entry
      - apps/<app>/src-tauri/src/<bc_rs>/slices/<slice_rs>/commands.rs
      - apps/<app>/src/<bc>/shared/ipc/bindings.ts
  production_fixture:          # pattern.md DoD rule 3: production wiring 必須
    builder: setupProductionBuilder
    location: apps/<app>/src/<bc>/shared/test-fixtures/
  cross_root_contracts:        # pattern.md DoD rule 4: 同期対象 (任意)
    - generator: tauri-specta
      source: commands.rs
      output: shared/ipc/bindings.ts
```

### slice 種別 (`slice_kind`) ごとの required boundary

| `slice_kind` | required `sub_layers` | required `boundary.kind` | 補足 |
| --- | --- | --- | --- |
| `command` | `domain`, `application`, `infrastructure`, `tests` | `tauri_command` (Tauri stack) / `http_handler` (他 stack) | state を変更する use case。boundary は generator binding 必須 (pattern.md DoD rule 2) |
| `query` | `domain`, `application`, `infrastructure`, `tests` | 同上 | read-only。`application/` は副作用なし pure projection が原則 |
| `page` | 上記 + `presentation` | `direct_public_entry` | UI fragment 込みの slice。tests は presentation 層を `getByRole`/`data-testid` 経由で検証 (`ui-test.instructions.md` 参照) |

- `sub_layers` の宣言は **pattern.md DoD rule 1** の「全埋め強制」と直接連動する。
  宣言したのに実体 file が空 or placeholder のままなら `/ori-doctor` が `dod-violation`
  label 付き issue を起票する (`task-management.instructions.md` 参照)
- `cross_root_contracts` を宣言した slice は **pattern.md DoD rule 4** の対象となり、
  `/ori-flow` の `flow-impl-red` / `flow-impl-green` phase で generator 再走を強制される
