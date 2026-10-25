# Review: s23-update-same-version-once {#review-s23-update-same-version-once}

## Pass 1 {#pass-1}

> reviewer: ori-reviewer (fresh context)。E2E 実測 (main session): 3 passing。application.rs の I-U4 抑止を無効化する mutation で step 1 が RED になることを確認済み (参考)。

### Then ↔ assertion coverage {#pass-1-coverage}

Then 件数: spec.md#scenario-steps に 3 件 (main session 申告と一致)。

| Then (source#anchor) | 期待される assertion | テスト (file:line) | 状態 |
|---|---|---|---|
| spec.md#scenario-steps Then 0.2.3 通知済みのため NewVersionDetected 非発行・UI 通知なし (E2E: 9.9.8) | invoke が `latest_release.version == "9.9.8"` を返す (Some のまま) かつ pause(500ms) 後に Toast root が不在 | tests/s23-update-same-version-once.spec.ts:62-66 | VERIFIED |
| (同上・event 非発行そのもの) | bus.count() が 2 回目で増えない / 戻り値は Some のまま | apps/promptnotes/src-tauri/src/update_distribution/slices/check_for_updates/tests.rs:502 `tp_d1_same_version_is_notified_only_once` (E2E の Toast 不在を補強) | VERIFIED (E2E+unit 二重) |
| spec.md#scenario-steps Then 0.2.4 は未通知のため NewVersionDetected 発行・Toast 表示 (E2E: 9.9.9) | invoke が `9.9.9` を返し、Toast が出現し latest-version テキストが `9.9.9` | tests/s23-update-same-version-once.spec.ts:75-78 | VERIFIED |
| spec.md#scenario-steps Then 再起動すると通知済み記録が消え、起動時の確認で再び通知される | 新しい state では同一バージョンを再通知 (event count 1) | E2E 不能 (プロセス再起動)。tests.rs:562 `tp_d4_fresh_state_notifies_again` | N/A(代替担保) |

補足: Given の成立 (起動時確認で 9.9.8 の Toast 表示 → dismiss で消える) は spec.ts:44-54 で assert され、step 1 の「Toast 不在」が dismiss 後の状態であることを保証する。step 2 が陽性対照となり step 1 の不在が空振り (event 経路故障) でないことも示す。

### test-points 網羅 gate {#pass-1-test-points}

| TP | map 状態 | 実体の確認 | 判定 |
|---|---|---|---|
| TP1 起動時通知 + dismiss | COVERED (spec.ts:44) | `waitForExist` + latest-version == 9.9.8、dismiss 後 `isExisting()==false` を assert | OK |
| TP2 (核) 再通知しない | COVERED (spec.ts:56) | version==9.9.8 と Toast 不在を assert。500ms 待機あり | OK |
| TP3 より新しい版は通知 | COVERED (spec.ts:69) | version==9.9.9、Toast 出現、text==9.9.9 | OK |
| TP4 再起動後再通知 | N/A(代替担保) tests.rs:562 | `tp_d4_fresh_state_notifies_again` 実在、count==1 を assert | OK |
| TP5 24h 周期 | N/A(代替担保) tests.rs:681-756 | `tp_p1`〜`tp_p7` 実在 (tp_p1 681 / tp_p7 752 を確認) | OK |

spec の全 TP が map に存在し、行番号はテストコードと一致。UNCOVERED なし。

### Mode checklist (local tauri) {#pass-1-mode-checklist}

| item | status | note |
|---|---|---|
| spec `runner: wdio` ↔ test ↔ wdio.conf.ts | PASS | `browser.tauri.execute` / `$` (wdio globals)、conf は `@wdio/tauri-service`、`.ori/architecture.md` の `scenario_test_runner: wdio` と一致 |
| テストコード内に service lifecycle が無い | PASS | updater server / 隔離 / seed は `onPrepare`、`onComplete` で close + rmSync。spec は version ファイル書換 + invoke + assert のみ |
| docker-compose 不在 | PASS | compose-service 参加者ゼロ |
| `tauri:options.application` == runtime binary | PASS | `target/debug/app` (`.ori/architecture.md` runtime.binary と一致) |
| 実 user 環境の隔離 | PASS | XDG_* 4 種を temp へ、非 Linux は SevereServiceError で停止、残留 tauri-driver port 検査あり |

### Findings {#pass-1-findings}

- [1 spec 整合性] PASS: spec の Then 3 件すべてが VERIFIED または代替担保付き N/A。バージョン読み替え (9.9.8/9.9.9) は spec.md#impl-notes に明記。
- [5 edge case] LOW: spec.ts:65 の `pause(500)` は固定待機で、遅い環境では「event が遅れて到着し Toast が出る」ケースを取りこぼしうる (false GREEN 方向)。mutation で RED を実測済みであり、unit tp_d1 が event 非発行を直接担保しているため許容。推奨: 余裕があれば待機を延長または `waitUntil` による安定確認に置換。non-blocking。
- [3 DDD 規約] PASS: テスト内に domain 実装の複製や throw による Result 代用なし。
- [6 トレース] PASS: it 名に `validation#s23-given` / `validation#s23-then` を引用。`describe('scenario:s23-update-same-version-once')` が最外殻。
- [7 冗長性] 指摘なし。

## 総合判定

**PASS**

理由:
1. Then 3 件すべてが assertion に対応付いた (VERIFIED 2、N/A(代替担保) 1 で代替 tests.rs:562 は実在し内容も整合)。UNVERIFIED なし。
2. test-points TP1〜TP5 は map とコードが一致し、UNCOVERED なし。
3. mode checklist 全項目 PASS。指摘は LOW (固定待機) のみ。

verdict: PASS
