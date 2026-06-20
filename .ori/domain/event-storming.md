---
coherence:
  source: human
  last_validated: 2026-06-20
  upstream:
    - discovery.md
---

# Event Storming {#event-storming}

PromptNotes の主要ユースケース（プロンプトの起案 → 編集 → 検索 → コピー → 削除）
を時系列で並べ、各 event に対応する command / actor / aggregate 候補を抽出する。

## Timeline {#timeline}

### Note のライフサイクル {#timeline-note-lifecycle}

| #  | event（過去形）                            | command              | actor       | aggregate 候補   |
|----|--------------------------------------------|----------------------|-------------|------------------|
| 1  | 新規 Note 入力欄が空のまま用意された       | -                    | system      | Note (Draft)     |
| 2  | Note 本文が入力された                      | TypeNoteBody         | user        | Note (Draft)     |
| 3  | Note が作成・保存された                    | CreateNote           | user        | Note             |
| 4  | Note 本文が編集された                      | EditNoteBody         | user        | Note             |
| 5  | Note が自動保存された（500ms debounce）    | AutoSaveNote         | system      | Note             |
| 6  | Note がフォーカスアウトで保存された        | FlushNote            | system      | Note             |
| 6a | ウィンドウ非アクティブ化で保存された       | FlushNote            | system      | Note             |
| 6b | アプリ終了シグナルで保存された             | FlushNote            | system      | Note             |
| 7  | Note が OS のゴミ箱へ移動された            | DeleteNote           | user        | Note             |
| 8  | Note 削除が取り消された（トースト表示中）  | RestoreDeletedNote   | user        | Note             |
| 9  | Note 本文がクリップボードへ書き出された    | CopyNoteBody         | user        | Note             |

### Tag 操作 {#timeline-tag}

| #  | event（過去形）              | command       | actor | aggregate 候補 |
|----|------------------------------|---------------|-------|----------------|
| 10 | Note に Tag が付与された     | AssignTag     | user  | Note + Tag     |
| 11 | Note から Tag が外された     | RemoveTag     | user  | Note + Tag     |

### フィード表示 {#timeline-feed}

| #  | event（過去形）                          | command            | actor | aggregate 候補 |
|----|------------------------------------------|--------------------|-------|----------------|
| 12 | フィードが検索文字列で絞り込まれた       | FilterByQuery      | user  | NoteFeed       |
| 13 | フィードが期間で絞り込まれた             | FilterByDateRange  | user  | NoteFeed       |
| 14 | フィードが Tag で絞り込まれた            | FilterByTag        | user  | NoteFeed       |
| 15 | フィードのソート順が変更された           | ChangeSortOrder    | user  | NoteFeed       |

### 設定・配布 {#timeline-settings}

| #  | event（過去形）                              | command              | actor  | aggregate 候補 |
|----|----------------------------------------------|----------------------|--------|----------------|
| 16 | 保存ディレクトリが変更された                 | ChangeStorageDir     | user   | Settings       |
| 17 | テーマが変更された                           | ChangeTheme          | user   | Settings       |
| 18 | 新バージョンの存在が起動時に通知された       | NotifyNewVersion     | system | UpdateChannel  |

## Aggregate Candidates {#aggregate-candidates}

時系列に登場する名詞をクラスタリングした結果、以下を集約候補とする。

- **Note**（中核）
  - 構成: `body`, `tags`, `createdAt`, `updatedAt`, `filename`
  - ライフサイクル: Draft → Saved → (Edited)* → DeletedToTrash → Restorable
  - `Note` ファイル名はタイムスタンプ秒精度（spec 準拠）
  - **削除 Undo 決定**（2026-06-16, Q5; 改訂 2026-06-20 by Phase 11a UI 設計）:
    - Undo 有効期間 = 各 Toast 表示中のみ（仮 5 秒、各 Toast 個別、UI Phase で確定）
    - **削除ごとに新規 Toast を発行**し、画面下部に **縦パイル** で複数表示
    - Toast 消失（時間切れ / 明示クローズ / 対応する Undo クリック）と同時に
      その Toast 対応の DeletedNote 保持を破棄
    - 各 Toast は独立した有効期間を持つ（一斉消失ではない）
    - 復元手段は各 Toast 内「元に戻す」ボタンのみ。`Cmd+Z` は採用しない（編集中 Undo と衝突回避）
    - トースト消失後は OS のゴミ箱からの手動復元に委ねる（アプリ責務外）
    - application service は DeletedNote の **スタック** (`Vec<DeletedNote>`) を保持
      （旧 Q5 では「in-memory に直近 1 件のみ」だったが、UI が複数 Toast 表示に
      変更されたため domain 側も複数保持に変更）
  - **保存トリガー決定**（2026-06-16, Q4）:
    - キー入力後 500ms debounce（`AutoSaveNote`）
    - 編集中ブロックの CodeMirror から focus 喪失（`FlushNote`）
    - メインウィンドウの blur（別アプリへの切替, `FlushNote`）
    - ウィンドウ close / アプリ quit シグナル（`FlushNote`）
    - OS sleep / shutdown までは拾わない（最大欠損 500ms を許容）

