---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s11-storage-dir-change
      hash: 4a02c17cc025
    - path: domain/workflows/update-settings.md#update-settings
      hash: f420da94bd93
    - path: domain/aggregates.md#settings-aggregate
      hash: 56f7a54a8ab2
    - path: domain/domain-events.md#storage-dir-changed
      hash: 71db66eafe03
    - path: domain/ui-fields/screen-2.md#screen-2
      hash: f1e4fd24bf05
---

# s11-storage-dir-change — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s11-storage-dir-change phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

設定モーダルで `storage_dir` を変更して保存したとき、**Note の物理的な引っ越しは行わず、
再起動を要求するだけ**であることを検証する scenario。`update-settings` workflow は
`Settings::change_storage_dir(new_dir)` を適用して `app_config_dir/settings.json` に永続化し、
`StorageDirChanged { old_dir, new_dir }` を発行する。UI 層はこの event を購読して
**再起動を促すモーダル**を表示し、Note Capture / Note Feed は **再起動まで旧
`storage_dir` を見続ける**（I-S4）。

- 対応 workflow: `domain/workflows/update-settings.md`（Steps: `loadCurrent` → `validateStorageDir` →
  `applyChanges` → `persist` → `emitConditional`。storage_dir 差分時のみ `StorageDirChanged` を発行）
- 対応 aggregate: `domain/aggregates.md#settings-aggregate`（I-S4: `change_storage_dir` は即時には
  Note の引っ越しを起こさない。再起動を要求する想定）
- 対応 event: `domain/domain-events.md#storage-dir-changed`（Subscribers: **UI 層** = 再起動を促す
  モーダル表示、**Infrastructure 層** = ファイルウォッチャーの監視対象切替。後者は S22 の範囲で本
  scenario では扱わない。Timing: 同期。Note Capture / Note Feed は再起動まで反映しない）
- 対応 page: `page-main`（`settings:storage_dir_changed` を購読して再起動モーダルを表示）+
  `widget-settings-modal`（`screen-2-save` が `update-settings` の唯一の UI trigger）

> domain/validation.md#s11-storage-dir-change より:

- Given: Settings (`storage_dir = /old/path`)、フィードに Note 3 件表示中
- When: 設定モーダルで `storage_dir = /new/path` に変更して保存
- Then:
  - `Settings::change_storage_dir(new_dir)` 実行
  - `app_config_dir/settings.json` に永続化
  - event **StorageDirChanged** `{ old_dir: /old/path, new_dir: /new/path }` 発行
  - UI: 再起動を促すモーダル表示（I-S4）
  - フィードの 3 件は **表示されたまま**（`/old/path` の Note を見続ける）
  - 再起動後に `/new/path` のスキャン結果が表示される

> domain/workflows/update-settings.md#update-settings より:
> 「設定モーダルからの保存操作で Settings を更新する。`storage_dir` / `theme` のいずれか（または両方）
> の変更を扱う」

> domain/workflows/update-settings.md#notes より:
> 「**storage_dir 変更は即時マイグレーションしない**（I-S4, S11）→ 再起動を促すモーダルは UI 層が
> StorageDirChanged を購読して表示」

> domain/aggregates.md#settings-aggregate より:
> 「**I-S4**: `change_storage_dir` 操作は即時には Note の引っ越しを起こさない（再起動を要求する想定）」

> domain/domain-events.md#storage-dir-changed-subscribers より:
> 「UI 層: 再起動を促すモーダルを表示（即時マイグレーションは行わない、I-S4）」
> 「Timing: 同期。Note Capture / Note Feed は再起動まで反映しない」

> domain/ui-fields/screen-2.md#cross-storage-dir-restart より:
> 「保存ボタン押下で `update-settings` workflow が走り、`StorageDirChanged` 発行」「直後に
> 『設定を反映するにはアプリを再起動してください』というモーダル表示（即時マイグレーションは
> しない、I-S4 / S11）」「ユーザは『今すぐ再起動』『あとで』の 2 択」

## シナリオステップ {#scenario-steps}

### Given {#given}

1. `settings.json` の `storage_dir` は **old dir**（`XDG_CONFIG_HOME` 配下の隔離 config dir に
   seed 済み）
