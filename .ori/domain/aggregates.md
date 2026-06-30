---
ori:
  node_id: aggregate:collection
  type: aggregate
  depends_on:
    - bounded-context:collection
    - context-map:map
---

# Aggregates {#aggregates}

PromptNotes の 4 BC に対応する 4 集約を定義する。
Note Aggregate は Note Capture / Note Feed の Shared Kernel として両 BC に登場する
（Phase 4 決定）。

## Note Aggregate {#note-aggregate}

Note Capture BC の core aggregate。Note Feed BC からも Shared Kernel として参照される。

### 構成要素 {#note-aggregate-elements}

- **Note** (root entity)
  - `id: NoteId` — タイムスタンプ秒精度 (`YYYYMMDDhhmmss`)、`.md` 拡張子を除いた basename
  - `body: NoteBody` — Markdown 本文（frontmatter を含まない）
  - `tags: TagSet` — 正規化済み Tag の集合（順序保持、重複なし）
  - `createdAt: Timestamp` — 作成時刻（秒精度、`id` と一致）
  - `updatedAt: Timestamp` — 最終編集時刻（秒精度）
- **NoteId** (VO)
  - 文字列表現は `^\d{14}$`（`YYYYMMDDhhmmss`）
  - identity の唯一の源（filename と 1:1）
- **NoteBody** (VO)
  - 任意の UTF-8 文字列（空文字も許容）
  - **不変条件 (I-N8)**: frontmatter 由来の delimiter 行（行全体が `---`、末尾空白許容）を含まない
  - **construction**: `NoteBody::new(raw: String) -> Result<NoteBody, NoteBodyError>` の smart constructor で I-N8 を enforce。`NoteBodyError::ContainsFrontmatterDelimiter` で表面化
- **BodyHash** (VO)
  - `body` の SHA-256 ハッシュ（hex 文字列）
  - Note 構築時に `NoteBody` から計算され、以降は `body` 変更時に再計算される
  - 外部変更との競合検出に使用（I-N9）
- **Tag** (VO)
  - `name: String` — 正規化済み（lowercase + trim, CJK 許容）
  - 禁止文字（` `, `\t`, `\n`, `,`, `[`, `]`）を含まない
  - construction 時に **trim 後の空文字** は `TagError::Empty` で、**禁止文字を含む入力** は `TagError::InvalidChar { raw }` で reject
- **TagSet** (VO)
  - `Vec<Tag>` の薄いラッパー
  - 順序を保持（YAML inline list の表示順を保つ）
  - 同一 `name` の重複を構築時に排除（後勝ち / 先勝ちは「先勝ち = 既存順序維持」）
- **Timestamp** (VO)
  - 秒精度の `OffsetDateTime`（ファイル名と frontmatter の表現フォーマット差を吸収）

### ビジネス不変条件 {#note-aggregate-invariants}

- **I-N1**: `id` は immutable。`Note::new` 後に書き換え不可
- **I-N2**: `createdAt` は `id` の文字列パース結果と一致する
- **I-N3**: `updatedAt >= createdAt` を常に満たす
- **I-N4**: `body` を変更する操作は `updatedAt` を「現在時刻 (秒精度)」に更新する
  - 同一秒内の連続編集では `updatedAt` は同じ値に留まる（時計の解像度で十分）
- **I-N5**: `tags` 内に同一 `Tag::name` は 1 件のみ
- **I-N6**: `Tag::name` は正規化規則（lowercase + trim、禁止文字排除）を必ず満たす
- **I-N7**: 削除（trash 移動）された Note の identity は application service の
  **DeletedNote スタック** (`Vec<DeletedNote>`) に push され、各 DeletedNote は
  対応する Toast の有効期間中のみ復元可能。Toast 消失でその要素のみスタックから除去
  （各 Toast / DeletedNote は独立した有効期間を持ち、互いに干渉しない）
- **I-N8**: `body` の構築は `NoteBody::new` 経由でのみ可能であり、frontmatter
  delimiter 行 (`---`、末尾空白許容) を含まない。永続化フォーマット (`.md` ファイルの
  YAML frontmatter) との分離を construction-time に保証する不変条件
