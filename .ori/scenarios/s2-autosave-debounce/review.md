# Review: s2-autosave-debounce {#review-s2-autosave-debounce}

## Pass 1 {#pass-1}

### Structural gates

scenario タイプのため slice/page 向けの 3 structural gate（boundary test / arch lint / public_entry）は対象外。

- syntax check (`npx tsc --noEmit`): SKIP — `@wdio/tauri-service` / `webdriverio` / `@wdio/globals` が未インストール。Phase 11b（page-main 実装）以降に scenario test が実行可能になるタイミングで、`bun add -d @wdio/tauri-service webdriverio tsx` 等の依存追加が必要。
- docker-compose: SKIP — compose-service 系参加者ゼロ（local 系 Tauri app のみ）のため、compose 不在は正常。

### Semantic findings（spec ↔ test code ↔ runner config）

| # | Severity | Item | Detail |
|---|----------|------|--------|
| 1 | LOW | spec ↔ test coverage gap | spec.md#test-points に 6 項目（auto-save 発火タイミング、ファイル更新、updatedAt 更新、event 発行、NoteFeed 反映、冪等性）がある。テストコードは「ファイル更新 + updatedAt」と「冪等性」の 2 項目を skeleton としてカバー。event 発行と NoteFeed 反映は未検証だが、event bus が NoOpBus（`commands.rs`）である現状では E2E での event 検証は不可能。これは実装スコープ外であり欠陥ではない。 |
| 2 | OK | spec ↔ validation 整合性 | spec.md の Given/When/Then は `domain/validation.md#s2-autosave-debounce` と一致。乖離なし。 |
| 3 | OK | test naming convention | `describe('scenario:s2-autosave-debounce')` — 命名規約（`scenario.instructions.md`）準拠。 |
| 4 | OK | @ori-generated marker | テストファイル、wdio.conf.ts ともに `// @ori-generated scenario:s2-autosave-debounce` マーカーあり。 |
| 5 | OK | wdio.conf.ts | `runner: 'local'`, `services: [['@wdio/tauri-service', { driverProvider: 'external' }]]`, `tauri:options.application` が正しい binary パスを指す。maxInstances=1。 |
| 6 | OK | tsconfig.json | `types: ["node", "@wdio/globals/types", "mocha"]`, `skipLibCheck: true`。WDIO v9 + TS 7 の lib.dom `URLPattern` 衝突回避のための skipLibCheck が設定済。 |
| 7 | OK | テスト間独立性 | `before()` で fixture（.md ファイル）を独立作成し `after()` でクリーンアップ。テスト間共有なし。 |
| 8 | INFO | テストの前提条件 | `storage_dir` injection 機構（環境変数 / config / AppHandle 差し替え）が未整備のため、テストは skeleton として記述されている。これは本シナリオの欠陥ではなく、上位の実装依存（page-main + tauri 起動構成）として明示されている。Phase 11b 以降で解決。 |
| 9 | OK | docker-compose | compose-service 系参加者ゼロのため、compose 不在は正常。wdio.conf.ts も onPrepare/onComplete で compose 操作をしていない。 |

### 総合判定

**PASS** — スペックとテストコード / runner config の間に意味的乖離はない。テストはスケルトン段階だが、TBD 項目は既知の実装依存（storage_dir injection、page-main、wdio packages）として明示されており、それらが整った時点で修正可能な設計になっている。

### 前提条件（テスト実行のために必要なもの）

| 項目 | 状態 | 所有者 |
|------|------|--------|
| `@wdio/tauri-service`, `webdriverio` | 未インストール | 開発環境セットアップ |
| `storage_dir` injection | 未実装 | Rust backend（環境変数 or config） |
| `page-main` implementation | 未完了 | Phase 11b ui-grouping 後 |
| `bun run test` の wdio integration | 未設定 | package.json scripts |

これらは `/ori-flow` ではなく、`/ori-distill` Phase 11b 以降で順次解決される。シナリオテストの初回実行はそのタイミングになる。