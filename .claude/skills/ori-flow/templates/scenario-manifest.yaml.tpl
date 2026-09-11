# scenario manifest — SSoT（人間が書く）。new-scenario.js が scaffold する。
#
# scenario id は .ori/domain/validation.md の H2 section anchor と 1:1
# （例: `## Scenario S1: ... {#s1-note-created-happy}` → scenario id = s1-note-created-happy）。
# rename 禁止。field 仕様の SSoT: instructions/scenario.instructions.md §manifest-schema。
scenario_id: "{{id}}"
type: scenario
derives_from:
  # 必須: この scenario が anchor する validation section（1:1）
  - domain/validation.md#{{id}}
  # 任意: 関連 workflow section
  # - domain/workflows.md#<workflow-id>
# ---- optional fields（詳細は scenario.instructions.md §manifest-schema）----
# pages:                # 参照する page ID
#   - <page-id>
# contracts:           # サービス間契約
#   http:
#     - "POST /api/orders"
#   events:
#     - "OrderCreated"
#   slices:
#     - create-order
# runner: playwright    # 明示 override（未指定時 derive phase が優先チェーンで解決）
# infrastructure:       # 参加者（app 名 + infra 名。起動方法は runtime / infra catalog から解決）
#   services:
#     - web
#     - postgres
#   overrides: {}
