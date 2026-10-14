//! Tauri command surface for the `update-settings` slice.
//!
//! `LoadSettingsUseCase` で現在値を解決した後、`PreloadedFsSettingsRepository` と
//! `TauriEventBus` を通して `UpdateSettingsUseCase` を実行する。
//! 戻り値は更新後の `Settings`、エラーは `UpdateSettingsErrorDto` で frontend へ surface。

use std::env;
use std::path::PathBuf;

use serde::Deserialize;
use tauri::{AppHandle, Manager, Runtime};

use super::application::UpdateSettingsUseCase;
use super::domain::{PathErrorReason, UpdateSettingsCommand, UpdateSettingsError};
use crate::user_preferences::shared::adapters::{PreloadedFsSettingsRepository, TauriEventBus};
use crate::user_preferences::shared::default_storage_dir::resolve_default_storage_dir;
use crate::user_preferences::shared::types::{Settings, Theme};
use crate::user_preferences::slices::load_settings::application::LoadSettingsUseCase;
use crate::user_preferences::slices::load_settings::domain::LoadSettingsCommand;
use crate::user_preferences::slices::load_settings::infrastructure::{FixedOsDirs, StdFileSystem};

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsInput {
    #[serde(default)]
    pub storage_dir: Option<PathBuf>,
    #[serde(default)]
    pub theme: Option<Theme>,
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UpdateSettingsErrorDto {
    InvalidPath { path: String, reason: String },
    PersistError { path: String, reason: String },
}

impl From<UpdateSettingsError> for UpdateSettingsErrorDto {
    fn from(e: UpdateSettingsError) -> Self {
        match e {
            UpdateSettingsError::InvalidPath { path, reason } => Self::InvalidPath {
                path: path.display().to_string(),
                reason: match reason {
                    PathErrorReason::NotAbsolute => "not_absolute".into(),
                    PathErrorReason::ContainsConfigPath => "contains_config_path".into(),
                },
            },
            UpdateSettingsError::PersistError(err) => Self::PersistError {
                path: err.path.display().to_string(),
                reason: err.cause.to_string(),
            },
        }
    }
}

fn resolve_config_path<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    app.path()
        .app_config_dir()
        .ok()
        .map(|p| p.join("settings.json"))
        .unwrap_or_else(|| env::temp_dir().join("promptnotes/settings.json"))
}

#[tauri::command]
pub async fn update_settings<R: Runtime>(
    app: AppHandle<R>,
    input: UpdateSettingsInput,
) -> Result<Settings, UpdateSettingsErrorDto> {
    let config_path = resolve_config_path(&app);
    let default_storage_dir = resolve_default_storage_dir(&app);

    let loader = LoadSettingsUseCase::new(StdFileSystem, FixedOsDirs::new(default_storage_dir));
    let current = loader.execute(LoadSettingsCommand {
        config_path: config_path.clone(),
    });

    let repo = PreloadedFsSettingsRepository::new(current, config_path.clone());
    let bus = TauriEventBus::new(app.clone());
    let uc = UpdateSettingsUseCase::new(repo, bus, config_path);

    uc.execute(UpdateSettingsCommand {
        new_storage_dir: input.storage_dir,
        new_theme: input.theme,
    })
    .map_err(Into::into)
}
