---
name: ori-ddd-11b-ui-grouping
description: ori 独自 Phase 11b（Page Grouping）。ui-fields の依存関係から page を切り出し、各 screen の depended_by を確定する
---

ユーザが `/ori-ddd-11b-ui-grouping` を呼んだ際、ori 独自の Phase 11b（Page Grouping）を実行します。**distill-ddd 上流には存在しない ori オリジナル phase** です。`.ori/domain/ui-fields/screen-*.md` を読み、画面群を意味のある page に束ね、`page-groups.md` の `page-grouping:<id>` node に `depends_on: [ui-field:screen-N, ...]` を declare します（reverse 参照は `/ori-sync` が depends_on edge から計算するため、screen 側 frontmatter への back-edge 書き戻しは不要）。

## 役割

- **ファシリテーター**：画面間の包含関係・遷移・共有 VO から page 候補を提案
- **挑戦者**：「この page は 1 画面に閉じるか、複数画面にまたがるか」を問う
- **記録係**：合意した grouping を `page-groups.md` の各 `page-grouping:<id>` H2 セクションに `depends_on: [ui-field:screen-N, ...]` で declare

## 入力 / 出力

- 入力：`.ori/domain/ui-fields/screen-*.md`（Phase 11a 完了後）と `workflows/index.md`
- 出力：
  - 新規ファイル：`.ori/domain/ui-fields/page-groups.md`
    - file-level frontmatter: `node_id: page-grouping:overview`, `type: page-grouping`, `depends_on: [ui-field:index, workflow:index]`
    - 各 H2 = 1 page grouping。`{#<page-id>}` anchor から個別 `page-grouping:<page-id>` node が resolve される
    - 各 page section の本文表に `depends_on: [ui-field:screen-N, ...]` を列挙（slice 列挙は consume 側 manifest が担当）

## 手順

1. **前提確認**：
   - `.ori/domain/ui-fields/screen-*.md` が 1 つ以上ある事を確認。無ければ `/ori-ddd-11a-ui-fields` を先に促す
   - 既存の `page-groups.md` があれば内容を表示し「追記 / 上書き / 中断」を選ばせる
2. **画面と workflow の対応表を提示**：
   ```
   screen-1 (capture-auto-save) — 単独完結
   screen-2 (edit-past-note-start) — screen-3 から遷移
   screen-3 (note-list) — root
   ```
3. **AI が grouping 候補を 1-3 案提案**：
   - **方針 A: workflow 単位**（1 workflow = 1 page）— 開発粒度小、テスト容易
   - **方針 B: ナビゲーション単位**（遷移グラフの強連結成分）— UX 観点の凝集度高
   - **方針 C: 役割単位**（capture / edit / list を 1 page にまとめる）— 大きめ page
   - 各案について「pros / cons / 適合条件」を表で示す
4. **対話で 1 案を選ぶ**：ユーザの意図（MVP の粒度・チームサイズ）を踏まえ確定
5. **各 page の `depends_on` を決定**：
   - その page が host する screen 群（`ui-field:screen-N`）
   - 各 page が間接的に参照する `workflow:<id>` 群（screen の depends_on から推移閉包）
6. **書き出し**：
   - `page-groups.md` を新規生成（H2 ごとに `page-grouping:<page-id>` を declare、各 page の本文に `depends_on:` 表）
   - reverse 参照 (screen → page) は `/ori-sync` が depends_on edge から自動計算するため、screen-N.md 側 frontmatter には書き戻さない
7. **page ↔ slice マッピングを `.ori/pages/<id>/manifest.yaml` に反映**：
    - 確定した `depends_on` (page-groups.md) から `.ori/architecture.md` の `## Page Map` section を自動生成
    - マーカー（`<!-- BEGIN ori-distill phase-11b auto-generated; do not edit between markers -->` ～ `<!-- END ori-distill phase-11b auto-generated -->`）の外側は保持される
