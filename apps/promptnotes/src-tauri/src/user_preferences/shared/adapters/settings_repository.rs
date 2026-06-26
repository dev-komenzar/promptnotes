//! Production `SettingsRepository` adapter backed by `settings.json` file IO.
//!
//! `update-settings` slice 用。`load()` は composition root が事前解決した現在値を返し、
//! `save()` は `serde_json::to_string_pretty` で settings.json に書き出す。
//! parent directory が存在しなければ `create_dir_all` で確保する (load-settings の
//! ensure_dir とは別経路: config_path の親 = OS app_config_dir 側)。

use std::cell::RefCell;
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::user_preferences::shared::ports::SettingsRepository;
use crate::user_preferences::shared::types::Settings;

pub struct PreloadedFsSettingsRepository {
    current: RefCell<Settings>,
    config_path: PathBuf,
}

impl PreloadedFsSettingsRepository {
    pub fn new(initial: Settings, config_path: PathBuf) -> Self {
        Self {
            current: RefCell::new(initial),
            config_path,
        }
    }
}

impl SettingsRepository for PreloadedFsSettingsRepository {
    fn load(&self) -> Settings {
        self.current.borrow().clone()
    }

    fn save(&self, settings: &Settings) -> io::Result<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(settings)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(&self.config_path, json)?;
        *self.current.borrow_mut() = settings.clone();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user_preferences::shared::types::{
        SortDirection, SortField, SortOrder, StorageDir, Theme,
    };
    use tempfile::tempdir;

    fn sample_settings() -> Settings {
        Settings::new(
            StorageDir::try_from(PathBuf::from("/tmp/promptnotes-adapter-test/notes"))
                .expect("absolute"),
            Theme::Dark,
            SortOrder::new(SortField::UpdatedAt, SortDirection::Asc),
        )
    }

    #[test]
    fn save_writes_settings_json_and_load_returns_updated_value() {
        let dir = tempdir().expect("tempdir");
        let config_path = dir.path().join("settings.json");
        let initial = sample_settings();
        let repo = PreloadedFsSettingsRepository::new(initial.clone(), config_path.clone());

        assert_eq!(repo.load(), initial);
        assert!(!config_path.exists());

        let updated = initial.clone().change_theme(Theme::Light);
        repo.save(&updated).expect("save succeeds");

        assert!(config_path.exists());
        let raw = std::fs::read_to_string(&config_path).expect("read back");
        let parsed: Settings = serde_json::from_str(&raw).expect("deserialize");
        assert_eq!(parsed, updated);
        assert_eq!(repo.load(), updated);
    }

    #[test]
    fn save_creates_missing_parent_directories() {
        let dir = tempdir().expect("tempdir");
        let config_path = dir.path().join("nested/parent/settings.json");
        let repo = PreloadedFsSettingsRepository::new(sample_settings(), config_path.clone());
        repo.save(&sample_settings()).expect("save creates parents");
        assert!(config_path.exists());
    }
}
