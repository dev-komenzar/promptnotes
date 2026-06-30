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

## Scenario S16: 外部プログラムが `.md` を新規作成 → NoteFeed に自動反映 {#s16-external-file-created}

### Given {#s16-given}

- アプリ起動済み、ファイルウォッチャー稼働中
- `storage_dir` に Note A のみ存在（フィード表示 1 件）
- ユーザは別プログラム（vim 等）で `storage_dir/` を開いている

### When {#s16-when}

1. `t0` に外部プログラムが `storage_dir/20260630120000.md` を作成
   （frontmatter: `tags: [rust]`, body: `"外部から作成"`）
2. ファイルウォッチャーが作成を検知（debounce 500ms 後、`t0 + 0.5s = t1`）
3. infrastructure 層が `.md` を parse、Note 構築に成功
4. event **NoteFileCreatedExternally** `{ note_id: 20260630120000, note, file_path, detected_at: t1 }` 発行

### Then {#s16-then}

- NoteFeed: `upsert_note(note)` で `source` に Note を追加（I-F8）
- フィード表示が 1 件 → 2 件に更新
- UI 通知は不要（フィードの自然な更新で十分）
- 現在の filter/sort が維持されたまま新規 Note が表示される
- 補足: parse 失敗時（malformed frontmatter 等）は event を発行せず skip

## Scenario S17: 外部プログラムが `.md` を変更（競合なし）→ 自動反映 {#s17-external-file-modified-no-conflict}

### Given {#s17-given}

- Note A (`body="hello"`, `updatedAt=t0`) が表示されている
- Note A のブロックは **IDLE 状態**（編集中ではない）
- ファイルウォッチャー稼働中

### When {#s17-when}

1. `t1` に外部プログラム（Syncthing 経由等）が `storage_dir/<A.id>.md` の
   body を `"hello world"` に変更
2. ファイルウォッチャーが変更を検知（debounce 500ms 後、`t1 + 0.5s = t2`）
3. infrastructure 層が `.md` を再 parse、`BodyHash` を計算
4. event **NoteFileModifiedExternally**
   `{ note_id: A.id, disk_body_hash, note, file_path, detected_at: t2 }` 発行

### Then {#s17-then}

- フロントエンドが event 受信 → Block A の状態を確認 → **IDLE** のため競合なし
- NoteFeed: `upsert_note(note)` で source 内の Note A を差し替え（I-F8）
- フィード表示が `"hello world"` に更新される
- `updatedAt` ソート時、表示順が変わる可能性がある
- event **NoteBodyEdited** は発行されない（これは外部変更であり、アプリ内編集ではない）

## Scenario S18: 外部プログラムが `.md` を削除 → NoteFeed から除去 {#s18-external-file-deleted}

### Given {#s18-given}

- Note A が表示されている（IDLE 状態）
- ファイルウォッチャー稼働中

### When {#s18-when}

1. `t0` に外部プログラムが `storage_dir/<A.id>.md` を削除
2. ファイルウォッチャーが削除を検知（debounce 500ms 後、`t0 + 0.5s = t1`）
3. infrastructure 層がファイル名から `NoteId` を解決（`^\d{14}$` に一致）
4. event **NoteFileDeletedExternally**
   `{ note_id: A.id, file_path, detected_at: t1 }` 発行

### Then {#s18-then}

- NoteFeed: `remove_note(&A.id)` で source から除外（I-F8）
- フィード表示から Note A が消える
- UI 通知は不要（自然な更新）
- Note A が EDITING 状態だった場合は S20 の挙動に従う
- 補足: ファイル名が `^\d{14}$` に一致しない場合（非 Note ファイル）は event 非発行

## Scenario S19: 編集中ノートが外部変更された → 競合ダイアログ {#s19-external-modify-while-editing}

### Given {#s19-given}

