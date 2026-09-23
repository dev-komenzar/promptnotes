# s12-startup-state — Scenario implementation notes

## 概要 {#overview}

アプリ再起動時の NoteFeed 初期化を検証する。**filter（query / date_range / tag）は揮発**で起動時は
常に空（Q3 / I-F6）、**sort のみ Settings.sort_preference から復元**される。`settings.json` に
`sort_preference = { updated_at, asc }` を seed した状態でアプリを新規起動し、起動直後の filter 空 /
sort 復元 / 表示順（updatedAt 昇順）を検証する。

## テスト方式: config dir 隔離 + 新規プロセス起動 {#test-approach}

- **config dir 隔離**: wdio `onPrepare` が `XDG_CONFIG_HOME` / `XDG_DATA_HOME` を temp dir に設定する。
  Tauri の `app_config_dir()` は `$XDG_CONFIG_HOME/com.komenzar.promptnotes` に解決されるため、そこへ
  `settings.json`（`sort_preference = { updated_at, asc }`）を seed して Given の Settings を再現する。
- **Note seed**: 3 件を `onPrepare` が書き込む。`updatedAt` 昇順が `createdAt` 昇順 / `createdAt` 降順
  （I-S3 default）のいずれとも**相異なる**よう createdAt / updatedAt をずらす（空振り防止）:
  - `20260101100000` "alpha"（created 01-01 / updated 06-01）
  - `20260201100000` "beta"（created 02-01 / updated 04-01）
  - `20260301100000` "gamma"（created 03-01 / updated 05-01）
  - updatedAt 昇順 → `[beta, gamma, alpha]` / createdAt 昇順 → `[alpha, beta, gamma]` /
    createdAt 降順（default）→ `[gamma, beta, alpha]`
- **再起動の扱い**: filter は Rust `InMemoryNoteFeedState` 上の揮発状態で、webview リロード
  （`window.location.reload()`）ではリセットされない。一方 E2E の新規セッション起動は **プロセス新規
  起動**であり、`NoteFeed::empty()` の default filter（空）から始まる = S12 の「再起動後」状態そのもの。
  したがって本 E2E は seed 済み `settings.json` でアプリを新規起動し、起動直後の状態を検証する。
  filter が非永続であることは `settings.json` に filter 相当キーが存在しないことで確認する。
- **アサーション**:
  - 表示順は `[data-block-id]` の並びで判定
  - filter 空は `screen-1-toolbar-search-query` の value / `screen-1-toolbar-date-range-all` の
    `aria-pressed` / `screen-1-toolbar-tag-chip` の不在で判定
  - toolbar の sort 状態は `screen-1-toolbar-sort-field-updated_at` の `aria-pressed` と
    `screen-1-toolbar-sort-direction` の `aria-label` で判定
  - `settings.json` は `node:fs` の `readFileSync` で直接 read

## テストカバレッジ {#coverage}

| 検証項目 | 検証方法 |
|---|---|
| 起動時 sort 復元（updatedAt 昇順で表示） | E2E step1（`[data-block-id]` の並び = `[beta, gamma, alpha]`。default の createdAt 降順と区別） |
| toolbar の sort 状態（field=updated_at / direction=asc） | E2E step1（`screen-1-toolbar-sort-field-updated_at` の aria-pressed + `screen-1-toolbar-sort-direction` の aria-label） |
| 起動時 filter 空（query / date_range / tag） | E2E step1（search value 空 / date-range-all の aria-pressed / tag-chip 不在） |
| Settings が起動で書き換わらない | E2E step1（`settings.json` の storage_dir / sort_preference が seed 値のまま） |
| filter は Settings に永続化されない（揮発） | E2E step1（`settings.json` に query / tag / date_range / filter キーが無い） |
| `load_settings` の field 単位 fallback / `list_notes` の sort 適用 | 既存 slice の Rust unit test（load-settings / list-feed）が担保 |

## RED → GREEN 検証 {#red-green}

production 側は S12 の前提を**既に満たしていた**（frontend `PageMain.svelte` の起動時
`feedStore.hydrateSort(settings.sort_preference)`、backend `list_notes` の
`change_sort(settings.sort_preference())`）。テストが空振りでないことを示すため、production を
一時変異させて RED を実測した:

- **変異 A（`PageMain.svelte` の起動時 `feedStore.hydrateSort(settings.sort_preference)` を削除）**:
  `VITE_WDIO_TEST=1 bun run tauri build --debug --no-bundle` → E2E **1 passing / 1 failing**（exit 1）。
  step1 が表示順で RED:
  `Expected ["20260201100000","20260301100000","20260101100000"]`（updatedAt 昇順 = beta, gamma, alpha）
  に対し `Received ["20260301100000","20260201100000","20260101100000"]`（createdAt 降順 = I-S3 default
  = gamma, beta, alpha）。
- **復元後**: 再ビルド → E2E **1 passing (~0.7s)** で GREEN。

**補足（sort 適用の二重性）**: backend `list_notes` も `settings.sort_preference()` を feed に適用して
DTO を返すが、frontend は `visibleNotes` 派生で `feedStore.sort` により**再ソート**する。したがって
ユーザー可視の表示順を支配するのは frontend の `hydrateSort` であり、変異 A のように frontend 側の
復元を壊すと表示順が default に戻って RED になる。backend 側のみを壊した場合は frontend が補正する
ため GREEN のまま（冗長だが無害）。本 E2E はユーザー可視の起動時 state を検証する。

## production 変更 {#production-change}

**なし（0 files changed）。** S12 の不変条件は既存実装で充足していた:
1. `load_settings` command が `settings.json` の `sort_preference` を field 単位 fallback 付きで復元
2. `list_notes` command が `settings.sort_preference()` を feed に適用
3. `page-main` が起動時に `load_settings` の `sort_preference` を feedStore へ反映（`hydrateSort`）
4. 起動時 filter は `feedStore` の `DEFAULT_FILTER`（空）のまま（filter を Settings から hydrate しない）

回帰確認:

- frontend unit test: **143 passed / 15 files**
- svelte-check: **0 errors / 0 warnings**
- E2E 回帰: s11-storage-dir-change **3 passing**、s4-tag-assign-normalize **2 passing**、
  s10-tag-invalid-char **3 passing**
- Rust: 変更なし

## 既知の未対応 / 補足 {#known-issues}

- **`Settings::load_or_default` は実 Rust の関数名ではない**: validation.md の表記は概念名で、実体は
  `LoadSettingsUseCase::execute`（`-> Settings`、`Result` なし）。`sort_preference` の欠損 / 不正は
  `resolve_sort_preference` が field 単位で default（`{ created_at, desc }`、I-S3）へ降格する。
- **webview リロードでは filter がリセットされない**: filter は Rust プロセスメモリ上の
  `InMemoryNoteFeedState` が保持する。真のプロセス再起動でのみ default（空）に戻る。本 E2E は新規
  プロセス起動で post-restart 状態を再現している。
- **`window.location.reload()` による remount は本 scenario では使わない**: filter が保持されるため
  「起動時 filter リセット」の検証には不適。s11 step3 は settings.json 再読込のみが観測点のため
  reload で成立するが、s12 はプロセス新規起動が必要。
