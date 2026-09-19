---
ori:
  schema:
    propagation_level: file
coherence:
  source: derived
  upstream:
    - path: domain/validation.md#s10-tag-invalid-char
      hash: 4a02c17cc025
    - path: domain/workflows/assign-tag.md#assign-tag
      hash: 4efe2dfe63c4
    - path: domain/aggregates.md#note-aggregate
      hash: 56f7a54a8ab2
    - path: domain/domain-events.md#note-tags-changed
      hash: 71db66eafe03
---

# s10-tag-invalid-char — Scenario Specification

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-flow s10-tag-invalid-char phase=derive`. Use `/ori-sync` if you need to edit here directly; ori will create a proposal for the upstream review.

## 概要 {#overview}

タグ付与の入力経路において、**禁止文字を含む raw 文字列が `Tag` 構築時に reject される**ことを
検証する scenario。reject は `Tag::new`（domain）で発生し、`parseTag` ステップで `InvalidTag` と
して application service から返る。この経路では `Note` は load されず、`Note::assign_tag` にも
到達しないため、永続化も event 発行も発生しない。

- 対応 workflow: `domain/workflows/assign-tag.md`（Step 1 `parseTag` が `InvalidTag` を返すと
  step 2 `loadNote` 以降に進まない。`InvalidChar { raw }` は入力 raw 文字列全体を保持）
- 対応 aggregate: `domain/aggregates.md#note-aggregate`（Tag VO の construction 時に trim 後の
  空文字は `TagError::Empty`、禁止文字 ` `, `\t`, `\n`, `,`, `[`, `]` を含む入力は
  `TagError::InvalidChar { raw }` で reject — I-N6）
- 対応 event: `domain/domain-events.md#note-tags-changed`（Trigger は「`Note::assign_tag` /
  `Note::remove_tag` の永続化成功時」— reject 経路では発行されない）
- 対応 page: `page-main`（Block のタグ編集モード。`assign_tag` の `invalid_tag` を catch して
  エラーメッセージを表示する）

> domain/validation.md#s10-tag-invalid-char より:

- Given: Note A、タグ編集モード
- When: ユーザが `"foo,bar"`（カンマ含む）を入力
- Then:
  - `Tag::new("foo,bar")` が `TagError::InvalidChar` を返す（I-N6）
  - `Note::assign_tag` には到達しない
  - event は発行されない
  - UI: エラーメッセージ表示（禁止文字 ` `, `\t`, `\n`, `,`, `[`, `]`）

> domain/workflows/assign-tag.md#steps より:
> 「1. `parseTag: String → Result<Tag, InvalidTag>`」「lowercase + trim 適用 → 結果が空文字なら
> `Empty`」「禁止文字 (` `, `\t`, `\n`, `,`, `[`, `]`) を含めば `InvalidChar { raw }`（入力 raw 文字列
> 全体を payload に保持）」

> domain/workflows/assign-tag.md#notes より:
> 「`parseTag` 単独で reject 可能（Note を load する前にバリデーション）」

> domain/aggregates.md#note-aggregate より:
> 「**Tag** (VO) … construction 時に **trim 後の空文字** は `TagError::Empty` で、**禁止文字を
> 含む入力** は `TagError::InvalidChar { raw }` で reject」「**I-N6**: `Tag::name` は正規化規則
> （lowercase + trim、禁止文字排除）を必ず満たす」

> domain/domain-events.md#note-tags-changed-trigger より:
> 「`Note::assign_tag(tag)` または `Note::remove_tag(tag_name)` の永続化成功時」

## シナリオステップ {#scenario-steps}

### Given {#given}

1. Note A（ID: `20260620120000`、body: `"hello"`、`tags: []`、`updatedAt: 20260620120000` = t0）が
   表示されている
2. A の `.md` ファイルは `TAURI_TEST_STORAGE_DIR` に seed 済み
3. Tauri App が起動済み

### When {#when}

1. ユーザがブロック A をクリックして EDITING 状態に入る（タグ入力欄が表示される）
2. タグ入力欄に `"foo,bar"`（カンマ含む）を入力し Enter する

### Then {#then}

- `Tag::new("foo,bar")` が `TagError::InvalidChar { raw: "foo,bar" }` を返す（I-N6）
- `parseTag` が失敗し、application service は step 2 `loadNote` に進まない
  → `Note::assign_tag` には到達しない
- `.md` ファイルは不変（tags は `[]` のまま、`updatedAt = t0` のまま）
- event `NoteTagsChanged` は発行されない
- UI: エラーメッセージが表示される（禁止文字 ` `, `\t`, `\n`, `,`, `[`, `]`）
- UI: 禁止文字を含むタグはチップとして追加されない

### 補足（禁止文字集合全体の検証） {#when-forbidden-set}

validation の例は `,`（カンマ）だが、S10 の本質は「禁止文字集合 ` `, `\t`, `\n`, `,`, `[`, `]` の
いずれかを含む入力が構築時に reject される」こと。UI の `<input type="text">` は `\t` / `\n` を
直接入力できないため、UI 経由では `,` / `[` / `]` を、Tauri command 境界（`assign_tag` 直接
invoke）では集合全体 + 空文字（`TagError::Empty`）を検証する。

