//! Tauri command surface for the `flush-note` slice.
//!
//! Only this file depends on `tauri::*`. The frontend invokes `flush_note`
//! on block focus loss, window blur, or app-quit; the body of the file is
//! mostly serialization glue around the pure use case.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};
use time::OffsetDateTime;

use super::application::FlushNoteUseCase;
use super::domain::{FlushError, FlushNoteCommand, FlushTrigger};
use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, DebounceTimer, EventBus};
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

/// Cancellation is owned by the UI-side timer; the backend command is the
/// trailing edge that just records "we flushed for this id". `NoOpDebounceTimer`
/// keeps the pipeline honest (C-FL1 still runs) without coupling Rust to
/// the JS timer handle.
struct NoOpDebounceTimer;
impl DebounceTimer for NoOpDebounceTimer {
    fn cancel(&self, _note_id: &NoteId) {}
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlushTriggerDto {
    BlockBlur,
    WindowBlur,
    AppQuit,
}

impl From<FlushTriggerDto> for FlushTrigger {
    fn from(dto: FlushTriggerDto) -> Self {
        match dto {
            FlushTriggerDto::BlockBlur => FlushTrigger::BlockBlur,
            FlushTriggerDto::WindowBlur => FlushTrigger::WindowBlur,
            FlushTriggerDto::AppQuit => FlushTrigger::AppQuit,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum FlushOutcome {
    Flushed { id: String, updated_at: String },
    NoOp,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FlushErrorDto {
    NoteNotFound { id: String },
    InvalidBody { reason: String },
    LoadError { path: String, reason: String },
    PersistError { path: String, reason: String },
}

impl From<FlushError> for FlushErrorDto {
    fn from(e: FlushError) -> Self {
        match e {
            FlushError::NoteNotFound { id } => Self::NoteNotFound {
                id: id.as_str().to_string(),
            },
            FlushError::InvalidBody { source } => Self::InvalidBody {
                reason: source.to_string(),
            },
            FlushError::LoadError { path, source } => Self::LoadError {
                path: path.display().to_string(),
                reason: source.to_string(),
            },
            FlushError::PersistError { path, source } => Self::PersistError {
                path: path.display().to_string(),
                reason: source.to_string(),
            },
        }
    }
}

/// Same `NoteId` boundary handling as `auto-save-note`. See
/// `.ori/slices/flush-note/spec.md#oq-invalid-note-id`.
fn parse_note_id(raw: &str) -> NoteId {
    match Timestamp::parse_yyyymmddhhmmss(raw) {
        Ok(ts) => NoteId::from_timestamp(ts),
        Err(_) => {
            NoteId::from_timestamp(Timestamp::from_offset_datetime(OffsetDateTime::UNIX_EPOCH))
        }
    }
}

#[tauri::command]
pub async fn flush_note<R: Runtime>(
    app: AppHandle<R>,
    note_id: String,
    pending_body: String,
    trigger: FlushTriggerDto,
) -> Result<FlushOutcome, FlushErrorDto> {
    let storage_dir = resolve_storage_dir(&app);
    let uc = FlushNoteUseCase::new(
        FsNoteRepository::new(storage_dir),
        SystemClock,
        NoOpBus,
        NoOpDebounceTimer,
    );

    let cmd = FlushNoteCommand {
        note_id: parse_note_id(&note_id),
        pending_body,
        trigger: trigger.into(),
    };

    match uc.execute(cmd) {
        Ok(Some(note)) => Ok(FlushOutcome::Flushed {
            id: note.id().as_str().to_string(),
            updated_at: note.updated_at().format_rfc3339(),
        }),
        Ok(None) => Ok(FlushOutcome::NoOp),
        Err(e) => Err(e.into()),
    }
}
