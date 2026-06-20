---
coherence:
  source: human
  last_validated: 2026-06-20
  upstream:
    - bounded-contexts.md
    - context-map.md
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
  - 任意の UTF-8 文字列（空文字も許容、frontmatter 由来の `---` を含まない）
- **Tag** (VO)
  - `name: String` — 正規化済み（lowercase + trim, CJK 許容）
  - 禁止文字（` `, `\t`, `\n`, `,`, `[`, `]`）を含まない
  - construction 時に禁止文字を含む入力は `TagError::InvalidChar` で reject
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
- **I-N7**: 削除（trash 移動）された Note の identity は **同一プロセス内に保持される単一の path** で復元可能
  （複数削除した古い順は復元不能、トースト消失で破棄）

### 公開操作 {#note-aggregate-operations}

#### Commands {#note-aggregate-commands}

- `Note::create(body: NoteBody, tags: TagSet, now: Timestamp) -> Note`
  - 新規 Note を生成。`id = now.format(YYYYMMDDhhmmss)`、`createdAt = updatedAt = now`
  - Cmd+Enter による確定経路の唯一の入口
- `Note::edit_body(self, new_body: NoteBody, now: Timestamp) -> Note`
  - 本文を差し替え、`updatedAt = now` に更新（I-N4）
- `Note::assign_tag(self, tag: Tag) -> Note`
  - TagSet に追加（既存なら no-op、I-N5）。`updatedAt` は **更新する**
    （tags も frontmatter 経由で永続化されるため）
- `Note::remove_tag(self, tag_name: &str) -> Note`
  - TagSet から削除。`updatedAt` を更新
- `Note::delete_to_trash(self) -> DeletedNote`
  - OS のゴミ箱に移動し、`DeletedNote { id, original_path }` を返す
  - Undo 用 in-memory state を 1 件保持（複数同時保持しない、Q5 決定）
- `DeletedNote::restore(self) -> Note`
  - OS ゴミ箱から復帰。トースト消失後の呼び出しは不可（呼び出し側で時間管理）

#### Queries {#note-aggregate-queries}

- `Note::body_for_clipboard(&self) -> String`
  - frontmatter / タグ情報を除外し `body` の文字列のみを返す（spec の core 動作）
- `Note::id(&self) -> &NoteId`
- `Note::tags(&self) -> &TagSet`
- `Note::created_at(&self) -> Timestamp`
- `Note::updated_at(&self) -> Timestamp`

## NoteFeed Aggregate {#note-feed-aggregate}

Note Feed BC の唯一の集約。read model。

### 構成要素 {#note-feed-aggregate-elements}

- **NoteFeed** (root, 揮発)
  - `source: &[Note]` — Shared Kernel 経由で Note Aggregate 群を参照
  - `filter: FeedFilter` — 揮発（起動時 reset）
  - `sort: SortOrder` — Settings から復元 / 変更で永続化
- **FeedFilter** (VO)
  - `query: Option<NormalizedQuery>` — NFC + lowercase 化済み
  - `date_range: DateRangeFilter`
  - `tag: Option<Tag>` — メタ行クリックで設定される
- **NormalizedQuery** (VO)
  - 入力文字列を NFC 正規化 + lowercase 化した結果
  - 1 文字以上の場合に保持（空文字は `None`）
- **DateRangeFilter** (VO, enum)
  - `Last7Days | Last30Days | Last90Days | All | Custom { from: Date, to: Date }`
- **SortOrder** (VO, enum)
  - `{ field: createdAt | updatedAt, direction: asc | desc }`

### ビジネス不変条件 {#note-feed-aggregate-invariants}

- **I-F1**: `query` は常に NFC 正規化済み（マッチング時に再正規化しない）
- **I-F2**: filter が空のとき、`source` 全件を sort 順で返す
- **I-F3**: `sort` の決定論性: 同一 sort key の Note は `id`（タイムスタンプ秒精度）で tiebreak
  → 安定したソート順を保証
- **I-F4**: filter の合成は AND（date_range ∧ tag ∧ query すべて満たすもの）
- **I-F5**: マッチング対象は `body` 全文 + `tags[*].name` のみ
  （Q7 決定: createdAt / updatedAt / filename は対象外）
- **I-F6**: 起動時、`filter` は常に空状態で初期化（フィルター・検索は揮発、Q3 決定）
- **I-F7**: 削除 (trash) された Note は次の visible_notes 取得から除外される

### 公開操作 {#note-feed-aggregate-operations}

#### Commands {#note-feed-aggregate-commands}

- `NoteFeed::filter_by_query(self, raw: &str) -> NoteFeed`
  - 入力を NFC + lowercase に正規化して filter.query を更新
- `NoteFeed::filter_by_date_range(self, r: DateRangeFilter) -> NoteFeed`
- `NoteFeed::filter_by_tag(self, t: Tag) -> NoteFeed`
- `NoteFeed::change_sort(self, s: SortOrder) -> NoteFeed`
  - 副作用として Settings.sort_preference を更新（Customer-Supplier 経由）
- `NoteFeed::clear_filters(self) -> NoteFeed`

#### Queries {#note-feed-aggregate-queries}

- `NoteFeed::visible_notes(&self) -> Vec<&Note>`
  - filter を適用後、sort 順に並べて返す
- `NoteFeed::count(&self) -> usize`

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
  （Q6 決定: 循環参照回避）
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

- `UpdateChannel::check_at_startup() -> Result<UpdateChannel, UpdateError>`
  - async ネットワーク呼び出し。Tauri updater plugin に委譲
  - 失敗は silent（ユーザの作業を妨げない）

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
- 復元状態は **Note Capture BC の application service** が 1 件のみ保持
- ドメインに Undo stack を持たない（Q5 決定の純粋反映）

### NoteFeed の sort 副作用 {#notes-sort-side-effect}

- `NoteFeed::change_sort` は Settings.sort_preference を更新する副作用を持つ
- これは **Customer-Supplier の唯一の逆流**（NoteFeed → Settings の書き込み）
- application service 層で「NoteFeed の状態変更」と「Settings の永続化」を 1 トランザクションで扱う

## Open Questions {#open-questions}

Phase 5 時点で未決事項はない。

- Phase 6 (domain-events) で「Note の状態変化を event として外部に通知するか」を決定
  （現状は全 BC が単一プロセスなので event は不要の見込み）
- Phase 7 (validation) で `NoteBody` の最大文字数や `Tag::name` の最大長を確定
  （現時点では制限なし）
