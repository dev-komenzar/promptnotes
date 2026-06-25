//! Pure application layer for `load-settings`. No I/O: all side effects go
//! through the `FileSystem` / `OsDirs` ports. `serde_json` is used for
//! in-memory parsing only.

use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::user_preferences::shared::ports::{FileSystem, OsDirs};
use crate::user_preferences::shared::types::{
    Settings, SortDirection, SortField, SortOrder, StorageDir, Theme,
};

use super::domain::LoadSettingsCommand;

type JsonObject = Map<String, Value>;

/// `load-settings` の use case。常に `Settings` を返す (C-LS1)。
///
/// pipeline (domain/workflows/load-settings.md#steps):
/// 1. `try_read`: PathBuf → Option<String>
/// 2. `try_parse`: Option<String> → Option<Value> (top-level JSON)
/// 3. `apply_defaults`: field-level fallback (C-LS3 / C-LS4)
/// 4. `ensure_storage_dir`: 物理ディレクトリ作成 (C-LS5、失敗は silent / C-LS6)
pub struct LoadSettingsUseCase<F: FileSystem, O: OsDirs> {
    fs: F,
    os_dirs: O,
}

impl<F: FileSystem, O: OsDirs> LoadSettingsUseCase<F, O> {
    pub fn new(fs: F, os_dirs: O) -> Self {
        Self { fs, os_dirs }
    }

    pub fn execute(&self, cmd: LoadSettingsCommand) -> Settings {
        let obj = self
            .fs
            .try_read(&cmd.config_path)
            .as_deref()
            .and_then(parse_top_level_object);

        let storage_dir = self.resolve_storage_dir(obj.as_ref(), &cmd.config_path);
        let theme = pick_or_default::<Theme>(obj.as_ref(), "theme");
        let sort_preference = resolve_sort_preference(obj.as_ref());

        // C-LS6: ensure_dir の失敗は silent。Result を返さない。
        let _ = self.fs.ensure_dir(storage_dir.as_path());

        Settings::new(storage_dir, theme, sort_preference)
    }

    fn resolve_storage_dir(&self, obj: Option<&JsonObject>, config_path: &Path) -> StorageDir {
        obj.and_then(|m| m.get("storage_dir"))
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .and_then(|p| StorageDir::try_from(p).ok())
            .filter(|sd| !violates_i_s2(sd, config_path))
            .unwrap_or_else(|| self.os_dirs.default_storage_dir())
    }
}

/// JSON 文字列を「top-level Object」に絞り込んで返す (C-LS3: 構造的に Object でないもの — null / array / scalar — は全デフォルトへ降格)。
fn parse_top_level_object(content: &str) -> Option<JsonObject> {
    match serde_json::from_str::<Value>(content).ok()? {
        Value::Object(m) => Some(m),
        _ => None,
    }
}

/// `obj.get(key)` を `T` に decode 出来ない / 欠損 / null の場合 `T::default()` (C-LS4: field-level fallback)。
fn pick_or_default<T: DeserializeOwned + Default>(obj: Option<&JsonObject>, key: &str) -> T {
    obj.and_then(|m| m.get(key))
        .and_then(|v| serde_json::from_value::<T>(v.clone()).ok())
        .unwrap_or_default()
}

/// I-S2: `settings.json` は `storage_dir` 配下にしない（Q6: 循環参照回避）。
/// `config_path.starts_with(storage_dir)` ならば違反とみなす。
///
/// 同じ親ディレクトリ (e.g. `Application Support/promptnotes/{settings.json, notes/}`)
/// は許容される — config が storage_dir の **子孫** であるケースのみ reject する。
fn violates_i_s2(storage_dir: &StorageDir, config_path: &Path) -> bool {
    config_path.starts_with(storage_dir.as_path())
}

fn resolve_sort_preference(obj: Option<&JsonObject>) -> SortOrder {
    let Some(sub) = obj
        .and_then(|m| m.get("sort_preference"))
        .and_then(Value::as_object)
    else {
        return SortOrder::default();
    };
    let field = pick_or_default::<SortField>(Some(sub), "field");
    let direction = pick_or_default::<SortDirection>(Some(sub), "direction");
    SortOrder::new(field, direction)
}
