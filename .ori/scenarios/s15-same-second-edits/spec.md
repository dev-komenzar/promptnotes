---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s15-same-second-edits
      hash: 4a02c17cc025
    - path: domain/workflows/auto-save-note.md#auto-save-note
      hash: 642c5094fd1a
    - path: domain/aggregates.md#note-aggregate
      hash: 56f7a54a8ab2
    - path: domain/domain-events.md#note-body-edited
      hash: 71db66eafe03
    - path: domain/aggregates.md#note-feed-aggregate
      hash: 56f7a54a8ab2
    - path: domain/glossary.md#glossary-timestamp
      hash: 9405b16bb835
---

# s15-same-second-edits — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s15-same-second-edits phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

`Note::edit_body(new_body, now)` において `now` が **秒精度**（`Timestamp`）に truncate される
ため、**同一秒内に連続して body を編集しても `updatedAt` は同じ値に留まる**（I-N4 補足）ことを
E2E で検証する scenario。

ただし S9（同一 body の AutoSave は no-op）とは逆に、**body 自体は変化している**ため:

- `Note::edit_body` は呼ばれる（`AutoSaveNoteUseCase` の冪等ガードは body 同値時のみ no-op）
- `NoteRepository::write` による **永続化は実行される**（`.md` の body が更新される）
- event **NoteBodyEdited**（`updated_at = 12:00:00`）が **発行される**
- 結果として `updatedAt` の値だけが変化しない

購読側 NoteFeed は `updatedAt` 秒精度で de-duplicate し、sort を再計算しても結果は同じ
（決定論的: I-F3 の `id` tiebreak）である。つまり「秒内連続編集」は UI / フィードに
並び替えのゆらぎを与えない。

- 対応 workflow: `domain/workflows/auto-save-note.md#auto-save-note`
  （step 5 `updateBody: Note::edit_body(new_body, now)` / step 6 `persist` / step 7 `emit`）
- 対応 aggregate: `domain/aggregates.md#note-aggregate`
  （公開操作 `Note::edit_body` は `updatedAt = now` に更新。`now` は `Timestamp` = 秒精度）
- 対応 event: `domain/domain-events.md#note-body-edited`
  （Trigger は「`Note::edit_body` の永続化成功時」。同一 Note への連続発行は `updated_at`
  秒精度で de-duplicate される）
- 対応 aggregate (read side): `domain/aggregates.md#note-feed-aggregate`
  （I-F3: 同一 sort key は `id` で tiebreak → 再 sort が冪等）
- 対応語彙: `domain/glossary.md#glossary-timestamp`
  （`Timestamp` は秒精度。同一秒内の連続編集では同値）
- 対応 page: `page-main`（AutoSave 経路の composition root。本 scenario は Tauri 境界 invoke で
  AutoSave を再現する）

> domain/validation.md#s15-same-second-edits より:

- Given: Note A (`updatedAt = 2026-06-20T12:00:00`) / 秒精度の `OffsetDateTime` を使用
- When:
  1. `2026-06-20T12:00:00.100` に edit_body
  2. `2026-06-20T12:00:00.800` に edit_body（同一秒内）
- Then:
  - 1 回目: `Note::edit_body(now=12:00:00)` → `updatedAt = 12:00:00`（変化なし）
    - I-N4 の補足通り「同一秒内は同じ値に留まる」
    - 永続化は実行される
    - event **NoteBodyEdited** 発行（updated_at = 12:00:00）
  - 2 回目: 同上、`updated_at = 12:00:00`
  - 購読側 (NoteFeed) は冪等処理: 2 回 sort 再計算しても結果は同じ

> domain/workflows/auto-save-note.md#notes より:
> 「同一秒内の連続編集は `updated_at` が変わらない（I-N4 補足、S15）。ただし永続化と event 発行は
> 実行される（body は変わっているため）」

