---
ori:
  node_id: workflow:check-for-updates-manually
  type: workflow
  depends_on:
    - aggregate:UpdateChannel
    - scenario:s27-manual-update-check-new-version
    - scenario:s28-manual-update-check-up-to-date-or-failure
---

# check-for-updates-manually {#check-for-updates-manually}

ユーザが設定モーダル（widget-settings-modal）の「Check for updates」を押したときに、
GitHub Releases の最新リリースをその場で 1 回確認し、結果を要求元の UI にそのまま返す（I-U9）。
起動時 / 24 時間ごとの自動確認（[check-for-updates](check-for-updates.md)）とは異なり、
失敗も UI に返し、`NewVersionDetected` は発行しない。

## Input {#input}

```rust
struct CheckForUpdatesManuallyCommand {
  current_version: Version,    // ビルド時に埋め込まれた app version (I-U1)
}
```

## Output {#output}

- `Result<UpdateChannel, UpdateError>`
  - `Ok(UpdateChannel { latest_release: Some(_) })` — 新しいリリースあり（I-U2）
  - `Ok(UpdateChannel { latest_release: None })` — 最新（同一 / 古いリリースは `None` に正規化、I-U2）
  - `Err(UpdateError)` — 確認失敗
- domain event: **なし**（通知は要求元 UI の画面内表示で完結する。I-U9）

## Errors {#errors}

- `UpdateError` — UI に返し、設定モーダルに確認失敗を表示する（I-U9。silent にしない）
  - `NetworkError`
  - `ParseError`
  - `RateLimited`

## Steps {#steps}

1. `fetchLatestRelease: () → Result<RawRelease, UpdateError>`
   - Tauri v2 updater plugin が GitHub Releases API を叩く（check-for-updates と同じ取得経路）
2. `parseVersion: RawRelease → Result<Version, UpdateError>`
3. `compareVersions: (Version, Version) → Comparison`
   - `Comparison = NewVersion(Release) | UpToDate | OlderVersion`
4. `buildUpdateChannel:`
   - `NewVersion(release)` → `UpdateChannel { latest_release: Some(release) }`
   - `UpToDate | OlderVersion` → `UpdateChannel { latest_release: None }`（I-U2）

check-for-updates の step 6（通知済み抑止）/ step 7（emit）に相当する処理は **持たない**。
通知済みバージョン（I-U4）が同じでも、新しいリリースがあれば `Some` を返す。

## Error Handling {#error-handling}

- すべての `UpdateError` を `Err` として UI に返す。ログにも詳細を出す
- 自動リトライはしない。再確認はユーザがもう一度ボタンを押したときだけ

## Dependencies {#dependencies}

- `UpdaterPort` — Tauri v2 updater plugin の薄いラッパー（check-for-updates と共有）

## Notes {#notes}

- check-for-updates の揮発 state（前回確認時刻 I-U3 / 通知済みバージョン I-U4）は **読みも書きもしない**。
  手動確認の後も、自動確認は通常の 24 時間周期で走り、未通知の新バージョンなら通知（Toast）される
- 手動確認の結果を Toast に出さない理由: 要求元（設定モーダル）が結果を表示するため、
  同じ情報を二重に見せない。Toast 表示中に手動確認しても Toast は変化しない
- 新しいリリースが見つかったときのインストールは [install-update](install-update.md) を再利用する
  （I-U5 / I-U6 / I-U8）。install-update は最新リリースを取得し直すので、手動確認の結果は使い回さない