- **I-N9**: `body_hash` は `body` から決定論的に導出され、`body` 変更時に必ず再計算される。
  外部プログラム（vim, VSCode, Syncthing 等）が `.md` ファイルを変更した場合、
  application service 層がディスクから読み込んだ `body` のハッシュと
  メモリ上の `body_hash` を比較することで競合を検出できる。
  Note Aggregate 自身は `is_stale(disk_hash)` クエリを提供するのみで、
  競合解決の判断は application service 層（ユーザへの選択肢提示）に委ねる

### 公開操作 {#note-aggregate-operations}

#### Commands {#note-aggregate-commands}

- `Note::create(body: NoteBody, tags: TagSet, now: Timestamp) -> Note`
  - 新規 Note を生成。`id = now.format(YYYYMMDDhhmmss)`、`createdAt = updatedAt = now`
  - Cmd+Enter による確定経路の唯一の入口
- `Note::from_persisted(body: NoteBody, tags: TagSet, created_at: Timestamp, updated_at: Timestamp) -> Note`
  - 永続化済 Note の再構築（`NoteRepository::load_by_id` 経由のみ）
  - `id = NoteId::from_timestamp(created_at)` で I-N2 を construction-time に保証
  - 呼び出し側は `.md` ファイルの YAML frontmatter から各 field を解放してから渡す
  - 再構築失敗（malformed frontmatter / missing key 等）は port (`NoteRepository::load_by_id`) 側で
    `io::ErrorKind::InvalidData` として表面化（aggregate には到達しない）
- `Note::edit_body(self, new_body: NoteBody, now: Timestamp) -> Note`
  - 本文を差し替え、`updatedAt = now` に更新（I-N4）
- `Note::assign_tag(self, tag: Tag, now: Timestamp) -> Note`
  - TagSet に追加（既存なら no-op、I-N5）。新規追加時は `updatedAt = now`（同一 `name` の no-op 経路では updatedAt も据え置き — 永続化の必要が無いため）
- `Note::remove_tag(self, tag_name: &str, now: Timestamp) -> Note`
  - TagSet から削除。削除があった場合は `updatedAt = now`
- `Note::delete_to_trash(self) -> DeletedNote`
  - OS のゴミ箱に移動し、`DeletedNote { id, original_path }` を返す
  - 戻り値は application service の DeletedNote スタックに **push** される
    （複数の DeletedNote を同時保持、Phase 11a UI 設計改訂による）
- `DeletedNote::restore(self) -> Note`
  - OS ゴミ箱から復帰。対応する Toast の有効期間外の呼び出しは不可
    （呼び出し側 application service が DeletedNote ごとの有効期間を管理）

#### Queries {#note-aggregate-queries}

- `Note::body_for_clipboard(&self) -> String`
  - frontmatter / タグ情報を除外し `body` の文字列のみを返す（spec の core 動作）
- `Note::id(&self) -> &NoteId`
- `Note::tags(&self) -> &TagSet`
- `Note::created_at(&self) -> Timestamp`
- `Note::updated_at(&self) -> Timestamp`
- `Note::body_hash(&self) -> &BodyHash`
  - 現在の `body` から計算された SHA-256 ハッシュ（外部変更検出用）
- `Note::is_stale(&self, disk_hash: &BodyHash) -> bool`
  - `self.body_hash() != disk_hash` のとき `true`（I-N9 の判定に使用）
  - `disk_hash` は application service 層がディスクから読み込んだ `.md` ファイルの
    body 部分から計算したハッシュ

## NoteFeed Aggregate {#note-feed-aggregate}

Note Feed BC の唯一の集約。read model。

### 構成要素 {#note-feed-aggregate-elements}

- **NoteFeed** (root, 揮発)
  - `source: Vec<Note>` — Shared Kernel 経由で Note Aggregate 群を一括 hydration
    (workflow:list-feed が `NoteRepository::list_all` で `storage_dir/*.md` から構築)。
    `Vec<NoteId>` ではなく `Vec<Note>` を保持する根拠は workflows/list-feed.md#notes
  - `filter: FeedFilter` — 揮発（起動時 reset）
  - `sort: SortOrder` — Settings から復元 / 変更で永続化
