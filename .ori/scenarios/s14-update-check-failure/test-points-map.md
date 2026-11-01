<!-- @ori-generated scenario:s14-update-check-failure -->
# s14-update-check-failure — test-points map {#test-points-map}

| test point | 状態 | テストケース / 代替担保 |
|---|---|---|
| TP1（核）— 更新チェック失敗は silent | COVERED | `tests/s14-update-check-failure.spec.ts:69` `step 1` |
| TP2 — NewVersionDetected 非発行 / UI 通知なし | COVERED | `tests/s14-update-check-failure.spec.ts:78` `step 2` |
| TP3 — ユーザの作業を妨げない | COVERED | `tests/s14-update-check-failure.spec.ts:84` `step 3` |
| TP4（陽性対照 / 非空虚性）— 成功時は通知される | COVERED | `tests/s14-update-check-failure.spec.ts:90` `step 4` |
| TP5 — 即時リトライなし / 24 時間周期 | N/A(代替担保) | `apps/promptnotes/src-tauri/src/update_distribution/slices/check_for_updates/tests.rs:397` `tp_r2_failure_path_does_not_retry` / `apps/promptnotes/src-tauri/src/update_distribution/slices/check_for_updates/tests.rs:691` `tp_p2_due_boundary` / `apps/promptnotes/src-tauri/src/update_distribution/slices/check_for_updates/tests.rs:717` `tp_p5_begin_check_resets_baseline` / `apps/promptnotes/src-tauri/src/update_distribution/slices/check_for_updates/tests.rs:731` `tp_p6_begin_due_check_only_when_due` |
| TP6 — ログ出力のみ | N/A(代替担保) | `apps/promptnotes/src-tauri/src/update_distribution/slices/check_for_updates/tests.rs:805` `tp_s14_6_failure_is_logged_as_warning` / `apps/promptnotes/src-tauri/src/update_distribution/slices/check_for_updates/tests.rs:839` `tp_s14_7_success_paths_do_not_log_failure` |
| check-for-updates slice unit test との分担 | N/A(代替担保) | `apps/promptnotes/src-tauri/src/update_distribution/slices/check_for_updates/tests.rs` TP-S14-1..7 / TP-I3 / TP-T1（Rust 層の silent failure。本 scenario は Tauri 境界 + UI の縦断） |
