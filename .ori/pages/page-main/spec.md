---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  last_derived: 2026-06-29
  derives_from:
    - domain/ui-fields/screen-1.md#screen-1
    - domain/ui-fields/page-groups.md#page-main
    - domain/domain-events.md#theme-changed
    - domain/workflows/create-note.md#create-note
    - domain/workflows/auto-save-note.md#auto-save-note
    - domain/workflows/flush-note.md#flush-note
    - domain/workflows/assign-tag.md#assign-tag
    - domain/workflows/remove-tag.md#remove-tag
    - domain/workflows/delete-note.md#delete-note
    - domain/workflows/restore-deleted-note.md#restore-deleted-note
    - domain/workflows/copy-note-body.md#copy-note-body
    - domain/workflows/update-feed-filter.md#update-feed-filter
    - domain/workflows/change-sort-order.md#change-sort-order
---

# page-main — Page Specification {#page-main-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive page-main`. Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

PromptNotes アプリの **root page**（唯一の ui-page）。`.ori/domain/ui-fields/page-groups.md#page-main` の方針 D（root + widgets）に基づき、`screen-1` の 4 region すべてをこの 1 page で表示する。

- **kind**: ui-page (root)
- **mount lifecycle**: アプリ起動と同時に mount、`app_quit` まで存在
- **hosts**: 10 slice の application path をすべて trigger できる
- **mounts**:
  - [widget-settings-modal](../../domain/ui-fields/page-groups.md#widget-settings-modal) — on-demand (toolbar 歯車 / `Cmd+,`)
  - [widget-update-toast](../../domain/ui-fields/page-groups.md#widget-update-toast) — 起動時 conditional

このページは **シングルペイン制約** の物理的実体である。spec が禁じる「複数 window」「URL ルーティング」「複数カラム」のいずれも導入してはならない。

## ホストする slices {#hosted-slices}

| region | slice | trigger | 副作用 |
|--|--|--|--|
| Draft Input | [create-note](../../slices/create-note/spec.md) | `Cmd+Enter` / `screen-1-draft-submit` | 新 Note 作成 → Feed 先頭に挿入 |
| Block (EDITING) | [auto-save-note](../../slices/auto-save-note/spec.md) | EDITING debounce trailing edge | `updated_at` 更新 |
| Block / window | [flush-note](../../slices/flush-note/spec.md) | block blur / window blur / app quit | debounce を bypass して即時 flush |
| Block (tag input) | [assign-tag](../../slices/assign-tag/spec.md) | `screen-1-block-tag-input` Enter | Tag 追加 |
| Block (tag chip) | [remove-tag](../../slices/remove-tag/spec.md) | `screen-1-block-tag-remove` × | Tag 削除 |
| Block (hover) | [delete-note](../../slices/delete-note/spec.md) | `screen-1-block-delete` icon | trash 移動 + Toast push |
| Toast | [restore-deleted-note](../../slices/restore-deleted-note/spec.md) | `screen-1-toast-undo` / `Cmd+Z` | trash 復元 |
| Block (hover) | [copy-note-body](../../slices/copy-note-body/spec.md) | `screen-1-block-copy` icon | clipboard 書き込み |
| Toolbar | [update-feed-filter](../../slices/update-feed-filter/spec.md) | query / 期間 / tag-chip クリック | filter 即時反映 |
| Toolbar | [change-sort-order](../../slices/change-sort-order/spec.md) | sort field / direction 変更 | Block 再配置 + Settings 永続化 |

`screen-1` に含まれない 2 slice（`load-settings` / `check-for-updates`）は page-main の **mount-time effect** として呼ぶ：

- `load-settings`: page mount 時に Settings を取得し、`screen-1-toolbar-sort-field` / `sort-direction` の initial value を設定（domain/aggregates.md#settings-load-only-on-startup）
- `check-for-updates`: page mount 時に呼び出し、`NewVersionDetected` 受信時のみ [widget-update-toast](../../domain/ui-fields/page-groups.md#widget-update-toast) を mount

## レイアウト {#layout}

```
┌──────────────────────────────────────────────────────────────┐
│ Toolbar region (top, sticky)                                 │
│  [🔍 検索] [期間ｾｸﾞ] [Sort↑↓] [⚙️]                            │
├──────────────────────────────────────────────────────────────┤
│ Draft Input region (sticky top of feed)                      │
│  ┌────────────────────────────────────────────┐  [＋追加]      │
│  │ CodeMirror 6 (markdown, runtime-editable)  │              │
│  └────────────────────────────────────────────┘              │
├──────────────────────────────────────────────────────────────┤
│ Feed region (scrollable, 単一カラム)                          │
│  ┌── Block 1 (IDLE/FOCUSED/EDITING) ──────────────┐ [📋][🗑️] │
│  │ meta: [tag-chip][tag-chip] ... createdAt 右寄  │          │
│  │ body: CodeMirror 6 (全文表示, 折りたたみ禁止)   │          │
│  └────────────────────────────────────────────────┘          │
│  ┌── Block 2 ───────────────────────────────────┐            │
│  │ ...                                           │            │
│  └───────────────────────────────────────────────┘            │
├──────────────────────────────────────────────────────────────┤
│ Toast region (bottom-center, 縦パイル / 新しい順に上積み)       │
│  ┌── Toast (delete-note) ─ "削除しました" [元に戻す][×] ──┐  │
│  └─────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
       ┌── widget-update-toast (右下, conditional) ──┐
       └─────────────────────────────────────────────┘

  Overlay (on-demand): widget-settings-modal (中央 modal)
```

- 4 region は `screen-1.md#fields` の id 体系（`screen-1-toolbar-*` / `screen-1-draft-*` / `screen-1-block-*` / `screen-1-toast-*`）でそのまま data-testid に写す
- Tailwind 4 を用いたユーティリティ first 配置。複数カラムレイアウトは禁止（spec 末尾の禁止事項）

## Unique Selling Points / 差別化点 {#unique-points}

このページが **PromptNotes の差別化点をすべて UI 化する唯一の場** であることを忘れない（domain/discovery.md より）：

1. **本文のみコピー**: [copy-note-body](../../slices/copy-note-body/spec.md) のホバーボタン経由でクリップボードに body だけ書き込む（YAML frontmatter / タグ表現を含めない）
2. **座標ズレのない Markdown 編集**: Draft + Block body 共に **同一 CodeMirror 6 構成**（HTML プレビューへの切替なし、レンダラ差し替え禁止）
3. **プロンプト・ブロックのストック**: フィードは時系列で全文表示、切り捨て / "show more" / 折りたたみ等のトリミング UI を **置かない**

UI 実装はこの 3 点を曖昧化してはならない。

## 不変条件 {#invariants}

### Page lifecycle {#invariants-lifecycle}

- **I-PM1（root unique）**: アプリプロセス内に page-main の instance は同時に高々 1 つ。SvelteKit の SPA shell（`+page.svelte`）として実装され、ナビゲーションによる再 mount は発生しない（routing なし）
- **I-PM2（widget 子従属）**: `widget-settings-modal` / `widget-update-toast` の lifecycle は page-main に従属する。page-main が unmount すれば widget も unmount される（実態としてはアプリ quit のみが unmount trigger）
- **I-PM3（mount-time effect 順序）**: `load-settings` → toolbar 初期値反映 → `check-for-updates` の順で起動。`load-settings` 失敗時の挙動は `Settings::defaults()` への fallback（domain/aggregates.md#settings-loading）に従い、UI は警告 region を出さない（silent fallback）

### Region 不変条件 {#invariants-regions}

- **I-PM4（シングルペイン）**: 同時に表示される pane は 1 つのみ。Draft / Feed は同一 viewport 内のスクロール領域として連続する。Toolbar / Toast は overlay 的だが viewport 分割ではない
- **I-PM5（Draft 唯一）**: Draft Input region は viewport 内に 1 つだけ存在し、Feed 最上部に **常時固定**。複数 Draft は禁止（screen-1.md `{#fields-draft}` 準拠）
- **I-PM6（Block 縦並び単一カラム）**: Feed は 1 列縦並びのみ。複数カラム / グリッド表示禁止
- **I-PM7（Toast 独立）**: Toast スタックは互いに独立。1 つの Undo 失効が他 Toast の Undo を失効させない（screen-1.md#cross-toast-display）
- **I-PM8（widget 排他なし）**: `widget-settings-modal` open 中でも page-main の Toolbar / Feed は維持される（Modal は modal だが、page の state は dispose しない）

### Cross-region 不変条件 {#invariants-cross-region}

- **I-PM9（Draft submit → Feed 挿入）**: `create-note` 成功 → Draft 即時クリア + 新 Block を Feed 最上部に挿入 + 新 Block にフォーカス遷移（FOCUSED, `Esc` で IDLE）
- **I-PM10（Block state machine の唯一性）**: 同時に EDITING な Block は高々 1 つ。別 Block クリックで前 EDITING は IDLE へ遷移（screen-1.md#cross-block-state）。EDITING 中の `↑/↓` はナビゲーションを起こさない
- **I-PM10a（click-to-edit カーソル位置保持）**: IDLE / FOCUSED 状態の Block をクリックして EDITING に遷移する際、CodeMirror EditorView のカーソルはクリック位置に対応するドキュメントオフセットに配置される。カーソルが先頭 (position 0) に強制移動されてはならない。実装はクリック座標から `EditorView.posAtCoords()` で位置を解決するか、EditorState 再構築時に旧 selection を保持することで実現する
- **I-PM11（filter / sort の即時反映）**: Toolbar の filter / sort 変更は debounce なしで Feed に反映。Sort 変更はアニメーション付き再配置（screen-1.md#cross-sort-immediate）
- **I-PM12（auto-save と flush の二重実行禁止）**: 1 つの Block の EDITING → 別状態への遷移時、`flush-note` が先に呼ばれ、debounce timer は cancel される。`auto-save-note` と `flush-note` が同 body に対して二重 write を起こしてはならない（C-AS3 / C-FL3 の NoOp 契約に依存）

### 経路境界 {#invariants-boundary}

- **I-PM13（UI 層 → Tauri command の唯一経路）**: page-main は `apps/promptnotes/src/lib/<bc>/slices/<id>/index.ts` 経由でのみ Tauri command を呼ぶ。`@tauri-apps/api/core` の直接 import は禁止（`.ori/architecture.md` `forbidden_imports` 参照）
- **I-PM14（Domain 層への直接依存禁止）**: page-main は domain types を直接 import せず、slice の TS bindings が export する DTO 型のみを使う。Brand 化された VO（NoteId 等）の構築は slice 内で完結する
- **I-PM15（Cross-slice の直接連携禁止）**: 例えば create-note の outcome を delete-note に直接渡す等の **slice 間結線は page state（Svelte store）経由のみ**。slice モジュールが他 slice を import してはならない（`.ori/architecture.md` `cross_slice.prohibited_direct: true`）

### Theme 適用 {#invariants-theme}

> domain/domain-events.md#theme-changed-subscribers より：「**UI 層**: CodeMirror テーマと CSS 変数を即時切り替え」

- **I-PM16（theme DOM 反映）**: page-main は `Settings.theme` に応じて `<html>` 要素の `dark` class を toggle する
  - `Dark`: `dark` class を付与
  - `Light`: `dark` class を削除
  - `System`: `prefers-color-scheme: dark` media query の結果に従い、`dark` class を toggle する。**media query の変更を監視**し、変化時に即時反映する
- **I-PM17（ThemeChanged 購読）**: page-main は `settings:theme_changed` Tauri event を購読し、受信時に即座に `Settings.theme` を更新して DOM に反映する（domain-events.md#theme-changed-timing: 同期）
- **I-PM18（初期 theme 適用）**: `load-settings` で取得した `Settings.theme` を page mount 時に DOM に反映する（I-PM3 の mount-time effect の一環）

## テスト観点 (E2E + smoke + a11y) {#test-points}

### tp-mount: 起動 smoke {#tp-mount}

`bun tauri dev` で起動 → page-main が mount され、4 region がすべて DOM に存在する（`data-testid="region-toolbar"` / `region-draft` / `region-feed` / `region-toast`）。`load-settings` の失敗時も `Settings::defaults()` で続行し、UI は表示される（I-PM3）。

### tp-golden-create: Cmd+Enter で Note 作成 → Feed 表示 {#tp-golden-create}

1. Draft Input に `"hello"` を入力
2. `Cmd+Enter` 押下
3. Draft 即時クリア + Feed 最上部に新 Block 挿入 + 新 Block に FOCUSED フォーカス
4. `Esc` で IDLE へ遷移

I-PM9 + create-note slice の I-CN3（空 body は NoOp）の page 側 propagation を検証。

### tp-empty-body-noop: 空 body は NoOp {#tp-empty-body-noop}

Draft 空のまま `Cmd+Enter` → create-note が呼ばれ outcome が `no_op` → Feed に変化なし / Toast 非表示。

### tp-block-state-machine: Block の IDLE/FOCUSED/EDITING 遷移 {#tp-block-state-machine}

screen-1.md#cross-block-state の遷移を **全 8 transition** について検証：

- IDLE → FOCUSED: `↑` / `↓`
- IDLE → EDITING: click
- FOCUSED → EDITING: `Enter`
- FOCUSED → FOCUSED (別): `↑` / `↓`
- FOCUSED → IDLE: `Esc`
- EDITING → FOCUSED: `Esc`
- EDITING → 別 EDITING: click（前 EDITING は IDLE）
- EDITING 中 `↑/↓` は no-op（I-PM10）
- **IDLE → EDITING (click, cursor position)**: body=`hello world`（11 文字）の Block を body 末尾付近でクリック → EDITING 遷移後、CodeMirror のカーソル位置が position 0 ではなくクリック位置付近（`>= 8` 程度）にあること。ブロック先頭付近クリックでは逆に position 0 付近になること（I-PM10a）

### tp-flush-on-blur: blur で flush-note 即時呼出 {#tp-flush-on-blur}

EDITING Block の body を編集 → 別 Block クリック → debounce timer cancel + `flush-note` が `block_blur` trigger で呼ばれる（I-PM12）。

### tp-toast-stack: 削除 toast の縦パイル {#tp-toast-stack}

2 件連続削除 → 2 つの Toast が縦に積まれ（新しい順）、片方の Undo が他方に影響しない（I-PM7, screen-1.md#cross-toast-display）。

### tp-sort-immediate: sort 変更で Feed 再配置 {#tp-sort-immediate}

Toolbar の sort field / direction を変更 → Feed の Block 順序が即時更新 + change-sort-order slice 経由で Settings 永続化（screen-1.md#cross-sort-immediate）。

### tp-no-multi-pane: 単一ペイン制約 {#tp-no-multi-pane}

DOM 構造に複数カラム grid / 並列 viewport が存在しないこと（I-PM4 / I-PM6）。`document.querySelectorAll('[data-testid="region-feed"]').length === 1`。

### tp-no-raw-invoke: 生 invoke 禁止 {#tp-no-raw-invoke}

eslint static check で `apps/promptnotes/src/ui-page/**` / `apps/promptnotes/src/ui-widget/**` から `@tauri-apps/api/core` の import が 0 件であること（I-PM13、`.ori/architecture.md` `forbidden_imports` 経由で自動検出）。

### tp-a11y-basic: 基本 a11y {#tp-a11y-basic}

- Draft Input は `aria-label="新規 Note"`
- 各 Block は `role="article"` + `aria-label` に `createdAt` を含める
- Toolbar のアイコンボタンは `aria-label` 必須（`screen-1-toolbar-settings-button` 等）
- 最低限の Tab フォーカス順序: Toolbar → Draft → Feed Blocks

詳細な a11y 監査（コントラスト / SR ナビゲーション）は MVP 範囲外。

### tp-theme-apply: theme の DOM 反映 {#tp-theme-apply}

> domain/domain-events.md#theme-changed-subscribers より：「UI 層が CodeMirror テーマと CSS 変数を即時切り替え」

- **TP-T1**: `load-settings` で取得した `Settings.theme == Dark` の場合、`<html>` 要素に `dark` class が付与される
- **TP-T2**: `Settings.theme == Light` の場合、`dark` class が削除される
- **TP-T3**: `Settings.theme == System` かつ `prefers-color-scheme: dark` の場合、`dark` class が付与される
- **TP-T4**: `Settings.theme == System` かつ `prefers-color-scheme: light` の場合、`dark` class が削除される

### tp-theme-system-media: System theme の media query 監視 {#tp-theme-system-media}

- **TP-T5**: `Settings.theme == System` の状態で `prefers-color-scheme` が `dark` → `light` に変化した場合、`dark` class が即座に削除される
- **TP-T6**: `Settings.theme == System` の状態で `prefers-color-scheme` が `light` → `dark` に変化した場合、`dark` class が即座に付与される
- **TP-T7**: `Settings.theme == Dark` または `Light` の状態では `prefers-color-scheme` の変化を無視する（固定）

### tp-theme-changed-event: ThemeChanged event 購読 {#tp-theme-changed-event}

- **TP-T8**: `settings:theme_changed` Tauri event で `new_theme: Dark` を受信した場合、即座に `dark` class が付与される
- **TP-T9**: `settings:theme_changed` Tauri event で `new_theme: Light` を受信した場合、即座に `dark` class が削除される
- **TP-T10**: `settings:theme_changed` Tauri event で `new_theme: System` を受信した場合、`prefers-color-scheme` media query の監視を再開し、現在値で `dark` class を toggle する
- **TP-T11**: `settings:theme_changed` event 購読は非 Tauri 環境（browser / test）で silent fallback する（例外を投げない）

## 実装ノート {#impl-notes}

### Svelte 5 + SvelteKit static SPA {#impl-svelte-stack}

- ルートエントリ: `apps/promptnotes/src/routes/+page.svelte`（既存の SvelteKit プロジェクト、`prerender = true; ssr = false`）
- ランタイム mode: Svelte 5 **runes mode** (`vite.config.ts` で `runes: true` 強制)
- 状態管理: Svelte 5 `$state` + module-scoped store (`*.svelte.ts`) で region 間に共有
- スタイル: Tailwind 4（`@tailwindcss/vite` プラグイン経由）

### ディレクトリ構成 (page-main + widgets) {#impl-layout-dir}

`.ori/architecture.md` の `ddd-vsa-hex-ts` layer に従い：

```
apps/promptnotes/src/
├── routes/+page.svelte                 # SvelteKit エントリ (page-main を mount するだけ)
├── ui-page/
│   └── page-main/                      # kind: ui-page (order 2)
│       ├── PageMain.svelte             # root layout
│       ├── regions/
│       │   ├── ToolbarRegion.svelte
│       │   ├── DraftRegion.svelte
│       │   ├── FeedRegion.svelte
│       │   └── ToastRegion.svelte
│       ├── stores/
│       │   ├── feed.svelte.ts          # 表示中 notes / filter / sort
│       │   ├── draft.svelte.ts         # draft body / submit state
│       │   ├── focus.svelte.ts         # Block state machine (IDLE/FOCUSED/EDITING)
│       │   ├── toasts.svelte.ts        # delete-note Toast スタック
│       │   ├── sort-preference-subscriber.svelte.ts  # SortPreferenceChanged event 購読
│       │   └── theme-subscriber.svelte.ts            # ThemeChanged event 購読 + .dark class toggle
│       └── tests/                      # *.svelte.test.ts (vitest browser)
└── ui-widget/                          # kind: ui-widget (order 1)
    ├── settings-modal/
    │   └── WidgetSettingsModal.svelte
    └── update-toast/
        └── WidgetUpdateToast.svelte
```

- UI layer rule: `ui-page → ui-widget → {shared, domain}`（`.ori/architecture.md`）。`ui-page/` から `ui-widget/` を import するのは OK、逆は禁止
- 各 slice の Tauri 呼出 は **`$lib/<bc>/slices/<id>/index.ts` のみ** を import（I-PM13 / eslint で自動検査）

### Sub-task 分割 {#impl-subtasks}

page-main は実装範囲が大きいため、以下 5 sub-task に分割して順次 PR 化する：

1. **page-main-shell**（本 PR）— SvelteKit root + 4 region コンテナのスケルトンを実装、Tauri 連携なしの DOM mount を smoke test で確認。`tp-mount` / `tp-no-multi-pane` を pass
2. **page-main-toolbar** — Toolbar region（検索 / 期間 / sort / 設定アイコン）+ `update-feed-filter` / `change-sort-order` 連携 + `widget-settings-modal` open trigger
3. **page-main-draft** — Draft Input region（CodeMirror 6）+ `create-note` 連携 + `Cmd+Enter` shortcut。`tp-golden-create` / `tp-empty-body-noop` を pass
4. **page-main-feed** — Feed region + Block コンポーネント（CodeMirror 6 + state machine）+ `auto-save-note` / `flush-note` / `assign-tag` / `remove-tag` / `copy-note-body` 連携。`tp-block-state-machine` / `tp-flush-on-blur` を pass
5. **page-main-toast** — Toast region（縦パイル）+ `delete-note` / `restore-deleted-note` 連携 + `Cmd+Z` shortcut + `widget-update-toast` mount。`tp-toast-stack` を pass

`tp-sort-immediate` / `tp-no-raw-invoke` / `tp-a11y-basic` は段階的に追加し、page-main-toast 完了時点で全 test 観点を網羅する。

### 既存 slice TS bindings との関係 {#impl-bindings}

`apps/promptnotes/src/lib/note-capture/slices/*/index.ts` / `note-feed/slices/*/index.ts` / `user-preferences/slices/*/index.ts` は既に 10 slice + load-settings/update-settings 分が実装済み。page-main は **DTO 型と関数のみを import** し、Tauri command を直接呼ばない。

### Feed の初期値 {#impl-feed-initial}

現状 list-feed slice が未実装（`bd show ori-64x.10` / `bd show ori-hpo.11`）のため、Feed の起動時 hydration は **後続 slice 起票後に対応**。page-main-shell では空の Feed で起動し、`create-note` 経由で session 内に作成した Note のみ表示する。

将来 list-feed slice 実装後に Feed store の hydration source を差し替える前提。

### 非責務 {#impl-non-responsibility}

- セッション間の永続化 layout（カラム幅・スクロール位置）は MVP 範囲外
- マルチウィンドウ / マルチタブ対応は禁止（spec シングルペイン制約）
- 印刷 / PDF export 等のレンダラ差し替え機能は禁止（spec 末尾の禁止事項）

## Open Questions {#open-questions}

- **OQ-PM1（list-feed slice 待ち）**: 既存 Note の起動時 hydration は list-feed slice 起票後に実装する。それまで session 内 state のみで完結する設計でよいか → page-main-shell では Yes（後続で差し替え可能な store 構造にする）
- **OQ-PM2（CodeMirror state 共有）**: Draft と Block body で **同一 CodeMirror 構成** を要求する（screen-1.md#notes-codemirror-consistency）が、共通 helper を `ui-page/page-main/stores/codemirror.svelte.ts` に切るか `lib/shared/codemirror/` に切るかは page-main-draft / page-main-feed sub-task で決める
- **OQ-PM3（widget mount 順序）**: `widget-update-toast` の起動時 check 失敗は silent fallback だが、`widget-settings-modal` open 中に `NewVersionDetected` を受信した場合の表示順序（modal の裏 / 上）は未確定 → page-main-toast sub-task で決定
- **OQ-PM4（Block ↑/↓ の境界）**: Feed 上端の Block で `↑` を押した時、Draft Input にフォーカスが移るか / 何も起こらないかは screen-1.md に明記なし → page-main-feed sub-task でユーザに確認
