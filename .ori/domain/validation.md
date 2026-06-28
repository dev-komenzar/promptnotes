---
ori:
  node_id: scenario:collection
  type: scenario
  depends_on:
    - aggregate:collection
    - event:collection
    - event-storming:timeline
---

# Validation Scenarios {#validation-scenarios}

Phase 5 (aggregates) と Phase 6 (domain-events) の整合性を、
typical / edge / error の各シナリオを Given/When/Then で walkthrough して検証する。
表現できないシナリオが見つかったら前 phase に戻る。

シナリオ ID は `S1` 〜 で連番。表記凡例：
- `Cmd+X` は macOS、Linux/Windows は `Ctrl+X` に読み替え
- `t0 < t1 < t2 ...` は秒精度の論理時刻
- event 名は domain-events.md 準拠

## Scenario S1: 新規 Note を Cmd+Enter で確定 {#s1-note-created-happy}

### Given {#s1-given}

- アプリ起動済み、保存先は OS 慣習パス
- フィードに既存 Note は 0 件
- Draft 入力欄が空

### When {#s1-when}

1. `Cmd+N` で Draft 入力欄にフォーカス
2. `"docs を書く"` を入力
3. `Cmd+Enter` を押下

### Then {#s1-then}

- `Note::create(body="docs を書く", tags=∅, now=t0)` が実行される
- `storageDir/20260620120000.md` が書き出される（frontmatter: `tags: []`, `createdAt`, `updatedAt`）
- event **NoteCreated** `{ note_id: 20260620120000, created_at: t0, initial_tags: ∅ }` 発行
- NoteFeed: 表示 1 件、フィード最上部に新規ブロック
- UI: Draft 入力欄がクリア、新規ブロックへフォーカス遷移

## Scenario S2: 既存 Note 編集後 500ms debounce で AutoSave {#s2-autosave-debounce}

### Given {#s2-given}

- 既存 Note A (`body="hello"`, `updatedAt=t0`)
- ブロック A が EDITING 状態

### When {#s2-when}

1. `t1` 時点で `"hello world"` まで追記
2. キー入力を止め、500ms 経過（`t1 + 0.5s = t2`）

### Then {#s2-then}

- `Note::edit_body(new_body="hello world", now=t2)` が実行される
- `storageDir/<id>.md` の body と `updatedAt: t2` が永続化
- event **NoteBodyEdited** `{ note_id, updated_at: t2 }` 発行
- NoteFeed: updatedAt sort 時、A が最上部へ移動

## Scenario S3: フォーカス喪失で即時 Flush（debounce 待たず） {#s3-flush-on-blur}

### Given {#s3-given}

- 既存 Note A、EDITING 状態
- `t1` に編集（`AutoSave` debounce timer は 500ms 待ち中）

### When {#s3-when}

1. `t1 + 0.2s = t2` 時点でユーザが別ブロックをクリック
2. ブロック A は EDITING → IDLE 遷移、フォーカス喪失

### Then {#s3-then}

- debounce timer をキャンセル
- 即時 `Note::edit_body(now=t2)` を実行（Flush）
- event **NoteBodyEdited** `{ note_id, updated_at: t2 }` 発行
- ウィンドウ blur / アプリ quit でも同じ流れ（Q4 決定: 3 種の Flush トリガー）

## Scenario S4: タグ付与の正規化と重複排除 {#s4-tag-assign-normalize}

### Given {#s4-given}

- Note A (`tags=["gpt"]`)
- ユーザが入力欄に `"  GPT  "` を入力して Enter

### When {#s4-when}

- `Tag::new("  GPT  ")` でコンストラクト
- `Note::assign_tag(tag)` を実行

### Then {#s4-then}

- Tag 構築時に正規化: `"gpt"` (lowercase + trim)
- TagSet に同一 `name` が既存（I-N5）→ assign は **no-op**
- event **NoteTagsChanged** は発行しない（変化がないため）
- 補足: assign により tags が変化した場合のみ NoteTagsChanged を発行する

## Scenario S5: 削除 → トースト中に Undo 復元 {#s5-delete-undo-in-window}

### Given {#s5-given}

- Note A が表示されている
- 削除トースト未表示

### When {#s5-when}

1. `t0` にユーザがホバー → 削除ボタンクリック
2. `t0 + 2s = t1` に「元に戻す」ボタンをクリック

### Then {#s5-then}

- `t0`: `Note::delete_to_trash()` 実行
  - OS ゴミ箱へ移動成功
  - event **NoteDeletedToTrash** `{ note_id, original_path, deleted_at: t0 }` 発行
  - NoteFeed: 表示から除外
  - UI: 「元に戻す」トースト表示（仮 5 秒）
  - Application service: `DeletedNote { id, original_path }` を 1 件保持
- `t1`: `DeletedNote::restore()` 実行
  - OS ゴミ箱から原 path に復帰
  - event **NoteRestoredFromTrash** `{ note_id, restored_at: t1 }` 発行
  - NoteFeed: 表示に再登場
  - UI: トーストを閉じる