8. `for f in .ori/domain/ui-fields/*.md; do bash ./scripts/lint-domain.sh "$f"; done` を実行して自己検証
9. lint 失敗時は **1 回だけ** 自動修正を試み、それでも失敗ならユーザに判断を委ねる

### Phase 完了時：page の一括 scaffold 提案

確定した page 群を読み上げ、ユーザに確認：

```
以下の page を scaffold しますか？

  - capture-form    (screen-1)
  - note-browser    (screen-2, screen-3)

[1] 一括で作成（各 page の新規作成を提案）
[2] 個別に選択
[3] スキップ（後から手動で page を作成）
```

ユーザの選択に応じて page の新規作成を提案する。

## 出力テンプレート

### `page-groups.md` 新規

```markdown
---
ori:
  node_id: page-grouping:overview
  type: page-grouping
  depends_on:
    - ui-field:index
    - workflow:index
---

# Page Groups {#page-groups}

## Chosen Strategy {#chosen-strategy}

方針 B（ナビゲーション単位）を採用。理由：edit と list が強い遷移依存を持ち、別 page にすると往復のテストが煩雑になるため。

## capture-form {#capture-form}

- type: ui-page
- depends_on:
  - ui-field:screen-1
- 対応 workflow: capture-auto-save
- 単独完結（他画面から遷移しない）

## note-browser {#note-browser}

- type: ui-page
- depends_on:
  - ui-field:screen-2
  - ui-field:screen-3
- 対応 workflow: edit-past-note-start, list-notes
- screen-3 → screen-2 への遷移を含む

## Open Questions {#open-questions}

- 検索画面（screen-4）は note-browser に含めるか別 page か（暫定で含める）
```

各 page H2 は `{#<page-id>}` anchor を持ち、それぞれ `page-grouping:<page-id>` node として resolve される（例: `## capture-form {#capture-form}` → `page-grouping:capture-form`）。

### screen-N.md は書き換えない

旧仕様では screen-N.md の `coherence.depended_by:` に back-edge を書き戻していたが、`/ori-sync` が `depends_on` edge から reverse 参照を計算するため不要。screen-N.md の `ori:` frontmatter は Phase 11a の出力のまま保持する。

### `/ori-arch sync-page-map` への入力

`page-groups.md` の各 page H2 (`page-grouping:<id>`) の `depends_on` を読み取り、`.ori/architecture.md` の `## Page Map` section を生成する（screen → workflow → slice の推移閉包は sync-page-map が解決）。

## 注意

- **これは ori 独自 phase**：distill-ddd 上流には存在しない（ori README 参照）
- **grouping は再変更可能**：後から `page-groups.md` の各 page の `depends_on` を編集して `/ori-sync` すれば伝播
- このスキルは workflow を回さない。実装は `/ori-flow` の責務
- slice（`/ori-ddd-9-workflows` で作成）と page は独立。両方を持つ場合は dep を貼る

## 次のアクション

**これが distill-ddd 系の最終 phase**。完了したら実装に進む：

まず `ls .ori/architecture.md` で architecture.md の有無を確認し、**無ければ `/ori-arch` を最初に案内する**（`/ori-flow` は `.ori/architecture.md` 前提で動作する）。

- **architecture 未確定**：`/ori-arch` — pattern (ddd-vsa-hex 等) / stack を決めて `.ori/architecture.md` を render。完了後に下記メインパスへ
- **メインパス**：`/ori-flow <first-id>` — 1 slice / page を 7 phase で実装開始
  - 推奨：slice → page の順（domain ロジック先行）
- **scaffold だけ済ませて休む**：page の新規作成を提案し beads issue だけ作っておく
- **戻る**：grouping が決まらない場合、`/ori-ddd-9-workflows` で workflow 境界を見直す
- **未確定なら**：`page-groups.md` の `open questions` に残し、次セッションで再開
