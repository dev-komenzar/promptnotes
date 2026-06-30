---
ori:
  node_id: page-grouping:overview
  type: page-grouping
  depends_on:
    - ui-field:index
    - workflow:index
---

# Page Groups {#page-groups}

PromptNotes の 4 screens を page 構成として整理する。
spec の **シングルペイン制約** により page は 1 つのみ、残り 3 screens は
**widget** として page に従属する形を採用。

## Chosen Strategy {#chosen-strategy}

**方針 D (root + widgets)** を採用。

### 検討した代替案 {#chosen-strategy-alternatives}

| 方針 | 内訳 | 不採用理由 |
|--|--|--|
| A: workflow 単位 | 13 page (1 workflow = 1 page) | screen-1 が分裂し、シングルペイン制約と矛盾 |
| B: ナビゲーション単位 | page-main (s1+s2), page-update (s3) | s2/s3 を page にすると「ペイン感」が出て spec の overlay 性を曖昧化 |
| C: 役割単位 | 1 page (s1+s2+s3 全部) | overlay と root の区別が消える |
| **D: root + widgets (採用)** | page-main = s1 / widget-settings-modal = s2 / widget-update-toast = s3 / widget-external-change-conflict = s4 | spec のシングルペイン + OS ネイティブ modal/toast の意図に合致 |

### 採用理由 {#chosen-strategy-rationale}

- **spec「シングルペイン」の厳守**: 「メインウィンドウ」は 1 つだけ → page も 1 つ
- **OS ネイティブ overlay の表現**: Settings Modal と Update Toast はいずれも
  メインウィンドウに付随する ephemeral overlay であり、URL や別 window を持たない
  → page ではなく widget が適切
- **テスト境界**: widget は page-main に lifecycle を従属させるため、
  「メインウィンドウが開いている時のみ存在する」という不変条件が型で表現できる

## Groups {#groups}

### page-main {#page-main}

- **kind**: ui-page (root)
- **screens**: [screen-1](screen-1.md)
- **対応 workflow**: 10 件 (create-note / auto-save-note / flush-note
  / assign-tag / remove-tag / delete-note / restore-deleted-note
  / copy-note-body / update-feed-filter / change-sort-order)
- **ライフサイクル**: アプリ起動と同時に mount、アプリ quit まで存在
- **mount widgets**:
  - [widget-settings-modal](#widget-settings-modal) (on-demand)
  - [widget-update-toast](#widget-update-toast) (起動時自動 mount, 条件付)
  - [widget-external-change-conflict](#widget-external-change-conflict) (event 駆動, 条件付)

### widget-settings-modal {#widget-settings-modal}

- **kind**: ui-widget
- **screens**: [screen-2](screen-2.md)
- **対応 workflow**: update-settings
- **mount trigger**:
  - page-main の `screen-1-toolbar-settings-button` クリック
  - macOS menu bar 「PromptNotes → Preferences」 (`Cmd+,`)
- **unmount trigger**: save / cancel / Esc キー
- **依存する page**: page-main (modal は parent page を持つ)

### widget-update-toast {#widget-update-toast}

- **kind**: ui-widget
- **screens**: [screen-3](screen-3.md)
- **対応 workflow**: check-for-updates
- **mount trigger**: アプリ起動時の `NewVersionDetected` event 受信
- **unmount trigger**: dismiss / view-release のいずれかをユーザがクリック
- **依存する page**: page-main (toast は parent page の overlay area に表示)
- **failure mode**: `NewVersionDetected` 未発行時は **mount されない** (silent, S14)

### widget-external-change-conflict {#widget-external-change-conflict}

- **kind**: ui-widget
- **screens**: [screen-4](screen-4.md)
- **対応 workflow**: detect-external-changes
- **mount trigger**: `NoteFileModifiedExternally` event 受信 + `Note::is_stale()` が `true`
- **unmount trigger**: Apply / Cancel / Esc / ×ボタン
- **依存する page**: page-main (modal overlay)
- **failure mode**: 同一 `note_id` の重複 event は最初のダイアログが開いている間無視
- **注意**: `NoteFileCreatedExternally` / `NoteFileDeletedExternally` では mount されない（Feed 自動更新で十分）

## Layering {#layering}

```
page-main (root, ui-page)
├── widget-settings-modal (on-demand, ui-widget)
├── widget-update-toast (startup conditional, ui-widget)
└── widget-external-change-conflict (event-driven, ui-widget)
```

- page-main の中に **削除トーストスタック** ([screen-1-toast-stack](screen-1.md#fields-toast))
  も存在するが、これは page-main の自前 region（同一 screen 内）であり widget ではない
- widget-update-toast と削除トーストスタックは表示位置が異なる
  (右下: update / 中央下: delete) ため衝突しない (screen-3.md cross-position-duration)

## Open Questions {#open-questions}

Phase 11b 時点で未決事項はない。

- 将来 macOS menu bar や system tray icon を追加する場合、それぞれ widget として
  追加する余地がある（現状 spec 範囲外）
- `.ori/architecture.md` は未生成のため `## Page Map` 自動生成は skip
  （Phase 0 / `/ori-arch` で生成された後に再走するか、手動で同期）

## Notes {#notes}

### page と widget の区別基準 {#notes-page-vs-widget}

- **ui-page**: ライフサイクルが独立 (URL / window) で root として mount される
- **ui-widget**: parent (page or 別 widget) のライフサイクルに従属する

PromptNotes はデスクトップアプリで URL ルーティングがなく、メインウィンドウが唯一の
root context のため、page = 1 つに収束する。

### 将来の page 増加シナリオ {#notes-future-pages}

以下のいずれかが発生した場合に page 構成の見直しが必要：

- **マルチウィンドウ化**: spec 拡張で「複数 window を開ける」要件が出た場合
  → 各 window が独立 page になる
- **ウィザード型オンボーディング**: 初回起動時の多段画面が必要になった場合
  → onboarding-* page 群を追加
- **設定モーダルの肥大化**: タブ式に分割した場合
  → settings-* page に格上げを検討

いずれも MVP 範囲外。
