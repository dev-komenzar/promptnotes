---
ori:
  node_id: workflow:check-for-updates
  type: workflow
  depends_on:
    - aggregate:UpdateChannel
    - event:NewVersionDetected
    - scenario:s14-update-check-failure
---

# check-for-updates {#check-for-updates}

アプリ起動時、および以降 24 時間ごとに GitHub Releases へ HTTP リクエストを送り、
同一プロセス内で未通知の新バージョンがあれば `NewVersionDetected` を発行する（I-U3 / I-U4）。失敗は silent。

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

- `UpdateError` — application service が握り潰す（silent failure, S14）
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
6. `suppressNotified: (UpdateChannel, Option<Version>) → Option<UpdateChannel>`
   - 同一プロセス内の通知済みバージョン以下なら event 非発行で終了（I-U4）
7. `emit: UpdateChannel → NewVersionDetected`
   - 発行したバージョンを通知済みとして記録する（揮発）

## Error Handling {#error-handling}

- すべての `UpdateError` を application service の outer layer で握り潰す
- ログは出すが UI 通知はしない（S14: silent）
- 失敗時の即時リトライは行わない。次の確認は通常の 24 時間周期を待つ（I-U3）

## Dependencies {#dependencies}

- `UpdaterPlugin` — Tauri v2 updater plugin の薄いラッパー
- `EventBus`

## Notes {#notes}

- **起動時 1 回 + 24 時間ごとに実行**（I-U3）。手動チェックボタンは MVP 範囲外
- sleep / 休止復帰で timer が遅れる場合の吸収（monotonic clock での経過時間計測、
  ウィンドウ focus 復帰時の経過確認など）は実装詳細とし、domain では規定しない
- 通知 UI（Toast vs Modal）の選択は Phase 11a (ui-fields) で確定
- ネットワーク失敗時に「更新確認できませんでした」を見せない方針:
  ユーザの作業フローを妨げない判断（discovery の core を尊重）