2. old dir に Note 3 件（`20260601100000` "alpha" / `20260615100000` "beta" /
   `20260626100000` "gamma"）が seed されている
3. new dir に **別の** Note 1 件（`20260701100000` "delta-from-new-dir"）が seed されている
4. Tauri App が起動済みで、フィードに old dir の 3 件が表示されている

### When {#when}

1. ユーザが toolbar の設定ボタン（`screen-1-toolbar-settings-button`）を押して設定モーダルを開く
2. 保存ディレクトリ（`screen-2-storage-dir`）を new dir に変更する
3. 「保存」（`screen-2-save`）を押す

### Then {#then}

- `Settings::change_storage_dir(new_dir)` が実行される（`update-settings` workflow 経由）
- `settings.json` の `storage_dir` が **new dir** に更新される（永続化成功）
- event `StorageDirChanged { old_dir, new_dir }` が発行される
- UI: 再起動を促すモーダル（`restart-prompt`）が表示される（I-S4）
- フィードの 3 件は **表示されたまま**（old dir の Note を見続ける。new dir の Note は現れない）
- 「今すぐ再起動」押下でアプリが再読込され、`/new/path` のスキャン結果（new dir の Note 1 件）が
  表示される

### 補足（storage_dir 変更の駆動方法） {#when-boundary-invoke}

validation の When は「設定モーダルで変更して保存」である。保存ディレクトリは read-only 表示 +
OS native folder picker（screen-2）のため dialog 自体は駆動できない。そこで E2E は storage_dir の
変更を `update_settings` command の Tauri 境界 invoke で行い（s10 の境界 invoke と同方式）、
**modal を開いた状態**でこれを起こす。その後 modal から theme を変更して保存すると、
`handleSettingsSaved` は storage_dir 差分を含む Save として呼ばれるため、Feed 据え置き guard も
UI 経路で検証できる。production コードに test seam は入れない。

### 補足（theme 変更は再起動不要） {#when-theme-contrast}

`update-settings` は `theme` 差分時には `ThemeChanged` のみを発行し `StorageDirChanged` は発行しない。
したがって theme のみの保存では再起動モーダルは表示されない（再起動要求は `storage_dir` 変更に
固有の挙動であることの対照）。

### 補足（watcher 再起動は範囲外） {#when-watcher-out-of-scope}

`StorageDirChanged` の Infrastructure 層 subscriber（ファイルウォッチャーの監視対象切替）は
**S22 (`domain/validation.md#s22-storage-dir-change-watcher-restart`) の範囲**であり、本 scenario では
実装・検証しない。

## テスト観点 {#test-points}

- **初期状態**: old dir の Block 3 件が表示され、new dir 由来の Note は表示されない。
  `settings.json` の `storage_dir` が old dir であること
- **設定モーダル表示（UI 統合）**: toolbar の設定ボタンで `widget-settings-modal` が開き、
  `screen-2-storage-dir` に old dir が表示される
- **対照: theme のみの UI save は再起動モーダルを出さない**: modal で theme を変更して
  `screen-2-save` を押すと `settings.json` の theme が更新され、`ThemeChanged` のみ発行されるため
  `restart-prompt` は表示されない（modal の save 経路の統合確認を兼ねる）
- **storage_dir 変更（境界 invoke・本 scenario の核）**: modal を開いた状態で
  `update_settings { input: { storage_dir } }` を Tauri 境界で invoke →
  `settings.json` の `storage_dir` が new dir に更新される
- **再起動モーダル表示（I-S4）**: storage_dir 変更後 `restart-prompt` が表示される
  （`settings:storage_dir_changed` subscriber 経由）
- **フィードは旧ディレクトリのまま（I-S4）**: storage_dir 変更後に modal から theme を変更して
  保存（storage_dir 差分を含む Save 経路）しても old dir の 3 件が表示されたままで、
  new dir の Note は現れない（`handleSettingsSaved` が Feed を再 hydrate しない）
- **再起動後の新ディレクトリ反映**: `restart-prompt-restart`（今すぐ再起動）押下でアプリが再読込され、
  new dir の Note 1 件が表示される

