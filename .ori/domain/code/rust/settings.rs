use crate::errors::{InvalidPath, PathError};
use crate::note_feed::SortOrder;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    System,
    Light,
    Dark,
}

impl Default for Theme {
    fn default() -> Self {
        Self::System
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageDir(PathBuf);

impl StorageDir {
    pub fn try_from_path(p: PathBuf) -> Result<Self, InvalidPath> {
        if p.is_absolute() {
            Ok(Self(p))
        } else {
            Err(InvalidPath {
                path: p.clone(),
                reason: PathError::NotAbsolute(p),
            })
        }
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }

    pub fn into_path_buf(self) -> PathBuf {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub storage_dir: StorageDir,
    pub theme: Theme,
    pub sort_preference: SortOrder,
}

impl Settings {
    pub fn defaults(default_storage_dir: StorageDir) -> Self {
        Self {
            storage_dir: default_storage_dir,
            theme: Theme::default(),
            sort_preference: SortOrder::default_value(),
        }
    }

    pub fn change_storage_dir(mut self, new_dir: StorageDir) -> Self {
        self.storage_dir = new_dir;
        self
    }

    pub fn change_theme(mut self, new_theme: Theme) -> Self {
        self.theme = new_theme;
        self
    }

    pub fn change_sort_preference(mut self, new_sort: SortOrder) -> Self {
        self.sort_preference = new_sort;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsDiff {
    pub storage_dir_changed: bool,
    pub theme_changed: bool,
}