### 対照（正常タグの受入） {#when-contrast}

reject 経路が過剰一般化されていないことを示すため、正常タグを assign した場合は
`NoteTagsChanged` の永続化成功として `.md` の tags が更新され `updatedAt` が更新される。

## テスト観点 {#test-points}

- **初期状態**: A の Block が表示され、`.md` の tags が `[]`、`updatedAt` が `t0`
- **UI 経由 reject（本 scenario の核・UI 側）**: タグ入力欄に `"foo,bar"` を入力し Enter →
  `[data-testid="screen-1-block-tag-error"]` が表示され、文言に `invalid characters` を含む。
  タグチップは追加されず、`.md` の tags `[]` / `updatedAt = t0` が不変
- **Tauri 境界 reject（禁止文字集合全体）**: `assign_tag { rawTag }` を直接 invoke し、各禁止文字
  （`"foo bar"`, `"foo\tbar"`, `"foo\nbar"`, `"foo,bar"`, `"foo[bar"`, `"foo]bar"`）が
  `{ kind: "invalid_tag" }` で reject されること。`.md` は不変のまま
  → `Tag::new` の `InvalidChar` と「`Note::assign_tag` 非到達（永続化なし）」を Tauri 境界で観測
- **空文字の reject（補助）**: `assign_tag { rawTag: "   " }` が `{ kind: "invalid_tag" }`
  （`TagError::Empty`）で reject される
- **対照: 正常タグは永続化される**: `"valid"` を assign → `.md` の tags が `["valid"]`、
  `updatedAt` が `t0` から更新され、UI にチップ `valid` が現れる
- **event `NoteTagsChanged` 非発行**: 永続化成功時のみ発行されるため reject 経路では発行されない
  （`NoOpBus` の境界のため E2E では直接観測せず、`assign_tag` の reject と `.md` 不変で間接検証する。
  application service 単体検証は slice `assign-tag` の unit test が担当）

## 実装ノート {#impl-notes}

- **runner**: `wdio`（優先チェーン step 2 — manifest に `runner:` 明示なし。参加 app `promptnotes` は
  `local` 系で `runtime.runner = wdio`。`.ori/architecture.md` の `scenario_test_runner: wdio` とも一致）
- **infrastructure**: `promptnotes` のみ（`mode: local`）。compose-service 系 app が参加しないため
  docker-compose は不要
- **runner config**: `wdio.conf.ts` が `/ori-generate` により生成される
- **Note A の準備**: `wdio.conf.ts` の `onPrepare` が `TAURI_TEST_STORAGE_DIR` に Note A
  （`20260620120000`, body=`hello`, `tags: []`, frontmatter `updatedAt: 20260620120000`）を seed する
- **アサーション戦略**:
  - EDITING 遷移は Block の `[data-block-id="20260620120000"]` の `data-block-state` 属性で判定する
  - UI 経由の reject は `[data-testid="screen-1-block-tag-error"]` の出現と文言で判定する
    （`Block.svelte#submitTagInput` が `assignTag` の `invalid_tag` を catch して設定）
  - `.md` の不変は `node:fs` の `readFileSync` で tags 行と frontmatter `updatedAt` を比較する
  - 禁止文字集合全体は `window.__TAURI_INTERNALS__.invoke('assign_tag', { noteId, rawTag })` を
    同期 `browser.execute` で kick-off → window 上の結果を poll する方式で検証する
    （`@wdio/tauri-service` (driverProvider=external) の `patchedExecute` は `browser.executeAsync` を
    扱えないため。s7 / s8 / s9 と同方式）
- **テストデータのクリーンアップ**: `wdio.conf.ts` の `onComplete` が temp `storageDir` を
  `rmSync` で削除する
- **production 側の前提（本 scenario が検証する不変条件）**:
  1. `Tag::new` が `FORBIDDEN_TAG_CHARS = [' ', '\t', '\n', ',', '[', ']']` を検査し
     `TagError::InvalidChar` を返すこと
     （`apps/promptnotes/src-tauri/src/note_capture/shared/types/tag.rs`）
  2. `AssignTagUseCase::execute` の Step 1 `parseTag` が `load_note` より前に走り、失敗時に
     `AssignTagError::InvalidTag` を返して後続 step に進まないこと
     （`apps/promptnotes/src-tauri/src/note_capture/slices/assign_tag/application.rs`）
  3. `assign_tag` command が `InvalidTag` を `AssignTagErrorDto::InvalidTag { name, reason }`
     （serde `kind: "invalid_tag"`）として frontend へ返すこと
     （`apps/promptnotes/src-tauri/src/note_capture/slices/assign_tag/commands.rs`）
  4. UI が `invalid_tag` を catch してエラーメッセージを表示すること
     （`apps/promptnotes/src/ui-page/page-main/components/Block.svelte#submitTagInput`）
  これらのいずれかが欠落すると S10 が RED になる