---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s12-startup-state
      hash: 4a02c17cc025
    - path: domain/workflows/load-settings.md#load-settings
      hash: 0c241b32210d
    - path: domain/workflows/list-feed.md#list-feed
      hash: b05c95558e97
    - path: domain/workflows/change-sort-order.md#change-sort-order
      hash: 137e642d209c
    - path: domain/workflows/update-feed-filter.md#update-feed-filter
      hash: 51cdf06a33f9
    - path: domain/aggregates.md#settings-aggregate
      hash: 56f7a54a8ab2
    - path: domain/aggregates.md#note-feed-aggregate
      hash: 56f7a54a8ab2
    - path: domain/domain-events.md#sort-preference-changed
      hash: 71db66eafe03
    - path: domain/ui-fields/screen-1.md#screen-1
      hash: 4bc0f83f71f3
---

# s12-startup-state — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s12-startup-state phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

アプリ再起動時の NoteFeed 初期化を検証する scenario。**filter（query / date_range / tag）は揮発**
で起動時は常に空、**sort のみ Settings から復元**される（Q3 決定）。`load-settings` workflow が
`settings.json` から `Settings` を読み、`Settings.sort_preference` を返す。`list-feed` workflow が
`storage_dir/*.md` を hydrate してその sort を適用し、全 Note を `updatedAt` 昇順で表示する。

- 対応 workflow: `domain/workflows/load-settings.md#load-settings`（起動時に `settings.json` を読み、
  不在 / 部分破損は field 単位 fallback。NoteFeed 初期化はその後段）
- 対応 workflow: `domain/workflows/list-feed.md#list-feed`（`list_all` → `hydrate` → `applyFilter` →
  `applySort` → `projectVisibleNotes`。起動時トリガーの read pipeline）
- 対応 workflow: `domain/workflows/change-sort-order.md#change-sort-order`（sort は
  `Settings.sort_preference` として永続化され、起動時に復元される）
- 対応 workflow: `domain/workflows/update-feed-filter.md#update-feed-filter`（filter は揮発。起動時は
  ClearAll 相当 = `query=None, date_range=All, tag=None` で初期化）
- 対応 aggregate: `domain/aggregates.md#settings-aggregate`（`sort_preference: SortOrder`、I-S3 default
  は `{ createdAt, desc }`）
- 対応 aggregate: `domain/aggregates.md#note-feed-aggregate`（`filter` は揮発・起動時 reset、
  `sort` は Settings から復元。I-F2 / I-F3 / I-F6）
- 対応 event: `domain/domain-events.md#sort-preference-changed`（起動復元では発行されない。sort 変更の
  永続化経路のみが発行する）
- 対応 page: `page-main`（`screen-1` の toolbar が起動時の filter / sort 状態を表示）

> domain/validation.md#s12-startup-state より:

- Given: 前回終了時 — 検索バー `"gpt"`、TagFilter `coding`、SortOrder `{ updatedAt, asc }`、
  Settings 永続化済み
- When: アプリ再起動
- Then:
  - `Settings::load_or_default` で `sort_preference = { updatedAt, asc }` を取得
  - NoteFeed 初期化:
    - filter は **空**（query=None, date_range=All, tag=None）— Q3 決定: 揮発
    - sort = `{ updatedAt, asc }` — Settings から復元
  - 全 Note が updatedAt 昇順で表示

> domain/workflows/load-settings.md#load-settings より:
> 「NoteFeed の初期化はこの workflow の **後段** で実行される。filter は常に空（S12）、sort は
> `Settings.sort_preference` から復元」

> domain/workflows/list-feed.md#list-feed より:
> 「**アプリ起動時**: load-settings 完了後に 1 回呼ぶ (S12 と整合)。全件 hydrate」

> domain/workflows/update-feed-filter.md#notes より:
> 「起動時は ClearAll 相当の状態で初期化（Q3 決定、S12）」

> domain/aggregates.md#note-feed-aggregate-invariants より:
> 「**I-F6**: 起動時、`filter` は常に空状態で初期化（フィルター・検索は揮発、Q3 決定）」
> 「**I-F2**: filter が空のとき、`source` 全件を sort 順で返す」
> 「**I-F3**: `sort` の決定論性: 同一 sort key の Note は `id`（タイムスタンプ秒精度）で tiebreak」

> domain/aggregates.md#settings-aggregate-invariants より:
> 「**I-S3**: 不在時のデフォルト — `sort_preference`: `{ field: createdAt, direction: desc }`」

> domain/ui-fields/screen-1.md#cross-sort-immediate より:
> 「sort-field または sort-direction の変更 → NoteFeed.change_sort 即時呼び出し +
> Settings.sort_preference 永続化」

## シナリオステップ {#scenario-steps}

### Given {#given}

1. `settings.json` に `sort_preference = { field: updated_at, direction: asc }` が永続化済み
   （`XDG_CONFIG_HOME` 配下の隔離 config dir に seed 済み。S12 の「前回終了時 Settings 永続化済み」を
   この seed で再現する）
2. 前回セッションで filter（検索バー `"gpt"` / TagFilter `coding`）が設定されていたが、filter は
   **永続化されない**（揮発。Q3 / I-F6）ため `settings.json` には現れない
3. `storage_dir` に Note 3 件が seed されている。`updatedAt` 昇順が `createdAt` 順（および I-S3 default
   の `createdAt desc`）と異なるように、createdAt と updatedAt をずらしてある
4. Tauri App が起動済み

### When {#when}

1. アプリが起動し、`load-settings` → `list-feed` の初期化パイプラインが走る

### Then {#then}

- `Settings::load_or_default`（実 Rust では `LoadSettingsUseCase::execute`）で
  `sort_preference = { updated_at, asc }` を取得する
