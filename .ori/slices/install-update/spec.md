---
coherence:
  source: derived
  last_derived: 2026-11-02
  hash:
    domain/workflows/install-update.md#.*: 4b291a6c25a9
    domain/aggregates.md#.*: 0549c2d38dd2
    domain/bounded-contexts.md#.*: c3bc130f98d5
    domain/validation.md#.*: a47549dec6ff
ori:
  schema:
    propagation_level: file
---

# install-update spec {#install-update-spec}

> This file is a derived document. Edit the source manifest + domain docs and re-run `/ori-derive install-update`.

## 概要 {#overview}

ユーザが更新通知 (widget-update-toast) の「Update & restart」を押したときに、最新リリースを取得し直して
download → 署名検証 → install する command slice。成功時はインストールしたバージョンを返し、
再起動は呼出側 (page-main) が全 Note の Flush 後に `restart_app` で行う。

> domain/workflows/install-update.md#install-update より：「インストールできるのは updater 対応の配布形式だけ（I-U6）。
> 成功したらインストールしたバージョンを返し、呼出側（page-main）が全 Note を Flush してからアプリを再起動する（I-U8）。」

併せて、通知 UI がボタンを出してよいかを返す query `is_update_installable` と、Flush 後に呼ぶ
`restart_app` を同じ slice の command として公開する (workflow の Dependencies `AppRestarter`)。

## 入出力 {#io}

### Input {#io-input}

```rust
pub struct InstallUpdateCommand {
    pub current_version: Version,        // build_version() を parse (I-U1)
    pub installation: InstallationKind,  // tauri::utils::platform::bundle_type() から導出
}
```

- `is_update_installable` / `restart_app` は引数なし

### Output {#io-output}