- Note A (`body="hello"`, `body_hash=H1`) をユーザが **EDITING 状態** で編集中
- ユーザは body を `"hello 編集中"` に変更済み（AutoSave 未発火）
- ファイルウォッチャー稼働中

### When {#s19-when}

1. `t1` に外部プログラム（Syncthing 経由等）が `storage_dir/<A.id>.md` の
   body を `"hello world"` に変更
2. ファイルウォッチャーが変更を検知 → **NoteFileModifiedExternally** 発行
   `{ note_id: A.id, disk_body_hash: H2, note, ... }`
3. フロントエンドが event 受信 → Block A の状態を確認 → **EDITING** を検出
4. `Note::is_stale(&H2)` を呼出 → `H1 ≠ H2` → `true`（I-N9）

### Then {#s19-then}

- フロントエンドが**競合ダイアログ**を表示:
  - 「外部でこのノートが変更されました」
  - 選択肢: 「外部変更を適用」（編集中の内容は破棄）、「編集中を保持」（外部変更を無視）
- ユーザが「外部変更を適用」を選択:
  - 編集中の内容を破棄し、ディスクの内容で Note A を置換
  - NoteFeed: `upsert_note(note)` 実行
  - Block は IDLE 状態に遷移
- ユーザが「編集中を保持」を選択:
  - 外部変更を無視、編集中の内容を維持
  - NoteFeed への upsert は行わない（ただし次回の AutoSave/Flush 時に上書きされる）
  - Block は EDITING 状態を維持
- 補足: ダイアログ表示中もファイルウォッチャーは稼働継続（後続の変更も検知）

## Scenario S20: 編集中ノートが外部削除された → 通知 {#s20-external-delete-while-editing}

### Given {#s20-given}

- Note A をユーザが **EDITING 状態** で編集中
- ユーザは body を `"hello 編集中"` に変更済み

### When {#s20-when}

1. `t0` に外部プログラムが `storage_dir/<A.id>.md` を削除
2. ファイルウォッチャーが削除を検知 → **NoteFileDeletedExternally** 発行
   `{ note_id: A.id, ... }`
3. フロントエンドが event 受信 → Block A の状態を確認 → **EDITING** を検出

### Then {#s20-then}

- フロントエンドが通知を表示:
  - 「編集中のノートが外部で削除されました」
  - 選択肢: 「新規ファイルとして保存」（現在の内容で `.md` を再作成）、「破棄」（編集を諦める）
- ユーザが「新規ファイルとして保存」を選択:
  - 現在の編集中内容で `storage_dir/<A.id>.md` を再作成
  - NoteFeed: `upsert_note(note)` 実行
  - Block は IDLE 状態に遷移
- ユーザが「破棄」を選択:
  - NoteFeed: `remove_note(&A.id)` 実行
  - 編集中の内容は破棄
- NoteFeed の `remove_note` は S18 と同様に発動するが、
  EDITING 検出がある場合は上記の通知が優先される

## Scenario S21: Syncthing 一括同期（複数ファイル変更）→ debounce + 個別処理 {#s21-batch-sync-debounce}

### Given {#s21-given}

- Note A, B, C が表示されている（いずれも IDLE 状態）
- ファイルウォッチャー稼働中（debounce 500ms）
- 別デバイスで Note A, B, C の 3 ファイルすべてが変更され、
  Syncthing が一括同期を開始

### When {#s21-when}

1. `t0`〜`t0 + 0.1s` の間に `A.md`, `B.md`, `C.md` の 3 ファイルが
   連続して変更される（Syncthing の高速転送）
2. ファイルウォッチャーが `A.md`, `B.md`, `C.md` の変更イベントを
   ほぼ同時に受信（それぞれに debounce 500ms が適用）
3. `t0 + 0.5s = t1` に A の debounce が完了 → **NoteFileModifiedExternally(A)** 発行
4. `t0 + 0.6s = t2` に B の debounce が完了 → **NoteFileModifiedExternally(B)** 発行
5. `t0 + 0.7s = t3` に C の debounce が完了 → **NoteFileModifiedExternally(C)** 発行

