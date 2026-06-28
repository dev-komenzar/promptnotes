//! Tauri command surface for the `restore-deleted-note` slice.
//!
//! Wires the pure use case (`application.rs`) to the same runtime adapters
//! that `delete-note` uses so the Undo round-trip observes the same trash
//! files and undo stack (I-N7):
//!   - `FsNoteRepository` for writing the restored note back
//!   - `FsTrashService` (in-app `<storage_dir>/trash/` adapter)
//!   - `InMemoryUndoStack` shared via `tauri::State`

use serde::Serialize;
use tauri::{AppHandle, Runtime, State};
use time::OffsetDateTime;

use super::application::RestoreDeletedNoteUseCase;
use super::domain::{RestoreDeletedNoteCommand, RestoreDeletedNoteError};
use crate::note_capture::shared::adapters::trash_service::FsTrashService;
use crate::note_capture::shared::adapters::undo_stack::InMemoryUndoStack;
use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, EventBus};
use crate::note_capture::shared::storage::resolve_storage_dir;
use crate::note_capture::shared::types::{DeletedNote, NoteId, Timestamp};
use crate::note_capture::slices::create_note::FsNoteRepository;
use crate::note_capture::slices::delete_note::UndoStack;

struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_offset_datetime(OffsetDateTime::now_utc())
    }
}

struct NoOpBus;
impl EventBus for NoOpBus {
    fn publish(&self, _event: DomainEvent) {}
}

/// Bridges the `tauri::State<InMemoryUndoStack>` (which only yields `&T`)
/// into the use case's `U: UndoStack` slot. Mirrors `delete_note::UndoStackRef`.
struct UndoStackRef<'a>(&'a InMemoryUndoStack);
impl<'a> UndoStack for UndoStackRef<'a> {
    fn push(&self, deleted: DeletedNote) {
        self.0.push(deleted)
    }
    fn find_by_id(&self, id: &NoteId) -> Option<DeletedNote> {
        self.0.find_by_id(id)
    }
    fn remove_by_id(&self, id: &NoteId) -> Option<DeletedNote> {
        self.0.remove_by_id(id)
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum RestoreDeletedNoteOutcome {
    Restored {
        id: String,
        body: String,
        tags: Vec<String>,
        updated_at: String,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RestoreDeletedNoteErrorDto {
    /// Client-side parse failure of the supplied `note_id` string. Surfaced
    /// at the Tauri boundary so the frontend can distinguish a malformed
    /// payload from a legitimate `NoUndoAvailable` domain outcome (review
    /// Pass 1 MED-4: silent UNIX_EPOCH fallback was masking parse errors as
    /// no-undo).
    InvalidNoteId {
        raw: String,
    },
    NoUndoAvailable {
        id: String,
    },
    TrashRestoreError {
        path: String,
        reason: String,
    },
    ReadError {
        path: String,
        reason: String,
    },
}

impl From<RestoreDeletedNoteError> for RestoreDeletedNoteErrorDto {
    fn from(e: RestoreDeletedNoteError) -> Self {
        match e {
            RestoreDeletedNoteError::NoUndoAvailable { id } => Self::NoUndoAvailable {
                id: id.as_str().to_string(),
            },
            RestoreDeletedNoteError::TrashRestoreError { path, cause } => Self::TrashRestoreError {
                path: path.display().to_string(),
                reason: cause.to_string(),
            },
            RestoreDeletedNoteError::ReadError { path, source } => Self::ReadError {
                path: path.display().to_string(),
                reason: source.to_string(),
            },
        }
    }
}

#[tauri::command]
pub async fn restore_deleted_note<R: Runtime>(
    app: AppHandle<R>,
    undo: State<'_, InMemoryUndoStack>,
    note_id: String,
) -> Result<RestoreDeletedNoteOutcome, RestoreDeletedNoteErrorDto> {
    // Parse first so a malformed payload surfaces distinctly from
    // NoUndoAvailable (review Pass 1 MED-4).
    let parsed = match Timestamp::parse_yyyymmddhhmmss(&note_id) {
        Ok(ts) => NoteId::from_timestamp(ts),
        Err(_) => {
            return Err(RestoreDeletedNoteErrorDto::InvalidNoteId { raw: note_id });
        }
    };

    let storage_dir = resolve_storage_dir(&app);
    let uc = RestoreDeletedNoteUseCase::new(
        FsNoteRepository::new(storage_dir.clone()),
        FsTrashService::new(storage_dir),
        UndoStackRef(undo.inner()),
        SystemClock,
        NoOpBus,
    );

    let cmd = RestoreDeletedNoteCommand { note_id: parsed };

    match uc.execute(cmd) {
        Ok(note) => Ok(RestoreDeletedNoteOutcome::Restored {
            id: note.id().as_str().to_string(),
            body: note.body_for_clipboard(),
            tags: note
                .tags()
                .as_slice()
                .iter()
                .map(|t| t.name().to_string())
                .collect(),
            updated_at: note.updated_at().format_rfc3339(),
        }),
        Err(e) => Err(e.into()),
    }
}
