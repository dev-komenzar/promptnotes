---
ori:
  node_id: workflow:get-default-storage-dir
  type: workflow
  depends_on:
    - aggregate:Settings
---

# get-default-storage-dir {#get-default-storage-dir}

設定モーダルの「デフォルトに戻す」（[screen-2](../ui-fields/screen-2.md#cross-storage-dir-reset)）のために、
OS 慣習に基づく default `storage_dir`（[I-S3](../aggregates.md#settings-aggregate-invariants)）を返す
読み取り専用 query。`settings.json` は読まない / 書かない。

## Input {#input}

```rust
struct GetDefaultStorageDirQuery;    // 入力なし（OS 慣習パスを解決するだけ）
```

## Output {#output}

- `StorageDir` — OS 慣習の default 保存先（I-S1 を満たす絶対パス）
  - macOS `~/Library/Application Support/promptnotes/notes/`
  - Linux `~/.local/share/promptnotes/notes/`
  - Windows `%APPDATA%\promptnotes\notes\`
- domain event: **なし**（状態を変更しない query のため）

## Errors {#errors}

- なし（OS dir の解決に失敗した場合は load-settings と同じ fallback で一時ディレクトリ配下を返す）

## Steps {#steps}

1. `resolveDefaultStorageDir: () → StorageDir`
   - `OsDirs::default_storage_dir`（port）から OS 慣習パスを得る
   - port 契約により I-S1 / I-S2 を満たす（[aggregates](../aggregates.md#settings-aggregate-invariants)）

## Dependencies {#dependencies}

- `OsDirs` port（load-settings と共有）。`FileSystem` / `SettingsRepository` / `EventBus` は使わない

## Notes {#notes}

- load-settings が `settings.json` 不在時に使う default と **同じ情報源** を使う
  （両者の default が食い違ってはならない）
- 本 query は Settings を変更しない。default への切替の永続化と `StorageDirChanged` の発行は、
  ユーザが保存したときに [update-settings](update-settings.md) が行う
- 呼び出しは widget-settings-modal の mount 時。値は起動中不変なので結果をキャッシュしてよい
