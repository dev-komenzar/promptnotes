//! Tauri command surface for the `assign-tag` slice.
//!
//! Only this file depends on `tauri::*`. The frontend invokes `assign_tag`
//! when the user confirms a tag entry; the body of the file is mostly
//! serialization glue around the pure use case.

use serde::Serialize;
use tauri::{AppHandle, Runtime};
use time::OffsetDateTime;

use super::application::AssignTagUseCase;
use super::domain::{AssignTagCommand, AssignTagError};
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
pub enum AssignTagOutcome {
    Assigned {
        id: String,
        tags: Vec<String>,
        updated_at: String,
    },
    NoOp,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AssignTagErrorDto {
    NoteNotFound { id: String },
    InvalidTag { name: String, reason: String },
    LoadError { path: String, reason: String },
    PersistError { path: String, reason: String },
}

impl From<AssignTagError> for AssignTagErrorDto {
    fn from(e: AssignTagError) -> Self {
        match e {
            AssignTagError::NoteNotFound { id } => Self::NoteNotFound {
                id: id.as_str().to_string(),
            },
            AssignTagError::InvalidTag { name, reason } => Self::InvalidTag {
                name,
                reason: reason.to_string(),
            },
            AssignTagError::LoadError { path, source } => Self::LoadError {
                path: path.display().to_string(),
                reason: source.to_string(),
            },
            AssignTagError::PersistError { path, source } => Self::PersistError {
                path: path.display().to_string(),
                reason: source.to_string(),
            },
        }
    }
}

#[tauri::command]
pub async fn assign_tag<R: Runtime>(
    app: AppHandle<R>,
    note_id: String,
    raw_tag: String,
) -> Result<AssignTagOutcome, AssignTagErrorDto> {
    let storage_dir = resolve_storage_dir(&app);
    let uc = AssignTagUseCase::new(FsNoteRepository::new(storage_dir), SystemClock, NoOpBus);

    // NoteId has no validating constructor yet (spec.md#oq-invalid-note-id-reuse);
    // round-trip through Timestamp parsing as in auto-save-note.
    let cmd = AssignTagCommand {
        note_id: parse_note_id(&note_id),
        raw_tag,
    };

    match uc.execute(cmd) {
        Ok(Some(note)) => Ok(AssignTagOutcome::Assigned {
            id: note.id().as_str().to_string(),
            tags: note
                .tags()
                .as_slice()
                .iter()
                .map(|t| t.name().to_string())
                .collect(),
            updated_at: note.updated_at().format_rfc3339(),
        }),
        Ok(None) => Ok(AssignTagOutcome::NoOp),
        Err(e) => Err(e.into()),
    }
}

fn parse_note_id(raw: &str) -> NoteId {
    match Timestamp::parse_yyyymmddhhmmss(raw) {
        Ok(ts) => NoteId::from_timestamp(ts),
        Err(_) => {
            NoteId::from_timestamp(Timestamp::from_offset_datetime(OffsetDateTime::UNIX_EPOCH))
        }
    }
}