## Scenario S6: 連続削除で Toast がスタック、両方とも独立 Undo 可能 {#s6-delete-replace}

### Given {#s6-given}

- Note A, B が表示
- どちらも未削除
- Undo スタックは空

### When {#s6-when}

1. `t0` に A を削除
2. `t0 + 1s = t1` に B を削除（A の Undo トースト表示中）
3. `t1 + 1s = t2` に A の Toast 内「元に戻す」ボタンをクリック
4. `t2 + 1s = t3` に B の Toast 内「元に戻す」ボタンをクリック

### Then {#s6-then}

- `t0`:
  - A の **NoteDeletedToTrash** 発行
  - A の Toast が画面下部に表示
  - Undo スタック: `[DeletedNote(A)]`
- `t1`:
  - B の **NoteDeletedToTrash** 発行
  - B の Toast を **A の上に積む** (縦パイル、最新が上)
  - Undo スタック: `[DeletedNote(A), DeletedNote(B)]`
  - A の Toast は維持 (置換されない、独立 TTL)
- `t2`: A の Toast の Undo クリック → A が復元
  - event **NoteRestoredFromTrash** `{ note_id: A, ... }`
  - Undo スタックから A を除去: `[DeletedNote(B)]`
  - B の Toast は表示維持 (有効期間内)
- `t3`: B の Toast の Undo クリック → B が復元
  - event **NoteRestoredFromTrash** `{ note_id: B, ... }`
  - Undo スタックから B を除去: `[]`
- 補足: A の Toast が `t0 + 5s` に時間切れで消えた場合は A のみが破棄され、
  B の Toast / Undo は影響を受けない（各 Toast は独立 TTL）

## Scenario S7: Toast 消失後の Undo は no-op (per-toast 個別) {#s7-undo-after-toast}

### Given {#s7-given}

- Note A を `t0` に削除、Toast 表示中
- Undo スタック: `[DeletedNote(A)]`

### When {#s7-when}

1. `t0 + 5s = t1`（仮 Toast 有効期間）で A の Toast が消失
2. `t1 + 1s = t2` に何らかの方法で A に対する復元 API 呼び出し試行

### Then {#s7-then}

- `t1`: A の Toast 消失と同時に application service が Undo スタックから A を除去
  - Undo スタック: `[]`
- `t2`: 復元 API は **対応する DeletedNote がスタックに無い** ため reject
  - event **NoteRestoredFromTrash** は発行されない
  - workflow restore-deleted-note は `NoUndoAvailable` を返す
- A は OS ゴミ箱に残る（アプリ責務外）
- 補足: 他の Toast (例: B の Toast が同時に存在) がある場合、B の Undo は
  影響を受けず引き続き有効（per-toast 独立性）

## Scenario S8: 検索文字列の NFKC + lowercase 正規化 {#s8-query-normalize}

### Given {#s8-given}

- Note A (`body="GPT を試す"`), Note B (`body="ｇｐｔ のメモ"` 全角)
- 検索バーは空

### When {#s8-when}

- 検索バーに `"gpt"` を入力（半角）

### Then {#s8-then}

- `NoteFeed::filter_by_query("gpt")`:
  - 入力を NFKC (compatibility normalization) + lowercase: `"gpt"` （変化なし）
- NoteFeed.visible_notes:
  - A の body を NFKC + lowercase: `"gpt を試す"` → match
  - B の body を NFKC + lowercase: `"gpt のメモ"`（全角 → 半角化、NFKC により互換等価変換） → match
  - 両者表示
- event は発行されない（NoteFeed は read model）

## Scenario S9: 同一 body の AutoSave は重複 event を出さない {#s9-idempotent-autosave}

### Given {#s9-given}

- Note A (`body="hello"`, `updatedAt=t0`)

### When {#s9-when}

1. ユーザがブロックをクリックして EDITING 状態に
2. body を一切変更せず 500ms 経過

### Then {#s9-then}

- AutoSave 経路は body 変化を検知 → **何もしない**
- `Note::edit_body` は呼び出されない
- event **NoteBodyEdited** は発行されない
- `updatedAt = t0` のまま

補足: 「同一 body 編集」を application service レベルで弾く方針。
Note Aggregate 自体は呼ばれれば `updatedAt` を更新する（I-N4 通り）。

## Scenario S10: 禁止文字を含む Tag は構築時に reject {#s10-tag-invalid-char}

### Given {#s10-given}

- Note A、タグ編集モード

### When {#s10-when}

- ユーザが `"foo,bar"`（カンマ含む）を入力

### Then {#s10-then}

- `Tag::new("foo,bar")` が `TagError::InvalidChar` を返す（I-N6）
- `Note::assign_tag` には到達しない
- event は発行されない
- UI: エラーメッセージ表示（禁止文字 ` `, `\t`, `\n`, `,`, `[`, `]`）

## Scenario S11: storage_dir 変更は再起動要求のみ {#s11-storage-dir-change}

