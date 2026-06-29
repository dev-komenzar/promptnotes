//! Domain types for `update-settings` slice. Pure: no I/O.
//!
//! Re-exports `SettingsEvent` from `shared::types` for slice ergonomics.
//! `PersistError` は `shared::types::PersistError` を利用 (ori-hpo.8 抽出)。

use std::path::PathBuf;

use thiserror::Error;

pub use crate::user_preferences::shared::types::PersistError;
pub use crate::user_preferences::shared::types::SettingsEvent;
use crate::user_preferences::shared::types::Theme;

/// `update-settings` slice の input (`workflows/update-settings.md#input`)。
///
/// `sort_preference` は `change-sort-order` workflow の担当のため本 command には含めない。
#[derive(Debug, Clone)]
pub struct UpdateSettingsCommand {
    pub new_storage_dir: Option<PathBuf>,
    pub new_theme: Option<Theme>,
}

/// `applyChanges` で算出される変更フラグ (`workflows/update-settings.md#steps`)。
///
/// 両 field が `false` の場合は no-op として persist / publish を skip する (C-US1 / C-US2)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsDiff {
    pub storage_dir_changed: bool,
    pub theme_changed: bool,
}

impl SettingsDiff {
    pub fn is_noop(&self) -> bool {
        !self.storage_dir_changed && !self.theme_changed
    }
}

/// `update-settings` slice のエラー型 (`workflows/update-settings.md#errors`)。
///
/// `PersistError` variant は `shared::types::PersistError` を wrap する (ori-hpo.8)。
/// これにより `change-sort-order` slice が `shared::types::PersistError` を直接
/// 参照でき、slice 間直接依存 (oq-cross-bc-import) が解消される。
#[derive(Debug, Error)]
pub enum UpdateSettingsError {
    /// I-S1 (絶対パス) または I-S2 (循環参照禁止) 違反。
    #[error("invalid storage_dir path {path:?}: {reason}")]
    InvalidPath {
        path: PathBuf,
        reason: PathErrorReason,
    },
    /// `SettingsRepository::save` 失敗 (`shared::types::PersistError` を wrap)。
    #[error(transparent)]
    PersistError(#[from] PersistError),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PathErrorReason {
    #[error("path must be absolute")]
    NotAbsolute,
    #[error("storage_dir must not contain config_path (I-S2)")]
    ContainsConfigPath,
}
