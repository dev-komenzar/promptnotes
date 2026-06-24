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
}
