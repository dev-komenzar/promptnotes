---
coherence:
  source: human
  last_validated: 2026-06-20
  upstream:
    - types.md
    - workflows/index.md
---

# UI Fields {#ui-fields}

PromptNotes は **シングルペイン制約** により画面数が極小（3 画面）。
spec の禁止事項（サイドバー / アプリ名ヘッダー / レンダラ切替）を
ui-fields 層でも遵守する。

## Screens Summary {#screens-summary}

| id | purpose | 関連 workflow |
|--|--|--|
| [screen-1](screen-1.md) | メインウィンドウ（Toolbar + Draft + Feed + Toast） | create-note / auto-save-note / flush-note / assign-tag / remove-tag / delete-note / restore-deleted-note / copy-note-body / update-feed-filter / change-sort-order |
| [screen-2](screen-2.md) | 設定モーダル | update-settings |
| [screen-3](screen-3.md) | 更新通知 | check-for-updates |

## Cross-Cutting VO Mapping {#cross-vo-mapping}

| VO / 型 | UI コントロール | validation の責務 |
|--|--|--|
| `NoteBody` | textarea (CodeMirror 6) | `NoteBody::try_from_string` で frontmatter delimiter を reject |
| `Tag` | chip input + plain text | `Tag::try_from_string` で正規化 + 禁止文字 reject |
| `TagSet` | chip list (順序保持) | `TagSet::insert` で重複排除 |
| `NormalizedQuery` | search input | `NormalizedQuery::from_raw` で NFC + lowercase |
| `DateRangeFilter` | preset segmented control + date picker | enum 直接マッピング |
| `SortField` | enum select / segmented | `CreatedAt | UpdatedAt` |
| `SortDirection` | toggle button / icon | `Asc | Desc` |
| `StorageDir` | folder picker | `StorageDir::try_from_path` で絶対パス検証 |
| `Theme` | radio / segmented | `System | Light | Dark` |
| `Version` | read-only label | semver 表示 |

## Naming Conventions {#naming}

- field id: `<screen>-<region>-<purpose>`（例: `screen-1-toolbar-search-query`）
- placeholder: ドメイン用語を使う（「タグ」「本文」など、UI 寄りの語を避ける）
- error message: VO の `*Error` enum と 1:1 対応させる（例: `TagError::InvalidChar` →
  「タグに使えない文字（カンマ・ブラケット・空白）が含まれています」）

## Cross-Screen Constraints {#cross-screen}

### CodeMirror 一貫性 (spec の core 制約) {#cross-screen-codemirror}

- **Draft 入力欄** と **Block 本文** は同一の CodeMirror 6 インスタンスタイプを使う
- 編集中: `readOnly: false`、非編集時: `readOnly: true`
- 「プレビュー用 HTML」と「編集用 CodeMirror」を切り替える実装は禁止（spec ミス再発防止 #3）

### OS ネイティブ要素 {#cross-screen-os-native}

- ウィンドウタイトルバーは **OS ネイティブ**（CSS で再描画しない）
- タイトルバー直下にアプリ名ヘッダーを置かない（spec ミス再発防止 #2）
- メニューバー（macOS）からも「Preferences」で設定モーダルを開ける
- 設定モーダルは OS ネイティブのモーダルダイアログ（不可なら border-only の薄い overlay）

### キーボードショートカット {#cross-screen-shortcuts}

| ショートカット | 動作 | 対応 workflow |
|--|--|--|
| `Cmd+N` / `Ctrl+N` | Draft 入力欄にフォーカス | (screen-1) |
| `Cmd+Enter` | Draft 確定 → 新規 Note 作成 | create-note |
| `Cmd+F` / `Ctrl+F` | 検索バーにフォーカス | (screen-1) |
| `Esc` | EDITING → FOCUSED / モーダル閉じる | (state machine) |
| `↑` / `↓` | ブロック間フォーカス移動（非編集時のみ） | (state machine) |
| `Enter` | FOCUSED → EDITING | (state machine) |

## Validation 一元管理 {#validation-policy}

- すべての入力 validation は **VO の smart constructor** に集中（bd memory:
  `garde` 採用方針）
- UI 層は VO 構築の `Result<_, *Error>` を受け取って error 表示するだけ
- 「UI 側だけで弾く軽量チェック」は **UX 向上の即時フィードバックに限定**
  （文字数表示など、ドメイン invariant ではないもの）

## Open Questions {#open-questions}

Phase 11a 時点で未決事項はない。

- Phase 11b で `depended_by` を確定し、page 構成を切り出す
  - PromptNotes は 3 画面なので page 化はトリビアル（おそらく 1 page にまとめる）
- 削除トーストの表示位置・有効秒数（仮 5 秒）は Phase 11b の UX 検討で確定
