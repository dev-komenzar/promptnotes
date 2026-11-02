---
coherence:
  source: derived
  last_derived: 2026-11-02
  hash:
    domain/workflows/check-for-updates-manually.md#.*: c3f0e2d63d42
    domain/aggregates.md#.*: 675676f60daa
    domain/bounded-contexts.md#.*: c3bc130f98d5
    domain/validation.md#.*: e64a7d3e3083
ori:
  schema:
    propagation_level: file
---

# check-for-updates-manually spec {#check-for-updates-manually-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive check-for-updates-manually`.

## 概要 {#overview}

ユーザが設定モーダル (widget-settings-modal) の「Check for updates」を押したときに、最新リリースを
その場で 1 回確認し、結果 (新しいリリースあり / 最新 / 確認失敗) を要求元の UI にそのまま返す query slice。

> domain/workflows/check-for-updates-manually.md#check-for-updates-manually より：「起動時 / 24 時間ごとの自動確認
> （check-for-updates）とは異なり、失敗も UI に返し、`NewVersionDetected` は発行しない。」

取得経路 (`UpdaterPort` の production impl `TauriUpdaterPort`) と parse / 比較 (I-U2) は check-for-updates と共有する。

## 入出力 {#io}

### Input {#io-input}

```rust
pub struct CheckForUpdatesManuallyCommand {
    pub current_version: Version,  // build_version() を parse (I-U1)
}
```

### Output {#io-output}

- `check_for_updates_manually` → `Result<UpdateChannelResponse, UpdateCheckErrorResponse>`
  - `UpdateChannelResponse` は check-for-updates と同じ DTO (`{ current_version, latest_release: { version, url, notes } | null }`)
  - `latest_release` が `Some` のとき `version > current_version` (I-U2)。同一 / 古い版は `None`
- domain event: なし (I-U9)

### Errors {#io-errors}

`UpdateError` (UI に返す。silent にしない、I-U9):

| variant | 条件 |
|--|--|
| `NetworkError` | 取得失敗 (ネットワーク / HTTP / plugin がリリースなしを返した) |
| `ParseError` | version 文字列 / payload の parse 失敗 |
| `RateLimited` | upstream の rate limit |

TS へは `{ kind: "<variant>", message: string }` に射影する。現在バージョン自体が parse 不能な場合も `ParseError`。

## 不変条件 {#invariants}

### UpdateChannel Aggregate 由来 {#invariants-update-channel-aggregate}

- **I-U2**: `latest_release` が `Some` なら `version > current_version`
- **I-U9**: ユーザ要求時のみ、その場で 1 回確認する。失敗は `Err` で返す。通知済み (I-U4) でも新版を返す。
  `NewVersionDetected` を発行せず、揮発 state (前回確認時刻 I-U3 / 通知済みバージョン I-U4) を読み書きしない

### slice 固有制約 {#invariants-slice-specific}

- **C-CFUM1**: 1 回の呼び出しで updater の fetch は 1 回 (自動リトライなし)
- **C-CFUM2**: use case は `EventBus` / `NotifiedVersionLedger` に依存しない (型レベルで event 発行・ledger 更新を不可能にする)
- **C-CFUM3**: command は check-for-updates の `InMemoryUpdateCheckState` を参照しない (`begin_check` を呼ばない)
- **C-CFUM4**: version 比較は `shared::release_check::compare` (check-for-updates と同一実装) を使う

## 境界契約 {#boundary-contract}

- boundary kind: `tauri_command`
- Rust: `apps/promptnotes/src-tauri/src/update_distribution/slices/check_for_updates_manually/commands.rs`
  - `check_for_updates_manually(app_handle) -> Result<UpdateChannelResponse, UpdateCheckErrorResponse>` (async)
  - `lib.rs` の `invoke_handler!` に配線
- TS: `apps/promptnotes/src/lib/update-distribution/slices/check-for-updates-manually/index.ts`
  - `checkForUpdatesManually(): Promise<UpdateChannelResponse>` (reject 値は `UpdateCheckErrorResponse`)
- 禁止: UI 層が `@tauri-apps/plugin-updater` の `check()` を直接呼ぶこと (I-U2 の比較を迂回する)

## テスト観点 {#test-points}

Rust 側は `check_for_updates_manually/tests.rs`。`UpdaterPort` は adapter 境界の test double (`FakeUpdater`)。

### 結果の 3 分岐 {#tp-results}

- **TP-R1**: current `0.2.4`、fetch `0.2.5` → `Ok(latest_release = Some(0.2.5))` (S27)
- **TP-R2**: fetch `0.2.5` (同一) / `0.2.3` (古い) → `Ok(latest_release = None)` (S28 / I-U2)
- **TP-R3**: fetch 失敗 (`NetworkError` / `RateLimited`) → 同じ variant の `Err` (S28)
- **TP-R4**: fetch した version が parse 不能 → `Err(ParseError)`
- **TP-R5**: proptest — 任意の (current, fetched) で `Some` になるのは `fetched > current` のときだけ。fetch は常に 1 回 (C-CFUM1)

### I-U9: 揮発 state と event {#tp-no-side-effects}

- **TP-N1**: 同じ版で 2 回確認しても 2 回とも `Some` を返す (通知済み抑止なし)
- **TP-N2**: check-for-updates の `InMemoryUpdateCheckState` で `0.2.5` を通知済みにした後でも、手動確認は `Some(0.2.5)` を返し、その後の自動確認 (`claim`) の判定も変わらない (state 非変更)

### boundary test {#tp-boundary}

- **TP-B1**: TS — `checkForUpdatesManually()` が `invoke('check_for_updates_manually')` を 1 回呼び、resolve 値を返す / reject 値をそのまま reject
- **TP-B2**: エラー DTO の `kind` が variant 名と一致する (Rust unit)

## 実装ノート {#impl-notes}

### アーキ層への落とし込み {#impl-layers}

```
apps/promptnotes/src-tauri/src/update_distribution/
├── shared/
│   ├── release_check.rs   # Comparison / parse_release / compare (check-for-updates から移設、共有)
│   └── updater.rs         # TauriUpdaterPort (check-for-updates から移設、共有)
└── slices/check_for_updates_manually/
    ├── mod.rs
    ├── domain.rs          # CheckForUpdatesManuallyCommand
    ├── application.rs     # CheckForUpdatesManuallyUseCase<U>: fetch → parse → compare
    ├── commands.rs        # check_for_updates_manually
    └── tests.rs
```

- tauri-specta 未採用のため bindings 再生成は無い (DoD rule 4 対象外)

### Out of scope {#out-of-scope}

- 結果の表示文言と再試行 UI (widget-settings-modal の責務)
- インストール (install-update slice を再利用)
