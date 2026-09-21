# s21-batch-sync-debounce — Scenario implementation notes

## 検証対象 {#target}

`validation.md#s21-batch-sync-debounce` — Syncthing 等が Note A / B / C の 3 ファイルを
debounce 窓（500ms）以内に一括同期したとき、手動 Refresh / 再起動なしで 3 つの既存 Block の
body がすべて自動更新されること（取りこぼしなし）。加えて `.tmp` 一時ファイルが無視され、
rename 完了後の `.md` のみ処理されること。

経路: OS ファイルウォッチャー（`start_file_watcher` / `notify`）→ path 単位 500ms debounce →
各ファイルを `NoteRepository::load_by_id` で再 parse + `BodyHash` 計算 → domain event
`NoteFileModifiedExternally`（×3）→ subscriber が `InMemoryNoteFeedState::upsert_one` +
Tauri event `notes-changed` emit → `PageMain` の listener が `list_notes` 再取得 →
`feedStore.hydrateNotes` → 各 Block の `note.body` 更新 → CodeMirror `.cm-content` 更新。

## 実行方法 {#how-to-run}

```bash
export PATH=$HOME/.cargo/bin:$PATH
export DISPLAY=:0
# test build (VITE_WDIO_TEST=1 で tauri-plugin-wdio を有効化)
cd apps/promptnotes && bun run build:test
cd ../../.ori/scenarios/s21-batch-sync-debounce
../../../apps/promptnotes/node_modules/.bin/wdio run wdio.conf.ts
```

結果: **5 passing (~7–8s)**（3 回連続実行で安定）。

## debounce の粒度（重要） {#debounce-semantics}

実装（`detect_external_changes/infrastructure.rs`）の debounce は
`last_seen: HashMap<PathBuf, Instant>` による **path 単位の leading-edge + 500ms window 抑制**:

1. 各 path の最初のイベントは **即座に** `on_event` へ渡す（500ms 遅延しない）
2. 同一 path の 500ms 以内の後続イベントは skip（`last_seen` を現在時刻へ更新）
3. 500ms 無イベント（`recv_timeout`）で `last_seen` を clear

したがって **別ファイル（A / B / C）は互いに集約されず**、それぞれ独立に処理される。
validation.md の `t1 = t0 + 0.5s` / `t2 = t0 + 0.6s` / `t3 = t0 + 0.7s` という段階的タイミングは
説明上の近似であり、実装は各 path の初回イベントを即時処理する。不変条件
「3 ファイルすべてが個別に反映され取りこぼしがない」は保持される。

- 同一ファイルへの連続イベントは集約される（s17 で実測済み）。S21 の step 3 は
  「再送を模した同一内容の再バッチ」を debounce 窓（`browser.pause(700)`）を超えてから行い、
  冪等性（Block が重複しない）を確認している
- `.tmp` は `FsWatcher::is_tmp_file`（拡張子 `tmp`）で callback の最初に除外される

## `.tmp` / rename の実測 {#tmp-rename}

notify 6.1.1 の inotify backend は rename を 3 イベントに分解して emit する
（`src/inotify.rs`）: `Modify(Name(From))`（paths=[tmp]）/ `Modify(Name(To))`（paths=[md]）/
`Modify(Name(Both))`（paths=[tmp, md]）。watcher callback は `event.paths.next()` を見るため:

- From → `tmp` 拡張子 → 無視
- To → `md` 拡張子 → `RawFileEvent::Modified(md)` → `onFileModified` → `load_by_id` → 反映
- Both → 先頭が `tmp` なので無視（重複・誤処理なし）

結果として `D.md.tmp` は無視され、`rename` 後の `D.md` のみが新規 Note として反映される。
step 5 はこれを実測している（rename 前に Block D が出現しないこと、rename 後に出現することを assert）。

## event 観測の制約（重要） {#event-observation}

`NoteFileModifiedExternally` の domain event は frontend / E2E に直接露出しない:

- `start_file_watcher` は内部で `AppEventBus`（**in-process**）を生成し、
  `DetectExternalChangesUseCase` 経由で `DomainEvent::NoteFileModifiedExternally` を publish する
  （`commands.rs`。NoOpBus ではない）
- その subscriber は `InMemoryNoteFeedState::upsert_one` と Tauri event `notes-changed`
  （**payload なし**）の emit のみを行う（domain event payload / `disk_body_hash` は frontend に渡さない）
- したがって E2E は event payload を直接 assert できない

さらに **`list_notes` は watcher の証明にならない**: `list-feed/commands.rs` は毎回
`FsNoteRepository::list_all` で disk から全件 re-hydrate するため、外部変更後に `list_notes` を
invoke すれば watcher が壊れていても変更後 body が見える。実際、RED 実測（後述）では
**step 2（`list_notes` ベースの内容検証）だけが PASS のまま**だった — これが read DTO 経路の
非識別性の実証。

