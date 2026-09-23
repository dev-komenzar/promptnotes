# s15-same-second-edits — Scenario implementation notes

## 概要 {#overview}

`Note::edit_body(new_body, now)` の `now` は **秒精度 `Timestamp`** に truncate されるため、
同一秒内の連続 body 編集では `updatedAt` が同じ値に留まる（I-N4 補足）ことを E2E で検証する。
ただし S9（同一 body → no-op）とは逆に body は変化しているため、**永続化と `NoteBodyEdited`
発行は実行される**。その結果 `updatedAt` の値だけが据え置かれる、という点が S15 の核。

## テスト方式: debug clock seam + Tauri 境界 invoke {#test-approach}

- **「同一秒内」の再現課題**: 実時計（`SystemClock` = `OffsetDateTime::now_utc()`）では、
  2 回の編集が同一秒に収まるかは非決定論的。実装を待つと flaky になる。
- **採用した方法**: **debug build 限定の clock override seam** を `auto_save_note` の
  `SystemClock` に追加し、`now` を固定秒に pin する。
  - seam: 環境変数 `TAURI_TEST_FIXED_NOW_FILE`（`apps/promptnotes/src-tauri/src/
    note_capture/slices/auto_save_note/commands.rs`）。ファイルに RFC3339
    （例 `2026-06-20T12:00:00.100Z`）を書くと、その時刻を `Timestamp::from_offset_datetime`
    （秒 truncate）で返す。空 / `system` / 不正値は実時計にフォールバック（warn ログ）。
  - `#[cfg(debug_assertions)]` ガードにより **release build では分岐ごと compile out** され、
    本番の `SystemClock` 挙動は不変。
  - `wdio.conf.ts` の `onPrepare` が temp dir に固定時刻ファイルを作り、
    `TAURI_TEST_FIXED_NOW_FILE` を app に渡す。テストはステップ間にファイルを書き換えて
    `now` を `12:00:00.100` → `12:00:00.800` → `12:00:01` と制御する。
- **Tauri 境界 invoke**: `browser.tauri.execute((tauri, id, body) => tauri.core.invoke(
  'auto_save_note', { noteId: id, newBody: body }), noteId, body)`（`@wdio/tauri-service` v1.4.0。
  callback の引数は **execute の trailing args で渡す** — closure 変数はページ文脈に
  シリアライズされないため参照できない）。
- **陽性対照 (step 4)**: `now` を次の秒 `12:00:01` に進めると `updatedAt` が進むことを確認する。
  これにより step 1-2 の「値が据え置き」が「そもそも更新されない実装」ではなく
  「秒精度 truncate による同値」であることを示す（非空虚性）。

## テストカバレッジ {#coverage}

| 検証項目 | 検証方法 |
|---|---|
| 同一秒 1 回目は永続化するが `updatedAt` は据え置き（核 / I-N4） | E2E step 1（`now=12:00:00.100` → `outcome=saved` / `updated_at=12:00:00Z` / `.md` body=`hello v1` / `updatedAt=20260620120000`） |
| 同一秒 2 回目も同値（核 / I-N4） | E2E step 2（`now=12:00:00.800` → `outcome=saved` / `updated_at=12:00:00Z` / `.md` body=`hello v2` / `updatedAt=20260620120000`） |
| NoteFeed の再 sort が冪等（I-F3） | E2E step 3（`list_notes` ×2 が同一順序 `[A, B]`・同一 `updated_at` を返す） |
| 陽性対照（非空虚性）: 秒が変われば `updatedAt` が進む | E2E step 4（`now=12:00:01` → `updated_at=12:00:01Z` / `.md` `updatedAt=20260620120001`） |
| persist + publish の実行（`NoteBodyEdited` 発行） | `outcome=saved` は use case の step 6 persist → step 7 publish と同一分岐（下記 §event 参照） |
| `Note::edit_body` の秒 truncate / event payload | slice unit test（`auto_save_note/tests.rs`）に委譲 |

## event NoteBodyEdited の観測制約 {#event-observation}

production の `auto_save_note` command は `NoOpBus` を注入している
（`commands.rs`: 「The Note Feed BC will subscribe here once it lands.」）ため、E2E で domain
event を直接観測する対象は無い。`AutoSaveNoteUseCase::execute` は step 6 persist 成功後
step 7 で `bus.publish(NoteBodyEdited { updated_at })` を呼んでから `Ok(Some(note))` を返すので、
**`outcome=saved` は「persist + publish が実行された」と同値**。event payload の `updated_at` は
返却 `updated_at` と同一式（`updated.updated_at()`）。したがって E2E では `outcome=saved` と
`.md` 差分で間接検証し、event 自体の直接観測は slice unit test（S9 と同方針）に委ねる。

## RED → GREEN 検証 {#red-green}

production 側は初回から S15 の不変条件（`Timestamp` の秒 truncate）を備えており、
**初回 E2E は GREEN（4 passing）**。テストが空振りでないことを示すため、秒 truncate を
一時的に無効化して RED を実測した:

- **変異（`Timestamp::from_offset_datetime` の `replace_nanosecond(0)` を除去）**:
  `bun run build:test` → E2E **2 failing / 2 passing**:
  - step 1: `Expected: "2026-06-20T12:00:00Z"` に対し `Received: "2026-06-20T12:00:00.1Z"`
  - step 2: `Expected: "2026-06-20T12:00:00Z"` に対し `Received: "2026-06-20T12:00:00.8Z"`
  - step 3 / step 4 は pass（file は `YYYYMMDDhhmmss` 書き出しのため sub-second が落ちる）
- **復元後**（`git checkout` + `bun run build:test`）→ E2E **4 passing (~1.3s)** で GREEN。

したがって本 scenario のテストは「秒精度 truncate」の欠落を実際に検出する。

## production 変更 {#production-change}

正味の変更は **1 file**:

- `apps/promptnotes/src-tauri/src/note_capture/slices/auto_save_note/commands.rs`

`SystemClock::now()` に debug-only の `TAURI_TEST_FIXED_NOW_FILE` 分岐を追加（30 行）。
`#[cfg(debug_assertions)]` により release build は compile out されるため本番挙動に差はない。
S14 の `TAURI_TEST_UPDATER_ENDPOINT` seam と同型。

> 注意: E2E は必ず `bun run build:test`
> （`VITE_WDIO_TEST=1 bun run tauri build --debug --no-bundle`）で作った binary に対して実行する
> こと。`cargo build` 単体では frontend の test setup（`@wdio/tauri-plugin` import）が含まれず
> `browser.tauri.execute` が機能しない。

回帰確認:

- frontend unit test: **143 passed / 15 files**
- Rust `note_capture` unit: **passed**（`cargo test` の詳細は review.md 参照）
- E2E 回帰: s14-update-check-failure（4 passing）/ s9-idempotent-autosave（3 passing）

## 既知の未対応 / 補足 {#known-issues}

- **event の直接観測は不可**: 上記 §event-observation の通り `NoOpBus` 配線のため。
  `/ori-propose` で prod wiring（`ori-znq` NoOpBus → real EventBus）が既に follow-up として
  起票済み。本 scenario のスコープ外。
- **固定時刻ファイルは 1 プロセス共有**: app プロセスは起動時に env を 1 度だけ継承するため、
  テストはファイル内容を書き換えて時刻を切り替える（env の再設定は不可）。
- **storage_dir / config 隔離**: `TAURI_TEST_STORAGE_DIR` override + `XDG_CONFIG_HOME` /
  `XDG_DATA_HOME` で実 user 環境を汚さない。`onComplete` で temp dir を `rmSync`。
