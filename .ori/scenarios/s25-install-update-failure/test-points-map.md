<!-- @ori-generated scenario:s25-install-update-failure -->
# s25-install-update-failure — test-points map {#test-points-map}

| test point | 状態 | テストケース / 代替担保 |
|---|---|---|
| TP1 — Given の成立 | COVERED | `tests/s25-install-update-failure.spec.ts:116` `given — validation#s25-given` |
| TP2（核）— download 失敗を伝えて継続 | COVERED | `tests/s25-install-update-failure.spec.ts:132` `step 1 — validation#s25-then`（失敗表示 / 再押下可 / 同一 process の継続 (sentinel) / kind=DownloadFailed） |
| TP3 — install 失敗（署名検証失敗）を伝えて継続 | COVERED | `tests/s25-install-update-failure.spec.ts:150` `step 2 — validation#s25-then`（再押下 → 失敗表示 / 未インストール / kind=InstallFailed） |
| TP4 — View release は引き続き使える | COVERED | `tests/s25-install-update-failure.spec.ts:132` / `:150`（失敗表示中の view-release が表示・押下可） |
| TP5 — 自動リトライしない | COVERED | `tests/s25-install-update-failure.spec.ts:132` / `:150`（3 秒待っても request が増えない） |
| TP6 — 失敗時は Note の Flush も行わない | N/A(代替担保) | `apps/promptnotes/src/ui-page/page-main/tests/PageMain.update-check.svelte.test.ts:200` `S25 — install_update が失敗したら flush も restart もせず失敗を表示する` / `apps/promptnotes/src/ui-widget/update-toast/tests/store.test.ts:286` `spec#tp-ut-install-failure (I-UT11 / S25)` |
| TP7 — deb の権限昇格（pkexec）をユーザが拒否した場合の `InstallFailed` | N/A(代替担保) | `apps/promptnotes/src-tauri/src/update_distribution/slices/install_update/tests.rs:273` `tp_f3_download_and_install_failures_are_propagated` / `apps/promptnotes/src-tauri/src/update_distribution/slices/install_update/tests.rs:319` `tp_f4_signature_errors_map_to_install_failed_others_to_download_failed` |