このため watcher pipeline の判定は **「手動操作なしで UI（DOM）が自動更新されること」**
に置いた。`notes-changed` は当該 domain event の subscriber からのみ emit されるので、
既存 Block の `.cm-content` の自動更新は pipeline が動いたことの間接観測として成立する
（`spec.md#impl-notes`）。E2E は event payload を直接 assert せず、UI/FS 状態
（3 Block の DOM body 更新 + read DTO + Block 数不変 + ダイアログ / トースト非表示 +
`.tmp` 無視 / rename 反映）で間接検証する。

## RED 実測（テストの非空虚性検証） {#mutation-red}

テストが空振りでないことを確認するため、`PageMain.svelte` の
`invoke('start_file_watcher')` を一時的に無効化（`MUTATION_DISABLED_start_file_watcher`）して
test build → E2E を実行:

- **結果: 2 passing / 3 failing**（~35s）
  - ✖ step 1: 3 Block の DOM body が自動更新されない（watcher が動かない）
  - ✓ step 2: PASS のまま — `list_notes` は disk 直読みのため watcher 無効でも変更後 body が見える
    （read DTO 経路が非識別であることの実証）
  - ✖ step 3: `waitForAllBlockBodies` が sync body に到達しない
  - ✓ step 4: IDLE / ダイアログ・トースト非表示は watcher に依存しないため PASS
  - ✖ step 5: rename 後の Block D が出現しない
- **復元後** → 再ビルド → **5 passing** で GREEN

この RED 実測により、step 1 / 3 / 5 の DOM 自動更新判定が watcher pipeline に因果依存して
いることが確認できた（step 2 が PASS のままである点が list_notes 非識別性の証拠）。

## production 変更 {#production-change}

**なし（0 file）**。既存実装（`detect_external_changes` slice の `Modified` 分岐一式 +
path 単位 debounce + `.tmp` ignore + `PageMain` の watcher 起動 / `notes-changed` 購読 /
`Block.svelte` の body 同期 `$effect`）が S21 の不変条件を既に満たしていた。

S17（単一ファイル）で確認済みの pipeline が、**別 path の一括変更**でも取りこぼしなく
動作することを E2E で確認した。バッチ固有の production 欠落（別 path の誤集約、
rename destination の取りこぼし等）は検出されず、最小実装の追加は不要だった。
working tree の差分は新規 scenario ディレクトリのみ（`PageMain.svelte` の変異は完全 revert 済み）。

## 回帰確認 {#regression}

- frontend unit test: `bun run test` → **164 passed / 18 files**
- 既存 E2E 回帰: `s17-external-file-modified-no-conflict` **4 passing** /
  `s18-external-file-deleted` **4 passing**
- Rust: production 変更ゼロのため追加の `cargo test` 影響なし
- `tsc --noEmit -p tsconfig.json`（scenario）: 既知の `wdio.conf.ts TS2353 'tauri:options'` のみ
  （WDIO v9 型定義起因、s1〜s20 と同一の許容済み error）

## 既知の制約 {#known-issues}

- **frontend は全件 re-hydrate** — ドメイン仕様（I-F8）は `upsert_note` による部分更新を規定し、
  Rust `InMemoryNoteFeedState` は実際に `upsert_one` で差分更新する。一方 frontend は
  `notes-changed` 受信のたびに `listNotesFn()`（disk 全件 re-read）→ `hydrateNotes` で
  全件差し替えする。E2E は最終状態（3 Block の body 更新 + Block 数不変）を観測し、
  「3 回の部分更新」そのものは観測しない（s17 と同じ制約）
- **step 4 は弱い観測（競合ダイアログ）** — `WidgetExternalChangeConflict` の
  `defaultSubscribeFn` は no-op（`store.svelte.ts` OQ-WC1: Real event bridge 未配線）。
  IDLE では当然ダイアログは出ないが、EDITING（S19）でも E2E では区別できない。
  S19 の競合検出は slice / widget unit の領分
- **`updatedAt` ソートの挙動は未検証** — When は body のみ変更で `updatedAt` 据え置き。
  S21 は `created_at` desc（既定）で順序が不変であることのみ確認する。
  複数 Note × `updatedAt` ソートは S15 / NoteFeed sort の領分
- **`disk_body_hash`（I-N9 競合検出用）の計算は E2E では観測できない**（event payload 非露出）。
  slice unit test の領分
- **テストは Gherkin のステップ順に依存する**（step 1 で一括変更 → step 2 で読み →
  step 3 で再バッチ → step 4 で状態確認 → step 5 で `.tmp` / rename）。
  `maxInstances: 1` + mocha 逐次実行が前提（s1〜s20 と同方針。
  `.claude/rules/scenario-test.md`「テスト間独立」からの既知の逸脱）