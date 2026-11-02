---
ori:
  node_id: workflow:install-update
  type: workflow
  depends_on:
    - aggregate:UpdateChannel
    - scenario:s24-install-update-and-restart
    - scenario:s25-install-update-failure
    - scenario:s26-unbundled-notify-only
    - scenario:s27-manual-update-check-new-version
---

# install-update {#install-update}

ユーザが更新通知（widget-update-toast）、または設定モーダル（widget-settings-modal）の手動確認結果の
「Update & restart」を押したときに、最新リリースを
download → 署名検証 → install する（I-U5）。インストールできるのは updater 対応の配布形式だけ（I-U6）。
成功したらインストールしたバージョンを返し、呼出側（page-main）が全 Note を Flush してから
アプリを再起動する（I-U8）。

併せて、通知 UI / 設定モーダルがインストール操作を提示してよいかを返す読み取り query
（`isUpdateInstallable`）を持つ。

## Input {#input}

```rust
struct InstallUpdateCommand {
  current_version: Version,          // ビルド時に埋め込まれた app version (I-U1)
  installation: InstallationKind,    // ビルド (bundle) 時に埋め込まれた配布形式
}

struct IsUpdateInstallableQuery {
  installation: InstallationKind,
}
```

## Output {#output}

- `InstallUpdateCommand` → `Result<Version, InstallUpdateError>`（成功時はインストールしたバージョン）
- `IsUpdateInstallableQuery` → `bool`（`InstallationKind::is_installable`）
- domain event: **なし**（[domain-events.md#non-events](../domain-events.md#non-events) の UpdateInstalled 参照）

## Errors {#errors}

- `InstallUpdateError` — UI に返し、要求元（Toast / 設定モーダル）に失敗を表示する（I-U7。silent にしない）
  - `UnsupportedInstallation` — `InstallationKind::Unbundled`（I-U6）
  - `NoUpdateAvailable` — 取得し直した最新リリースが `current_version` 以下（I-U7）
  - `CheckFailed` — 最新リリースの取得 / parse に失敗（ネットワーク等）
  - `DownloadFailed` — パッケージの download に失敗
  - `InstallFailed` — 署名検証失敗、または OS へのインストールに失敗

## Steps {#steps}

1. `ensureInstallable: InstallationKind → Result<(), InstallUpdateError>`
   - `Unbundled` → `UnsupportedInstallation` で終了（I-U6）。ネットワークには触れない
2. `fetchLatestUpdate: () → Result<PendingUpdate, InstallUpdateError>`
   - Tauri v2 updater plugin で最新リリースを取得し直す（通知時の情報は使い回さない。
     通知後により新しいリリースが出ていれば、そちらを入れる）
3. `ensureNewer: (Version, PendingUpdate) → Result<PendingUpdate, InstallUpdateError>`
   - `PendingUpdate.version <= current_version` → `NoUpdateAvailable`（I-U7 / I-U2）
4. `downloadAndInstall: PendingUpdate → Result<Version, InstallUpdateError>`
   - updater plugin に委譲。署名検証もここ（失敗は `InstallFailed`）
5. 成功時、インストールしたバージョンを返す

再起動（I-U8）はこの workflow の外で、UI 層が次の順に行う:

1. 全 pending Note の Flush を await（S13 と同じ `flush-note` 経路）
2. `restartApp` を呼ぶ（アプリプロセスを再起動。インストール成功後にしか呼ばない）

## Error Handling {#error-handling}

- すべてのエラーを UI に返す。要求元の UI（Toast / 設定モーダル）はエラーを表示し、「Update & restart」を再度押せる状態に戻す
  （ユーザ操作を起点とした再試行のみ。自動リトライはしない）
- 失敗時はアプリを再起動しない。View release（手動更新）は引き続き使える
- ログにエラーの詳細を出す

## Dependencies {#dependencies}

- `UpdateInstaller` — Tauri v2 updater plugin の薄いラッパー（fetch + download_and_install）
- `InstallationKind` の取得 — bundle 時に埋め込まれる配布形式（Tauri の `bundle_type`）
- `AppRestarter` — アプリプロセスの再起動（UI 層が Flush 後に呼ぶ）

## Notes {#notes}

- check-for-updates とは state を共有しない（通知済みバージョン I-U4 とは独立）
- download の進捗表示（%）は MVP 範囲外。UI は「Updating…」の処理中表示のみ
- Linux の deb / rpm は updater plugin が `pkexec` で権限昇格してインストールする。
  ユーザが認証を拒否した場合は `InstallFailed`
- 定期確認（I-U3）が install 中に走っても干渉しない（install は通知済み state を変えない）
