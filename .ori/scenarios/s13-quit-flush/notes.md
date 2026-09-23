# s13-quit-flush — Scenario implementation notes

## 概要 {#overview}

アプリ quit 時に、debounce timer 待ちの pending_body を取りこぼさず **flush-note workflow（trigger=AppQuit）**
で永続化することを検証する。ユーザが編集直後（500ms debounce 未満）に quit したケースのデータ欠損防止が核。

## テスト方式: Tauri 境界 invoke + 実 UI pending {#test-approach}

- **quit トリガーの再現制約**: 本 app の S13 orchestration は frontend 実装（`PageMain.svelte` の
  `onCloseRequested` → `preventDefault` → `pendingFlushRegistry.flushAll('app_quit')` → `window.destroy()`）。
  しかし Tauri v2 capability（`src-tauri/capabilities/default.json`）に `core:window:allow-close` が無く
  （`core:window:allow-destroy` のみ）、E2E から `getCurrentWindow().close()` で CloseRequested を JS 発火
  できない。`destroy()` は許可済みだが flush を伴わず即終了するため quit flush の検証に使えない。
- **代替手段**: app_quit 経路は `@wdio/tauri-service` v1.4.0 の `browser.tauri.execute(({ core }) =>
  core.invoke('flush_note', { noteId, pendingBody, trigger: 'app_quit' }))` で Tauri 境界 invoke として再現する
  （`tauri-plugin-wdio` は `#[cfg(debug_assertions)]` で有効。test build が frontend で
  `@wdio/tauri-plugin` を import する = `VITE_WDIO_TEST=1` 必須）。
- **実 UI pending**: step3 は実ブロックを EDITING にし、debounce (500ms) 中の pending_body を作ってから
  app_quit invoke を発火する。`flush_note` が debounce より先に永続化した場合のみ `outcome=flushed` が返る
  （debounce が先に fire していれば body は同値となり `no_op`）。よって `outcome=flushed` は
  「debounce を待たず Flush が永続化した」ことの決定的な証拠になる。
- **A → B → C の逐次性**: 各 `flush_note` を `await` し、直後に `node:fs` で全 `.md` を読み、
  「当該 Note のみ更新済み・後続は未更新」を確認する（並列実行で後続が先に書かれるケースを検出）。
- **I-PM10 制約**: `screen-1` は同時 EDITING を高々 1 つに制限するため、validation の「A, B, C すべてが
  EDITING」は UI では再現できない。連続 Flush は Tauri 境界 invoke で再現する（spec.md#impl-notes / #test-points）。

## テストカバレッジ {#coverage}

| 検証項目 | 検証方法 |
|---|---|
| AppQuit Flush が pending body を欠損させない（核） | E2E step3（実 UI pending → app_quit invoke → `outcome=flushed` + `.md` 更新 + debounce 後の重複永続化なし） |
| 連続 Flush の逐次性（A → B → C、各段で当該 Note のみ更新） | E2E step1（各 await 直後に A/B/C の `.md` を読み分け） |
| 冪等性（body 不変 → no-op、ファイル不変） | E2E step2（同じ pending_body で app_quit invoke → `outcome=no_op` + bytes 不変） |
| quit 完了時の全反映 | E2E step1（A/B/C すべて更新済み） |
| trigger=app_quit 受理（Tauri 境界 / specta bindings 相当） | E2E step1-3（`flush_note` invoke が app_quit で flushed/no_op を返す） |
| `NoteBodyEdited` 購読冪等 / `pendingFlushRegistry` 順序 / `onCloseRequested` intercept | 既存 unit test（`pending-flush.test.ts`, flush-note Rust tests）+ production code。E2E の quit trigger 制約は notes#test-approach 参照 |

## RED → GREEN 検証 {#red-green}

production 側は S13 の前提を **既に満たしていた**（ori-73q: `flush_note` command + `pendingFlushRegistry` +
`PageMain.svelte` の onCloseRequested orchestration）。テストが空振りでないことを示すため、production を一時
変異させて RED を実測した:

- **変異 A（`flush_note/application.rs` の step 7 `repository.write` を skip）**:
  `VITE_WDIO_TEST=1 bun run tauri build --debug --no-bundle`（= `bun run build:test`）→ E2E **3 failing**:
  - step1: `Expected substring: "alpha quit-A"` に対し `Received` は seed のまま（永続化されない）
  - step2: `Expected: "no_op"` に対し `Received: "flushed"`（body がディスク上で変わらないため常に flushed）
  - step3: `Expected substring: "delta body!"` に対し seed のまま
- **復元後**: 再ビルド → E2E **3 passing (~3.9s)** で GREEN。

> 注意: 変異中の RED 実測は必ず `build:test`（`VITE_WDIO_TEST=1`）で行うこと。`cargo build` 単体で作った
> バイナリは frontend の test setup（`@wdio/tauri-plugin` import）を含まず、`browser.tauri.execute` が
> 機能せず別要因で fail する（初回に踏んだ罠）。

production への正味の変更は **0 files**（`git diff apps/` 空）。

回帰確認:

- frontend unit test: **143 passed / 15 files**
- E2E 回帰: s3-flush-on-blur **3 passing**、s12-startup-state **1 passing**
- Rust: 変更なし

## 既知の未対応 / 補足 {#known-issues}

- **CloseRequested の実発火は E2E 未実施**: capability 制約により `getCurrentWindow().close()` を JS から
  呼べない。frontend orchestration（registry の順序・intercept・destroy）は unit test と code review で担保し、
  E2E は Tauri 境界の `flush_note(app_quit)` を検証する。将来 `core:window:allow-close` を追加すれば
  `close()` → CloseRequested → flushAll → destroy の完全 E2E へ拡張可能。
- **`Settings::load_or_default` は概念名**: 実 Rust は `LoadSettingsUseCase::execute`（s12 notes と同様）。
- **storage_dir 解決**: `TAURI_TEST_STORAGE_DIR` override（`note_capture/shared/storage.rs` の
  `storage_dir_override`）で隔離 temp dir を使用。`settings.json` を経由しないため config dir も
  `XDG_CONFIG_HOME` で隔離し、実 user 環境を汚さない。
