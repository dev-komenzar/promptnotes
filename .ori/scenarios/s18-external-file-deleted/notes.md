# s18-external-file-deleted — Scenario implementation notes

## 検証対象 {#target}

`validation.md#s18-external-file-deleted` — 外部プログラムによる `.md` 削除が、
手動 Refresh / 再起動なしで NoteFeed から当該 Block を除去すること。

経路: OS ファイルウォッチャー（`start_file_watcher` / `notify`）→ 500ms debounce →
ファイル名から `NoteId` 解決（`^\d{14}$`）→ domain event `NoteFileDeletedExternally` 発行 →
subscriber が `InMemoryNoteFeedState::remove_one` + Tauri event `notes-changed` emit →
`PageMain` の listener が `list_notes` 再取得 → `feedStore.hydrateNotes` → UI 更新。

## 実行方法 {#how-to-run}

```bash
export PATH=$HOME/.cargo/bin:$PATH
export DISPLAY=:0
# test build (VITE_WDIO_TEST=1 で tauri-plugin-wdio を有効化)
cd apps/promptnotes && bun run build:test
cd ../../.ori/scenarios/s18-external-file-deleted
../../../apps/promptnotes/node_modules/.bin/wdio run wdio.conf.ts
```

結果: **4 passing (~4.5s)**。

## event 観測の制約（重要） {#event-observation}

`NoteFileDeletedExternally` の domain event は frontend / E2E に直接露出しない:

- `start_file_watcher` は内部で `AppEventBus`（in-process）を生成し、`DetectExternalChangesUseCase`
  経由で `DomainEvent::NoteFileDeletedExternally` を publish する
- その subscriber は `InMemoryNoteFeedState::remove_one` と Tauri event `notes-changed` の
  emit のみを行う（domain event payload は frontend に渡さない）
- したがって E2E は event payload を直接 assert できない

さらに **`list_notes` は watcher の証明にならない**: `list-feed/commands.rs` は毎回
`FsNoteRepository::list_all` で disk から全件 re-hydrate するため、外部削除後に
`list_notes` を invoke すれば watcher が壊れていても当該 Note は見えない。
実際、watcher を無効化した RED 実測（後述）では **step 2（`list_notes` ベースの内容検証）
だけが PASS のまま** だった — これが read DTO 経路の非識別性の実証。

このため watcher pipeline の判定は **「手動操作なしで UI（DOM）から Block A が消え、
B が残ること」** に置いた。`notes-changed` は当該 domain event の subscriber からのみ emit
されるので、DOM からの自動除去は pipeline が動いたことの間接観測として成立する
（`spec.md#impl-notes`）。

> 補足: task 指示の「NoOpBus のため E2E 観測不可なら UI/FS 状態で間接検証」に該当する。
> 正確には note-capture 系 command の NoOpBus とは別に、watcher は in-process `AppEventBus`
> を持つが、frontend には `notes-changed` 以外を出さない、という構造を採っている。

## RED 実測（テストの非空虚性検証） {#mutation-red}

テストが空振りでないことを確認するため、`PageMain.svelte` の
`invoke('start_file_watcher')` を一時的に無効化（`MUTATION_DISABLED_start_file_watcher`）して
test build → E2E を実行:

- **結果: 1 passing / 3 failing**
  - ✖ step 1: `feed blocks did not become [20260621090000]`（削除が UI に反映されない）
  - ✓ step 2: PASS のまま — `list_notes` は disk 直読みのため watcher 無効でも削除済み
    Note A は見えない（read DTO 経路が非識別であることの実証）
  - ✖ step 3: `feed blocks did not become []`（B の削除も未反映）
  - ✖ step 4: フィード件数が期待 `[]` ではなく `[B, A]` のまま
- **復元後** → 再ビルド → **4 passing (~4.5s)** で GREEN

この RED 実測により、step 1 / 3 の DOM 自動除去判定が watcher pipeline に因果依存して
いることが確認できた。

## 回帰確認 {#regression}

- frontend unit test: `bun run test` → **143 passed / 15 files**
- 既存 E2E 回帰: `s17-external-file-modified-no-conflict` → **4 passing**
- Rust: 本 scenario は production 変更ゼロのため追加の `cargo test` 影響なし

## production 変更 {#production-change}

**なし（0 file）**。既存実装（`detect_external_changes` slice 一式 + `PageMain` の watcher 起動 /
`notes-changed` 購読）が S18 の不変条件を既に満たしていた:

- `infrastructure.rs`: `EventKind::Remove(_)` → `RawFileEvent::Deleted`
- `application.rs`: `Deleted` → `resolve_note_id`（`^\d{14}$`）→ `NoteFileDeletedExternally` publish
- `commands.rs`: subscriber が `NoteFileDeletedExternally` で `remove_one` + `notes-changed` emit
- `note_feed_state.rs` / `note_feed.rs`: `remove_one` → `remove_note(note_id)`（I-F8）

S18 は E2E によるリグレッション・ガードとして追加された。

## 既知の制約 {#known-issues}

- **step 4 は弱い観測**: `notanote.md`（非 Note 名）の削除が UI に影響しないことは、
  watcher の `resolve_note_id` 失敗（event 未発行 skip）と `list_all` 側の非 Note 名 skip の
  両経路で同じ結果になるため、フィード不変 assert は watcher 側 skip を単独では識別しない
- **Given は 2 件 seed**: validation Given は「Note A が表示されている」であり「のみ」とは
  規定しないため、`remove_note(&A.id)` が blanket clear でなく該当 id のみを除外することを
  観測できるよう Note B も seed した（s16 の Given は明示的に「Note A のみ」だが s18 は異なる）
- **テストは Gherkin ステップ順に依存する**（step 1 削除 A → step 2 読み → step 3 削除 B →
  step 4 非 Note 名）。`maxInstances: 1` + mocha 逐次実行が前提（s1〜s17 と同方針。
  `.claude/rules/scenario-test.md` の「テスト間独立」からの既知の逸脱）
- **S20（編集中の外部削除）は範囲外**: `WidgetExternalChangeConflict` の `defaultSubscribeFn` は
  no-op（OQ-WC1: Real event bridge (Rust → TS) is not yet wired）のため、EDITING 削除競合の
  E2E は現状識別不能。S20 の領分
- **vim atomic write / `.tmp` rename は未検証**: 本テストは `unlinkSync` による直接削除である。
  rename（delete+create 2 イベント）と `.tmp` ignore の扱いは slice unit test の領分
