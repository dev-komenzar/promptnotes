# s17-external-file-modified-no-conflict — Scenario implementation notes

## 検証対象 {#target}

`validation.md#s17-external-file-modified-no-conflict` — 外部プログラムによる既存 `.md` の
body 変更（Note A は IDLE = 競合なし）が、手動 Refresh / 再起動なしで NoteFeed に自動反映されること。

経路: OS ファイルウォッチャー（`start_file_watcher` / `notify`）→ 500ms debounce →
`NoteRepository::load_by_id` で再 parse + `BodyHash` 計算 → domain event
`NoteFileModifiedExternally` 発行 → subscriber が `InMemoryNoteFeedState::upsert_one` +
Tauri event `notes-changed` emit → `PageMain` の listener が `list_notes` 再取得 →
`feedStore.hydrateNotes` → 既存 Block の `note.body` 更新 → CodeMirror `.cm-content` 更新。

## 実行方法 {#how-to-run}

```bash
export PATH=$HOME/.cargo/bin:$PATH
export DISPLAY=:0
# test build (VITE_WDIO_TEST=1 で tauri-plugin-wdio を有効化)
cd apps/promptnotes && bun run build:test
cd ../../.ori/scenarios/s17-external-file-modified-no-conflict
../../../apps/promptnotes/node_modules/.bin/wdio run wdio.conf.ts
```

結果: **4 passing (~3s)**。

## event 観測の制約（重要） {#event-observation}

`NoteFileModifiedExternally` の domain event は frontend / E2E に直接露出しない:

- `start_file_watcher` は内部で `AppEventBus`（in-process）を生成し、`DetectExternalChangesUseCase`
  経由で `DomainEvent::NoteFileModifiedExternally` を publish する
- その subscriber は `InMemoryNoteFeedState::upsert_one` と Tauri event `notes-changed` の
  emit のみを行う（domain event payload / `disk_body_hash` は frontend に渡さない）
- したがって E2E は event payload を直接 assert できない

さらに **`list_notes` は watcher の証明にならない**: `list-feed/commands.rs` は毎回
`FsNoteRepository::list_all` で disk から全件 re-hydrate するため、外部変更後に
`list_notes` を invoke すれば watcher が壊れていても変更後 body が見える。
実際、watcher を無効化した RED 実測（後述）では **step 2（`list_notes` ベースの内容検証）
だけが PASS のまま** だった — これが read DTO 経路の非識別性の実証。

このため watcher pipeline の判定は **「手動操作なしで UI（DOM）が自動更新されること」**
に置いた。`notes-changed` は当該 domain event の subscriber からのみ emit されるので、
既存 Block の `.cm-content` の自動更新は pipeline が動いたことの間接観測として成立する
（`spec.md#impl-notes`）。

## debounce 窓（500ms）の実測 {#debounce-observed}

初回 E2E は **2 passing / 2 failing** だった。step 3（2 回目の外部変更）が step 1 のイベント処理から
500ms 以内に発行され、`FsWatcher::run_event_loop` の debounce
（`last_seen: HashMap<PathBuf, Instant>` による同一 path の 500ms 集約、`infrastructure.rs`）で
**意図的に skip** されたため。診断 spec で 2s 間隔なら modify#1/#2/#3 + Create が
すべて反映されることを確認し、step 3 に `await browser.pause(700)`（debounce 窓超え）を
追加して GREEN 化した。

これは production の欠落ではなく **domain 仕様どおりの debounce 挙動**
（`domain/workflows/detect-external-changes.md#notes` — 「debounce 窓: 500ms（同一ファイルへの
連続イベントを 1 つに集約）」）を E2E が炙り出したもので、テスト側のタイミング修正が正しい対処。
S16 の「連続する外部作成」が同種の pause を要さなかったのは、対象が **別ファイル**
（別 path）で debounce が path 単位だからである。

## RED 実測（テストの非空虚性検証） {#mutation-red}

テストが空振りでないことを確認するため、`PageMain.svelte` の
`invoke('start_file_watcher')` を一時的に無効化（`MUTATION_DISABLED_start_file_watcher`）して
test build → E2E を実行:

- **結果: 1 passing / 3 failing**
  - ✖ step 1: `block 20260620120000 body did not become "hello world"`（UI が自動更新しない）
  - ✓ step 2: PASS のまま — `list_notes` は disk 直読みのため watcher 無効でも変更後 body が見える
    （read DTO 経路が非識別であることの実証）
  - ✖ step 3: `block ... body did not become "hello world 2"`
  - ✖ step 4: 同様に `"hello world 2"` に到達せず
- **復元後** → 再ビルド → **4 passing** で GREEN

この RED 実測により、step 1 / 3 / 4 の DOM 自動更新判定が watcher pipeline に因果依存して
いることが確認できた。

## 回帰確認 {#regression}

- frontend unit test: `bun run test` → **143 passed / 15 files**
- 既存 E2E 回帰: `s16-external-file-created`（**4 passing**）
- Rust: 本 scenario は production 変更ゼロのため追加の cargo test 影響なし

## production 変更 {#production-change}

**なし（0 file）**。既存実装（`detect_external_changes` slice の `Modified` 分岐一式 +
`PageMain` の watcher 起動 / `notes-changed` 購読 / `Block.svelte` の body 同期 `$effect`）が
S17 の不変条件を既に満たしていた。S17 は E2E によるリグレッション・ガードとして追加された。
working tree の差分は新規 scenario ディレクトリのみ（`PageMain.svelte` は変異を完全 revert 済み）。

## 既知の制約 {#known-issues}

- step 4（競合ダイアログ非表示）は弱い観測: `WidgetExternalChangeConflict` の
  `defaultSubscribeFn` は no-op（`store.svelte.ts` OQ-WC1: Real event bridge (Rust → TS) is
  not yet wired）。IDLE ではダイアログは出ないが、EDITING（S19）でも bridge 未実装のため
  E2E では区別できない。S19 の競合検出は slice / widget unit の領分
- テストは Gherkin のステップ順に依存する（step 1 で変更 → step 2 で読み → step 3 で追加変更 →
  step 4 で状態確認）。`maxInstances: 1` + mocha 逐次実行が前提（s1〜s16 と同方針）
- テストプロセスの `node:fs` 書き込みは vim の atomic write（`.swp` → rename）ではなく
  既存 `.md` の直接上書きである。`.tmp` の ignore と rename（delete+create）の扱いは
  S18/S21/S22 / slice unit test の領分
- `updatedAt` ソート時の表示順変化は本 scenario では未検証（When は body のみ変更で
  `updatedAt` 据え置き、Note A 1 件のため順序は自明）。複数 Note × `updatedAt` ソートは
  S15 / NoteFeed sort の領分
- `disk_body_hash`（I-N9 競合検出用）の計算そのものは E2E では観測できない
  （event payload 非露出）。slice unit test の領分