---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  last_derived: 2026-10-15
  derives_from:
    - domain/ui-fields/screen-2.md#screen-2
    - domain/ui-fields/page-groups.md#widget-settings-modal
    - domain/workflows/update-settings.md#update-settings
    - domain/workflows/get-app-version.md#get-app-version
    - domain/workflows/get-default-storage-dir.md#get-default-storage-dir
  hash:
    domain/ui-fields/screen-2.md#.*: 0a4971bac9fa
    domain/ui-fields/page-groups.md#.*: a9fb317174a9
    domain/workflows/update-settings.md#.*: f420da94bd93
    domain/workflows/get-app-version.md#.*: c3a5c313f0f5
    domain/workflows/get-default-storage-dir.md#.*: 84a2bc66ea78
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
- **hosts**: `update-settings`（唯一の command）/ `get-app-version`（表示用 query）/
  `get-default-storage-dir`（リセット用の既定値 query）

## ホストする slices {#hosted-slices}

| field | slice | trigger | 副作用 |
|--|--|--|--|
| `screen-2-save` | [update-settings](../../slices/update-settings/spec.md) | primary button | `storage_dir` / `theme` 差分を永続化 + 0〜2 件 event |
| `screen-2-app-version` | [get-app-version](../../slices/get-app-version/spec.md) | modal mount 時に 1 回 | なし（読み取り専用 query） |
| `screen-2-storage-dir-reset` | [get-default-storage-dir](../../slices/get-default-storage-dir/spec.md) | modal mount 時に 1 回 | なし（読み取り専用 query） |

`screen-2-storage-dir` の folder picker は OS ネイティブ dialog（`@tauri-apps/plugin-dialog`）
を起動するが、これは widget の form 状態を組み立てるための副作用であり slice ではない。
`update-settings` の呼出は **save ボタン押下時の 1 度のみ**（C-US1 / C-US5）。

> domain/ui-fields/screen-2.md#cross-app-version-display より:
> モーダル mount 時に `get-app-version` を 1 回呼ぶ。取得完了前、または呼び出し自体が失敗した場合は
> **行ごと非表示**（エラー表示はしない）。表示値は Save / Cancel / theme プレビューの影響を受けない。
> 差分判定（`cross-no-diff`）の対象外。

> domain/ui-fields/screen-2.md#cross-storage-dir-reset より:
> - モーダル mount 時に `get-default-storage-dir` を 1 回呼び、OS 慣習パス（I-S3）を得る
> - 「デフォルトに戻す」押下で、**編集中の** `screen-2-storage-dir` の値をその path に置き換える
> - 永続化と `StorageDirChanged` の発行は「保存」押下時に `update-settings` が行う。
>   リセットボタン単独では保存しない。キャンセル / Esc で破棄される
> - mount 時の値が default と同じなら、リセットしても差分なし（`cross-no-diff` に従う）
> - default の取得完了前、または取得自体が失敗した場合は、ボタンを **disabled** にする

## 入出力 {#io}

- **入力 (props)**: `initial: SettingsDto`（mount 時の settings）、`onClose` / `onSaved` callback、
  test 用の注入点 `updateSettingsFn` / `getAppVersionFn` / `getDefaultStorageDirFn` / `openDialogFn`
- **出力 (slice 呼出)**:
  - `updateSettings(input: UpdateSettingsInput)` — save 時、差分のみ
  - `getAppVersion(): Promise<string>` — mount 時に 1 回
  - `getDefaultStorageDir(): Promise<string>` — mount 時に 1 回
- **表示 (read-only)**: `screen-2-app-version` に `v{version}`（例: `v0.2.2`）

## 境界契約 {#boundary-contract}

- boundary kind: `tauri_command`（slice の TS wrapper 経由）
- contact point:
  - `apps/promptnotes/src/lib/user-preferences/slices/update-settings/index.ts`（`updateSettings`）
  - `apps/promptnotes/src/lib/update-distribution/slices/get-app-version/index.ts`（`getAppVersion`）
  - `apps/promptnotes/src/lib/user-preferences/slices/get-default-storage-dir/index.ts`（`getDefaultStorageDir`）
