//! Tauri command surface for the `change-sort-order` slice.
//!
//! Settings 永続化と NoteFeed in-memory 反映を 1 トランザクションで実行する
//! (`workflows/change-sort-order.md`)。`LoadSettingsUseCase` で現在値を解決した後、
//! `PreloadedFsSettingsRepository` + `TauriEventBus` を inject して
//! `ChangeSortOrderUseCase` を実行し、成功時のみ Tauri `State<InMemoryNoteFeedState>`
//! を更新する。エラーは [`ChangeSortOrderErrorDto`] で frontend へ surface。

use std::env;
use std::path::PathBuf;

use serde::Deserialize;
use std::sync::Arc;

use tauri::{AppHandle, Manager, Runtime, State};

use super::application::ChangeSortOrderUseCase;
use super::domain::{ChangeSortOrderCommand, ChangeSortOrderError};
use crate::note_feed::shared::adapters::InMemoryNoteFeedState;
use crate::user_preferences::shared::adapters::{PreloadedFsSettingsRepository, TauriEventBus};
use crate::user_preferences::shared::types::{SortOrder, StorageDir};
use crate::user_preferences::slices::load_settings::application::LoadSettingsUseCase;
use crate::user_preferences::slices::load_settings::domain::LoadSettingsCommand;
use crate::user_preferences::slices::load_settings::infrastructure::{FixedOsDirs, StdFileSystem};

#[derive(Debug, Deserialize)]
pub struct ChangeSortOrderInput {
    pub new_sort: SortOrder,
}

#[derive(Debug, serde::Serialize)]
pub struct NoteFeedDto {
    pub sort: SortOrder,
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChangeSortOrderErrorDto {
    /// `shared::types::PersistError` を surface (ori-hpo.8 / C-CSO6)。
    PersistError { path: String, reason: String },
}

impl From<ChangeSortOrderError> for ChangeSortOrderErrorDto {
    fn from(err: ChangeSortOrderError) -> Self {
        Self::PersistError {
            path: err.path.display().to_string(),
            reason: err.cause.to_string(),
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

#[tauri::command]
pub async fn change_sort_order<R: Runtime>(
    app: AppHandle<R>,
    feed_state: State<'_, Arc<InMemoryNoteFeedState>>,
    input: ChangeSortOrderInput,
) -> Result<NoteFeedDto, ChangeSortOrderErrorDto> {
    let config_path = resolve_config_path(&app);
    let default_storage_dir = resolve_default_storage_dir(&app);

    let loader = LoadSettingsUseCase::new(StdFileSystem, FixedOsDirs::new(default_storage_dir));
    let current_settings = loader.execute(LoadSettingsCommand {
        config_path: config_path.clone(),
    });

    let repo = PreloadedFsSettingsRepository::new(current_settings, config_path.clone());
    let bus = TauriEventBus::new(app.clone());
    let uc = ChangeSortOrderUseCase::new(repo, bus, config_path);

    let feed = feed_state.snapshot();
    let updated_feed = uc
        .execute(
            feed,
            ChangeSortOrderCommand {
                new_sort: input.new_sort,
            },
        )
        .map_err(ChangeSortOrderErrorDto::from)?;

    let sort = updated_feed.sort();
    feed_state.replace(updated_feed);
    Ok(NoteFeedDto { sort })
}
