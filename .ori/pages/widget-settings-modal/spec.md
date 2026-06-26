---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  last_derived: 2026-06-26
  derives_from:
    - domain/ui-fields/screen-2.md#screen-2
    - domain/ui-fields/page-groups.md#widget-settings-modal
    - domain/workflows/update-settings.md#update-settings
---

# widget-settings-modal — Widget Specification {#widget-settings-modal-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive widget-settings-modal`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

screen-2 を実体化する **on-demand overlay widget**。
[page-main](../page-main/spec.md) の中央に modal として表示し、`update-settings`
workflow の **唯一の trigger UI** となる。

- **kind**: ui-widget
- **mount lifecycle**: on-demand（page-main の toolbar 歯車 / `Cmd+,` で mount、
  save / cancel / Esc で unmount）
- **parent**: page-main（lifecycle は page-main に従属、I-PM2）
- **hosts**: `update-settings`（唯一）

## ホストする slices {#hosted-slices}

| field | slice | trigger | 副作用 |
|--|--|--|--|
| `screen-2-save` | [update-settings](../../slices/update-settings/spec.md) | primary button | `storage_dir` / `theme` 差分を永続化 + 0〜2 件 event |

`screen-2-storage-dir` の folder picker は OS ネイティブ dialog（`@tauri-apps/plugin-dialog`）
を起動するが、これは widget の form 状態を組み立てるための副作用であり slice ではない。
slice 呼出は **save ボタン押下時の 1 度のみ**（C-US1 / C-US5）。

## レイアウト {#layout}

```
┌── Modal (中央, OS ネイティブ風) ────────────────────────────┐
│  設定                                                     │
│                                                            │
│  保存ディレクトリ                                            │
│   ┌──────────────────────────────────────────────┐ [選択…] │
│   │ /Users/takuya/Documents/PromptNotes (read)   │         │
│   └──────────────────────────────────────────────┘         │
│   [デフォルトに戻す]                                          │
│   ⚠ inline error: 「絶対パスを指定してください」              │
│                                                            │
│  テーマ                                                     │
│   ( ) System   ( ) Light   ( ) Dark                        │
│                                                            │
│              [キャンセル]  [保存]                            │
└────────────────────────────────────────────────────────────┘
```

- HTML `<dialog>` element + `showModal()` で OS ネイティブ風の modal を実現
- `Cmd+,` / Toolbar 歯車で open、Esc / cancel / save で close（cross-screen-shortcuts と整合）
- 背景の dim 等の装飾はしない（screen-2.md#notes-os-native-modal）

## 不変条件 {#invariants}

### Modal lifecycle {#invariants-lifecycle}

- **I-SM1（mount 排他なし）**: modal open 中も page-main の Toolbar / Feed は維持される（I-PM8）
- **I-SM2（Esc / cancel は破棄）**: Esc キー押下と cancel ボタン押下は同一動作。
  modal を unmount し、編集中の form state を **破棄**（保存しない）
- **I-SM3（save は workflow を呼ぶ）**: save ボタン押下で `update-settings` workflow を呼ぶ。
  成功で modal を unmount、失敗（InvalidPath）で inline error 表示し open 維持

### Form state {#invariants-form-state}

- **I-SM4（draft state は modal scope）**: 編集中の `storage_dir` / `theme` は modal 内部の
  draft state として保持。cancel / Esc で破棄、save で workflow に渡す
- **I-SM5（theme preview）**: `screen-2-theme` の選択肢クリックで **即時 preview**
  （document.documentElement の class 切替）。cancel / Esc 時は **mount 時の値に rollback**
  （screen-2.md#cross-theme-immediate）
- **I-SM6（差分なし save は close のみ）**: form の値が mount 時 settings と完全一致なら
  workflow は no-op（C-US1）。modal は閉じる、成功通知 / 警告は出さない
  （screen-2.md#cross-no-diff）

### 経路境界 {#invariants-boundary}

- **I-SM7（slice 経由）**: `update-settings` slice の TS bindings (`$lib/user-preferences/slices/update-settings`)
  を経由してのみ Tauri command を呼ぶ。`@tauri-apps/api/core` の直接 import は禁止
  （architecture.md `forbidden_imports` for `ui-widget`）
- **I-SM8（folder picker は plugin-dialog）**: `@tauri-apps/plugin-dialog` の `open()` は
  raw `invoke` ではないため許容。ただし folder picker から得た path は **save 時に
  slice 経由で再検証**（slice 側で I-S1 / I-S2 を再評価）

## テスト観点 (vitest) {#test-points}

### tp-sm-mount-defaults: mount 時に初期値が反映 {#tp-sm-mount-defaults}

