---
coherence:
  source: human
  last_validated: 2026-06-16
  upstream:
    - event-storming.md
    - discovery.md
---

# Bounded Contexts {#bounded-contexts}

PromptNotes は単一デスクトップアプリだが、責務の性質で 4 BC に分割する。
分割の根拠は Phase 2 で確定した aggregate 候補（Note / NoteFeed / Settings / UpdateChannel）と
それぞれの永続化先・ライフサイクル・差別化価値の違い。

## Note Capture {#note-capture}

### Purpose {#note-capture-purpose}

ユーザの起案・編集・タグ付け・コピー・削除/復元といった **Note の write side** を司る。
discovery の Core Domain 3 点（プロンプト・ブロックのストック / 座標ズレのない Markdown 編集 /
本文のみのクリップボード出力）はすべてこの BC に集約される。

### Subdomain Type {#note-capture-subdomain-type}

**core** — PromptNotes でなければ実現できない差別化領域。
具体的には CodeMirror `readOnly` トグルによる座標一貫性、`.md` ファイル所有性、
frontmatter/タグ除外コピーが該当する。
spec 末尾「過去の実装で AI が犯したミス」3 点もこの BC の境界で防ぐ。

### Ubiquitous Language {#note-capture-ubiquitous-language}

- **Note** — `body` + frontmatter（`tags`, `createdAt`, `updatedAt`）を持つ `.md` ファイル単位
- **Block** — UI 上の Note 1 件の表示単位（IDLE / FOCUSED / EDITING のステートマシン）
- **Draft** — フィード最上部の新規入力欄の状態（まだ `filename` を持たない）
- **AutoSave** — キー入力後 500ms debounce での永続化
- **Flush** — focus 喪失・ウィンドウ blur・アプリ quit による即時永続化
- **Tag** — Note 内の値オブジェクト（lowercase + trim 正規化、CJK 許容）
- **DeleteToTrash** — OS のゴミ箱への移動（unlink ではない）
- **UndoWindow** — トースト表示中のみ復元可能な時間窓

### Core Aggregates {#note-capture-core-aggregates}

- Note Aggregate（Tag VO を内包）

## Note Feed {#note-feed}

### Purpose {#note-feed-purpose}

Note 集合に対する **read side** を司る。検索文字列・期間・タグでの絞り込みと
`createdAt` / `updatedAt` × 昇降順での並べ替えを提供する。

### Subdomain Type {#note-feed-subdomain-type}

**supporting** — あると有用だが差別化はしない（任意のローカルノートアプリで実現可能）。
ただし起動時の挙動（フィルター揮発・ソート復元）はユーザ体験に直結するため軽視はしない。

### Ubiquitous Language {#note-feed-ubiquitous-language}

- **NoteFeed** — 表示中の Note 一覧の read model（揮発状態）
- **Query** — 検索バーの文字列。case-insensitive substring + NFC 正規化で `body` と `tags` にマッチ
- **DateRangeFilter** — プリセット（7d / 30d / 90d / all / custom）による期間絞り込み
- **TagFilter** — メタ行のタグチップクリックで発生する絞り込み
- **SortOrder** — `createdAt` | `updatedAt` × `asc` | `desc` の組合せ。**Settings に永続化**

### Core Aggregates {#note-feed-core-aggregates}

- NoteFeed（read model）
- Note Aggregate は **Shared Kernel** として Note Capture と共有（Phase 4 で context map に明示）

## User Preferences {#user-preferences}

### Purpose {#user-preferences-purpose}

アプリ起動間で永続化される個人設定（保存先・テーマ・ソート嗜好）を司る。
Note の lifecycle とは独立に変更され、Note とは別の永続化先（OS 慣習パス）に住む。

### Subdomain Type {#user-preferences-subdomain-type}

**supporting** — プロダクト固有の設定項目はあるが Core ではない。
ただし `storageDir` の変更は Note Capture の挙動を切り替えるため軽量に扱えない。

### Ubiquitous Language {#user-preferences-ubiquitous-language}

- **Settings** — `storageDir` / `theme` / `sortPreference` を持つ単一エンティティ
- **StorageDir** — Note `.md` ファイルの保存先ディレクトリ（デフォルトは OS 慣習パス）
- **Theme** — `System` | `Light` | `Dark` の 3 値
- **SortPreference** — NoteFeed のソート初期値（起動時に復元）
- **ConfigPath** — Settings 自身の永続化先（`app_config_dir/settings.json`、storageDir とは別）

### Core Aggregates {#user-preferences-core-aggregates}

- Settings Aggregate

## Update Distribution {#update-distribution}

### Purpose {#update-distribution-purpose}

新バージョンの検出と通知を司る。実装は Tauri v2 updater plugin + GitHub Releases に
外注し、PromptNotes 側のロジックは「起動時に確認する」「通知する」のみ。

### Subdomain Type {#update-distribution-subdomain-type}

**generic** — 独自に作るロジックがほぼ存在しない。
ライブラリ/プラグインに完全に委譲できる領域。

### Ubiquitous Language {#update-distribution-ubiquitous-language}

- **UpdateChannel** — Tauri updater が参照する更新元（GitHub Releases）
- **VersionNotification** — 新バージョン検出時の起動時通知

### Core Aggregates {#update-distribution-core-aggregates}

- UpdateChannel（external service の薄いラッパー）

## Notes {#notes}

- **同名異義の Note**：Note Capture では「編集対象の `.md` ファイル」、Note Feed では
  「表示用の read projection」。同じ ubiquitous language を共有するが、責務の側面が違う。
  Phase 4 で **Shared Kernel** として明示する
- **CQRS 的分割の根拠**：Note Capture は file IO / debounce / focus イベント / OS ゴミ箱 API の世界、
  Note Feed は in-memory index / NFC 正規化 / 検索文字列マッチの世界。
  統合すると「Note は write model か read model か」が曖昧化し Phase 5 で aggregate 境界がブレる
- **Settings の独立性**：Q6 の決定により Settings の永続化先は `storageDir` から独立した
  OS 慣習パス。bootstrap が単方向（Settings → storageDir → Note）になる
- **subdomain type の投資配分**：core = Note Capture のみ。ここに最も時間と注意を割く。
  supporting 2 BC は標準実装で十分、generic 1 BC はライブラリ任せ
