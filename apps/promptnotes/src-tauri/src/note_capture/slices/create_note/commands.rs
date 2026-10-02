//! Tauri command surface for the `create-note` slice.
//!
//! This is the only file in the slice that depends on `tauri::*`. The
//! `#[tauri::command]` function receives raw input from the frontend, runs the
//! pure use case, and serializes the result as `serde` DTOs. A future
//! `tauri-specta` integration will emit TypeScript bindings from this file.

use serde::Serialize;
use tauri::{AppHandle, Runtime};
use time::OffsetDateTime;

use super::application::CreateNoteUseCase;
use super::domain::{CreateNoteCommand, CreateNoteError};
use super::infrastructure::FsNoteRepository;
use crate::note_capture::shared::adapters::event_bus::TauriEventBus;
use crate::note_capture::shared::ports::Clock;
use crate::note_capture::shared::storage::resolve_storage_dir;
use crate::note_capture::shared::types::Timestamp;

struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_offset_datetime(OffsetDateTime::now_utc())
    }
}


#[derive(Debug, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum CreateNoteOutcome {
    Created { id: String, created_at: String },
    NoOp,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CreateNoteErrorDto {
    InvalidTag { raw: String, reason: String },
    InvalidBody { reason: String },
    PersistError { path: String, reason: String },
}

impl From<CreateNoteError> for CreateNoteErrorDto {
    fn from(e: CreateNoteError) -> Self {
        match e {
            CreateNoteError::InvalidTag { raw, source } => Self::InvalidTag {
                raw,
                reason: source.to_string(),
            },
            CreateNoteError::InvalidBody { source } => Self::InvalidBody {
                reason: source.to_string(),
            },
            CreateNoteError::PersistError { path, source } => Self::PersistError {
                path: path.display().to_string(),
                reason: source.to_string(),
            },
        }
    }
}

#[tauri::command]
pub async fn create_note<R: Runtime>(
    app: AppHandle<R>,
    raw_body: String,
    raw_tags: Vec<String>,
) -> Result<CreateNoteOutcome, CreateNoteErrorDto> {
    let storage_dir = resolve_storage_dir(&app);
    let uc = CreateNoteUseCase::new(FsNoteRepository::new(storage_dir), SystemClock, TauriEventBus::new(app));

    match uc.execute(CreateNoteCommand { raw_body, raw_tags }) {
        Ok(Some(note)) => Ok(CreateNoteOutcome::Created {
            id: note.id().as_str().to_string(),
            created_at: note.created_at().format_rfc3339(),
        }),
        Ok(None) => Ok(CreateNoteOutcome::NoOp),
        Err(e) => Err(e.into()),
    }
}
