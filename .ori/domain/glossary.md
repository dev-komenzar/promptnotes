---
ori:
  node_id: glossary-term:collection
  type: glossary-term
  depends_on:
    - discovery:overview
    - event-storming:timeline
    - bounded-context:collection
    - context-map:map
    - aggregate:collection
    - event:collection
    - scenario:collection
---

# Glossary {#glossary}

PromptNotes のユビキタス言語を集約。
**技術用語**（repository / handler / service など）は含めない。
context 間で同じ単語が違う意味を持つ場合は最終セクション
[Cross-Context Differences](#cross-context-differences) に記載する。

## Aggregates {#glossary-aggregates}

### Note {#glossary-note}

- **定義**: ユーザが起案・編集する 1 つの `.md` ファイルに対応するドメイン単位。
  `body` + `tags` + `createdAt` + `updatedAt` + `filename` を持つ
- **context**: Note Capture（write side） / Note Feed（read side, Shared Kernel）
- **永続化**: `storageDir/YYYYMMDDhhmmss.md`
- **alias**: 「ブロック」は Note の UI 上の表示形態を指す別概念（[Block](#glossary-block) 参照）

### NoteFeed {#glossary-note-feed}

- **定義**: 表示中の Note 一覧の read model。filter と sort を持つ揮発状態
- **context**: Note Feed のみ

### Settings {#glossary-settings}

- **定義**: アプリ起動間で永続化されるユーザ個人設定
- **context**: User Preferences
- **永続化**: `app_config_dir/settings.json`（`storageDir` から独立, Q6 決定）

### UpdateChannel {#glossary-update-channel}

- **定義**: 新バージョン検出のための外部更新元（GitHub Releases）の domain 表現
- **context**: Update Distribution
- **実装**: Tauri v2 updater plugin の薄いラッパー

## Entities and Value Objects {#glossary-entities-vos}

### NoteId {#glossary-note-id}

- **定義**: Note の identity。タイムスタンプ秒精度（`YYYYMMDDhhmmss`）
- **kind**: VO
- **不変条件**: 構築後 immutable、`createdAt` と一致

### NoteBody {#glossary-note-body}

- **定義**: Note の本文（Markdown 文字列、frontmatter を除く）
- **kind**: VO
- **空文字許容**: 可（Draft 状態想定）

### Tag {#glossary-tag}

- **定義**: Note に付与される分類ラベル
- **kind**: VO
- **正規化**: lowercase + trim（CJK 文字はそのまま保持）
- **禁止文字**: 空白文字 (` `, `\t`, `\n`), `,`, `[`, `]`（YAML inline list 衝突回避）
- **context**: Note Capture / Note Feed（Shared Kernel 経由）

### TagSet {#glossary-tag-set}

- **定義**: Note 内の Tag 集合。順序保持 + 重複排除
- **kind**: VO
- **理由**: YAML inline list の表示順を保つため `Set` ではなく順序付き

### Timestamp {#glossary-timestamp}

- **定義**: 秒精度の時刻表現。ファイル名と frontmatter の両方で使用
- **kind**: VO
- **解像度**: 秒（同一秒内の連続編集では同値、S15 検証済み）

### FeedFilter {#glossary-feed-filter}

- **定義**: NoteFeed の絞り込み条件（query + date_range + tag）
- **kind**: VO
- **揮発**: アプリ起動時に空状態へリセット（Q3 決定）

### NormalizedQuery {#glossary-normalized-query}

- **定義**: 検索バー入力を NFKC (compatibility normalization) 正規化 + lowercase 化した文字列
- **kind**: VO
- **値域**: 1 文字以上のとき `Some`、空文字は `None`

### DateRangeFilter {#glossary-date-range-filter}

- **定義**: 期間絞り込み。`Last7Days | Last30Days | Last90Days | All | Custom { from, to }`
- **kind**: VO (enum)
- **UI**: ツールバーのプリセット + カスタム

### SortOrder {#glossary-sort-order}

- **定義**: 並べ替え順。`{ field: createdAt|updatedAt, direction: asc|desc }`
- **kind**: VO
- **共有**: NoteFeed と Settings の両方で使う同一型（context 間意味差は
  [Cross-Context Differences](#cross-context-differences) 参照）

### StorageDir {#glossary-storage-dir}

- **定義**: Note `.md` ファイルの保存先ディレクトリの絶対パス
- **kind**: VO
- **デフォルト**:
  - macOS: `~/Library/Application Support/promptnotes/notes/`
  - Linux: `~/.local/share/promptnotes/notes/`
  - Windows: `%APPDATA%\promptnotes\notes\`

### Theme {#glossary-theme}

- **定義**: UI テーマ。`System | Light | Dark`
- **kind**: VO (enum)

### Version {#glossary-version}

- **定義**: アプリの semver バージョン
- **kind**: VO

### Release {#glossary-release}

- **定義**: GitHub Releases の 1 リリース（version + url + notes）
- **kind**: VO

### DeletedNote {#glossary-deleted-note}

- **定義**: `Note::delete_to_trash` の戻り値。Undo 復元のための短命なハンドル
- **kind**: VO
- **保持**: application service の **Undo スタック** (`Vec<DeletedNote>`) に push
  （Q5 改訂 2026-06-20 / Phase 11a: 各 DeletedNote は対応する Toast の有効期間中のみ保持。
  独立 aggregate 化はしない方針を維持）

### BodyHash {#glossary-body-hash}

- **定義**: `NoteBody` の SHA-256 ハッシュ（hex 文字列）。外部ファイル変更との競合検出に使用
- **kind**: VO
- **導出**: Note 構築時（`Note::new` / `Note::from_persisted`）および
  `Note::edit_body` 時に body から決定論的に計算される
- **用途**: `Note::is_stale(disk_hash)` でディスク上の body との差異を判定（I-N9）
- **context**: Note Capture（競合検出） / Note Feed（変更検知時）

## Domain Events {#glossary-events}

### NoteCreated {#glossary-note-created}

- **発行者**: Note Aggregate
- **トリガー**: `Note::create` の永続化成功（Cmd+Enter 確定）

### NoteBodyEdited {#glossary-note-body-edited}

- **発行者**: Note Aggregate
- **トリガー**: `Note::edit_body` の永続化成功（AutoSave / Flush 経路）

### NoteTagsChanged {#glossary-note-tags-changed}

- **発行者**: Note Aggregate
- **トリガー**: `Note::assign_tag` / `Note::remove_tag` で TagSet が変化した時

### NoteDeletedToTrash {#glossary-note-deleted-to-trash}

- **発行者**: Note Aggregate
- **トリガー**: `Note::delete_to_trash` の OS ゴミ箱移動成功

### NoteRestoredFromTrash {#glossary-note-restored-from-trash}

- **発行者**: Note Aggregate (DeletedNote::restore)
- **トリガー**: トースト有効期間中の Undo 操作

### StorageDirChanged {#glossary-storage-dir-changed}

- **発行者**: Settings Aggregate
- **トリガー**: `Settings::change_storage_dir` の永続化成功

### ThemeChanged {#glossary-theme-changed}

- **発行者**: Settings Aggregate
- **トリガー**: `Settings::change_theme` の永続化成功

### SortPreferenceChanged {#glossary-sort-preference-changed}

- **発行者**: Settings Aggregate
- **トリガー**: `Settings::change_sort_preference` の永続化成功
  （ツールバーのソートトグル経路も含む）

### NewVersionDetected {#glossary-new-version-detected}

- **発行者**: UpdateChannel Aggregate
- **トリガー**: `UpdateChannel::check_at_startup` 成功 かつ新バージョンあり

### NoteFileCreatedExternally {#glossary-note-file-created-externally}

- **発行者**: infrastructure 層（ファイルウォッチャー）
- **トリガー**: `storage_dir` 配下に外部プログラムが新規 `.md` を作成したことを検知
- **購読者**: NoteFeed（`upsert_note`）

### NoteFileModifiedExternally {#glossary-note-file-modified-externally}

- **発行者**: infrastructure 層（ファイルウォッチャー）
- **トリガー**: `storage_dir` 配下の既存 `.md` が外部プログラムによって変更されたことを検知
- **購読者**: NoteFeed（`upsert_note`）、Note Capture（競合検出: `is_stale()` + EDITING 判定）
- **payload**: `disk_body_hash` を含み、競合検出に使用される

### NoteFileDeletedExternally {#glossary-note-file-deleted-externally}

- **発行者**: infrastructure 層（ファイルウォッチャー）
- **トリガー**: `storage_dir` 配下の `.md` が外部プログラムによって削除されたことを検知
- **購読者**: NoteFeed（`remove_note`）、Note Capture（EDITING 中の場合は通知）

## Domain Concepts {#glossary-concepts}

### Draft {#glossary-draft}

- **定義**: フィード最上部の新規入力欄の状態。まだ `NoteId` を持たない
- **遷移**: `Cmd+Enter` で確定すると Note となり [NoteCreated](#glossary-note-created) を発行
- **context**: Note Capture

### Block {#glossary-block}

- **定義**: フィード上の Note 1 件の UI 表示単位。3 値のステートマシンを持つ
- **states**: `IDLE | FOCUSED | EDITING`
- **重要**: Block は Note の UI 表現であり、ドメインモデルではない
  （domain event の対象外）
- **context**: UI 層（厳密にはドメイン外だが、ubiquitous language として保持）

### AutoSave {#glossary-autosave}

- **定義**: キー入力後 500ms debounce による Note 本文の自動永続化
- **発行 event**: [NoteBodyEdited](#glossary-note-body-edited)
- **context**: Note Capture

### Flush {#glossary-flush}

- **定義**: debounce を待たず即時永続化を行う動作
- **トリガー**: (1) ブロックからの focus 喪失 (2) ウィンドウ blur (3) アプリ quit（Q4 決定）
- **発行 event**: [NoteBodyEdited](#glossary-note-body-edited)
- **context**: Note Capture

### UndoWindow {#glossary-undo-window}

- **定義**: Note 削除後に **その削除に対応する Toast** が表示されている間の Undo 有効期間
- **長さ**: 仮 5 秒（Toast ごとに独立、UI Phase で確定）
- **特性**: 該当 Toast 消失と同時に対応する [DeletedNote](#glossary-deleted-note) が
  Undo スタックから除去され、その Note のみ復元不能（他の DeletedNote は影響を受けない）
- **context**: Note Capture

### DeleteToTrash {#glossary-delete-to-trash}

- **定義**: Note を OS のゴミ箱へ移動する操作（`unlink` ではない）
- **発行 event**: [NoteDeletedToTrash](#glossary-note-deleted-to-trash)
- **context**: Note Capture

### Shared Kernel (Note) {#glossary-shared-kernel}

- **定義**: Note Capture と Note Feed が共有する Note Aggregate の型定義
- **管理**: 同一 Rust crate に置く（`domain::note::Note`）
- **変更ルール**: 構造変更 PR は両 BC の aggregates.md を同時更新
- **context**: Note Capture ↔ Note Feed

### Customer-Supplier {#glossary-customer-supplier}

- **定義**: Settings が上流、Note Capture / Note Feed が下流の片方向依存
- **連携**: 起動時 DI 注入（`storageDir` / `sortPreference`）
- **context**: User Preferences → Note Capture / Note Feed

### ConfigPath {#glossary-config-path}

- **定義**: Settings 自身の永続化先（`app_config_dir/settings.json`）
- **特性**: `storageDir` から独立（Q6 決定: 循環参照回避）
- **context**: User Preferences

### Conformist + ACL {#glossary-conformist-acl}

- **定義**: GitHub Releases API のスキーマに Update Distribution が従う関係。
  Tauri updater plugin が ACL を兼ねる
- **context**: GitHub Releases → Update Distribution

### 外部ファイル変更 (External File Change) {#glossary-external-file-change}

- **定義**: PromptNotes 以外のプログラム（vim, VSCode, Syncthing 等）が
  `storage_dir/*.md` を変更すること。ファイル作成・変更・削除の 3 種がある
- **検出**: OS レベルのファイルウォッチャーが検知し、
  [NoteFileCreatedExternally](#glossary-note-file-created-externally) /
  [NoteFileModifiedExternally](#glossary-note-file-modified-externally) /
  [NoteFileDeletedExternally](#glossary-note-file-deleted-externally)
  のいずれかの domain event に変換される
- **反映**: NoteFeed が `upsert_note` / `remove_note` で差分更新（I-F8）
- **context**: Note Feed / Note Capture
- **ユースケース**: Syncthing による複数デバイス間のノート同期

### ファイルウォッチャー (File Watcher) {#glossary-file-watcher}

- **定義**: OS ネイティブのファイルシステム監視機構（Linux: inotify, macOS: FSEvents）を
  用いて `storage_dir/*.md` の変更を検知する infrastructure コンポーネント
- **実装**: Rust `notify` crate
- **ライフサイクル**: アプリ起動時に開始、quit 時に停止。
  `StorageDirChanged` event 購読時に監視ディレクトリを切り替え
- **debounce**: 500ms（同一ファイルへの連続変更を 1 イベントに集約）
- **context**: infrastructure 層（domain 概念としては
  [detect-external-changes](#glossary-detect-external-changes) workflow が表現）

### 競合 (Conflict) {#glossary-conflict}

- **定義**: ユーザが編集中（EDITING 状態）の Note の `.md` ファイルが
  外部プログラムによって変更された状態
- **判定**: Block が EDITING 状態 かつ `Note::is_stale(disk_body_hash)` = true（I-N9）
- **解決**: 競合ダイアログを表示し、ユーザに「外部変更を適用」か「編集中を保持」を選択させる（S19）
- **context**: Note Capture / UI 層

### 差分更新 (Delta Update) {#glossary-delta-update}

- **定義**: 外部ファイル変更検知時に、`NoteFeed.source` 全体を再読み込みするのではなく、
  変更された Note のみを `upsert_note` / `remove_note` で部分更新すること（I-F8）
- **対義語**: 全件ハイドレート（`hydrate` — 起動時・手動 Refresh で使用）
- **context**: Note Feed

### upsert_note {#glossary-upsert-note}

- **定義**: `NoteFeed` の操作。`source` 内の指定 `note.id` と一致する要素があれば
  置換、なければ末尾に追加する insert-or-update（I-F8）
- **対になる操作**: `remove_note`（外部削除検知時に使用）
- **context**: Note Feed

### detect-external-changes {#glossary-detect-external-changes}

- **定義**: ファイルウォッチャーが検知した raw OS イベントを
  domain event（`NoteFileCreated/Modified/DeletedExternally`）に変換する workflow
- **文書**: `workflows/detect-external-changes.md`
- **context**: Note Feed（infrastructure 層との橋渡し）

### Syncthing {#glossary-syncthing}

- **定義**: P2P ファイル同期ツール。PromptNotes の `storage_dir` を複数デバイス間で
  同期するためにユーザが外部で運用する
- **PromptNotes の責務**: Syncthing の管理・設定は行わない。
  `.md` ファイルの変更を検知して UI に反映するのみ
- **context**: 外部ツール（domain 外だがユースケースの理解に必要）

## Cross-Context Differences {#cross-context-differences}

同じ単語が文脈で異なる意味を持つもの。Shared Kernel 採用により多くは「型は同じ
だが責務が違う」形になる。

| 用語 | Note Capture | Note Feed | User Preferences |
|--|--|--|--|
| **Note** | 編集対象の `.md` ファイル（write side、永続化責務） | 表示用の read projection（順序・絞り込み責務） | （扱わない） |
| **SortOrder** | （扱わない） | 現在の表示順（揮発、即時反映対象） | 起動時復元値（永続化対象） |
| **Tag** | 入力・正規化・付与/削除の対象（write side） | TagFilter の値（read 側、検索キー） | （扱わない） |

### Note の同名異義 詳細 {#cross-context-note-diff}

- Note Capture: ファイル IO / debounce / focus イベント / OS ゴミ箱 API の世界。
  操作の主体（command を受け取る側）
- Note Feed: in-memory index / NFKC (compatibility normalization) 正規化 / 検索文字列マッチの世界。
  検索・並べ替えの対象（query を受ける側）
- 同じ型を共有しても、責務の側面が違うため両 BC の独立性は保たれる
  （Phase 4 で Shared Kernel として明示）

### SortOrder の同名異義 詳細 {#cross-context-sort-order-diff}

- Note Feed の SortOrder: ユーザがツールバーで切り替える「いまの並び順」。
  変更すると即座に表示順が変わる（揮発）
- Settings の sort_preference: アプリ起動時に NoteFeed の初期 SortOrder に
  読み込まれる「保存された嗜好」（永続化）
- NoteFeed.change_sort は Settings.sort_preference を更新する副作用を持つ
  （aggregates.md の "NoteFeed の sort 副作用" 参照）

## Notes {#notes}

### 用語選定の方針 {#notes-selection-policy}

- **含めるもの**: aggregate / VO / event / 業務概念（AutoSave, Flush, Draft 等）
- **含めないもの**:
  - 技術用語（repository, handler, channel, observer など）
  - フレームワーク固有名（Tauri, CodeMirror, serde 等は明示が必要な箇所のみ補足）
  - UI 部品名（toolbar, modal 等は ui-fields/ で扱う）
- **例外**: [Block](#glossary-block) は UI 概念だが spec の ubiquitous language として保持

### 多言語 (日英) 表記 {#notes-bilingual}

- 主用語は **英語名 + 日本語訳**を併記（例：AutoSave = 自動保存）
- 文書本文では英語名を優先（aggregate 名・event 名と統一）
- 検索性のため定義文には日本語説明を必ず含める

### Open Questions の取り扱い {#notes-open-questions}

Phase 1-7 で生じた未決事項はすべて解決済み（Q1〜Q7）。
Phase 5/6 改訂（外部ファイル変更検知）で追加された用語を本 glossary に追記済み。

## Open Questions {#open-questions}

Phase 8 改訂（外部ファイル変更検知関連用語の追加に伴う）:

- 未決事項なし。Phase 10 (types) で BodyHash, upsert_note 等の具体的な型定義を行う
