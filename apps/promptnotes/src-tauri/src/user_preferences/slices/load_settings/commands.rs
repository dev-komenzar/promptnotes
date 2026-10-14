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
use crate::user_preferences::shared::default_storage_dir::resolve_default_storage_dir;
use crate::user_preferences::shared::test_support::apply_storage_dir_override;
use crate::user_preferences::shared::types::Settings;

fn resolve_config_path<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    app.path()
        .app_config_dir()
        .ok()
        .map(|p| p.join("settings.json"))
        .unwrap_or_else(|| env::temp_dir().join("promptnotes/settings.json"))
}

/// E2E scenario override: `TAURI_TEST_STORAGE_DIR` が設定されていれば、永続化済み
/// `settings.json` の `storage_dir` より優先してテスト用ディレクトリへ差し替える。
/// wdio の `onPrepare` が起動前に env を設定する前提。
fn override_storage_dir_for_test(settings: Settings) -> Settings {
    apply_storage_dir_override(settings)
}

#[tauri::command]
pub async fn load_settings<R: Runtime>(app: AppHandle<R>) -> Settings {
    let default = resolve_default_storage_dir(&app);
    let config_path = resolve_config_path(&app);
    let uc = LoadSettingsUseCase::new(StdFileSystem, FixedOsDirs::new(default));
    override_storage_dir_for_test(uc.execute(LoadSettingsCommand { config_path }))
}
