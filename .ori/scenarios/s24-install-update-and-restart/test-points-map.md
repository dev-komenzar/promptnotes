<!-- @ori-generated scenario:s24-install-update-and-restart -->
# s24-install-update-and-restart — test-points map {#test-points-map}

| test point | 状態 | テストケース / 代替担保 |
|---|---|---|
| TP1 — Given の成立（インストール可能な配布形式で通知される） | COVERED | `tests/s24-install-update-and-restart.spec.ts:104` `given — validation#s24-given` |
| TP2 — 処理中表示 | COVERED | `tests/s24-install-update-and-restart.spec.ts:121` `step 1 — validation#s24-then` |
| TP3 — 最新リリースを取得し直す | COVERED | `tests/s24-install-update-and-restart.spec.ts:139` `step 2 — validation#s24-then`（latest.json の request 増加） |
| TP4 — download + 署名検証 + install | COVERED | `tests/s24-install-update-and-restart.spec.ts:139` `step 2 — validation#s24-then`（download 1 回 + AppImage がパッケージと一致） |
| TP5（核）— Flush してから再起動 | COVERED | `tests/s24-install-update-and-restart.spec.ts:139` `step 2 — validation#s24-then`（再起動時 snapshot に install 中の未保存編集を含み、Note A の永続化は最後の打鍵の debounce 成立前） |
| TP6 — インストールしたバージョンで再起動 | COVERED | `tests/s24-install-update-and-restart.spec.ts:139` `step 2 — validation#s24-then`（marker = 9.9.8） |
| TP7 — `0.2.5 > 0.2.4` の比較（I-U7）と `0.2.5` を返すこと | N/A(代替担保) | `apps/promptnotes/src-tauri/src/update_distribution/slices/install_update/tests.rs:133` `tp_s1_installs_newer_release_and_returns_its_version` / `apps/promptnotes/src-tauri/src/update_distribution/slices/install_update/tests.rs:220` `tp_n1_same_or_older_release_is_not_installed` |
| TP8 — Flush → 再起動の呼出順 | N/A(代替担保) | `apps/promptnotes/src/ui-page/page-main/tests/PageMain.update-check.svelte.test.ts:162` `S24 (I-U8) — 全 Note の flush が完了するまで restart_app を呼ばない`（flushAll を保留し、完了まで restart_app を呼ばないことを検証） / `apps/promptnotes/src/ui-page/page-main/tests/PageMain.update-check.svelte.test.ts:140` `S24 — Update & restart で install_update → 全 Note flush → restart_app の順に呼ぶ` |
