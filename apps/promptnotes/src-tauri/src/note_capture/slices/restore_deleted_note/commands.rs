//! Tauri command surface for the `restore-deleted-note` slice.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};
use time::OffsetDateTime;

use super::application::RestoreDeletedNoteUseCase;
use super::domain::{RestoreDeletedNoteCommand, RestoreDeletedNoteError};
use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, EventBus};
use crate::note_capture::shared::types::{DeletedNote, NoteId, Timestamp};
use crate::note_capture::slices::create_note::FsNoteRepository;
use crate::note_capture::slices::delete_note::{TrashErrorKind, TrashService, UndoStack};

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

/// Placeholder Tauri-side trash adapter. The actual OS bridge will land with
/// the production wiring (analogous to delete-note's follow-up).
struct UnimplementedTrash;
impl TrashService for UnimplementedTrash {
    fn move_to_trash(&self, _path: &std::path::Path) -> Result<(), TrashErrorKind> {
        Err(TrashErrorKind::Unsupported)
    }
    fn restore_from_trash(&self, _path: &std::path::Path) -> Result<(), TrashErrorKind> {
        Err(TrashErrorKind::Unsupported)
    }
}

/// Placeholder Undo stack. Will be replaced by a process-wide singleton wired
/// to the delete-note command surface in the production setup.
struct UnimplementedUndo;
impl UndoStack for UnimplementedUndo {
    fn push(&self, _deleted: DeletedNote) {}
    fn find_by_id(&self, _id: &NoteId) -> Option<DeletedNote> {
        None
    }
    fn remove_by_id(&self, _id: &NoteId) -> Option<DeletedNote> {
        None
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

fn resolve_storage_dir<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("Tauri must resolve app_data_dir on supported platforms")
        .join("notes")
}

#[tauri::command]
pub async fn restore_deleted_note<R: Runtime>(
    app: AppHandle<R>,
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
        FsNoteRepository::new(storage_dir),
        UnimplementedTrash,
        UnimplementedUndo,
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
            updated_at: note.updated_at().format_yyyymmddhhmmss(),
        }),
        Err(e) => Err(e.into()),
    }
}
