//! Tauri command surface for the `load-settings` slice.
//!
//! 起動時に `app_config_dir/settings.json` を読み、Settings を復元する。
//! 戻り値は常に `Settings`（C-LS1: no Result）。失敗は I-S3 defaults に降格 + ensure_dir silent。

use std::env;
use std::path::PathBuf;

use tauri::{AppHandle, Manager, Runtime};

use super::application::LoadSettingsUseCase;
use super::domain::LoadSettingsCommand;
use super::infrastructure::{FixedOsDirs, StdFileSystem};
use crate::user_preferences::shared::types::{Settings, StorageDir};

/// Tauri が `app_data_dir()` 解決に失敗した場合の最終 fallback。
/// `std::env::temp_dir()` は POSIX/Windows ともに OS 契約上 **絶対パス** を返すため、
/// `StorageDir::try_from` は必ず成功する。
fn resolve_default_storage_dir<R: Runtime>(app: &AppHandle<R>) -> StorageDir {
    let candidate = app
        .path()
        .app_data_dir()
        .ok()
        .map(|p| p.join("notes"))
        .unwrap_or_else(|| env::temp_dir().join("promptnotes/notes"));
    StorageDir::try_from(candidate).unwrap_or_else(|_| {
        StorageDir::try_from(env::temp_dir())
            .expect("std::env::temp_dir() is absolute by OS contract")
    })
}

fn resolve_config_path<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    app.path()
        .app_config_dir()
        .ok()
        .map(|p| p.join("settings.json"))
        .unwrap_or_else(|| env::temp_dir().join("promptnotes/settings.json"))
}

#[tauri::command]
pub async fn load_settings<R: Runtime>(app: AppHandle<R>) -> Settings {
    let default = resolve_default_storage_dir(&app);
    let config_path = resolve_config_path(&app);
    let uc = LoadSettingsUseCase::new(StdFileSystem, FixedOsDirs::new(default));
    uc.execute(LoadSettingsCommand { config_path })
}