- **FeedFilter** (VO)
  - `query: Option<NormalizedQuery>` — NFKC (compatibility normalization) + lowercase 化済み
  - `date_range: DateRangeFilter`
  - `tag: Option<Tag>` — メタ行クリックで設定される
- **NormalizedQuery** (VO)
  - 入力文字列を NFKC (compatibility normalization) 正規化 + lowercase 化した結果
  - 1 文字以上の場合に保持（空文字は `None`）
- **DateRangeFilter** (VO, enum)
  - `Last7Days | Last30Days | Last90Days | All | Custom { from: Date, to: Date }`
- **SortOrder** (VO, enum)
  - `{ field: createdAt | updatedAt, direction: asc | desc }`

### ビジネス不変条件 {#note-feed-aggregate-invariants}

- **I-F1**: `query` は常に **NFKC 正規化済み + lowercase 済み**（マッチング時に再正規化しない）。NFKC を使う理由: 全角 Latin / 半角 Latin、半角カナ / 全角カナ等の互換等価文字を同一視するため。canonical decomposition のみの NFC では半角化が起きず、S8 シナリオが成立しない
- **I-F2**: filter が空のとき、`source` 全件を sort 順で返す
- **I-F3**: `sort` の決定論性: 同一 sort key の Note は `id`（タイムスタンプ秒精度）で tiebreak
  → 安定したソート順を保証
- **I-F4**: filter の合成は AND（date_range ∧ tag ∧ query すべて満たすもの）
- **I-F5**: マッチング対象は `body` 全文 + `tags[*].name` のみ
  （Q7 決定: createdAt / updatedAt / filename は対象外）
- **I-F6**: 起動時、`filter` は常に空状態で初期化（フィルター・検索は揮発、Q3 決定）
- **I-F7**: 削除 (trash) された Note は次の visible_notes 取得から除外される
- **I-F8**: NoteFeed は外部ファイル変更の検知を契機とした差分更新を受け付ける。
  `upsert_note` 操作により、変更された `.md` ファイルに対応する Note のみを
  部分更新できる。ファイル削除 / 新規作成も upsert または後続の remove で反映する。
  検知機構そのもの（OS レベルのファイルウォッチャー）は infrastructure 層の責務

### 公開操作 {#note-feed-aggregate-operations}

#### Commands {#note-feed-aggregate-commands}

- `NoteFeed::filter_by_query(self, raw: &str) -> NoteFeed`
  - 入力を NFKC (compatibility normalization) + lowercase に正規化して filter.query を更新
- `NoteFeed::filter_by_date_range(self, r: DateRangeFilter) -> NoteFeed`
- `NoteFeed::filter_by_tag(self, t: Tag) -> NoteFeed`
- `NoteFeed::change_sort(self, s: SortOrder) -> NoteFeed`
  - 副作用として Settings.sort_preference を更新（Customer-Supplier 経由）
- `NoteFeed::clear_filters(self) -> NoteFeed`
- `NoteFeed::hydrate(self, notes: Vec<Note>) -> NoteFeed`
  - `source` を**全件差し替え**る (workflow:list-feed の `hydrateFeedSource` ステップ)。
    起動時 + 手動 Refresh + `storage_dir` 変更時 に使用する pure 関数。
    外部変更検知による差分更新には `upsert_note` / `remove_note` を使用する（I-F8）
- `NoteFeed::upsert_note(self, note: Note) -> NoteFeed`
  - `source` 内の `note.id` と一致する要素があれば置換、なければ末尾に追加（I-F8）
  - 変更後も現在の filter / sort は維持される（`visible_notes` の結果が変わる可能性はある）
- `NoteFeed::remove_note(self, note_id: &NoteId) -> NoteFeed`
  - `source` から `note_id` に一致する要素を削除。外部削除の検知時に使用（I-F8）
  - 該当する要素がない場合は no-op