- public_entry: `apps/promptnotes/src/ui-widget/settings-modal/WidgetSettingsModal.svelte`（page-main から mount）
- 禁止: `ui-widget/settings-modal/` から `@tauri-apps/api/core` の `invoke` / `@tauri-apps/api/app` の
  `getVersion` を直接 import すること（I-SM7 / I-SM9。get-app-version spec#boundary-contract の C-GAV5）。
  `@tauri-apps/api/path` で default path を独自に計算すること（I-SM12。get-default-storage-dir の C-GDSD3）

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
│  v0.2.2                     [キャンセル]  [保存]            │
└────────────────────────────────────────────────────────────┘
```

- HTML `<dialog>` element + `showModal()` で OS ネイティブ風の modal を実現
- `Cmd+,` / Toolbar 歯車で open、Esc / cancel / save で close（cross-screen-shortcuts と整合）
- 背景の dim 等の装飾はしない（screen-2.md#notes-os-native-modal）
- `screen-2-app-version` はモーダル下部（ボタン行の左）に控えめな secondary text で表示。
  取得前 / 失敗時は要素ごと描画しない
- `screen-2-storage-dir-reset` は保存ディレクトリ行の直下に secondary button で表示。
  default 取得前 / 失敗時は要素を残したまま `disabled`

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

### App version 表示 {#invariants-app-version}

- **I-SM9（slice 経由で取得）**: バージョンは `get-app-version` slice の `getAppVersion()` からのみ取得する。
  modal mount ごとに **1 回だけ** 呼ぶ
- **I-SM10（失敗時は非表示）**: 取得前 / reject 時は `screen-2-app-version` を描画しない。
  エラー表示・save 阻害はしない
- **I-SM11（form 状態と独立）**: 表示値は draft state に含めず、`dirty` 判定 / save payload /
  cancel rollback の対象外

### デフォルトに戻す {#invariants-storage-dir-reset}

- **I-SM12（draft の置換のみ）**: default path は `get-default-storage-dir` slice の
  `getDefaultStorageDir()` からのみ取得し、modal mount ごとに **1 回だけ** 呼ぶ。
  ボタン押下は draft の `storage_dir` を default path に置き換えるだけで、`update-settings` は呼ばない。
  永続化と `StorageDirChanged` は save 時に既存の I-SM3 の経路で行い、cancel / Esc では破棄する（I-SM2）
- **I-SM13（取得できないときは disabled）**: 取得前 / reject 時はボタンを `disabled` にし、押下しても
  draft を変えない。エラー表示・save 阻害はしない
- **I-SM14（default と同じなら差分なし）**: リセット後の draft が mount 時の `storage_dir` と同じなら
  `dirty` は false のままで、save は workflow を呼ばずに close する（I-SM6）

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

`kind === 'invalid_path'` かつ `reason === 'contains_config_path'`（I-S2）の場合は
「設定ファイルを含むフォルダは指定できません」を同位置に表示する。

### tp-sm-no-raw-invoke: 生 invoke 禁止 {#tp-sm-no-raw-invoke}

eslint static check で `apps/promptnotes/src/ui-widget/settings-modal/` から
`@tauri-apps/api/core` の import が 0 件（I-SM7）。
`@tauri-apps/plugin-dialog` は許容（I-SM8）。

### tp-sm-app-version-shown: mount 後にバージョン表示 {#tp-sm-app-version-shown}

`getAppVersionFn` が `'0.2.2'` で resolve → store の `appVersion` が `'0.2.2'` になり、
`getAppVersionFn` の呼出は **1 回**（I-SM9）。component では `v0.2.2` が表示される。

### tp-sm-app-version-hidden-before-resolve: 取得前は非表示 {#tp-sm-app-version-hidden-before-resolve}

`getAppVersionFn` が未 resolve の間、store の `appVersion` は `null`（I-SM10）。

### tp-sm-app-version-hidden-on-error: 取得失敗は非表示 {#tp-sm-app-version-hidden-on-error}

`getAppVersionFn` が reject → `appVersion` は `null` のまま、例外は外に漏れず、
`saveState` は `idle` のまま（I-SM10）。

### tp-sm-app-version-not-in-diff: バージョンは差分判定の対象外 {#tp-sm-app-version-not-in-diff}

バージョン取得後に何も編集せず save → `updateSettings` は呼ばれず close（I-SM6 / I-SM11）。

### tp-sm-reset-sets-draft: リセットで draft を default に置換 {#tp-sm-reset-sets-draft}

`getDefaultStorageDirFn` が default path で resolve → リセットボタン押下 →
`storage_dir` の表示が default path になり、`dirty` が true になる。この時点で `updateSettings` は
呼ばれず、save で `updateSettings({ storage_dir: <default> })` が 1 回呼ばれる。
`getDefaultStorageDirFn` の呼出は **1 回**（I-SM12）。

### tp-sm-reset-same-as-default-no-diff: default と同じなら差分なし {#tp-sm-reset-same-as-default-no-diff}

mount 時の `storage_dir` が default と同じ → リセット → save で `updateSettings` は呼ばれず close。
別 path を選んだ後にリセットして mount 時の値に戻った場合も `dirty` は false（I-SM14）。

### tp-sm-reset-disabled-before-resolve: 取得前は disabled {#tp-sm-reset-disabled-before-resolve}

`getDefaultStorageDirFn` が未 resolve の間、store の `defaultStorageDir` は `null`、リセットは no-op、
component のボタンは `disabled`（I-SM13）。

### tp-sm-reset-disabled-on-error: 取得失敗は disabled {#tp-sm-reset-disabled-on-error}

`getDefaultStorageDirFn` が reject → `defaultStorageDir` は `null` のまま、例外は外に漏れず、
`saveState` は `idle` のまま。リセットは no-op で、component のボタンは `disabled`（I-SM13）。

### tp-sm-reset-clears-error: リセットで inline error を消す {#tp-sm-reset-clears-error}

save が `invalid_path` で失敗して inline error が出ている状態でリセット → `saveState` は `idle` に戻る
（folder picker で選び直したときと同じ扱い）。

## 実装ノート {#impl-notes}

### Svelte 5 + `<dialog>` element {#impl-svelte-dialog}

- ルートは HTML `<dialog>` element + `showModal()` で modal をネイティブに表現
- Esc キーは `<dialog>` の `cancel` event で拾い、preventDefault しない（OS 既定の挙動）
- mount は parent (page-main) の `$state` flag で制御。flag が true の時のみ
  `WidgetSettingsModal` を `{#if}` で render

### Form state は createSettingsModalStore で testable に {#impl-store}

- `createSettingsModalStore(initialSettings, deps)` factory を切り、
  draft state / save / cancel logic を **vitest server (node)** で単体テスト可能にする
- deps: `updateSettingsFn` (slice の `updateSettings` 関数) / `getAppVersionFn`
  (slice の `getAppVersion` 関数) / `getDefaultStorageDirFn` (slice の `getDefaultStorageDir` 関数) を注入
- store に `appVersion: string | null` と `loadAppVersion(): Promise<void>`（reject を握り潰す）を持たせ、
  component の mount 時に `loadAppVersion()` を 1 回呼ぶ
- 同様に `defaultStorageDir: string | null` / `loadDefaultStorageDir(): Promise<void>`（reject を握り潰す）/
  `resetStorageDir()`（未取得なら no-op）を持たせ、deps に `getDefaultStorageDirFn` を追加する
- component (`*.svelte`) は store の薄い presentation layer

### ディレクトリ構成 {#impl-layout-dir}

```
apps/promptnotes/src/ui-widget/settings-modal/
├── WidgetSettingsModal.svelte    # presentation (dialog + form)
├── store.svelte.ts               # createSettingsModalStore factory
└── tests/
    ├── store.test.ts             # form / save / cancel / reset の unit test (server)
    └── WidgetSettingsModal.svelte.test.ts  # reset ボタンの enabled / disabled (client)
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