- **Tag**（Note 内の値オブジェクト）
  - 構成: `name`（正規化済み文字列）
  - **決定 1**: 独立 aggregate にせず、Note の値オブジェクトとして扱う（2026-06-11）
  - **決定 2**: 正規化ルールは **小文字化 + trim**（2026-06-11）
    - ASCII 文字は lowercase 化、前後空白を削る
    - **日本語タグも許容**（CJK 文字はそのまま保存）
    - 禁止文字: 空白文字（` `, `\t`, `\n`）, `,`, `[`, `]`
      （YAML inline list の構造と衝突するため入力時に reject）
    - 表示・検索・比較すべて正規化後の文字列を使用
  - frontmatter の `tags: [...]` を信頼源とする。Tag マスタは持たない
  - 将来 Tag rename / 色付け等が必要になれば独立 aggregate へ格上げを再検討

- **NoteFeed**（read model 候補）
  - 構成: filter (query / dateRange / tag), sort (`createdAt`|`updatedAt` × asc/desc)
  - **決定**: ソートのみ永続化、フィルター・検索は揮発（2026-06-11）
    - ソート（嗜好性が高い）は **Settings** の一部として永続化し起動時に復元
    - 検索文字列・期間フィルター・タグフィルターは **起動時にリセット**
    - 起動直後に「絞り込み残存でノートが見えない」事故を避けるための判断
  - **検索範囲決定**（2026-06-16, Q7）:
    - 対象: `body` 全文 + `tags[]` の各要素（spec の「本文+タグ」に厳密準拠）
    - frontmatter の `createdAt` / `updatedAt` / `filename` は検索対象外
      （日付絞り込みは期間フィルターの責務、filename は UI 非表示）
    - マッチング: case-insensitive substring + NFC 正規化（regex / wildcard は Non-Goal）
    - 入力 1 文字ごとに即時フィルタ（ローカル完結で debounce 不要）
    - 空文字 = 絞り込み解除

- **Settings**
  - 構成: `storageDir`, `theme` (`System` | `Light` | `Dark`), `sortPreference`
  - Q3 の決定により NoteFeed のソート嗜好も Settings に含める
  - **永続化先決定**（2026-06-16, Q6）: OS 慣習パスに準拠（Tauri v2 `app_config_dir` 経由）
    - macOS: `~/Library/Application Support/promptnotes/settings.json`
    - Linux: `${XDG_CONFIG_HOME:-~/.config}/promptnotes/settings.json`
    - Windows: `%APPDATA%\promptnotes\settings.json`
    - 形式は JSON（serde 単体で扱えるため）
    - 不在時はコード内デフォルト（storageDir = OS 慣習のノート保存先 / theme = System / sortPreference = createdAt desc）
    - `storageDir` 配下には置かない（Settings の場所が storageDir に依存する循環を回避）
    - 将来「マシン間同期」要望が出た場合は export/import で対応する想定

- **UpdateChannel**
  - 構成: 最新リリース情報
  - Tauri updater plugin が GitHub Releases を参照（external service 経由）

## Open Questions {#open-questions}

Phase 2 で挙がった未決事項はすべて解決済み（2026-06-16）。

- Q1: Tag は独立 aggregate か → 値オブジェクト（Note 内）
- Q2: Tag の正規化 → lowercase + trim、CJK 許容、YAML 構造文字を禁止
- Q3: NoteFeed の状態保持 → ソートのみ永続化（Settings）、フィルター・検索は揮発
- Q4: フォーカスアウトの定義 → ブロック離脱 + ウィンドウ blur + アプリ quit
- Q5: 削除 Undo の有効時間 → 各 Toast 表示中のみ（複数 Toast を縦パイルで保持、改訂 2026-06-20）
- Q6: Settings の永続化先 → OS 慣習パス（`app_config_dir`）
- Q7: 検索の対象範囲 → 本文 + タグのみ、case-insensitive + NFC

Phase 3 以降で新たな未決事項が生じたらここに追記する。

## Notes {#notes}

- **UI イベントは混入禁止**：「コピーボタンが押された」「↑↓でフォーカス移動した」
  といったブロックのステートマシン（IDLE/FOCUSED/EDITING）に属する遷移は
  ビジネスイベントではないため除外。これらは Phase 11a の UI 設計で扱う
- **過去形・具体名詞**：「保存された」ではなく「Note が保存された」を一貫
- **system actor**：自動保存・アップデート確認は system 起点。actor 列で明示
- **aggregate 候補の数**：5 個（Note / Tag / NoteFeed / Settings / UpdateChannel）。
  Phase 3 で bounded context を切る際の判断材料とする