## 実装ノート {#impl-notes}

- **runner**: `wdio`（優先チェーン step 2 — manifest に `runner:` 明示なし。参加 app `promptnotes` は
  `local` 系で `runtime.runner = wdio`。`.ori/architecture.md` の `scenario_test_runner: wdio` とも一致）
- **infrastructure**: `promptnotes` のみ（`mode: local`）。compose-service 系 app が参加しないため
  docker-compose は不要
- **runner config**: `wdio.conf.ts` が `/ori-generate` により生成される
- **config dir の隔離**: `onPrepare` が `XDG_CONFIG_HOME` / `XDG_DATA_HOME` を temp dir に設定する。
  Tauri の `app_config_dir()` は `$XDG_CONFIG_HOME/<identifier>`（identifier =
  `com.komenzar.promptnotes`）に解決されるため、`<temp>/config/com.komenzar.promptnotes/settings.json`
  を `storage_dir = old dir` で seed すれば Given の Settings を再現できる
  （`TAURI_TEST_STORAGE_DIR` override は使わない。これを使うと `storage_dir` が常に固定され、
  再起動後の new dir 反映を検証できないため）
- **Note の seed**: old dir に 3 件、new dir に 1 件を `onPrepare` が書き込む
- **storage_dir 変更の駆動方法**: 保存ディレクトリは read-only 表示 + OS native folder picker のため
  dialog 自体は駆動できない。E2E は `update_settings { input: { storage_dir } }` を Tauri 境界で
  直接 invoke する（s10 の境界 invoke と同方式）。これを modal open 中に起こし、その後 modal から
  theme を変更して保存することで、`handleSettingsSaved` を storage_dir 差分ありで呼ばせる
  （Feed 据え置き guard の UI 経路検証）。production コードに test seam は入れない
- **再起動モーダル**: `page-main` が `settings:storage_dir_changed` を購読して `restart-prompt` を
  表示する。`screen-2-save` 後の Feed 再 hydrate は `storage_dir` 非変更時のみ行い、`storage_dir`
  変更時は行わない（フィードが旧ディレクトリに留まる = I-S4 の観測点）
- **アサーション戦略**:
  - 初期 / 変更後の Feed は `[data-block-id="<id>"]` の存在集合で判定する
  - モーダル表示は `[data-testid="widget-settings-modal"]` / `[data-testid="restart-prompt"]` で判定
  - `settings.json` の `storage_dir` / theme は `node:fs` の `readFileSync` で直接 read する
  - storage_dir 変更は `window.__TAURI_INTERNALS__.invoke('update_settings', { input })` を同期
    `browser.execute` で kick-off し window 上の結果を poll する（`@wdio/tauri-service`
    (driverProvider=external) の patchedExecute は `browser.executeAsync` を扱えないため。s10 と同方式）
  - 「今すぐ再起動」は `restart-prompt-restart` 押下 → `window.location.reload()` による remount で
    再現し、settings.json を再読込して new dir の Note が現れることを確認する
- **テストデータのクリーンアップ**: `wdio.conf.ts` の `onComplete` が temp dir を `rmSync` で削除する
- **production 側の前提（本 scenario が検証する不変条件）**:
  1. `update_settings` command が `storage_dir` 差分時に `settings.json` へ永続化し
     `StorageDirChanged` を `app.emit('settings:storage_dir_changed')` すること
     （`apps/promptnotes/src-tauri/src/user_preferences/slices/update_settings/`）
  2. `page-main` が `settings:storage_dir_changed` を購読して再起動モーダルを表示すること
     （`apps/promptnotes/src/ui-page/page-main/PageMain.svelte`）
  3. `storage_dir` 変更時に Feed を新ディレクトリへ再 hydrate **しない**こと（I-S4）
     （`apps/promptnotes/src/ui-page/page-main/PageMain.svelte#handleSettingsSaved`）
  4. 設定モーダルの `screen-2-save` が `update-settings` slice（TS bindings）経由で
     `update_settings` command を呼ぶこと
     （`apps/promptnotes/src/ui-widget/settings-modal/WidgetSettingsModal.svelte`）
  これらのいずれかが欠落すると S11 が RED になる