#### Queries {#note-feed-aggregate-queries}

- `NoteFeed::visible_notes(&self) -> Vec<&Note>`
  - filter を適用後、sort 順に並べて返す (workflow:list-feed の `applyFilter` + `applySort`)
- `NoteFeed::count(&self) -> usize`
- `NoteFeed::source(&self) -> &[Note]` — hydration 結果の確認用 read accessor

## Settings Aggregate {#settings-aggregate}

User Preferences BC の唯一の集約。

### 構成要素 {#settings-aggregate-elements}

- **Settings** (root entity)
  - `storage_dir: StorageDir` — Note の `.md` 保存先
  - `theme: Theme` — UI テーマ
  - `sort_preference: SortOrder` — NoteFeed の初期 sort 順（Q3 決定）
- **StorageDir** (VO)
  - `PathBuf` の薄いラッパー
  - 構築時に絶対パスへの正規化を行う
  - 実在ディレクトリでなくてもよい（初回起動時の自動作成を許容）
- **Theme** (VO, enum)
  - `System | Light | Dark`
- **SortOrder** (VO)
  - NoteFeed と共有する VO（型は 1 つ）。Shared Kernel に近い扱いだが Settings から
    NoteFeed への一方向供給なので Customer-Supplier の範疇

### ビジネス不変条件 {#settings-aggregate-invariants}

- **I-S1**: `storage_dir` は絶対パス
- **I-S2**: Settings の永続化先 (`app_config_dir/settings.json`) は `storage_dir` 配下にしない
  （Q6 決定: 循環参照回避）。判定方向: `config_path.starts_with(storage_dir)` を違反とみなす
  （sibling layout `Application Support/promptnotes/{settings.json, notes/}` は許容）。
  **port-level 契約**: OS 慣習パスを返す port (例: `OsDirs::default_storage_dir`) は、
  返す `StorageDir` が任意の妥当な `config_path` (`app_config_dir` 配下) に対して I-S2 を満たすことを契約として保証する責務を負う
  （load-settings slice 側で defensive re-check はしない）
- **I-S3**: 不在時のデフォルト
  - `storage_dir`: OS 慣習パス（macOS `~/Library/Application Support/promptnotes/notes/`,
    Linux `~/.local/share/promptnotes/notes/`, Windows `%APPDATA%\promptnotes\notes\`）
  - `theme`: `System`
  - `sort_preference`: `{ field: createdAt, direction: desc }`
- **I-S4**: `change_storage_dir` 操作は即時には Note の引っ越しを起こさない
  （再起動を要求する想定。Phase 9 workflow で確認）

### 公開操作 {#settings-aggregate-operations}

#### Commands {#settings-aggregate-commands}

- `Settings::load_or_default(config_path: &Path) -> Settings`
  - JSON を読む。不在 / parse 失敗時はデフォルト
- `Settings::change_storage_dir(self, new_dir: StorageDir) -> Settings`
- `Settings::change_theme(self, new_theme: Theme) -> Settings`
- `Settings::change_sort_preference(self, new_sort: SortOrder) -> Settings`
- `Settings::persist(&self, config_path: &Path) -> Result<()>`
  - serde で JSON 書き出し

#### Queries {#settings-aggregate-queries}

- `Settings::storage_dir(&self) -> &StorageDir`
- `Settings::theme(&self) -> Theme`
- `Settings::sort_preference(&self) -> SortOrder`

## UpdateChannel Aggregate {#update-channel-aggregate}

Update Distribution BC の唯一の集約。Tauri v2 updater plugin の薄いラッパー。

### 構成要素 {#update-channel-aggregate-elements}

- **UpdateChannel** (root entity, 揮発)
  - `current_version: Version` — ビルド時に埋め込まれる
  - `latest_release: Option<Release>` — 起動時チェックの結果
- **Version** (VO)
  - semver 文字列（例: `0.3.1`）
  - 比較順序を持つ
- **Release** (VO)
  - `version: Version`
  - `url: Url`（GitHub Releases page）
  - `notes: String`（リリースノート Markdown）

### ビジネス不変条件 {#update-channel-aggregate-invariants}

- **I-U1**: `current_version` は immutable（ビルド時定数）
- **I-U2**: `latest_release` が `Some` のとき、`latest_release.version > current_version`
  を満たす（同一 / 古いリリースは `None` に正規化）
- **I-U3**: 確認は **アプリ起動時 1 回のみ**。常駐 polling はしない
  （spec の core 動作: 通知のみ）

### 公開操作 {#update-channel-aggregate-operations}

#### Commands {#update-channel-aggregate-commands}

- `UpdateChannel::check_at_startup() -> UpdateChannel`
  - async ネットワーク呼び出し。Tauri updater plugin に委譲
  - 失敗は **application service 内部で silent に握り潰し**、`latest_release: None` の
    `UpdateChannel` を返す（S14 / I-U2 / `workflows/check-for-updates.md#error-handling`）。
  - 内部実装は `Result<UpdateChannel, UpdateError>` を持つ private fn を経由してよいが、
    外部 API は **`Result` を露出しない**。