- `install_update` → `Result<String /* installed version */, InstallUpdateErrorResponse>`
- `is_update_installable` → `bool`
- `restart_app` → 戻らない (プロセス再起動)
- domain event: なし (domain-events.md#non-events UpdateInstalled)

### Errors {#io-errors}

`InstallUpdateError` (UI に返す。silent にしない、I-U7):

| variant | 条件 |
|--|--|
| `UnsupportedInstallation` | `InstallationKind::Unbundled` (I-U6) |
| `InstallInProgress` | 別の `install_update` が実行中、または成功後の再起動待ち (I-U10)。updater には触れない |
| `NoUpdateAvailable` | 取得し直した最新版 `<= current_version` (I-U7)、または plugin がリリースなしを返した |
| `CheckFailed` | 最新リリースの取得 / version parse に失敗 |
| `DownloadFailed` | download 失敗 |
| `InstallFailed` | 署名検証失敗 / OS へのインストール失敗 (deb/rpm の権限昇格拒否を含む) |

TS へは `{ kind: "<variant>", message: string }` に射影する。

## 不変条件 {#invariants}

### UpdateChannel Aggregate 由来 {#invariants-update-channel-aggregate}

- **I-U5**: install はユーザ要求時のみ。本 slice は起動時 / 定期確認から呼ばれない
- **I-U6**: `Unbundled` は `UnsupportedInstallation`。updater には一切触れない
- **I-U7**: `current_version` より新しいリリースのみ install。失敗は UI に返す
- **I-U8**: `restart_app` は install 成功後、Flush 完了後にのみ呼ばれる (呼出順序は page-main の責務)。再起動が失敗したら UI が手動再起動を促す (UI 層の責務)
- **I-U10**: 同時に実行する install は 1 本だけ。実行中 / 成功後の再起動待ちは、どの UI からの要求も `InstallInProgress` で拒否する

### slice 固有制約 {#invariants-slice-specific}

- **C-IU1**: 1 回の `install_update` で updater の fetch は 1 回、download_and_install は高々 1 回
- **C-IU2**: version 比較は domain (`Version` の Ord) が担う。updater plugin の comparator は permissive にする (check-for-updates と同じ方針)
- **C-IU3**: `NoUpdateAvailable` / `UnsupportedInstallation` / `CheckFailed` のときは download_and_install を呼ばない
- **C-IU4**: `InstallationKind` は `BundleType` から導出する: `App`/`Dmg` → `MacApp`、`AppImage`/`Deb`/`Rpm`/`Msi`/`Nsis` は同名、`None` (bundle 種別なし) → `Unbundled`。開発ビルド (`debug_assertions`) は bundle 種別によらず `Unbundled` (macOS は bundler 未経由でも `App` を返すため)
- **C-IU5**: check-for-updates の揮発 state (通知済みバージョン I-U4 / 前回確認時刻) を読まない・変えない
- **C-IU6**: 実行状態 `InstallPhase` (`Idle` / `Installing` / `Installed`) はプロセス内の揮発 state。`lib.rs` で `Arc<InMemoryInstallState>` を 1 つ manage し、toast / 設定モーダルのどちらから invoke されても同じ instance を使う。判定と遷移は Mutex 内で 1 操作 (`try_begin`) で行う
- **C-IU7**: `Unbundled` の判定 (step 1) は slot を取る前に行う。slot を取った後は、成功なら `Installed` (再起動まで保持)、失敗 (`NoUpdateAvailable` / `CheckFailed` / `DownloadFailed` / `InstallFailed`) なら `Idle` に戻す

## 境界契約 {#boundary-contract}

- boundary kind: `tauri_command`
- Rust: `apps/promptnotes/src-tauri/src/update_distribution/slices/install_update/commands.rs`
  - `install_update(app_handle) -> Result<String, InstallUpdateErrorResponse>` (async)
  - `is_update_installable() -> bool`
  - `restart_app(app_handle)`
  - `lib.rs` の `invoke_handler!` に配線
- TS: `apps/promptnotes/src/lib/update-distribution/slices/install-update/index.ts`
  - `installUpdate(): Promise<string>` (reject 値は `InstallUpdateErrorResponse`)
  - `isUpdateInstallable(): Promise<boolean>`
  - `restartApp(): Promise<void>`
- 禁止: UI 層が `@tauri-apps/plugin-updater` を直接呼んで install すること (I-U6 / I-U7 の判定を迂回する)

## テスト観点 {#test-points}

Rust 側は `install_update/tests.rs`。`UpdateInstaller` は adapter 境界の test double (`FakeInstaller`)。

### happy path {#tp-install-success}

- **TP-S1**: `Bundled(AppImage)`、current `0.2.4`、fetch `0.2.5` → `Ok(0.2.5)`、download_and_install 1 回 (S24)
- **TP-S2**: fetch した版が通知時と異なる新しい版 (`0.2.6`) でもそれを install して `Ok(0.2.6)`

### I-U6: 非対応配布形式 {#tp-unbundled}

- **TP-U1**: `Unbundled` → `Err(UnsupportedInstallation)`、fetch も install も 0 回 (S26)
- **TP-U2**: `InstallationKind::is_installable` は `Bundled(_)` 全 variant で true、`Unbundled` で false
- **TP-U3**: `BundleType` → `InstallationKind` の導出 (C-IU4)。`None` → `Unbundled`

### I-U7: 新しくない版 {#tp-not-newer}

- **TP-N1**: fetch `0.2.4` (同一) / `0.2.3` (古い) → `Err(NoUpdateAvailable)`、install 0 回
- **TP-N2**: proptest — 任意の (current, fetched) で install が呼ばれるのは `fetched > current` のときだけ

### 失敗の伝搬 {#tp-failures}

- **TP-F1**: fetch 失敗 → `Err(CheckFailed)`、install 0 回
- **TP-F2**: fetch した version が parse 不能 → `Err(CheckFailed)`
- **TP-F3**: download 失敗 → `Err(DownloadFailed)` / install 失敗 → `Err(InstallFailed)` (S25)、それぞれ fetch 1 回・install 1 回 (C-IU1)

- **TP-F4**: plugin の署名エラー (`SignatureUtf8` / `Minisign` / `Base64`) は `InstallFailed`、それ以外の download エラーは `DownloadFailed` に写す

### boundary test {#tp-boundary}

- **TP-B1**: TS — `installUpdate()` が `invoke('install_update')` を 1 回呼び、resolve 値を返す / reject 値をそのまま reject
- **TP-B2**: TS — `isUpdateInstallable()` / `restartApp()` がそれぞれ `invoke('is_update_installable')` / `invoke('restart_app')` を呼ぶ
- **TP-B3**: エラー DTO の `kind` が variant 名と一致する (Rust unit)

### I-U10: 同時実行の拒否 {#tp-concurrency}

- **TP-G1**: `InstallPhase` の遷移 — `Idle` だけが `Installing` に進め、`Installing` は成功で `Installed` / 失敗で `Idle`
- **TP-G2**: `Installing` 中の要求は `Err(InstallInProgress)`、fetch も install も 0 回、実行中の状態は変えない
- **TP-G3**: 成功後 (`Installed`) の再要求も `Err(InstallInProgress)` (再起動まで再インストールしない)
- **TP-G4**: 失敗 (`CheckFailed` / `NoUpdateAvailable` / `DownloadFailed` / `InstallFailed`) 後は `Idle` に戻り、再試行で install できる
- **TP-G5**: `Unbundled` は slot を取らない (`Idle` のまま)
- **TP-G6**: 2 thread から同時に要求すると、download 中の 1 本目だけが install し、2 本目は `InstallInProgress` (S29)
- **TP-G7**: TS — `isInstallInProgressError()` は `kind === 'InstallInProgress'` の DTO だけを true にする

## 実装ノート {#impl-notes}

### アーキ層への落とし込み {#impl-layers}

```
apps/promptnotes/src-tauri/src/update_distribution/
├── shared/
│   ├── types/installation_kind.rs  # InstallationKind / BundleFormat (新設)
│   ├── types/install_update_error.rs # InstallUpdateError (新設)
│   └── ports.rs                    # + UpdateInstaller / PendingUpdate / InstallGuard (I-U10)
└── slices/install_update/
    ├── mod.rs
    ├── domain.rs          # InstallUpdateCommand / InstallPhase / ensure_installable / ensure_newer (純粋)
    ├── application.rs     # InstallUpdateUseCase<I, G>: ensure_installable → try_begin → fetch → ensure_newer → download_and_install → finish
    ├── infrastructure.rs  # TauriUpdateInstaller / InMemoryInstallState / current_installation_kind()
    ├── commands.rs        # install_update / is_update_installable / restart_app
    └── tests.rs
```

- updater の async API は check-for-updates と同じく `block_in_place` で同期化する
- tauri-specta 未採用のため bindings 再生成は無い (DoD rule 4 対象外)

### Out of scope {#out-of-scope}

- download 進捗 (%) の通知
- 再起動前の Flush (page-main / widget-update-toast の責務)
- install 中のアプリ終了の扱い (updater plugin の挙動に従う)
