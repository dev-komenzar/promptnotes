---
coherence:
  source: human
  last_validated: 2026-06-20
  upstream:
    - aggregates.md#update-channel-aggregate
    - domain-events.md#new-version-detected
    - validation.md#s14-update-check-failure
---

# check-for-updates {#check-for-updates}

アプリ起動時に GitHub Releases へ HTTP リクエストを送り、
新バージョンがあれば `NewVersionDetected` を発行する。失敗は silent。

## Input {#input}

```rust
struct CheckForUpdatesCommand {
  current_version: Version,    // ビルド時に埋め込まれた app version
}
```

## Output {#output}

- `UpdateChannel`（チェック結果）
- domain event: [NewVersionDetected](../domain-events.md#new-version-detected)
  （新バージョン検出時のみ）

## Errors {#errors}

- `UpdateError` — application service が握り潰す（silent failure, I-U3, S14）
  - `NetworkError`
  - `ParseError`
  - `RateLimited`

## Steps {#steps}

1. `fetchLatestRelease: () → Result<RawRelease, UpdateError>`
   - Tauri v2 updater plugin が GitHub Releases API を叩く
2. `parseVersion: RawRelease → Result<Version, UpdateError>`
3. `compareVersions: (Version, Version) → Comparison`
   - `Comparison = NewVersion(Release) | UpToDate | OlderVersion`
4. `branchOnComparison:`
   - `NewVersion(release)` → step 5 へ
   - `UpToDate | OlderVersion` → 早期 return（event 非発行、I-U2 通り）
5. `buildUpdateChannel: (Version, Release) → UpdateChannel`
6. `emit: UpdateChannel → NewVersionDetected`

## Error Handling {#error-handling}

- すべての `UpdateError` を application service の outer layer で握り潰す
- ログは出すが UI 通知はしない（S14: silent）
- リトライ・常駐 polling は行わない（I-U3）

## Dependencies {#dependencies}

- `UpdaterPlugin` — Tauri v2 updater plugin の薄いラッパー
- `EventBus`

## Notes {#notes}

- **起動時 1 回のみ実行**。常駐 polling や手動チェックボタンは MVP 範囲外
- 通知 UI（Toast vs Modal）の選択は Phase 11a (ui-fields) で確定
- ネットワーク失敗時に「更新確認できませんでした」を見せない方針:
  ユーザの作業フローを妨げない判断（discovery の core を尊重）
