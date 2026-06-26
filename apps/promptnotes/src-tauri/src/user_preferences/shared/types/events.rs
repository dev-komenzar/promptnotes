use std::path::PathBuf;

use super::Theme;

/// User Preferences BC が発行する domain event (`domain-events.md#settings-aggregate-events`)。
///
/// `update-settings` slice の差分検出結果に応じて 0〜2 件発行される。
/// 順序は **`StorageDirChanged` → `ThemeChanged`**（spec.md#tp-event-order C-US5）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsEvent {
    /// `Settings::change_storage_dir` 永続化成功時 (`domain-events.md#storage-dir-changed`)。
    StorageDirChanged { old_dir: PathBuf, new_dir: PathBuf },
    /// `Settings::change_theme` 永続化成功時 (`domain-events.md#theme-changed`)。
    ThemeChanged { new_theme: Theme },
}
