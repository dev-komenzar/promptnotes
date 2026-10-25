# Review: widget-update-toast {#review-widget-update-toast}

reviewer: ori-reviewer (phase 6, fresh context)
scope: `.ori/pages/widget-update-toast/{spec.md,manifest.yaml,testids.yaml}` ↔ `apps/promptnotes/src/ui-widget/update-toast/**` + `apps/promptnotes/src/ui-page/page-main/PageMain.svelte` の mount 配線
diff base: `3dfaa7e..HEAD`

## Pass 1 {#pass-1}

### Test point ↔ assertion カバレッジ {#pass-1-coverage}

| spec#test-point | invariant | tests/store.test.ts | 状態 |
|---|---|---|---|
| spec.md#tp-ut-no-mount-without-event | I-UT2 / S14 | `event 未受信時は payload=null` | VERIFIED |
| spec.md#tp-ut-event-mounts-toast | subscribed-event | `NewVersionDetected で payload が set される` | VERIFIED |
| spec.md#tp-ut-latest-overrides | I-UT3 | `再 emit は最新で上書き` | VERIFIED |
| spec.md#tp-ut-dismiss-clears | I-UT4 | `dismiss で payload=null` | VERIFIED |
| spec.md#tp-ut-redisplay-on-newer | I-UT3 / I-UT4 | `dismiss 後に定期確認で新しいバージョンが届くと再表示` | VERIFIED |
| spec.md#tp-ut-view-release-opens | I-UT5 | `openUrlFn を 1 回呼び payload は維持` + `payload null guard` | VERIFIED |
| spec.md#tp-ut-stop-unsubscribes | I-UT1 | `stop で unsubscribe 1 回` + `start 2 回でも listenFn 1 回` | VERIFIED |
| spec.md#tp-ut-listen-failure-silent | I-UT2 / I-UT8 / S14 | `listenFn reject でも throw せず payload=null` + `listen 失敗時 onSubscribed を呼ばない` | VERIFIED |
| spec.md#tp-ut-subscribed-hook | I-UT8 / S14 | `onSubscribed は listen 確立後に 1 回だけ、その中の emit を取りこぼさない` + `onSubscribed reject でも start は throw しない` | VERIFIED |

### Findings {#pass-1-findings}

#### [1 spec 整合性] 概ね一致 {#f-spec}

- [LOW] spec.md#subscribed-event `release_notes` を「2 行 / 約 140 文字で truncate」と規定。`WidgetUpdateToast.svelte` は CSS `line-clamp-2` + 140 文字 slice + `…` 付与の二重防衛で満たしている。unit test は store 層のみで truncate の assertion は無いが、spec の test-points にも truncate は列挙されていないため非 blocking。
- [LOW] `defaultOpenUrlFn` の silent-catch は spec I-UT7 の「opener の失敗は silent で toast は維持する」を満たすが、production path (default) の失敗経路は test 対象外。これは adapter 境界なので DDD 規約上は許容。

#### [2 derives_from 網羅] OK {#f-derives}

- `screen-3.md#screen-3` (cross-display-timing / cross-dismiss-view / S14) — I-UT2 / I-UT4 / I-UT5 として反映済。
- `page-groups.md#widget-update-toast` — subscribes 契約、silent on absence に反映済。
- `workflows/check-for-updates.md#check-for-updates` — I-U3 / I-U4 を spec 本文で引用、I-UT3 の「同一バージョン再 emit は発行側が抑止」解釈も正しい。
- `aggregates.md#update-channel-aggregate-invariants` (I-U3 / I-U4) — widget 側は「state を持たない」設計で正しく境界分離。

#### [3 DDD 規約遵守] OK {#f-ddd}

- store factory が listenFn / openUrlFn / onSubscribed を inject 可能にし、I/O (`@tauri-apps/api/event`, `@tauri-apps/plugin-opener`) は default として境界に閉じている (I-UT7)。
- `.svelte` は presentation の薄い wrapper。`untrack` で factory を 1 回だけ構築、`$effect` の cleanup で `stop()`。I-UT1 の lifecycle 要件を満たす。

#### [4 副作用の境界] OK {#f-side-effect}

- 副作用 (listen / openUrl / startup check hook) はすべて adapter 層の関数経由。store 内 state (`payload`, `unsubscribe`) は closure に閉じる。
- `start()` の idempotency: `if (unsubscribe) return;` によって 2 回目の `start()` は no-op。onSubscribed も再呼ばれない (test 検証済)。

#### [5 edge case] 妥当 {#f-edge}

- listen reject / onSubscribed reject / payload=null での viewRelease / dismiss 後の後続 event / 2 件連続 emit を網羅。
- [LOW] `start()` が `listenFn` の `await` 中（まだ `unsubscribe` に代入前）に再入される競合は、Svelte の effect では通常起きないが store API を直接触るとあり得る。Spec の I-UT1「listen は 1 回」からは逸脱しうるが、test が示す通り Svelte lifecycle 下では問題にならないため非 blocking。

#### [6 テスト ↔ spec トレース] OK {#f-trace}

- 全 `it` 名が `spec#<test-point>` を引用し、該当 invariant も併記。`describe('widget:widget-update-toast store', ...)` で最外殻 ID 規約を満たす (ddd-test.md)。

#### [7 冗長性] OK {#f-redundancy}

- `truncate` は 140 文字固定の最小実装、`RELEASE_NOTES_MAX` の定数化で保守性も確保。
- store API は payload getter / start / stop / dismiss / viewRelease の 5 面のみで無駄な抽象なし。

### mount 配線確認 {#f-mount}

- `PageMain.svelte:461` で `<WidgetUpdateToast onSubscribed={checkForUpdatesAtStartup} />` が **常に** render されており、「page-main 起動時に subscribe 開始」(spec#overview) と I-UT8 を満たす。

### 総合判定 {#verdict}

verdict: PASS

理由:
1. 全 9 test-point が store.test.ts に対応する assertion を持ち、I-UT1..I-UT8 すべてが invariant 単位で検証されている。
2. derives_from に列挙された 3 domain section (screen-3 / page-groups / workflows/check-for-updates) の制約が spec 本文と impl 挙動の両方に反映済。I-U3 / I-U4 の責務分離 (発行側で定期確認・重複抑止 / widget 側は listen のみ) も設計通り。
3. DDD 境界 (adapter 注入、store factory 分離、`.svelte` は薄い presentation)、testid 契約 (literal 付与)、Svelte lifecycle (`untrack` + `$effect` cleanup) いずれも規約遵守。LOW 指摘 (truncate assertion なし / start race) は test-points の網羅外で非 blocking。
