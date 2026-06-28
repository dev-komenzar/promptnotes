//! Shared helper for resolving the note storage directory at command time.
//!
//! All 8 `note_capture` commands (create / auto_save / assign_tag / remove_tag /
//! delete / restore_deleted / copy_body / flush) need to honor the user-chosen
//! `Settings.storage_dir` instead of writing to a hard-coded
//! `<app_data_dir>/notes/`. Centralizing the lookup here keeps the
//! `LoadSettingsUseCase` wiring out of every slice and matches the pattern
//! already used by `note_feed::list_feed::commands`.

use std::env;
use std::path::PathBuf;

use tauri::{AppHandle, Manager, Runtime};

use crate::user_preferences::shared::types::StorageDir;
use crate::user_preferences::slices::load_settings::application::LoadSettingsUseCase;
use crate::user_preferences::slices::load_settings::domain::LoadSettingsCommand;
use crate::user_preferences::slices::load_settings::infrastructure::{FixedOsDirs, StdFileSystem};

fn resolve_config_path<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    app.path()
        .app_config_dir()
        .ok()
        .map(|p| p.join("settings.json"))
        .unwrap_or_else(|| env::temp_dir().join("promptnotes/settings.json"))
}

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

/// Read `settings.json` via `LoadSettingsUseCase` and return the resolved
/// `storage_dir` as a `PathBuf`. Falls back to `<app_data_dir>/notes/` (the
/// I-S3 default) when the file is missing or malformed — the same behavior
/// the load-settings slice gives the rest of the app.
pub fn resolve_storage_dir<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    let config_path = resolve_config_path(app);
    let default_storage_dir = resolve_default_storage_dir(app);
    let loader = LoadSettingsUseCase::new(StdFileSystem, FixedOsDirs::new(default_storage_dir));
    let settings = loader.execute(LoadSettingsCommand { config_path });
    settings.storage_dir().as_path().to_path_buf()
}