> domain/aggregates.md#note-aggregate-invariants より:
> 「**I-N4**: `body` を変更する操作は `updatedAt` を「現在時刻 (秒精度)」に更新する
>   - 同一秒内の連続編集では `updatedAt` は同じ値に留まる（時計の解像度で十分）」

> domain/domain-events.md#note-body-edited-timing より:
> 「同期。同一 Note への連続発行は `updated_at` 秒精度で de-duplicate される。」

## シナリオステップ {#scenario-steps}

### Given {#given}

1. 実 user 環境から隔離した temp dir に Note A
   (`id=20260620120000`, `createdAt=updatedAt=20260620120000`, `body="hello"`) を seed 済み。
   Tauri App（test build）が起動済み
2. AutoSave 経路（`auto_save_note`）の `Clock` が **固定秒** を返す状態にある
   （[#impl-notes](#impl-notes) の debug-only clock seam で再現。実 production の
   `SystemClock` 挙動は変更しない）

### When {#when}

1. `now = 2026-06-20T12:00:00.100`（seed と同一秒）の状態で `auto_save_note`
   （body 変更 → `Note::edit_body`）を 1 回実行する
2. `now = 2026-06-20T12:00:00.800`（依然として同一秒）の状態で `auto_save_note`
   （body をさらに変更 → `Note::edit_body`）をもう 1 回実行する

### Then {#then}

- 1 回目: `outcome=saved` かつ `updated_at = 2026-06-20T12:00:00Z`。`.md` の `body` は更新され、
  `updatedAt` は seed 値 `20260620120000` のまま（= 秒精度 truncate により同値）
- 2 回目: 同上（`outcome=saved` / `updated_at = 2026-06-20T12:00:00Z` / `.md` の `updatedAt` 不変）
- event **NoteBodyEdited**（`updated_at = 12:00:00`）が各回で発行される
  （production 配線は `NoOpBus` のため E2E 直接観測は不可。`outcome=saved` が
  `publish` と同一分岐である点 + slice unit test で担保 — [#impl-notes](#impl-notes) 参照）
- 購読側 NoteFeed は冪等: `list_notes` を 2 回実行しても同一順序・同一 `updated_at` を返す

## テスト観点 {#test-points}

- **TP1（核）— 同一秒内の 1 回目編集で `updatedAt` は変わらないが永続化は実行される**:
  seed と同一秒の `now`（`12:00:00.100`）で `auto_save_note` を invoke すると
  `outcome=saved` かつ `updated_at = "2026-06-20T12:00:00Z"` を返す。`.md` は
  `body` が変更後（`hello v1`）に更新されつつ `updatedAt` は `20260620120000` のまま
  （= write が実行されたことは body 差分で観測、値の据え置きは I-N4 を実測）
- **TP2（核）— 同一秒内の 2 回目編集も同値を維持**:
  `now`（`12:00:00.800`）で再度 invoke しても `outcome=saved` /
  `updated_at = "2026-06-20T12:00:00Z"`。`.md` の `body` は `hello v2` に更新、
  `updatedAt` は `20260620120000` のまま
- **TP3（陽性対照 / 非空虚性）— 秒が変われば `updatedAt` は更新される**:
  `now = 12:00:01` に進めて invoke すると `outcome=saved` /
  `updated_at = "2026-06-20T12:00:01Z"` を返し、`.md` の `updatedAt` が
  `20260620120001` に変化する。これにより TP1/TP2 の「値が据え置き」が
  「そもそも更新されない実装」ではなく「秒精度 truncate による同値」であることを示す
- **TP4 — NoteFeed は冪等**: 2 回の同一秒編集後、`list_notes` を 2 回 invoke すると
  同一順序・同一 `updated_at`（`2026-06-20T12:00:00Z`）を返す（I-F3 の `id` tiebreak により
  再 sort が決定論的）。seed 済み Note B（別秒）との相対順序も不変
- **`auto-save-note` slice unit test との分担**: 本 scenario は Tauri 境界 + Runner + `.md`
  永続化までの縦断を検証する。`Note::edit_body` の秒 truncate と `NoteBodyEdited`
  payload の `updated_at` は slice unit test（`auto_save_note/tests.rs`）が担保する

## 実装ノート {#impl-notes}

- **runner**: `wdio`（優先チェーン step 2 — manifest に `runner:` 明示なし。参加 app
  `promptnotes` は `local` 系で `runtime.runner = wdio`。`.ori/architecture.md` の
  `scenario_test_runner: wdio` とも一致）
- **infrastructure**: `promptnotes` のみ（`mode: local`）。compose-service 系 app が参加しない
  ため docker-compose は不要
- **runner config**: `wdio.conf.ts` が `/ori-generate` により生成される。
  `tauri:options.application` は runtime block の `binary` =
  `apps/promptnotes/src-tauri/target/debug/app`（build-then-test）
- **固定秒の再現（test seam）**: 「同一秒内」は実時計では非決定論的なので、**debug build 限定の
  clock override** を `auto_save_note` の `SystemClock` に追加する（`TAURI_TEST_FIXED_NOW_FILE`）。
  wdio `onPrepare` が temp dir に固定時刻ファイルを作り、テストがステップ間に RFC3339 値
  （`2026-06-20T12:00:00.100Z` / `...00.800Z` / `...01Z`）を書き換えて `now` を制御する:
  - ファイル内容が空 / `system` → 通常の `OffsetDateTime::now_utc()`
  - それ以外 → RFC3339 parse した時刻を `Timestamp::from_offset_datetime`（秒 truncate）で返す
  - production (release build) では `#[cfg(debug_assertions)]` により env / file を一切読まない
    = 本番挙動は不変
  この方法は notes.md#test-approach にも明記する
- **Tauri 境界 invoke**: `browser.tauri.execute(({ core }) => core.invoke('auto_save_note', ...))`
  （`@wdio/tauri-service` v1.4.0。`tauri-plugin-wdio` は debug build で有効。test build は
  `VITE_WDIO_TEST=1 bun run tauri build --debug --no-bundle` = `bun run build:test`）
- **event NoteBodyEdited の観測制約**: production の `auto_save_note` command は `NoOpBus` を
  注入しており（`commands.rs`「The Note Feed BC will subscribe here once it lands.」参照）、
  domain event を UI / E2E に露出しない。`AutoSaveNoteUseCase::execute` は step 6 persist 成功後
  step 7 で `publish` してから `Ok(Some(note))` を返すため、**`outcome=saved` は
  「persist + publish が実行された」と同値**である。event payload の `updated_at` が返却
  `updated_at` と同一式（`updated.updated_at()`）である点と合わせ、E2E では
  `outcome=saved` + `.md` 差分で間接検証し、直接観測は slice unit test に委ねる（S9 と同方針）
- **config dir / storage dir の隔離**: `wdio.conf.ts` の `onPrepare` が `XDG_CONFIG_HOME` /
  `XDG_DATA_HOME` と専用 `storage_dir` を temp dir に隔離する（実 user 環境を汚さない）
- **production 側の前提（本 scenario が満たすべき不変条件）**:
  1. `auto_save_note` Tauri command が `Result<AutoSaveOutcome, AutoSaveErrorDto>` を返し、
     body 変化時に `Saved { id, updated_at }` を返すこと
     — `src-tauri/src/note_capture/slices/auto_save_note/`
  2. `Timestamp::from_offset_datetime` が秒精度へ truncate すること
     （`note_capture/shared/types/timestamp.rs`）
  3. `Note::edit_body(new_body, now)` が `updated_at = now`（truncate 済み）を設定し、
     `NoteRepository::write` が `updatedAt` を `YYYYMMDDhhmmss` で永続化すること
     — `note_capture/slices/create_note/infrastructure.rs`
  4. `auto_save_note` の `SystemClock` が debug build で固定秒ファイルを参照すること
     — これが欠落すると「同一秒」を再現できず S15 は RED になる
