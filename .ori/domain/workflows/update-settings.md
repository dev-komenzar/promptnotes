---
ori:
  node_id: workflow:update-settings
  type: workflow
  depends_on:
    - aggregate:Settings
    - event:StorageDirChanged
    - event:ThemeChanged
    - scenario:s11-storage-dir-change
---

# update-settings {#update-settings}

設定モーダルからの保存操作で Settings を更新する。
`storage_dir` / `theme` のいずれか（または両方）の変更を扱う。

## Input {#input}

```rust
struct UpdateSettingsCommand {
  new_storage_dir: Option<PathBuf>,
  new_theme: Option<Theme>,
  // sort_preference は change-sort-order workflow が担当
}
```

## Output {#output}

- `Settings`（更新後）
- domain events (差分に応じて 0〜2 件):
  - [StorageDirChanged](../domain-events.md#storage-dir-changed)
  - [ThemeChanged](../domain-events.md#theme-changed)

## Errors {#errors}

- `InvalidPath { path: PathBuf, reason: PathError }` — 絶対パス検証失敗（I-S1）
- `PersistError { path: PathBuf, cause: io::Error }` — `settings.json` 書き出し失敗

## Steps {#steps}

1. `loadCurrent: () → Settings`
2. `validateStorageDir: Option<PathBuf> → Result<Option<StorageDir>, InvalidPath>`
   - 絶対パスに正規化（None なら spread）
3. `applyChanges: (Settings, Option<StorageDir>, Option<Theme>) → (Settings, SettingsDiff)`
   - `SettingsDiff { storage_dir_changed: bool, theme_changed: bool }`
4. `persist: Settings → Result<(), PersistError>`
   - `app_config_dir/settings.json` 書き出し
5. `emitConditional: SettingsDiff → Vec<DomainEvent>`
   - storage_dir_changed → StorageDirChanged
   - theme_changed → ThemeChanged

## Dependencies {#dependencies}

- `SettingsRepository`
- `EventBus`

## Notes {#notes}

- **storage_dir 変更は即時マイグレーションしない**（I-S4, S11）
  → 再起動を促すモーダルは UI 層が StorageDirChanged を購読して表示
- **theme 変更は即時反映**（UI 層が ThemeChanged を購読して CodeMirror/CSS 更新）
- 両方を同時に変更した場合は 2 件の event を順次発行
- 差分なし（変更指示が現在値と同一）の場合は event 非発行
