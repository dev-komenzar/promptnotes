---
ori:
  node_id: ui-field:screen-3
  type: ui-field
  depends_on:
    - type-definitions:index
    - workflow:check-for-updates
    - workflow:install-update
---

# Screen 3: Update Notification {#screen-3}

新バージョン検出時の起動時通知 UI。
[check-for-updates](../workflows/check-for-updates.md) の `NewVersionDetected`
event を購読して表示する。対応配布形式では [install-update](../workflows/install-update.md)
でその場で更新して再起動できる。

## Purpose {#purpose}

`NewVersionDetected` event 発行時にのみ表示される軽量通知。
失敗時 (S14: silent) は表示されないため、ユーザの作業フローを妨げない。

実装形式は **Toast 形式** を採用（理由: Modal だと起動直後の作業を中断するため）。

## Fields {#fields}

| id | label | 型 | 必須 | UI | 備考 |
|--|--|--|--|--|--|
| `{#screen-3-current-version}` | 現在のバージョン | `Version` | ✓ | read-only label | 小さく薄く、ビルドバージョン |
| `{#screen-3-latest-version}` | 最新のバージョン | `Version` | ✓ | read-only label | 太字で強調 |
| `{#screen-3-release-notes-summary}` | リリースノート（要約） | `String` | - | truncated text (2 行まで) | 全文は外部リンク |
| `{#screen-3-view-release}` | 詳細を見る | (action → external URL) | - | inline link | `Release::url` を OS ブラウザで開く |
| `{#screen-3-install-update}` | Update & restart | (action → install-update) | - | primary button | `InstallationKind::is_installable()` のときだけ表示（I-U6）。押すと install → Flush → 再起動 |
| `{#screen-3-install-status}` | 更新状態 | `Idle \| Installing \| Failed \| AlreadyInstalling \| RestartRequired` | - | inline text | Installing 中は「Updating…」、Failed 時は失敗メッセージ、AlreadyInstalling は進行中の旨、RestartRequired は手動再起動の案内 |
| `{#screen-3-dismiss}` | × | (action) | - | small icon | Toast 閉じる（次回起動時 or より新しいバージョン検出時に再表示） |

## Cross-Field Rules {#cross-field-rules}

### 表示タイミング {#cross-display-timing}

- アプリ起動時 1 回 + 以降 24 時間ごとに `check-for-updates` 実行（I-U3）
- 同一バージョンの Toast は 1 プロセスにつき 1 回のみ（I-U4）
- `NewVersionDetected` 受信時のみ Toast 表示
- 失敗時 (NetworkError / ParseError / RateLimited) は **silent**（表示なし、S14）

### 表示位置・持続時間 {#cross-position-duration}

- 画面右下に固定表示（macOS 通知センター風の slide-in）
- 自動消失なし（ユーザが `screen-3-dismiss` または `screen-3-view-release` を
  操作するまで残る）
- 削除トースト (screen-1-toast) と表示位置が重ならないよう調整
  （updates toast は右下、delete toast は中央下）

### dismiss / view 後の挙動 {#cross-dismiss-view}

- `screen-3-dismiss`: Toast を閉じるだけ。同じバージョンは同一プロセス内では再表示しない（I-U4）。
  次回アプリ起動時、または定期確認でさらに新しいバージョンが見つかった時に再表示
- `screen-3-view-release`: OS デフォルトブラウザで `Release::url` を開く。
  Toast はそのまま残す（ユーザが明示クローズするまで）

### Update & restart の挙動 {#cross-install-update}

- `screen-3-install-update` は `is_update_installable` が true のときだけ表示する（I-U6）。
  false（nix 等）のときは View release / dismiss のみ（S26）
- 押すと `screen-3-install-status = Installing`（「Updating…」）にし、install 中はボタンと dismiss を無効化する
- 成功: page-main が全 pending Note を Flush してからアプリを再起動する（I-U8、S24）
- 失敗: `screen-3-install-status = Failed` で失敗メッセージを表示し、ボタンを再び押せるようにする。
  View release は使える（I-U7、S25）
- 別のインストールが実行中 / 再起動待ち（`InstallInProgress`、I-U10）: `screen-3-install-status = AlreadyInstalling` で
  「An update is already being installed.」を表示する。失敗扱いにしない（S29）
- インストール成功後に再起動が失敗: `screen-3-install-status = RestartRequired` で
  「The update was installed. Restart PromptNotes to finish updating.」を表示し、dismiss を有効に戻す。
  「Update & restart」は無効のまま（再インストールしない、I-U8 / I-U10、S29）
- download 進捗（%）は表示しない（MVP 範囲外）

### 抑制ロジック（MVP 範囲外） {#cross-suppression}

- 「このバージョンの通知を skip」のようなチェックボックスは MVP では実装しない
  （spec の「自動アップデート」セクションで詳細未記載）
- 将来追加するなら Settings に `dismissed_versions: Vec<Version>` を追加

## Depended By {#depended-by}

Phase 11b で確定。現時点では：

- アプリ起動シーケンスから自動的に表示される（ユーザ操作の trigger なし）
- 他の画面の上に overlay（screen-1 のフィード操作を阻害しない位置）

## Notes {#notes}

### Toast を選んだ理由 {#notes-toast-choice}

- Modal だと起動直後にユーザのキー入力フォーカスを奪う → spec の core 動作
  「Cmd+N で起案を 1 秒で始める」を阻害
- Toast なら通知を視認しつつ Cmd+N → Draft 編集を継続可能
- discovery の Business Drivers「起案 → 検索 → コピーの時間短縮」を尊重

### Tauri v2 updater plugin との分離 {#notes-tauri-separation}

- 実装側: `UpdaterPlugin` trait (Phase 10 types) が GitHub Releases から
  Release 情報取得
- このスクリーンは「取得済みの NewVersionDetected event を購読」するだけ
- 更新 install は [install-update](../workflows/install-update.md) が Tauri updater plugin に委譲する
  （ユーザが「Update & restart」を押したときだけ。I-U5）
- 再起動前の Flush は page-main が orchestrate する（I-U8。quit 時の S13 と同じ pending flush 経路）
- 非対応の配布形式では従来どおり View release で GitHub Release ページから手動更新する（I-U6）

### 表示文言 {#notes-wording}

- 「新しいバージョン `<latest>` が利用可能です（現在: `<current>`）」
- ボタン: 「詳細を見る」「閉じる」
- 過度な promotional 文言を避ける（ユーザの集中を尊重）
