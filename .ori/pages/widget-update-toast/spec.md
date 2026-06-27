---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  last_derived: 2026-06-26
  derives_from:
    - domain/ui-fields/screen-3.md#screen-3
    - domain/ui-fields/page-groups.md#widget-update-toast
    - domain/workflows/check-for-updates.md#check-for-updates
---

# widget-update-toast — Widget Specification {#widget-update-toast-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive widget-update-toast`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

screen-3 を実体化する **startup conditional overlay widget**。
[page-main](../page-main/spec.md) の右下 overlay area に slide-in toast として表示し、
[check-for-updates](../../domain/workflows/check-for-updates.md#check-for-updates) workflow
が発行する `NewVersionDetected` event を購読する唯一の UI。

- **kind**: ui-widget
- **mount lifecycle**: page-main 起動時に subscribe 開始、`NewVersionDetected`
  受信時のみ visible。dismiss / view-release クリックで hidden に戻る
- **parent**: page-main（subscribe 自体は page-main の lifecycle に従属、I-PM2）
- **subscribes**: `NewVersionDetected` (in-process or Tauri event channel)
- **silent on absence**: S14 — event 未発行時は完全に表示なし

## 購読する event {#subscribed-event}

`NewVersionDetected` payload (`domain/domain-events.md#new-version-detected`):

| field | 型 | 用途 |
|--|--|--|
| `current_version` | `Version` | `screen-3-current-version` に表示 |
| `latest_version` | `Version` | `screen-3-latest-version` に強調表示 |
| `release_url` | `String` | `screen-3-view-release` クリックで OS ブラウザに渡す |
| `release_notes` | `String` | `screen-3-release-notes-summary` を 2 行 / 約 140 文字で truncate |

Tauri event channel 名は `new_version_detected`（snake_case、Rust serde default 規約）。
実 emit 側 (`apps/promptnotes/src-tauri/src/lib.rs` の startup hook) は
[ori-6l4](https://github.com/dev-komenzar/promptnotes/issues) の updater plugin
有効化後に follow-up で繋ぐ。本 widget は listen 基盤のみ完成させ、stub event を
注入したテストで動作確認する。

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
- 自動消失なし — ユーザの dismiss / view-release アクションでのみ hidden

## 不変条件 {#invariants}

### Mount / lifecycle {#invariants-lifecycle}

- **I-UT1（startup subscribe のみ）**: page-main mount 時に `listen` を 1 回張り、
  unmount 時に解除する。常駐 polling は行わない（workflow I-U3 と整合）
- **I-UT2（silent on absence）**: `NewVersionDetected` 未発行時は DOM に何も出さない。
  `screen-3.md#cross-display-timing` S14 / `page-groups.md#widget-update-toast` failure mode を遵守
- **I-UT3（最新 event で上書き）**: 万一同一セッション中に 2 件 event が来た場合は最新で payload を
  上書きする（MVP では理論上発生しないが、再起動なしで dev サーバから複数 event を撃つテスト等で
  決定的挙動を持たせる）

### dismiss / view-release {#invariants-dismiss-view}

- **I-UT4（dismiss は hidden のみ）**: `screen-3-dismiss` クリックで toast hidden
  （`payload = null`）。次回起動の `check-for-updates` で再 emit されれば再表示
  （cross-dismiss-view）
- **I-UT5（view-release は open + 表示維持）**: `screen-3-view-release` クリックで
  `release_url` を OS デフォルトブラウザで開き、toast はそのまま残す（cross-dismiss-view）

### 経路境界 {#invariants-boundary}

- **I-UT6（event listen 経由）**: Tauri event channel `new_version_detected` の購読は
  `@tauri-apps/api/event` の `listen` を経由する。raw `invoke` での polling は禁止
- **I-UT7（URL 開きは inject 可能）**: `release_url` を開く実装は injectable に保ち、
  プロダクションでは Tauri の opener / `window.open` を使う。実 wiring は ori-6l4 follow-up

## テスト観点 (vitest) {#test-points}

### tp-ut-no-mount-without-event: event なしでは非表示 {#tp-ut-no-mount-without-event}

store を `start()` した直後、`NewVersionDetected` を 1 件も emit せずに observe すると
`store.payload === null`。UI 側は `{#if store.payload}` で nothing を render（I-UT2 / S14）。

### tp-ut-event-mounts-toast: event 受信で payload set {#tp-ut-event-mounts-toast}

stub listen に NewVersionDetected payload を 1 件 emit させると、`store.payload`
が当該 payload 全 4 field を保持する（I-UT3）。

### tp-ut-dismiss-clears: dismiss で payload=null {#tp-ut-dismiss-clears}

`store.dismiss()` で `store.payload === null` に戻る。openUrlFn は呼ばれない（I-UT4）。

### tp-ut-view-release-opens: view-release で URL open + payload 維持 {#tp-ut-view-release-opens}

`store.viewRelease()` で `openUrlFn(release_url)` が **1 回** 呼ばれ、`store.payload`
は **そのまま** 残る（I-UT5）。

### tp-ut-stop-unsubscribes: stop で listen 解除 {#tp-ut-stop-unsubscribes}

`store.start()` 後の `store.stop()` で stub listenFn が返した unsubscribe function が
**1 回** 呼ばれる（I-UT1）。

### tp-ut-listen-failure-silent: listen 失敗時 silent {#tp-ut-listen-failure-silent}

stub listenFn が reject しても `store.start()` は throw せず、`store.payload === null`
のまま（I-UT2 / S14: silent failure）。

## 実装ノート {#impl-notes}

### Subscription store は createUpdateToastStore で testable に {#impl-store}

- `createUpdateToastStore(deps)` factory:
  - `deps.listenFn?`: `(handler) => Promise<unsubscribe>`。default は
    `@tauri-apps/api/event` の `listen('new_version_detected', ...)`
  - `deps.openUrlFn?`: `(url) => void | Promise<void>`。default は
    `window.open(url, '_blank', 'noopener')`（Tauri v2 opener plugin 導入後に差し替え）
- component (`*.svelte`) は store の薄い presentation layer
- vitest **server (node)** で stub listenFn + stub openUrlFn を注入して I-UT1〜I-UT5 を検証

### ディレクトリ構成 {#impl-layout-dir}

```
apps/promptnotes/src/ui-widget/update-toast/
├── WidgetUpdateToast.svelte    # presentation (slide-in toast)
├── store.svelte.ts             # createUpdateToastStore factory
└── tests/
    └── store.test.ts           # subscribe / dismiss / view-release の unit test (server)
```

### page-main からの mount {#impl-mount-from-page-main}

- `PageMain.svelte` の末尾で **常に** `<WidgetUpdateToast />` を render
- 内部 store が `start()` で listen を張り、event 受信時のみ `{#if store.payload}` で
  toast を可視化（I-UT2 silent）
- `page-main` の unmount 時に store の `stop()` で unsubscribe

### 非責務 {#impl-non-responsibility}

- **自動 install / 再起動**: screen-3.md#notes-tauri-separation より対象外
- **このバージョンを skip**: cross-suppression より MVP 範囲外
- **emit 側（Rust）**: 本 widget は listen のみ。`NewVersionDetected` を Tauri event
  channel に bridge する Rust 側の `app.emit(...)` 呼出は ori-6l4 unblock 後の
  follow-up（ori-2lm.8）で対応

## Open Questions {#open-questions}

- **OQ-UT1（opener plugin 導入）**: `release_url` を OS ブラウザで開くために
  `@tauri-apps/plugin-opener` を追加するかは ori-6l4 と合わせて判断。
  暫定は `window.open(url, '_blank', 'noopener')` で十分（Tauri webview は
  external URL を OS デフォルトブラウザに委譲する設定が default のため）
- **OQ-UT2（event channel 命名）**: `new_version_detected` か
  `update_distribution/new_version_detected` のどちらに揃えるかは emit 側 follow-up で決定。
  本 widget は `new_version_detected` を仮採用、不整合があれば listen 側を寄せる
