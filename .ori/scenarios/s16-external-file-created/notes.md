# s16-external-file-created — Scenario implementation notes

## 検証対象 {#target}

`validation.md#s16-external-file-created` — 外部プログラムによる `.md` 新規作成が、
手動 Refresh / 再起動なしで NoteFeed に自動反映されること。

経路: OS ファイルウォッチャー（`start_file_watcher` / `notify`）→ 500ms debounce →
`NoteRepository::load_by_id` で parse → domain event `NoteFileCreatedExternally` 発行 →
subscriber が `InMemoryNoteFeedState::upsert_one` + Tauri event `notes-changed` emit →
`PageMain` の listener が `list_notes` 再取得 → `feedStore.hydrateNotes` → UI 更新。

## 実行方法 {#how-to-run}

```bash
export PATH=$HOME/.cargo/bin:$PATH
export DISPLAY=:0
# test build (VITE_WDIO_TEST=1 で tauri-plugin-wdio を有効化)
cd apps/promptnotes && bun run build:test
cd ../../.ori/scenarios/s16-external-file-created
../../../apps/promptnotes/node_modules/.bin/wdio run wdio.conf.ts
```

結果: **4 passing (~3.5s)**。

## event 観測の制約（重要） {#event-observation}

`NoteFileCreatedExternally` の domain event は frontend / E2E に直接露出しない:

- `start_file_watcher` は内部で `AppEventBus`（in-process）を生成し、`DetectExternalChangesUseCase`
  経由で `DomainEvent::NoteFileCreatedExternally` を publish する
- その subscriber は `InMemoryNoteFeedState::upsert_one` と Tauri event `notes-changed` の
  emit のみを行う（domain event payload は frontend に渡さない）
- したがって E2E は event payload を直接 assert できない

さらに **`list_notes` は watcher の証明にならない**: `list-feed/commands.rs` は毎回
`FsNoteRepository::list_all` で disk から全件 re-hydrate するため、外部作成後に
`list_notes` を invoke すれば watcher が壊れていても新規 Note が見える。
実際、watcher を無効化した RED 実測（後述）では **step 2（`list_notes` ベースの内容検証）
だけが PASS のまま** だった — これが read DTO 経路の非識別性の実証。

このため watcher pipeline の判定は **「手動操作なしで UI（DOM）が自動更新されること」**
に置いた。`notes-changed` は当該 domain event の subscriber からのみ emit されるので、
DOM の自動更新は pipeline が動いたことの間接観測として成立する（`spec.md#impl-notes`）。

## RED 実測（テストの非空虚性検証） {#mutation-red}

テストが空振りでないことを確認するため、`PageMain.svelte` の
`invoke('start_file_watcher')` を一時的に無効化（`MUTATION_DISABLED_start_file_watcher`）して
test build → E2E を実行:

- **結果: 1 passing / 3 failing**
  - ✖ step 1: `feed blocks did not become [20260630120000, 20260620120000]`（UI が自動更新しない）
  - ✓ step 2: PASS のまま — `list_notes` は disk 直読みのため watcher 無効でも新規 Note が見える
    （read DTO 経路が非識別であることの実証）
  - ✖ step 3: 2 つ目の外部作成も未反映
  - ✖ step 4: フィード件数が期待 `[NEW2, NEW, A]` ではなく `[A]` のまま
- **復元後** → 再ビルド → **4 passing** で GREEN

この RED 実測により、step 1 / 3 の DOM 自動更新判定が watcher pipeline に因果依存して
いることが確認できた。

## 回帰確認 {#regression}

- frontend unit test: `bun run test`（結果は review.md / 完了報告に記載）
- 既存 E2E 回帰: `s15-same-second-edits`（4 passing 期待）
- Rust: 本 scenario は production 変更ゼロのため追加の cargo test 影響なし

## production 変更 {#production-change}

**なし（0 file）**。既存実装（`detect_external_changes` slice 一式 + `PageMain` の watcher 起動 /
`notes-changed` 購読）が S16 の不変条件を既に満たしていた。S16 は E2E による
リグレッション・ガードとして追加された。

## 既知の制約 {#known-issues}

- step 4（malformed skip）は弱い観測: watcher の `load_by_id` 失敗（event 未発行）と
  `list_all` の parse 失敗 skip の両経路が同じく「UI に現れない」という結果になるため、
  件数不変の assert は watcher 側の skip を単独では識別しない
- テストは Gherkin のステップ順に依存する（step 1 で作成 → step 2 で読み → step 3 で追加作成 →
  step 4 で malformed）。`maxInstances: 1` + mocha 逐次実行が前提（s1〜s15 と同方針）
- テストプロセスの `node:fs` 書き込みは vim の atomic write（`.swp` → rename）ではなく
  直接作成である。`.tmp` の ignore と rename（delete+create）の扱いは S21/S22 / slice unit test
  の領分
