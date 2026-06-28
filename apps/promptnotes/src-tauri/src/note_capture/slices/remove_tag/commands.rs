//! Tauri command surface for the `remove-tag` slice.
//!
//! Only this file depends on `tauri::*`. The frontend invokes `remove_tag`
//! when the user clicks the × button on a tag chip; the body of the file is
//! mostly serialization glue around the pure use case.

use serde::Serialize;
use tauri::{AppHandle, Runtime};
use time::OffsetDateTime;

use super::application::RemoveTagUseCase;
use super::domain::{RemoveTagCommand, RemoveTagError};
use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, EventBus};
use crate::note_capture::shared::storage::resolve_storage_dir;
use crate::note_capture::shared::types::{NoteId, Timestamp};
use crate::note_capture::slices::create_note::FsNoteRepository;

struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_offset_datetime(OffsetDateTime::now_utc())
    }
}

struct NoOpBus;
impl EventBus for NoOpBus {
    fn publish(&self, _event: DomainEvent) {
        // The Note Feed BC will subscribe here once it lands.
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum RemoveTagOutcome {
    Removed {
        id: String,
        tags: Vec<String>,
        updated_at: String,
    },
    NoOp,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RemoveTagErrorDto {
    NoteNotFound { id: String },
    LoadError { path: String, reason: String },
    PersistError { path: String, reason: String },
}

impl From<RemoveTagError> for RemoveTagErrorDto {
    fn from(e: RemoveTagError) -> Self {
        match e {
            RemoveTagError::NoteNotFound { id } => Self::NoteNotFound {
                id: id.as_str().to_string(),
            },
            RemoveTagError::LoadError { path, source } => Self::LoadError {
                path: path.display().to_string(),
                reason: source.to_string(),
            },
            RemoveTagError::PersistError { path, source } => Self::PersistError {
                path: path.display().to_string(),
                reason: source.to_string(),
            },
        }
    }
}

#[tauri::command]
pub async fn remove_tag<R: Runtime>(
    app: AppHandle<R>,
    note_id: String,
    tag_name: String,
) -> Result<RemoveTagOutcome, RemoveTagErrorDto> {
    let storage_dir = resolve_storage_dir(&app);
    let uc = RemoveTagUseCase::new(FsNoteRepository::new(storage_dir), SystemClock, NoOpBus);

    let cmd = RemoveTagCommand {
        note_id: parse_note_id(&note_id),
        tag_name,
    };

    match uc.execute(cmd) {
        Ok(Some(note)) => Ok(RemoveTagOutcome::Removed {
            id: note.id().as_str().to_string(),
            tags: note
                .tags()
                .as_slice()
                .iter()
                .map(|t| t.name().to_string())
                .collect(),
            updated_at: note.updated_at().format_rfc3339(),
        }),
        Ok(None) => Ok(RemoveTagOutcome::NoOp),
        Err(e) => Err(e.into()),
    }
}

fn parse_note_id(raw: &str) -> NoteId {
    match Timestamp::parse_yyyymmddhhmmss(raw) {
        Ok(ts) => NoteId::from_timestamp(ts),
        Err(_) => NoteId::from_timestamp(Timestamp::from_offset_datetime(
            OffsetDateTime::UNIX_EPOCH,
        )),
    }
}
