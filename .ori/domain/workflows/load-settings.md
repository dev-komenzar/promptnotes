---
ori:
  node_id: workflow:load-settings
  type: workflow
  depends_on:
    - aggregate:Settings
    - scenario:s12-startup-state
---

# load-settings {#load-settings}

アプリ起動時に `app_config_dir/settings.json` を読み込み、
不在 / 不正時はデフォルト値で初期化する。

## Input {#input}

```rust
struct LoadSettingsCommand {
  config_path: PathBuf,    // app_config_dir/settings.json
}
```

## Output {#output}

- `Settings`
- domain event: **なし**（起動時の単純な状態復元のため）

## Errors {#errors}

- なし（不在 / parse 失敗はデフォルトに fallback）

## Steps {#steps}

1. `tryRead: PathBuf → Option<String>`
   - 不在なら `None`
2. `tryParse: Option<String> → Option<Settings>`
   - parse 失敗（不正な JSON 等）なら `None`
3. `applyDefaults: Option<Settings> → Settings`
   - `None` または欠損フィールドはデフォルト（I-S3）で補完
     - `storage_dir`: OS 慣習パス
     - `theme`: `System`
     - `sort_preference`: `{ createdAt, desc }`
4. `ensureStorageDir: StorageDir → ()`
   - ディレクトリが存在しなければ作成（初回起動）

## Dependencies {#dependencies}

- `SettingsRepository`
- `FileSystem`（ディレクトリ作成）
- `OsDirs` — OS 慣習パス取得（macOS/Linux/Windows）

## Notes {#notes}

- 部分的に壊れた `settings.json` の扱いは「全フィールドデフォルト」を採用（保守的）
  - 将来「フィールド単位でデフォルト補完」に変えてもよい
- NoteFeed の初期化はこの workflow の **後段** で実行される
  - filter は常に空（S12）、sort は `Settings.sort_preference` から復元
- このworkflow 自体は domain event を発行しないが、後段で NoteFeed 構築 →
  Note 群の load → 初回 NoteCreated 系 event は発行しない（既存ファイル読み込みのため）
