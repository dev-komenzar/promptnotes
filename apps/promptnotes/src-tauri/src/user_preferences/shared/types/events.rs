use std::path::PathBuf;

use super::{SortOrder, Theme};

/// User Preferences BC が発行する domain event (`domain-events.md#settings-aggregate-events`)。
///
/// 発行元 slice:
/// - `update-settings`: `StorageDirChanged` / `ThemeChanged` を差分に応じて 0〜2 件
/// - `change-sort-order`: `SortPreferenceChanged` を 1 件 (NoteFeed → Settings の唯一の逆流)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsEvent {
    /// `Settings::change_storage_dir` 永続化成功時 (`domain-events.md#storage-dir-changed`)。
    StorageDirChanged { old_dir: PathBuf, new_dir: PathBuf },
    /// `Settings::change_theme` 永続化成功時 (`domain-events.md#theme-changed`)。
    ThemeChanged { new_theme: Theme },
    /// `Settings::change_sort_preference` 永続化成功時 (`domain-events.md#sort-preference-changed`)。
    /// `change-sort-order` slice からのみ発行される。
    SortPreferenceChanged { new_sort: SortOrder },
}
