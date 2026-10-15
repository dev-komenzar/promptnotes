---
ori:
  node_id: ui-field:screen-2
  type: ui-field
  depends_on:
    - type-definitions:index
    - workflow:update-settings
    - workflow:get-app-version
    - workflow:get-default-storage-dir
---

# Screen 2: Settings Modal {#screen-2}

OS ネイティブのモーダルダイアログとして表示される設定画面。
spec 「設定モーダル」セクション + Q6 (Settings 永続化先) の反映。

## Purpose {#purpose}

[update-settings](../workflows/update-settings.md) workflow の trigger UI。
`storage_dir` と `theme` を編集する。
あわせて [get-app-version](../workflows/get-app-version.md) で取得した
現在のアプリバージョンを読み取り専用で表示する。
「デフォルトに戻す」は [get-default-storage-dir](../workflows/get-default-storage-dir.md)
で取得した OS 慣習パスを使う。

`sort_preference` は [change-sort-order](../workflows/change-sort-order.md) 経由
（ツールバーのソートトグル）でも変更されるため、本モーダルでは扱わない方針
（spec の「設定項目」に sort 嗜好は明示されていない）。

## Fields {#fields}

| id | label | 型 | 必須 | UI | 備考 |
|--|--|--|--|--|--|
| `{#screen-2-storage-dir}` | 保存ディレクトリ | `PathBuf → StorageDir` | ✓ | folder picker + read-only path display | OS ネイティブのフォルダ選択ダイアログ。デフォルト = OS 慣習パス |
| `{#screen-2-storage-dir-reset}` | デフォルトに戻す | (action) | - | button | 編集中の保存ディレクトリを OS 慣習パスに置き換える（保存は「保存」押下時） |
| `{#screen-2-theme}` | テーマ | `Theme` | ✓ | segmented control / radio | `System | Light | Dark` の 3 値 |
| `{#screen-2-app-version}` | バージョン | `AppVersion` | - | read-only text | モーダル下部に `v{version}` 形式で表示（例: `v0.2.2`）。編集不可・保存対象外 |
| `{#screen-2-cancel}` | キャンセル | (action) | - | secondary button | 変更を破棄してモーダル閉じる |
| `{#screen-2-save}` | 保存 | (action) | - | primary button | `update-settings` 呼び出し |

## Cross-Field Rules {#cross-field-rules}

### storage_dir の検証 {#cross-storage-dir-validation}

- フォルダ選択時、選択された path が **絶対パス** であることを保証
  （OS ネイティブの folder picker は通常絶対パスを返すため自動的に成立）
- `StorageDir::try_from_path` が `InvalidPath` を返した場合、
  `screen-2-storage-dir` の直下に inline error 表示
- メッセージ: 「絶対パスを指定してください」
- `update-settings` が `InvalidPath { reason: ContainsConfigPath }`（I-S2:
  `config_path.starts_with(storage_dir)`、判定は update-settings slice の application 層）を
  返した場合も、
  同じく `screen-2-storage-dir` の直下に inline error 表示
- メッセージ: 「設定ファイルを含むフォルダは指定できません」

### storage_dir 変更後の挙動 {#cross-storage-dir-restart}

- 保存ボタン押下で `update-settings` workflow が走り、`StorageDirChanged` 発行
- 直後に「設定を反映するにはアプリを再起動してください」というモーダル表示
  （即時マイグレーションはしない、I-S4 / S11）
- ユーザは「今すぐ再起動」「あとで」の 2 択

### storage_dir のデフォルトへのリセット {#cross-storage-dir-reset}

- モーダル mount 時に `get-default-storage-dir` を 1 回呼び、OS 慣習パス（I-S3）を得る
- 「デフォルトに戻す」押下で、**編集中の** `screen-2-storage-dir` の値をその path に置き換える
  （folder picker で選んだときと同じ扱い）
- 永続化と `StorageDirChanged` の発行は「保存」押下時に `update-settings` が行う。
  リセットボタン単独では保存しない。キャンセル / Esc で破棄される
- mount 時の値が default と同じなら、リセットしても差分なし（`cross-no-diff` に従う）
- default の取得完了前、または取得自体が失敗した場合は、ボタンを **disabled** にする
  （エラー表示はしない。設定編集の妨げにしない）

### theme 変更の即時反映 {#cross-theme-immediate}

- `screen-2-theme` の選択肢クリックで **プレビュー的に即時反映**
- ただし永続化は「保存」ボタン押下時のみ
- キャンセル時は変更前の theme に戻す（プレビューの巻き戻し）

### 差分なし保存の挙動 {#cross-no-diff}

- 変更がない状態で「保存」を押した場合、`update-settings` workflow は
  diff なしで event 非発行（domain-events.md 準拠）
- UI はモーダルを閉じるだけ（成功通知も不要）

### app version の表示 {#cross-app-version-display}

- モーダル mount 時に `get-app-version` を 1 回呼ぶ（値は不変のためキャッシュ可）
- 取得完了前、または呼び出し自体が失敗した場合は **行ごと非表示**
  （エラー表示はしない。設定編集の妨げにしない）
- 表示値は Save / Cancel / theme プレビューの影響を受けない
- 差分判定（`cross-no-diff`）の対象外

## Depended By {#depended-by}

Phase 11b で確定。現時点では：

- [screen-1](screen-1.md) の `screen-1-toolbar-settings-button` から開く
- macOS の menu bar 「PromptNotes → Preferences...」（`Cmd+,`）からも開く
- spec の「OSメニューバーの『Preferences』またはツールバーの設定ボタン」を尊重

## Notes {#notes}

### OS ネイティブモーダル優先 {#notes-os-native-modal}

- Tauri v2 の WebView 内 HTML overlay よりも OS ネイティブの modal dialog を優先
- 不可能な OS では薄い border のみの overlay（背景に dim 等の装飾はしない）
- ESC キーでキャンセル + 閉じる（cross-screen-shortcuts と整合）

### sort_preference をここに置かない理由 {#notes-no-sort-here}

- spec の「設定項目」は `storage_dir` と `theme` の 2 項目のみ
- sort_preference は `change-sort-order` の副作用として Settings に書かれるが、
  「設定」というユーザメンタルモデルでは「ツールバー操作の記憶」に近い
- 設定モーダルで sort を変更できると、ツールバーとの二箇所重複になり混乱
- 将来 spec が拡張されたら追加する余地はある（Phase 0 検討）

### storage_dir 変更時のデータ扱い {#notes-storage-dir-data}

- 変更前の `storage_dir` の Note ファイルは **そのまま残す**（移動しない）
- 変更後の `storage_dir` は再起動後に新規スキャン
- ユーザは手動で `.md` ファイルを新 dir へコピー / 移動する想定
- 将来「マイグレーションウィザード」を追加する余地あり（Phase 0）