modal mount 時、`storage_dir` と `theme` が引数で渡された initial settings から
draft state に複製される。両 input にその値が表示される。

### tp-sm-cancel-discards: cancel で draft 破棄 {#tp-sm-cancel-discards}

`storage_dir` / `theme` を編集 → cancel ボタン押下 → `onClose` callback が呼ばれ、
`update-settings` slice は **呼ばれない**（I-SM2）。

### tp-sm-esc-discards: Esc キーで draft 破棄 {#tp-sm-esc-discards}

`storage_dir` / `theme` を編集 → Esc キー → `onClose` callback が呼ばれ、
`update-settings` slice は **呼ばれない**（I-SM2）。
theme preview があれば mount 時の値に rollback される（I-SM5）。

### tp-sm-save-invokes-workflow: save で update-settings 呼出 {#tp-sm-save-invokes-workflow}

`theme` を `Dark` に変更 → save ボタン → `updateSettings({ theme: 'Dark' })` が
**1 回** 呼ばれる。`storage_dir` は変更してないので payload に含めない（diff 最小化）。
成功 resolve で `onClose` が呼ばれる（I-SM3）。

### tp-sm-save-no-diff: 差分なし save は close のみ {#tp-sm-save-no-diff}

mount 時から何も変更せず save → `updateSettings` は呼ばれない（C-US1 を modal 側で最適化）
or `updateSettings({})` で呼ばれて backend が no-op。modal は close される（I-SM6）。

### tp-sm-save-error-keeps-open: save 失敗で modal open 維持 {#tp-sm-save-error-keeps-open}

`updateSettings` が `InvalidPath` で reject → modal は **閉じない** + inline error
表示（`screen-2-storage-dir` 直下）。`onClose` は呼ばれない（I-SM3）。

### tp-sm-storage-dir-validation: 絶対パス検証エラーの表示 {#tp-sm-storage-dir-validation}

`updateSettings` reject で `kind === 'invalid_path'` の場合、
「絶対パスを指定してください」メッセージを `screen-2-storage-dir` の下に表示する
（screen-2.md#cross-storage-dir-validation）。

### tp-sm-no-raw-invoke: 生 invoke 禁止 {#tp-sm-no-raw-invoke}

eslint static check で `apps/promptnotes/src/ui-widget/settings-modal/` から
`@tauri-apps/api/core` の import が 0 件（I-SM7）。
`@tauri-apps/plugin-dialog` は許容（I-SM8）。

## 実装ノート {#impl-notes}

### Svelte 5 + `<dialog>` element {#impl-svelte-dialog}

- ルートは HTML `<dialog>` element + `showModal()` で modal をネイティブに表現
- Esc キーは `<dialog>` の `cancel` event で拾い、preventDefault しない（OS 既定の挙動）
- mount は parent (page-main) の `$state` flag で制御。flag が true の時のみ
  `WidgetSettingsModal` を `{#if}` で render

### Form state は createSettingsModalStore で testable に {#impl-store}

- `createSettingsModalStore(initialSettings, deps)` factory を切り、
  draft state / save / cancel logic を **vitest server (node)** で単体テスト可能にする
- deps: `updateSettingsFn` (slice の `updateSettings` 関数) を注入
- component (`*.svelte`) は store の薄い presentation layer

### ディレクトリ構成 {#impl-layout-dir}

```
apps/promptnotes/src/ui-widget/settings-modal/
├── WidgetSettingsModal.svelte    # presentation (dialog + form)
├── store.svelte.ts               # createSettingsModalStore factory
└── tests/
    └── store.test.ts             # form / save / cancel の unit test (server)
```

### 非責務 {#impl-non-responsibility}

- **sort_preference の編集**: screen-2.md#notes-no-sort-here より対象外（toolbar の sort で扱う）
- **マイグレーション ウィザード**: storage_dir 変更時の Note 移動は実装しない（I-S4 / MVP 範囲外）
- **再起動モーダル**: `storage_dir` 変更後の「再起動してください」モーダルは本 widget の
  scope 外。`StorageDirChanged` event の購読側で別途実装（cross-storage-dir-restart は将来課題）

## Open Questions {#open-questions}

- **OQ-SM1（folder picker plugin-dialog 制限）**: nix devshell で
  `@tauri-apps/plugin-dialog` の `open({ directory: true })` が動作するかは要検証
  （GTK FileChooser schema 問題が解決済みであれば動くはず）。
  fallback として「path を text input で直接編集」を MVP に残すかは Phase 0 判断
- **OQ-SM2（再起動モーダル）**: `storage_dir` 変更後の再起動モーダルは widget 外に切り出す
  予定だが、cross-storage-dir-restart の実装場所（page-main effect / 別 widget）は
  後続 PR で決める
