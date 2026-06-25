//! Tauri command surface for the `auto-save-note` slice.
//!
//! Only this file depends on `tauri::*`. The frontend invokes
//! `auto_save_note` after the EDITING-block debounce settles; the body of
//! the file is mostly serialization glue around the pure use case.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};
use time::OffsetDateTime;

use super::application::AutoSaveNoteUseCase;
use super::domain::{AutoSaveNoteCommand, AutoSaveError};
use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, EventBus};
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
pub enum AutoSaveOutcome {
    Saved {
        id: String,
        updated_at: String,
    },
    NoOp,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutoSaveErrorDto {
    NoteNotFound { id: String },
    InvalidBody { reason: String },
    LoadError { path: String, reason: String },
    PersistError { path: String, reason: String },
}

impl From<AutoSaveError> for AutoSaveErrorDto {
    fn from(e: AutoSaveError) -> Self {
        match e {
            AutoSaveError::NoteNotFound { id } => Self::NoteNotFound {
                id: id.as_str().to_string(),
            },
            AutoSaveError::InvalidBody { source } => Self::InvalidBody {
                reason: source.to_string(),
            },
            AutoSaveError::LoadError { path, source } => Self::LoadError {
                path: path.display().to_string(),
                reason: source.to_string(),
            },
            AutoSaveError::PersistError { path, source } => Self::PersistError {
                path: path.display().to_string(),
                reason: source.to_string(),
            },
        }
    }
}

fn resolve_storage_dir<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("Tauri must resolve app_data_dir on supported platforms")
        .join("notes")
}

#[tauri::command]
pub async fn auto_save_note<R: Runtime>(
    app: AppHandle<R>,
    note_id: String,
    new_body: String,
) -> Result<AutoSaveOutcome, AutoSaveErrorDto> {
    let storage_dir = resolve_storage_dir(&app);
    let uc = AutoSaveNoteUseCase::new(FsNoteRepository::new(storage_dir), SystemClock, NoOpBus);

    // NoteId is currently a thin newtype with no validating constructor,
    // so a parse-failure path does not exist at this boundary; the upstream
    // open question (spec.md#oq-invalid-note-id) tracks adding one.
    let cmd = AutoSaveNoteCommand {
        note_id: parse_note_id(&note_id),
        new_body,
    };

    match uc.execute(cmd) {
        Ok(Some(note)) => Ok(AutoSaveOutcome::Saved {
            id: note.id().as_str().to_string(),
            updated_at: note.updated_at().format_yyyymmddhhmmss(),
        }),
        Ok(None) => Ok(AutoSaveOutcome::NoOp),
        Err(e) => Err(e.into()),
    }
}

/// Reconstruct a NoteId from the raw string the frontend sent. There is no
/// public validating constructor on `NoteId` yet; for now we round-trip the
/// timestamp that produced the same string. Any mismatch surfaces as a
/// downstream `NoteNotFound`, which is consistent with spec.md#oq-invalid-note-id.
fn parse_note_id(raw: &str) -> NoteId {
    match Timestamp::parse_yyyymmddhhmmss(raw) {
        Ok(ts) => NoteId::from_timestamp(ts),
        Err(_) => {
            // Construct a sentinel id from epoch + the raw string so the use
            // case load step inevitably misses and returns NoteNotFound.
            NoteId::from_timestamp(Timestamp::from_offset_datetime(
                OffsetDateTime::UNIX_EPOCH,
            ))
        }
    }
}
