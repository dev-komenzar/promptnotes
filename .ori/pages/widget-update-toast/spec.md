---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  last_derived: 2026-11-02
  derives_from:
    - domain/ui-fields/screen-3.md#screen-3
    - domain/ui-fields/page-groups.md#widget-update-toast
    - domain/workflows/check-for-updates.md#check-for-updates
    - domain/workflows/install-update.md#install-update
  hash:
    domain/ui-fields/screen-3.md#.*: ddf71dd05ef4
    domain/ui-fields/page-groups.md#.*: f77c8a834634
    domain/workflows/check-for-updates.md#.*: 32f87895c0d8
    domain/workflows/install-update.md#.*: 4b291a6c25a9
---

# widget-update-toast — Widget Specification {#widget-update-toast-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive widget-update-toast`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

screen-3 を実体化する **conditional overlay widget**。
[page-main](../page-main/spec.md) の右下 overlay area に slide-in toast として表示し、
[check-for-updates](../../domain/workflows/check-for-updates.md#check-for-updates) workflow
が発行する `NewVersionDetected` event を購読する唯一の UI。

- **kind**: ui-widget
- **mount lifecycle**: page-main 起動時に subscribe 開始、`NewVersionDetected`
  受信時のみ visible。dismiss で hidden に戻る（view-release では残る）
- **parent**: page-main（subscribe 自体は page-main の lifecycle に従属、I-PM2）
- **subscribes**: `NewVersionDetected`（Tauri event channel `new_version_detected`）
- **silent on absence**: S14 — event 未発行時は完全に表示なし
- **install**: 対応配布形式では「Update & restart」で
  [install-update](../../domain/workflows/install-update.md#install-update) を呼び、成功後は page-main の
  hook（全 Note Flush → 再起動、I-U8）に委ねる。非対応形式では View release のみ（I-U6 / S26）

> domain/workflows/check-for-updates.md#check-for-updates より：「アプリ起動時、および以降 24 時間ごとに
> GitHub Releases へ HTTP リクエストを送り、同一プロセス内で未通知の新バージョンがあれば
> `NewVersionDetected` を発行する（I-U3 / I-U4）」
>
> domain/ui-fields/screen-3.md#cross-display-timing より：「アプリ起動時 1 回 + 以降 24 時間ごとに
> `check-for-updates` 実行（I-U3）」「同一バージョンの Toast は 1 プロセスにつき 1 回のみ（I-U4）」

event は起動時の確認と 24 時間ごとの定期確認のどちらからも届きうる。同一バージョンの再 emit は
発行側（check-for-updates の application service）が抑止するため、本 widget は届いた event を
そのまま表示するだけでよい（重複抑止の state を持たない）。

## 購読する event {#subscribed-event}

`NewVersionDetected` payload (`domain/domain-events.md#new-version-detected`):

| field | 型 | 用途 |
|--|--|--|
| `current_version` | `Version` | `widget.widget-update-toast.current-version` に表示 |
| `latest_version` | `Version` | `widget.widget-update-toast.latest-version` に強調表示 |
| `release_url` | `String` | `widget.widget-update-toast.view-release` クリックで OS ブラウザに渡す（GitHub Releases page） |
| `release_notes` | `String` | `widget.widget-update-toast.release-notes-summary` を 2 行 / 約 140 文字で truncate |

Tauri event channel 名は `new_version_detected`（snake_case）。emit 側は
`check_for_updates::infrastructure::TauriEventBus`（Rust、ori-2lm.8 で配線済み）。

## レイアウト {#layout}

```
                                      ┌── Toast (右下, slide-in) ───────┐
                                      │ 新しいバージョン 1.4.0 が利用可能 │
                                      │ (現在: 1.3.2)                  │
                                      │ - bug fix: 起動時 crash の修正  │
                                      │ - feature: search に正規化追加  │
                                      │  [詳細を見る]      [×]          │
                                      └────────────────────────────────┘
```

- 画面右下に固定表示（screen-3 cross-position-duration）
- 削除トースト (screen-1-toast) は中央下のため衝突しない
- 自動消失なし — ユーザの dismiss アクションでのみ hidden
- 各要素の data-testid は `.ori/pages/widget-update-toast/testids.yaml` の契約値を literal で付与する（ui-field id を写さない。詳細は [実装ノート](#impl-testid)）

## 不変条件 {#invariants}

### Mount / lifecycle {#invariants-lifecycle}

- **I-UT1（listen は page-main lifecycle で 1 回）**: page-main mount 時に `listen` を 1 回張り、
  unmount 時に解除する。widget 自身は確認を起動しない（起動時の確認は page-main が listen 確立後に
  invoke し、24 時間ごとの定期確認は発行側が行う。I-U3）
- **I-UT2（silent on absence）**: `NewVersionDetected` 未発行時は DOM に何も出さない。
  `screen-3.md#cross-display-timing` S14 / `page-groups.md#widget-update-toast` failure mode を遵守
- **I-UT3（最新 event で上書き）**: 同一プロセス中に 2 件目の event が来たら最新で payload を
  上書きする。定期確認でより新しいバージョンが見つかった時に実際に起きる（I-U3 / I-U4）
- **I-UT8（listen 確立後の hook）**: listen 確立 **後** に 1 回だけ `onSubscribed` hook を呼ぶ。
  page-main はここで起動時の `check_for_updates` を invoke する（listen 前の emit 取りこぼし防止、
  ori-m2ra.1）。listen 失敗時は hook を呼ばない。hook の失敗は silent（S14）

### dismiss / view-release {#invariants-dismiss-view}

- **I-UT4（dismiss は hidden のみ）**: `widget.widget-update-toast.dismiss` クリックで toast hidden
  （`payload = null`）。同じバージョンは同一プロセス内では再 emit されない（I-U4）ので再表示されない。
  次回起動時、または定期確認でより新しいバージョンの event が届いた時に再表示する（cross-dismiss-view）
- **I-UT5（view-release は open + 表示維持）**: `widget.widget-update-toast.view-release` クリックで
  `release_url` を OS デフォルトブラウザで開き、toast はそのまま残す（cross-dismiss-view）

> domain/ui-fields/screen-3.md#cross-dismiss-view より：「`screen-3-dismiss`: Toast を閉じるだけ。
> 同じバージョンは同一プロセス内では再表示しない（I-U4）。次回アプリ起動時、または定期確認で
> さらに新しいバージョンが見つかった時に再表示」

### install-update {#invariants-install}

- **I-UT9（installable 判定）**: `start()` で listen 確立後、`installableFn`（default は
  `is_update_installable` invoke）で「Update & restart」を出すかを決める。判定失敗は `false`（通知のみ、I-U6 / S26）
- **I-UT10（install は 1 本だけ）**: `install()` は `installStatus = installing` の間は多重実行しない。
  installing 中は dismiss も無効。成功時は `onInstalled(installedVersion)` を await し、再起動まで installing のまま
  （再起動前の Flush は page-main の責務、I-U8 / S24）
- **I-UT11（失敗は表示して再実行可能）**: install 失敗は `installStatus = failed` にして失敗メッセージを
  `widget.widget-update-toast.install-status` に表示し、ボタンを再び押せるようにする。payload は維持。
  `onInstalled` は呼ばない。より新しい版の event が届いたら `idle` に戻す（I-U7 / S25）
- **I-UT12（別 UI のインストール中は進行中表示）**: `installFn` が `kind = InstallInProgress` で reject したら
  （設定モーダルからのインストールが実行中 / 再起動待ち、I-U10）`installStatus = already-installing` にして
  「Another update is in progress or waiting for a restart.」を表示する。失敗扱いにせずエラーログも出さない。dismiss は有効。
  より新しい版の event が届いたら `idle` に戻す（S29）
- **I-UT13（再起動失敗）**: install 成功後に `onInstalled` が reject したら（page-main の `restart_app` 失敗）
  `installStatus = restart-required` にして「The update was installed. Restart PromptNotes to finish updating.」を表示し、
  dismiss を有効に戻す。「Update & restart」は無効のままで `install()` は no-op（再インストールしない、I-U8 / I-U10 / S29）

> domain/ui-fields/screen-3.md#cross-install-update より：「成功: page-main が全 pending Note を Flush してから
> アプリを再起動する（I-U8、S24）」「失敗: … ボタンを再び押せるようにする。View release は使える（I-U7、S25）」

### 経路境界 {#invariants-boundary}

- **I-UT6（event listen 経由）**: Tauri event channel `new_version_detected` の購読は
  `@tauri-apps/api/event` の `listen` を経由する。widget から `invoke` で polling しない
- **I-UT7（URL 開きは inject 可能）**: `release_url` を開く実装は injectable に保つ。プロダクションでは
  `@tauri-apps/plugin-opener` の `openUrl` を使う（Tauri webview の `window.open` は OS ブラウザを開かない）。
  opener の失敗は silent で toast は維持する

## テスト観点 (vitest) {#test-points}

store（`createUpdateToastStore`）を vitest server project で、stub `listenFn` / `openUrlFn` /
`onSubscribed` を注入して検証する（listen / opener は adapter 境界）。

### tp-ut-no-mount-without-event: event なしでは非表示 {#tp-ut-no-mount-without-event}

store を `start()` した直後、`NewVersionDetected` を 1 件も emit せずに observe すると
`store.payload === null`。UI 側は `{#if store.payload}` で nothing を render（I-UT2 / S14）。

### tp-ut-event-mounts-toast: event 受信で payload set {#tp-ut-event-mounts-toast}

stub listen に NewVersionDetected payload を 1 件 emit させると、`store.payload`
が当該 payload 全 4 field を保持する。

### tp-ut-latest-overrides: 再 emit は最新で上書き {#tp-ut-latest-overrides}

2 件 emit すると `store.payload` は 2 件目（I-UT3）。

### tp-ut-dismiss-clears: dismiss で payload=null {#tp-ut-dismiss-clears}

`store.dismiss()` で `store.payload === null` に戻る。openUrlFn は呼ばれない（I-UT4）。

### tp-ut-redisplay-on-newer: dismiss 後に新しいバージョンの event で再表示 {#tp-ut-redisplay-on-newer}

`1.4.0` を受信 → dismiss → 定期確認相当で `1.4.1` を emit すると `store.payload.latest_version === '1.4.1'`
で再表示される（I-UT4 / I-UT3）。

### tp-ut-view-release-opens: view-release で URL open + payload 維持 {#tp-ut-view-release-opens}

`store.viewRelease()` で `openUrlFn(release_url)` が **1 回** 呼ばれ、`store.payload`
は **そのまま** 残る（I-UT5）。payload 未 set なら何もしない。

### tp-ut-stop-unsubscribes: stop で listen 解除 {#tp-ut-stop-unsubscribes}

`store.start()` 後の `store.stop()` で stub listenFn が返した unsubscribe function が
**1 回** 呼ばれる。`start()` を 2 回呼んでも listenFn は 1 回（I-UT1）。

### tp-ut-listen-failure-silent: listen 失敗時 silent {#tp-ut-listen-failure-silent}

stub listenFn が reject しても `store.start()` は throw せず、`store.payload === null`
のまま。`onSubscribed` も呼ばれない（I-UT2 / I-UT8 / S14）。

### tp-ut-subscribed-hook: listen 確立後に hook {#tp-ut-subscribed-hook}

`onSubscribed` は listen 確立後に 1 回だけ呼ばれ、その中で emit された event を取りこぼさない。
`onSubscribed` が reject しても `start()` は throw しない（I-UT8 / S14）。

### tp-ut-installable: installable 判定 {#tp-ut-installable}

`installableFn` が true / false を返すと `store.installable` がそれに従う。reject したら false（I-UT9 / S26）。

### tp-ut-install-success: install 成功で onInstalled {#tp-ut-install-success}

`store.install()` で `installFn` が 1 回呼ばれ、`onInstalled` がインストール版で 1 回呼ばれる。
`installStatus` は `installing` のまま（I-UT10 / S24）。

### tp-ut-install-once: installing 中は多重実行しない {#tp-ut-install-once}

installing 中の `install()` 再呼出と `dismiss()` は無視される（I-UT10）。

### tp-ut-install-failure: 失敗は failed で再実行可能 {#tp-ut-install-failure}

`installFn` が reject すると `installStatus = failed`、`onInstalled` は呼ばれず payload は維持。再度 `install()`
できる。失敗後に新しい版の event が届くと `idle` に戻る（I-UT11 / S25）。

### tp-ut-install-not-installable: 非対応形式では何もしない {#tp-ut-install-not-installable}

`installable = false` のとき `install()` は `installFn` を呼ばない（I-U6）。

### tp-ut-install-in-progress: 別 UI のインストール中 {#tp-ut-install-in-progress}

`installFn` が `{ kind: 'InstallInProgress' }` で reject → `installStatus = already-installing`、`onInstalled` は呼ばれず、
`console.error` も出ない。dismiss でき、新しい版の event で `idle` に戻る（I-UT12）。

### tp-ut-restart-failure: 再起動失敗 {#tp-ut-restart-failure}

install 成功後に `onInstalled` が reject → `installStatus = restart-required`。再度 `install()` しても `installFn` は
呼ばれない。dismiss は有効（I-UT13）。page-main の browser test でも `restart_app` 失敗時にボタンが disabled・
dismiss が enabled になることを確認する（S29）。

### tp-ut-page-main-restart-order: page-main の再起動順序 {#tp-ut-page-main-restart-order}

page-main の browser test で、ボタン押下 → `install_update` → `flushAll` → `restart_app` の順に呼ばれること（S24）、
失敗時は flush / restart を呼ばず失敗表示すること（S25）、`is_update_installable = false` ではボタンを出さないこと（S26）を
Tauri IPC 境界だけ fake にして検証する（`PageMain.update-check.svelte.test.ts`）。

## 実装ノート {#impl-notes}

### testid 契約 {#impl-testid}

- testid は `.ori/pages/widget-update-toast/testids.yaml` の契約値（derived + extra）を literal で付与する。`/ori-impl-green` 完了条件で `testids.js check widget-update-toast` が検査する
- root は `widget.widget-update-toast.root`

### Subscription store は createUpdateToastStore で testable に {#impl-store}

- `createUpdateToastStore(deps)` factory:
  - `deps.listenFn?`: `(handler) => Promise<unsubscribe>`。default は
    `@tauri-apps/api/event` の `listen('new_version_detected', ...)`
  - `deps.openUrlFn?`: `(url) => void | Promise<void>`。default は
    `@tauri-apps/plugin-opener` の `openUrl`（失敗は silent）
  - `deps.onSubscribed?`: listen 確立後に 1 回呼ぶ hook（I-UT8）
  - `deps.installableFn?` / `deps.installFn?`: default は install-update slice の
    `isUpdateInstallable` / `installUpdate`（UI 層から updater plugin を直接呼ばない）
  - `deps.onInstalled?`: install 成功後の hook。page-main が `flushAll('app_quit')` → `restartApp()` を渡す
- component (`*.svelte`) は store の薄い presentation layer
- 重複通知の抑止 state は持たない（発行側の I-U4 に任せる）

### ディレクトリ構成 {#impl-layout-dir}

```
apps/promptnotes/src/ui-widget/update-toast/
├── WidgetUpdateToast.svelte    # presentation (slide-in toast)
├── store.svelte.ts             # createUpdateToastStore factory
└── tests/
    └── store.test.ts           # subscribe / dismiss / view-release / hook の unit test (server)
```

### page-main からの mount {#impl-mount-from-page-main}

- `PageMain.svelte` で **常に** `<WidgetUpdateToast onSubscribed={...} onInstalled={...} />` を render
- 内部 store が `start()` で listen を張り、event 受信時のみ `{#if store.payload}` で
  toast を可視化（I-UT2 silent）
- `page-main` の unmount 時に store の `stop()` で unsubscribe

### 非責務 {#impl-non-responsibility}

- **install の実処理 / 再起動前の Flush**: install-update slice と page-main の責務。widget は状態表示と hook 呼出のみ
- **download 進捗（%）**: screen-3.md#cross-install-update より MVP 範囲外
- **このバージョンを skip（永続）**: cross-suppression より MVP 範囲外
- **定期確認の timer / 重複通知の抑止**: 発行側（check-for-updates slice）の責務
- **emit 側（Rust）**: 本 widget は listen のみ

## Open Questions {#open-questions}

- **OQ-UT1（opener plugin 導入）**: 解決済み — `@tauri-apps/plugin-opener` を導入（ori-m2ra.2）
- **OQ-UT2（event channel 命名）**: 解決済み — emit 側（`TauriEventBus`）と listen 側とも `new_version_detected`
