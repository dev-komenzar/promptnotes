use serde::{Deserialize, Serialize};

use super::{SortOrder, StorageDir, Theme};

/// User Preferences BC の唯一の集約 root。`load_or_default` 経路で復元、`change_*` 経路で更新する。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    storage_dir: StorageDir,
    theme: Theme,
    sort_preference: SortOrder,
}

impl Settings {
    pub fn new(storage_dir: StorageDir, theme: Theme, sort_preference: SortOrder) -> Self {
        Self {
            storage_dir,
            theme,
            sort_preference,
        }
    }

    pub fn storage_dir(&self) -> &StorageDir {
        &self.storage_dir
    }

    pub fn theme(&self) -> Theme {
        self.theme
    }

    pub fn sort_preference(&self) -> SortOrder {
        self.sort_preference
    }

    /// `aggregates.md#settings-aggregate-operations` の `change_storage_dir`。
    /// 妥当性検証 (I-S1 / I-S2) は呼び出し側 (`update-settings` slice) の責務。
    pub fn change_storage_dir(mut self, new_dir: StorageDir) -> Self {
        self.storage_dir = new_dir;
        self
    }

    /// `aggregates.md#settings-aggregate-operations` の `change_theme`。
    pub fn change_theme(mut self, new_theme: Theme) -> Self {
        self.theme = new_theme;
        self
    }

    /// `aggregates.md#settings-aggregate-operations` の `change_sort_preference`。
    /// `change-sort-order` slice が NoteFeed.change_sort と同期して呼ぶ
    /// (Customer-Supplier の逆流、`#notes-sort-side-effect`)。
    pub fn change_sort_preference(mut self, new_sort: SortOrder) -> Self {
        self.sort_preference = new_sort;
        self
    }
}