### Then {#s21-then}

- NoteFeed は各 event を順次処理:
  - `t1`: `upsert_note(A)` → source 内の A を更新
  - `t2`: `upsert_note(B)` → source 内の B を更新
  - `t3`: `upsert_note(C)` → source 内の C を更新
- 各 `upsert_note` は独立かつ冪等（I-F8）
- フィードは 3 回の部分更新が行われる（全体再ハイドレートは不要）
- `visible_notes` の結果は各 upsert 後に再計算されるが、
  フィルター・ソート条件が変わらなければ最終結果は S16〜S18 の逐次適用と同じ
- 補足: Syncthing の `.tmp` ファイル → rename パターンは
  infrastructure 層の watcher が `.tmp` を無視し、rename 完了後の `.md` のみ処理する

## Scenario S22: storage_dir 変更 → ファイルウォッチャー再起動 {#s22-storage-dir-change-watcher-restart}

### Given {#s22-given}

- Settings (`storage_dir = /old/path`)、ファイルウォッチャーが `/old/path` を監視中
- フィードに `/old/path` の Note 3 件表示中

### When {#s22-when}

1. 設定モーダルで `storage_dir = /new/path` に変更して保存
2. event **StorageDirChanged** `{ old_dir: /old/path, new_dir: /new/path }` 発行
3. Infrastructure 層（ファイルウォッチャー）が subscriber として受信

### Then {#s22-then}

- `/old/path` のファイルウォッチャーを停止（`WatcherHandle` の Drop）
- `/new/path` で新規ファイルウォッチャーを起動
  - 監視対象: `/new/path/*.md`
  - debounce: 500ms（S21 と同じ）
- ウォッチャー再起動後、`/new/path` の変更が検知対象になる
- UI: 再起動を促すモーダル表示（S11 と同様、I-S4）
- ウォッチャー再起動に失敗した場合:
  - infrastructure 層が retry（最大 3 回、1 秒間隔）
  - 全 retry 失敗時はユーザにアプリ再起動を促す
- 補足: このシナリオは S11 を拡張したもの。
  S11 の `StorageDirChanged` subscriber に infrastructure 層が追加された形

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

- 22 シナリオすべてが既存の aggregate / event で表現できた
- ただし S4, S9 で「変化がない場合は event を発行しない」という暗黙の前提を
  application service 層に置く必要性を確認

### 外部変更の競合検出と編集中判定 {#notes-external-change-conflict}

- S19/S20 で、編集中判定はフロントエンドの Block ステートマシン
  （IDLE/FOCUSED/EDITING）を活用する方針を確定
- `NoteFileModifiedExternally` 受信時にフロントエンドが EDITING 状態を確認し、
  競合時のみダイアログを表示する
- これにより Note Aggregate / NoteFeed Aggregate に `is_editing` フラグを
  追加する必要はなく、既存の UI 状態管理を再利用できる
- ただし、将来的に複数デバイス間での編集中状態の共有が必要になった場合、
  この判定ロジックを backend に移す可能性がある（現時点では単一デバイス前提）

## Open Questions {#open-questions}

Phase 7 改訂（外部ファイル変更検知シナリオ追加に伴う）:

- S19/S20 の競合ダイアログ UI の具体的なデザイン（モーダルかトーストか、
  ボタン配置等）→ Phase 11a (UI fields) で確定
- S21 の debounce 500ms で Syncthing の全同期パターンをカバーできるか
  → 実装時に実機検証。現時点では設計上の仮定
- Phase 9 (workflows) で S5 (Undo) と S6 (連続削除) を正式な workflow として再記述する
- Phase 9 で「複数ブロック同時 EDITING を許容するか」を確定（S13 の前提）
  - 現状の spec は 1 ブロック EDITING を想定。S13 は将来拡張に対する保険として保持
