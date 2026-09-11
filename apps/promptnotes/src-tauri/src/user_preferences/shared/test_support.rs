use std::env;
use std::path::PathBuf;

use super::types::{Settings, StorageDir};

pub fn storage_dir_override() -> Option<StorageDir> {
    env::var("TAURI_TEST_STORAGE_DIR").ok().map(|raw| {
        StorageDir::try_from(PathBuf::from(raw))
            .expect("TAURI_TEST_STORAGE_DIR must be an absolute path (set by wdio onPrepare)")
    })
}

pub fn apply_storage_dir_override(settings: Settings) -> Settings {
    match storage_dir_override() {
        Some(dir) => Settings::new(dir, settings.theme(), settings.sort_preference()),
        None => settings,
    }
}
