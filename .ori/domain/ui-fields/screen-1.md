---
ori:
  node_id: ui-field:screen-1
  type: ui-field
  depends_on:
    - type-definitions:index
    - workflow:create-note
    - workflow:auto-save-note
    - workflow:flush-note
    - workflow:assign-tag
    - workflow:remove-tag
    - workflow:delete-note
    - workflow:restore-deleted-note
    - workflow:copy-note-body
    - workflow:update-feed-filter
    - workflow:change-sort-order
---

# Screen 1: Main Window {#screen-1}

PromptNotes のメインウィンドウ。spec の **シングルペイン制約** により、すべての
操作がこの 1 画面で完結する。Toolbar / Draft Input / Feed (Block list) / Toast
の 4 region で構成される。

## Purpose {#purpose}

10 workflow の trigger UI を集約する画面：

- 起案系: [create-note](../workflows/create-note.md)
- 編集系: [auto-save-note](../workflows/auto-save-note.md) / [flush-note](../workflows/flush-note.md)
- タグ系: [assign-tag](../workflows/assign-tag.md) / [remove-tag](../workflows/remove-tag.md)
- 削除/復元: [delete-note](../workflows/delete-note.md) / [restore-deleted-note](../workflows/restore-deleted-note.md)
- 利用: [copy-note-body](../workflows/copy-note-body.md)
- フィルター/ソート: [update-feed-filter](../workflows/update-feed-filter.md) / [change-sort-order](../workflows/change-sort-order.md)

## Fields {#fields}

### Toolbar region {#fields-toolbar}

| id | label | 型 | 必須 | UI | 備考 |
|--|--|--|--|--|--|
| `{#screen-1-toolbar-search-query}` | 検索 | `String → Option<NormalizedQuery>` | - | text input (icon 付) | `Cmd+F` で focus。1 文字入力ごと即時 filter |
| `{#screen-1-toolbar-date-range}` | 期間 | `DateRangeFilter` | - | segmented control + date picker | preset 4 + Custom |
| `{#screen-1-toolbar-sort-field}` | ソート対象 | `SortField` | ✓ | dropdown / segmented | `CreatedAt | UpdatedAt` |
| `{#screen-1-toolbar-sort-direction}` | ソート方向 | `SortDirection` | ✓ | toggle button (↑↓ icon) | `Asc | Desc` |
| `{#screen-1-toolbar-settings-button}` | 設定 | (action) | - | icon button (歯車) | クリックで screen-2 を開く |

### Draft Input region {#fields-draft}