- NoteFeed 初期化:
  - filter は **空**（`query=None, date_range=All, tag=None`）— Q3 / I-F6。前回セッションの
    `"gpt"` / `coding` は持ち越されない
  - sort = `{ updated_at, asc }` — Settings から復元
- filter が空のため全 Note が対象になり（I-F2）、`updatedAt` 昇順で表示される
- `SortPreferenceChanged` は発行されない（起動時の復元は event を伴わない）

## テスト観点 {#test-points}

- **起動時 sort 復元（本 scenario の核）**: `settings.json` の `sort_preference = { updated_at, asc }`
  が `load_settings` 経由で feedStore に反映され、`[data-block-id]` の並びが `updatedAt` 昇順に
  なる（I-S3 default の `created_at desc` や `created_at asc` とは異なる順序で並べた Note を seed し、
  空振りでないことを保証する）
- **起動時 filter 空（Q3 / I-F6）**: 検索バー `screen-1-toolbar-search-query` の value が空、
  DateRange が `All`（`screen-1-toolbar-date-range-all` が `aria-pressed=true`）、
  タグチップ `screen-1-toolbar-tag-chip` が不在
- **toolbar の起動時状態**: `screen-1-toolbar-sort-field-updated_at` が active、sort 方向 toggle の
  `aria-label` が `Ascending`
- **Settings の永続状態**: `settings.json` の `storage_dir` と `sort_preference` が seed 値のまま
  であること（起動が settings.json を書き換えない）
- **filter 非永続（対照）**: `settings.json` に filter 相当のキーが存在しない（`query` / `tag` /
  `date_range` を持たない）こと。filter が揮発である設計を裏付ける

## 実装ノート {#impl-notes}

- **runner**: `wdio`（優先チェーン step 2 — manifest に `runner:` 明示なし。参加 app `promptnotes` は
  `local` 系で `runtime.runner = wdio`。`.ori/architecture.md` の `scenario_test_runner: wdio` とも一致）
- **infrastructure**: `promptnotes` のみ（`mode: local`）。compose-service 系 app が参加しないため
  docker-compose は不要
- **runner config**: `wdio.conf.ts` が `/ori-generate` により生成される
- **config dir の隔離**: `onPrepare` が `XDG_CONFIG_HOME` / `XDG_DATA_HOME` を temp dir に設定する。
  Tauri の `app_config_dir()` は `$XDG_CONFIG_HOME/<identifier>`（identifier =
  `com.komenzar.promptnotes`）に解決されるため、`<temp>/config/com.komenzar.promptnotes/settings.json`
  を `sort_preference = { field: updated_at, direction: asc }` で seed すれば Given の Settings を
  再現できる
- **Note の seed**: 3 件を `onPrepare` が書き込む。`updatedAt` 昇順 = `createdAt` 昇順 でも
  `createdAt` 降順（I-S3 default）でもない並びになるよう createdAt / updatedAt を設計する:
  - `20260101100000` "alpha"（created 01-01 / updated 06-01）
  - `20260201100000` "beta"（created 02-01 / updated 04-01）
  - `20260301100000` "gamma"（created 03-01 / updated 05-01）
  - updatedAt 昇順 → `[beta, gamma, alpha]` / createdAt 昇順 → `[alpha, beta, gamma]` /
    createdAt 降順（default）→ `[gamma, beta, alpha]`。3 者が相異なるため sort 復元の有無を判別できる
- **再起動の扱い**: filter はプロセスメモリ（Rust `InMemoryNoteFeedState`）上の揮発状態で、webview
  リロード（`window.location.reload()`）ではリセットされない。一方 E2E の新規セッション起動は
  **プロセス新規起動**であり、`NoteFeed::empty()` の default filter（空）から始まる = S12 の
  「再起動後」状態そのもの。したがって本 E2E は seed 済み `settings.json` でアプリを新規起動し、
  起動直後の filter / sort / 表示順を検証する（filter を前セッションで設定して再起動する操作は
  filter が非永続のため不要。非永続であること自体は `settings.json` の検査で確認する）
- **アサーション戦略**:
  - 起動時 sort は `[data-block-id]` の存在集合と並び順で判定する
  - filter 空は `screen-1-toolbar-search-query` の value / `screen-1-toolbar-date-range-all` の
    `aria-pressed` / `screen-1-toolbar-tag-chip` の不在で判定する
  - toolbar の sort 状態は `screen-1-toolbar-sort-field-updated_at` の `aria-pressed` と
    `screen-1-toolbar-sort-direction` の `aria-label` で判定する
  - `settings.json` は `node:fs` の `readFileSync` で直接 read する
- **テストデータのクリーンアップ**: `wdio.conf.ts` の `onComplete` が temp dir を `rmSync` で削除する
- **production 側の前提（本 scenario が検証する不変条件）**:
  1. `load_settings` command が `settings.json` の `sort_preference` を field 単位 fallback 付きで
     復元して返すこと（`apps/promptnotes/src-tauri/src/user_preferences/slices/load_settings/`）
  2. `list_notes` command が `settings.sort_preference()` を feed に適用して `visible_notes` を返すこと
     （`apps/promptnotes/src-tauri/src/note_feed/slices/list_feed/commands.rs`）
  3. `page-main` が起動時に `load_settings` の `sort_preference` を feedStore へ反映すること
     （`apps/promptnotes/src/ui-page/page-main/PageMain.svelte`）
  4. 起動時 filter を空のまま初期化すること（`feedStore` の `DEFAULT_FILTER`。filter を Settings から
     hydrate しないこと = 揮発、Q3 / I-F6）
  これらのいずれかが欠落すると S12 が RED になる