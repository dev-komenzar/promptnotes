---
ori:
  node_id: workflow:change-sort-order
  type: workflow
  depends_on:
    - aggregate:NoteFeed
    - aggregate:Settings
    - event:SortPreferenceChanged
---

# change-sort-order {#change-sort-order}

ツールバーのソートトグルで NoteFeed の sort を変更し、同時に Settings に永続化する。
**aggregates.md で警告した「NoteFeed → Settings の唯一の逆流」**。

## Input {#input}

```rust
struct ChangeSortOrderCommand {
  new_sort: SortOrder,    // { field: createdAt|updatedAt, direction: asc|desc }
}
```

## Output {#output}

- `NoteFeed`（sort 更新後）
- domain event: [SortPreferenceChanged](../domain-events.md#sort-preference-changed)

## Errors {#errors}

- `PersistError { path: PathBuf, cause: io::Error }` — Settings 書き出し失敗

## Steps {#steps}

1. `loadSettings: () → Settings`
   - 現在の Settings を取得（in-memory cache）
2. `applySortToFeed: (NoteFeed, SortOrder) → NoteFeed`
   - NoteFeed.sort を更新（即時反映）
3. `applySortToSettings: (Settings, SortOrder) → Settings`
   - Settings::change_sort_preference
4. `persistSettings: Settings → Result<(), PersistError>`
   - `settings.json` 書き出し
5. `emit: SortOrder → SortPreferenceChanged`

## Dependencies {#dependencies}

- `SettingsRepository` — JSON 読み書き
- `EventBus`

## Notes {#notes}

- **冪等性**: 同一 sort への変更は no-op として扱う（UI でトグル状態が変わらないなら呼ばれない想定）
- **副作用の二重適用回避**: NoteFeed.change_sort 経由なので、`SortPreferenceChanged`
  の購読側は「NoteFeed が既に更新済み」と仮定して何もしない
  （別経路 = 設定モーダルからの変更時のみ NoteFeed を再ソート）
  → application service で適用済みフラグを管理
- 設定モーダル経由でも同じ workflow を再利用（trigger UI が違うだけ）
