<!-- @ori-generated scenario:s23-update-same-version-once -->
# s23-update-same-version-once — test-points map {#test-points-map}

| test point | 状態 | テストケース / 代替担保 |
|---|---|---|
| TP1 — 起動時の確認で通知される（Given の成立） | COVERED | `tests/s23-update-same-version-once.spec.ts:44` `given — validation#s23-given` |
| TP2（核）— 通知済みバージョンは再通知しない | COVERED | `tests/s23-update-same-version-once.spec.ts:56` `step 1 — validation#s23-then` |
| TP3 — より新しいバージョンは通知する | COVERED | `tests/s23-update-same-version-once.spec.ts:69` `step 2 — validation#s23-then` |
| TP4 — 再起動後は再び通知される（I-U4 揮発） | N/A(代替担保) | `apps/promptnotes/src-tauri/src/update_distribution/slices/check_for_updates/tests.rs:562` `tp_d4_fresh_state_notifies_again` |
| TP5 — 24 時間周期で確認が走る（I-U3） | N/A(代替担保) | `apps/promptnotes/src-tauri/src/update_distribution/slices/check_for_updates/tests.rs:681-756` `tp_p1_interval_and_tick_constants` 〜 `tp_p7_due_iff_elapsed_at_least_interval` |
