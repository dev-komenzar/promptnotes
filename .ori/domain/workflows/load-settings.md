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
2. `tryParse: Option<String> → Option<JsonObject>`
   - 「top-level が JSON Object である」場合のみ `Some`。
     parse 失敗 (構文エラー) / null / array / scalar はすべて `None`
3. `applyDefaults: Option<JsonObject> → Settings`
   - **フィールド単位** で値を取り出し、欠損 / 不正な型のフィールドのみデフォルト (I-S3) で補完
   - `Option<JsonObject>` が `None` の場合は「全フィールドが欠損した Object」として同じ経路を通る (degenerate case)
   - デフォルト:
     - `storage_dir`: OS 慣習パス
     - `theme`: `System`
     - `sort_preference`: `{ createdAt, desc }`
4. `ensureStorageDir: StorageDir → ()`
   - ディレクトリが存在しなければ作成（初回起動）
   - **冪等性は `FileSystem::ensure_dir` (mkdir -p 相当) の責務**。workflow は stateless で、同じ入力に対して同じ `Settings` を返すことが冪等性の保証範囲

## Dependencies {#dependencies}

- `SettingsRepository`
- `FileSystem`（ディレクトリ作成）
- `OsDirs` — OS 慣習パス取得（macOS/Linux/Windows）

## Notes {#notes}

- 部分的に壊れた `settings.json` の扱いは **フィールド単位 fallback** を採用
  - JSON 構造として valid なら、欠損 / 不正な型のフィールドのみ I-S3 で補完する。他フィールドは保持
  - JSON が Object でない (parse 失敗 / null / array / scalar) 場合は「全フィールドが欠損した Object」として同経路を通り、結果的に全フィールド I-S3 になる (degenerate case)
  - UX 上の根拠: `theme` の typo 1 つで `storage_dir` まで巻き戻すと user の設定が壊滅的に失われるため、フィールド単位 isolate が望ましい
- 冪等性は `FileSystem::ensure_dir` の `mkdir -p` 相当契約に委譲する
  - workflow 本体は stateless: 同じ `config_path` + 同じファイル内容で何度呼んでも同じ `Settings` を返す
  - `ensureStorageDir` は毎回呼ばれてよい (2 回目以降の `mkdir` 失敗は no-op であることを `FileSystem` impl が保証する)
- NoteFeed の初期化はこの workflow の **後段** で実行される
  - filter は常に空（S12）、sort は `Settings.sort_preference` から復元
- このworkflow 自体は domain event を発行しないが、後段で NoteFeed 構築 →
  Note 群の load → 初回 NoteCreated 系 event は発行しない（既存ファイル読み込みのため）
