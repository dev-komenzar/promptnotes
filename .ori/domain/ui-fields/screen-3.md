---
coherence:
  source: human
  last_validated: 2026-06-20
  upstream:
    - types.md
    - workflows/check-for-updates.md
---

# Screen 3: Update Notification {#screen-3}

新バージョン検出時の起動時通知 UI。
[check-for-updates](../workflows/check-for-updates.md) の `NewVersionDetected`
event を購読して表示する。

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
| `{#screen-3-dismiss}` | × | (action) | - | small icon | Toast 閉じる（次回起動時に再表示） |

## Cross-Field Rules {#cross-field-rules}

### 表示タイミング {#cross-display-timing}

- アプリ起動時 1 回のみ `check-for-updates` 実行（I-U3）
- `NewVersionDetected` 受信時のみ Toast 表示
- 失敗時 (NetworkError / ParseError / RateLimited) は **silent**（表示なし、S14）

### 表示位置・持続時間 {#cross-position-duration}

- 画面右下に固定表示（macOS 通知センター風の slide-in）
- 自動消失なし（ユーザが `screen-3-dismiss` または `screen-3-view-release` を
  操作するまで残る）
- 削除トースト (screen-1-toast) と表示位置が重ならないよう調整
  （updates toast は右下、delete toast は中央下）

### dismiss / view 後の挙動 {#cross-dismiss-view}

- `screen-3-dismiss`: Toast を閉じるだけ。次回アプリ起動時に再度 `check-for-updates`
  が走り、まだ新バージョンがあれば再表示
- `screen-3-view-release`: OS デフォルトブラウザで `Release::url` を開く。
  Toast はそのまま残す（ユーザが明示クローズするまで）

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
- 実際の更新 install は Tauri updater plugin の標準フロー（ユーザがバージョン
  リンクから GitHub Release ページに行き、download or auto-update を実行）
- MVP では「自動 install」「再起動して更新」までは扱わない（spec で明示されていない）

### 表示文言 {#notes-wording}

- 「新しいバージョン `<latest>` が利用可能です（現在: `<current>`）」
- ボタン: 「詳細を見る」「閉じる」
- 過度な promotional 文言を避ける（ユーザの集中を尊重）