| id | label | 型 | 必須 | UI | 備考 |
|--|--|--|--|--|--|
| `{#screen-1-draft-body}` | 本文 | `String → NoteBody` | - | CodeMirror 6 (readOnly: false) | フィード最上部に常時固定。Markdown シンタックスハイライト + [入力補助](#notes-markdown-helpers) 適用。`Cmd+N` で focus、`Enter` で改行、`Cmd+Enter` で確定 |
| `{#screen-1-draft-tag-chip}` | 下書きタグ | `Tag` | - | chip | 本文入力欄の下に表示。`Cmd+Enter` 確定時に `create-note` の `raw_tags` に渡される |
| `{#screen-1-draft-tag-input}` | 新規タグ入力 | `String → Tag` | - | text input (Draft 領域内) | 本文入力欄の下、タグチップの右。Enter で確定 → chip として追加。`Tag::try_from_string` 失敗時は [cross-tag-error](#cross-tag-error) 準拠で reject |
| `{#screen-1-draft-tag-remove}` | タグ削除 | (action) | - | × icon on chip | クリックで下書きタグから削除 |
| `{#screen-1-draft-submit}` | ＋追加 | (action) | - | button | クリックで `Cmd+Enter` と同等 |

### Block region (各 Note 1 ブロック) {#fields-block}

各 Block は `IDLE | FOCUSED | EDITING` の state machine を持つ（spec 準拠）。
**上下 2 段構造** (spec「各ブロックの構造（上から）」準拠):

1. **メタ行** (上段, 小さく薄いフォント)
   - 左: タグチップ群
   - 右: `createdAt` 右寄せ + `updatedAt` ホバー tooltip
2. **本文** (下段, CodeMirror 6, Markdown シンタックスハイライト, レンダリングなし)
   - 全文表示（切り捨て・折りたたみ **禁止**）

ホバー時アクション（copy / delete）はブロック右上 (メタ行右端の手前) に重ねて表示。

| id | label | 型 | 必須 | UI | 備考 |
|--|--|--|--|--|--|
| `{#screen-1-block-tag-chip}` | タグ | `Tag` | - | chip (click で `update-feed-filter SetTag`) | **メタ行左端**、タグ群を順序保持で表示。小さく薄いフォント |
| `{#screen-1-block-tag-input}` | 新規タグ入力 | `String → Tag` | - | text input (タグ編集モード時のみ表示) | メタ行内、Enter で確定 → `assign-tag` |
| `{#screen-1-block-tag-remove}` | タグ削除 | (action) | - | × icon on chip | クリックで `remove-tag` |
| `{#screen-1-block-created-at}` | 作成日時 | `Timestamp` | ✓ | read-only label | **メタ行右端**、右寄せ、小さく薄いフォント |
| `{#screen-1-block-updated-at}` | 更新日時 | `Timestamp` | - | read-only tooltip | メタ行右端、`createdAt` のホバー時のみ表示 |
| `{#screen-1-block-body}` | 本文 | `NoteBody` | ✓ | CodeMirror 6 (readOnly toggle) | **下段**。Markdown シンタックスハイライトあり、レンダリングなし。IDLE/FOCUSED: readOnly=true、EDITING: readOnly=false。**全文表示（切り捨て・折りたたみ禁止）** |
| `{#screen-1-block-copy}` | コピー | (action) | - | hover-only icon button | ブロック右上に重ねて表示、クリックで `copy-note-body` |
| `{#screen-1-block-delete}` | 削除 | (action) | - | hover-only icon button | ブロック右上に重ねて表示、クリックで `delete-note` |

### Toast region (画面下部) {#fields-toast}

削除操作ごとに 1 つの Toast を発行し、画面下部に **縦方向にパイル** して表示する。
各 Toast は独立した Undo 対象を保持し、互いに干渉しない。

| id | label | 型 | 必須 | UI | 備考 |
|--|--|--|--|--|--|
| `{#screen-1-toast-stack}` | トーストスタック | `Vec<DeletedNote>` | - | 縦パイル container | 新しい削除を **上** に積む（最新が画面上側）。古い Toast は下に押し下げられる |
| `{#screen-1-toast-message}` | メッセージ | `String` | - | read-only label (各 Toast 内) | 「Note を削除しました」など、削除した Note の identity を含めてもよい |
| `{#screen-1-toast-undo}` | 元に戻す | (action) | - | inline button (各 Toast 内) | 仮 5 秒間表示中のみ有効。クリックで対応する `DeletedNote` に対する `restore-deleted-note` |
| `{#screen-1-toast-close}` | × | (action) | - | small icon (各 Toast 内) | 明示クローズ。スタック内のその Toast のみ消える |

## Cross-Field Rules {#cross-field-rules}

### Block 内のステート遷移 {#cross-block-state}

- IDLE → FOCUSED: `↑` / `↓` キー
- IDLE → EDITING: ブロッククリック
- FOCUSED → EDITING: `Enter`
- FOCUSED → FOCUSED (別ブロック): `↑` / `↓`
- FOCUSED → IDLE: `Esc`
- EDITING → FOCUSED: `Esc`
- EDITING → 別ブロック EDITING: 別ブロッククリック（元ブロックは IDLE）
- EDITING 中の `↑` / `↓` は **変更なし**（編集中はナビ無効）

### Draft の Cmd+Enter 確定後 {#cross-draft-submit}

- 入力欄（本文 + 下書きタグ）を即時クリア
- 下書きタグを `raw_tags` として `create-note` に渡す
- 新規 Block がフィード最上部に挿入（作成日時 + タグチップ付き）
- フォーカスは新規 Block に移動（FOCUSED 状態、Q5: Esc で IDLE へ）

### Toast の表示制約 {#cross-toast-display}

- **削除ごとに新規 Toast を発行し、縦パイルで複数表示**
  - 各 Toast は独立した `DeletedNote` を保持し、それぞれ独立した有効期間を持つ
  - 新しい Toast は **上に積む** (最新が画面上側に表示される)
  - 古い Toast は下に押し下げられ、表示は維持される
- 各 Toast の消失条件: 仮 5 秒経過 / 明示クローズ / 対応する Undo クリック
  - いずれかで対応する `DeletedNote` の Undo 保持を破棄
- 各 Toast の `screen-1-toast-undo` は **その Toast の有効期間中のみ enable**
  - 期限切れ後はその 1 つの Undo のみが reject される（他の Toast は影響しない）
- **スタックの最大表示数**: MVP では制限なし（同時表示が増えすぎたら Phase 11b の UX 検討で
  上限導入を再検討、例: 最大 5 件で古いものから自動消失など）

### Sort 変更の即時反映 {#cross-sort-immediate}

- `screen-1-toolbar-sort-field` または `sort-direction` の変更
  → NoteFeed.change_sort 即時呼び出し + Settings.sort_preference 永続化
- Block 並びはアニメーションで再配置（ジャンプを避ける、UX）

### Filter 変更の即時反映 {#cross-filter-immediate}

- 検索文字列 / 期間 / タグ filter の変更は **debounce なしで即時 filter**
  （ローカル完結なので遅延不要、Q7）
- 結果 0 件のときは「該当する Note がありません」を Feed 領域に表示

### Tag 入力の禁止文字エラー {#cross-tag-error}

- `Tag::try_from_string` が `TagError::InvalidChar` を返した場合、
  `screen-1-block-tag-input` または `screen-1-draft-tag-input` の直下に inline error 表示
- メッセージ: 「タグに使えない文字（カンマ・ブラケット・空白）が含まれています」
- Enter キーは reject されたまま、入力欄はクリアしない

## Depended By {#depended-by}

Phase 11b で確定。現時点では：

- **アプリ起動時の唯一の表示画面**（screen-2 / screen-3 は overlay）
- メニューバー（macOS）の「Window → PromptNotes」で前面化
- screen-2 / screen-3 はこの画面の上に重なる形で表示

## Notes {#notes}

### CodeMirror の readOnly トグル一貫性 {#notes-codemirror-consistency}

- Draft 入力欄、Block 本文ともに **同一の CodeMirror 6 構成**
- `readOnly: false` (Draft 常時, Block EDITING 時)
- `readOnly: true` (Block IDLE / FOCUSED 時)
- 別レンダラ（プレビュー用 HTML 等）に切り替えない（spec 末尾の禁止事項 #3）

### Block の縦並び (フィード全体) {#notes-block-vertical}

- フィード上のブロック群は **一列縦並び** のみ（複数カラム禁止、spec）
- ブロック間の区切り: 細い水平線のみ（背景色・枠線・影によるコンテナなし）
- 背景色:
  - IDLE: デフォルト
  - FOCUSED: 薄いハイライト
  - EDITING: FOCUSED と同色

### Block 内の上下構造 {#notes-block-layout}

spec「各ブロックの構造（上から）」を厳密に遵守:

1. **メタ行** (上段)
   - 1 行で収まる高さ
   - フォント: 小さく薄い（本文より明らかに subdued）
   - 左端: [tag-chip](#fields-block) + [tag-input](#fields-block) (編集モード時) + [tag-remove](#fields-block)
   - 右端: [created-at](#fields-block) (常時) / [updated-at](#fields-block) (ホバー時 tooltip)
2. **本文** (下段)
   - CodeMirror 6 インスタンス
   - Markdown シンタックスハイライト **あり**
   - HTML/PDF へのレンダリング **なし**（spec 末尾の禁止事項 #3: 別レンダラ切替禁止）
   - **全文表示**（切り捨て / 折りたたみ / "show more" 等のトリミング UI 禁止）
   - 高さは内容に応じて自然伸縮（min-height はメタ行の数倍程度）

メタ行と本文の間に視覚的セパレータは置かない（spec の minimal レイアウト原則）。

### ホバーアクションの表示タイミング {#notes-hover-actions}

- `copy` / `delete` ボタンは ホバー時のみ表示（ブロック右上に重ねて配置）
- メタ行の `updated_at` tooltip もホバー時のみ
- マウス非依存（タッチデバイス）の代替は MVP 範囲外

### Markdown 入力補助 (CodeMirror 6 共通) {#notes-markdown-helpers}

[draft-body](#fields-draft) と [block-body](#fields-block) の **両 CodeMirror** で
有効化する補助機能（spec「Markdown入力補助」準拠）:

- `-` や `1.` 行で改行すると次行も自動でリスト記号を挿入
- `Tab` でインデント、`Shift+Tab` でアンインデント
- `**` 入力で `****` になりカーソルが中央に入るブラケット補完
- 見出し `#` のシンタックスハイライト強調

readOnly: true 時（IDLE/FOCUSED ブロック本文）は補助機能は不要だが、シンタックス
ハイライト自体は維持（CodeMirror の表示一貫性を保つため）。
