<!-- @ori-generated scenario:s26-unbundled-notify-only -->
# s26-unbundled-notify-only — test-points map {#test-points-map}

| test point | 状態 | テストケース / 代替担保 |
|---|---|---|
| TP1 — Given の成立 | COVERED | `tests/s26-unbundled-notify-only.spec.ts:31` `given — validation#s26-given` |
| TP2（核）— 通知のみ | COVERED | `tests/s26-unbundled-notify-only.spec.ts:44` `step 1 — validation#s26-then` |
| TP3 — install 要求の拒否 | COVERED | `tests/s26-unbundled-notify-only.spec.ts:58` `step 2 — validation#s26-then`（kind=UnsupportedInstallation） |
| TP4 — ネットワークに触れない | COVERED | `tests/s26-unbundled-notify-only.spec.ts:58` `step 2 — validation#s26-then`（request 数不変） |
| TP5 — release build（nix package）の配布形式判定 | N/A(代替担保) | `apps/promptnotes/src-tauri/src/update_distribution/slices/install_update/tests.rs:176` `tp_u3_bundle_type_maps_to_installation_kind` / `apps/promptnotes/src-tauri/src/update_distribution/slices/install_update/tests.rs:149` `tp_u1_unbundled_is_rejected_without_touching_updater` |