### Given {#s11-given}

- Settings (`storage_dir = /old/path`)
- フィードに Note 3 件表示中

### When {#s11-when}

1. 設定モーダルで `storage_dir = /new/path` に変更して保存

### Then {#s11-then}

- `Settings::change_storage_dir(new_dir)` 実行
- `app_config_dir/settings.json` に永続化
- event **StorageDirChanged** `{ old_dir: /old/path, new_dir: /new/path }` 発行
- UI: 再起動を促すモーダル表示（I-S4）
- フィードの 3 件は **表示されたまま**（`/old/path` の Note を見続ける）
- 再起動後に `/new/path` のスキャン結果が表示される

## Scenario S12: 起動時 filter リセット、sort 復元 {#s12-startup-state}

### Given {#s12-given}

- 前回終了時:
  - 検索バー: `"gpt"`
  - TagFilter: `coding`
  - SortOrder: `{ updatedAt, asc }`
  - Settings 永続化済み

### When {#s12-when}

- アプリ再起動

### Then {#s12-then}

- Settings::load_or_default で `sort_preference = { updatedAt, asc }` を取得
- NoteFeed 初期化:
  - filter は **空**（query=None, date_range=All, tag=None）— Q3 決定: 揮発
  - sort = `{ updatedAt, asc }` — Settings から復元
- 全 Note が updatedAt 昇順で表示

## Scenario S13: アプリ quit 時の連続 Flush {#s13-quit-flush}

### Given {#s13-given}

- Note A, B, C が EDITING 状態（複数ブロック編集を許容する場合の想定）
- いずれも AutoSave debounce timer 中

### When {#s13-when}

- ユーザが Cmd+Q で quit

### Then {#s13-then}

- quit シグナル受信 → 全 EDITING ブロックを Flush
- `Note::edit_body` を A, B, C の順に同期実行
- event **NoteBodyEdited** が A, B, C 順に連続発行
- NoteFeed の購読は冪等（1 個ずつ処理しても結果は同じ、domain-events.md Notes 参照）
- quit 完了まで永続化を待つ（最大欠損 500ms を許容、event-storming Q4 補足）

## Scenario S14: 新バージョン検出失敗は silent {#s14-update-check-failure}

### Given {#s14-given}

- アプリ起動直後
- ネットワーク断

### When {#s14-when}

- `UpdateChannel::check_at_startup()` 実行

### Then {#s14-then}

- HTTP 失敗 → `UpdateError` を application service が握り潰す
- event **NewVersionDetected** は **発行されない**（I-U3 / domain-events.md 参照）
- UI 通知なし、ログ出力のみ
- ユーザの作業を妨げない

## Scenario S15: 同一秒内の連続編集で updatedAt は変わらない {#s15-same-second-edits}

### Given {#s15-given}

- Note A (`updatedAt = 2026-06-20T12:00:00`)
- 秒精度の `OffsetDateTime` を使用

### When {#s15-when}

1. `2026-06-20T12:00:00.100` に edit_body
2. `2026-06-20T12:00:00.800` に edit_body（同一秒内）

### Then {#s15-then}

- 1 回目: `Note::edit_body(now=12:00:00)` → `updatedAt = 12:00:00`（変化なし）
  - I-N4 の補足通り「同一秒内は同じ値に留まる」
  - 永続化は実行される
  - event **NoteBodyEdited** 発行（updated_at = 12:00:00）
- 2 回目: 同上、`updated_at = 12:00:00`
- 購読側 (NoteFeed) は冪等処理: 2 回 sort 再計算しても結果は同じ

## Notes {#notes}

### シナリオ漏れの検出ポリシー {#notes-coverage-policy}

- 各 aggregate の **各 command を最低 1 シナリオでカバー** することを目標
- 各 event を **少なくとも 1 シナリオで発行する**
- error path は「禁止文字」「storage_dir 不可」「ネットワーク失敗」の代表 3 種で十分
  （全エラーの組合せは Phase 9 workflows で個別 workflow としてカバー）

### 同一 body の AutoSave 抑制の責務 {#notes-idempotent-save}

- S9 で application service レベルで弾く方針を明示
- Note Aggregate 自体は素朴に `updatedAt` を更新する設計を維持
- これは「ドメインを薄く保ち、IO 効率の最適化は application 層」という分離

### Phase 5 / 6 への手戻りは無し {#notes-no-regression}

- 15 シナリオすべてが既存の aggregate / event で表現できた
- ただし S4, S9 で「変化がない場合は event を発行しない」という暗黙の前提を
  application service 層に置く必要性を確認

## Open Questions {#open-questions}

Phase 7 時点で未決事項はない。

- Phase 9 (workflows) で S5 (Undo) と S6 (連続削除) を正式な workflow として再記述する
- Phase 9 で「複数ブロック同時 EDITING を許容するか」を確定（S13 の前提）
  - 現状の spec は 1 ブロック EDITING を想定。S13 は将来拡張に対する保険として保持
