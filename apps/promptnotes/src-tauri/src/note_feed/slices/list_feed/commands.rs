//! Tauri command surface for the `list-feed` slice.
//!
//! 起動時 / 手動 Refresh の唯一のエントリポイント。LoadSettingsUseCase で `storage_dir`
//! を解決した後、`FsNoteRepository::list_all` で `.md` を全件読み、`NoteFeed.source`
//! を hydrate する。可視 read DTO (`NoteFeedDto`) は `now()` 注入で date_range を評価する。

use std::env;
use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime, State};
use time::OffsetDateTime;

use super::application::{visible_notes_snapshot, ListFeedUseCase};
use super::domain::ListFeedCommand;
use crate::note_capture::shared::types::Note;
use crate::note_capture::slices::create_note::infrastructure::FsNoteRepository;
use crate::note_feed::shared::adapters::InMemoryNoteFeedState;
use crate::user_preferences::shared::types::StorageDir;
use crate::user_preferences::slices::load_settings::application::LoadSettingsUseCase;
use crate::user_preferences::slices::load_settings::domain::LoadSettingsCommand;
use crate::user_preferences::slices::load_settings::infrastructure::{FixedOsDirs, StdFileSystem};

#[derive(Debug, Clone, Serialize)]
pub struct NoteSummaryDto {
    pub id: String,
    pub body: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&Note> for NoteSummaryDto {
    fn from(n: &Note) -> Self {
        Self {
            id: n.id().as_str().to_string(),
            body: n.body().as_str().to_string(),
            tags: n
                .tags()
                .as_slice()
                .iter()
                .map(|t| t.name().to_string())
                .collect(),
            created_at: n.created_at().format_rfc3339(),
            updated_at: n.updated_at().format_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NoteFeedDto {
    pub notes: Vec<NoteSummaryDto>,
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
pub async fn list_notes<R: Runtime>(
    app: AppHandle<R>,
    feed_state: State<'_, InMemoryNoteFeedState>,
) -> Result<NoteFeedDto, String> {
    let config_path = resolve_config_path(&app);
    let default_storage_dir = resolve_default_storage_dir(&app);
    let loader = LoadSettingsUseCase::new(StdFileSystem, FixedOsDirs::new(default_storage_dir));
    let settings = loader.execute(LoadSettingsCommand { config_path });

    let storage_dir = settings.storage_dir().as_path().to_path_buf();
    let repo = FsNoteRepository::new(storage_dir);

    let feed = feed_state.snapshot();
    let uc = ListFeedUseCase::new(repo);
    let hydrated = uc
        .execute(feed, ListFeedCommand)
        .map_err(|e| e.to_string())?;
    let hydrated = hydrated.change_sort(settings.sort_preference());

    let now = OffsetDateTime::now_utc();
    let visible = visible_notes_snapshot(&hydrated, now);
    let dto = NoteFeedDto {
        notes: visible.iter().map(NoteSummaryDto::from).collect(),
    };
    feed_state.replace(hydrated);
    Ok(dto)
}