#### Queries {#update-channel-aggregate-queries}

- `UpdateChannel::has_new_version(&self) -> bool`
- `UpdateChannel::latest_release(&self) -> Option<&Release>`

## Notes {#notes}

### Note Aggregate と NoteFeed Aggregate の Shared Kernel 運用 {#notes-shared-kernel}

- 同一の `Note` 型を Rust の単一 crate に置く（`domain::note::Note`）
- Note Capture と Note Feed の両方が `&Note` / `Note` を直接持つ
- Note 構造を変える PR は両 BC の operations を同時更新する義務（Phase 4 決定）

### TagSet vs Vec\<Tag\> {#notes-tagset}

- spec の frontmatter `tags: [gpt, coding]` は順序を持つ list だが、
  ドメインモデルとしては「集合（重複なし）」が自然
- 妥協点として **順序保持 + 重複排除** の TagSet を採用
- 永続化時は YAML inline list（順序通り）に書き戻す

### 削除 Undo の集約境界 {#notes-undo}

- `DeletedNote` を独立 entity にせず、Note Aggregate の operation の戻り値とする
  （VO 的扱い: identity を持たない短命なハンドル）
- 復元状態は **Note Capture BC の application service** が **スタック**
  (`Vec<DeletedNote>`) として保持（Phase 11a UI 設計改訂による Q5 改定）
- 各 DeletedNote は対応する Toast と 1:1 対応し、独立した有効期間 (TTL) を持つ
- Toast 消失 / Undo 成功 / 明示クローズ のいずれかで該当 DeletedNote のみスタックから除去
- ドメイン本体 (Note Aggregate) に Undo stack を持たない方針は維持
  （Undo stack は application service 層の責務）

### NoteFeed の sort 副作用 {#notes-sort-side-effect}

- `NoteFeed::change_sort` は Settings.sort_preference を更新する副作用を持つ
- これは **Customer-Supplier の唯一の逆流**（NoteFeed → Settings の書き込み）
- application service 層で「NoteFeed の状態変更」と「Settings の永続化」を 1 トランザクションで扱う

## Open Questions {#open-questions}

Phase 5 改訂（外部ファイル変更検知の Core Domain 追加に伴う）:

- `BodyHash` は `body` から決定論的に導出される派生値であり、`from_persisted`
  経由の復元時にも `body` から正しく計算される。読み込み時のレースコンディション
  （読み込み直後にディスクが変更されるケース）は後続の `NoteFileModifiedExternally`
  で検出されるため、`from_persisted` 内での追加検証は不要（I-N9）
- `upsert_note` による新規 Note 挿入時、`createdAt` は frontmatter の `created_at` を
  優先する。frontmatter 不在時のみファイル mtime を使用。矛盾時も frontmatter を信頼
- ファイルウォッチャーの起動・停止ライフサイクルは `detect-external-changes` workflow に記述済み（Phase 9）

未決事項なし。Phase 7 (validation) で競合検出シナリオの検証、Phase 10 (types) で
`BodyHash` / `ConflictState` の型定義を行う。
