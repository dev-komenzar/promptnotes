//! Tauri command surface for the `delete-note` slice.
//!
//! Wires the pure use case (`application.rs`) to runtime adapters:
//!   - `FsNoteRepository` for loading the source note
//!   - `FsTrashService` (in-app `<storage_dir>/trash/` adapter) for I-DN2
//!   - `InMemoryUndoStack` as a process-wide `tauri::State` so the
//!     follow-up restore wiring observes the same push (I-N7).
//!
//! Error mapping follows spec.md#io-errors: `InvalidNoteId` is surfaced at
//! the boundary (mirrors restore-deleted-note's MED-4 fix) so the frontend
//! distinguishes a malformed payload from a legitimate `NoteNotFound`.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime, State};
use time::OffsetDateTime;

use super::application::DeleteNoteUseCase;
use super::domain::{DeleteNoteCommand, DeleteNoteError};
use super::ports::{TrashErrorKind, UndoStack};
use crate::note_capture::shared::adapters::trash_service::FsTrashService;
use crate::note_capture::shared::adapters::undo_stack::InMemoryUndoStack;
use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, EventBus};
use crate::note_capture::shared::types::{DeletedNote, NoteId, Timestamp};
use crate::note_capture::slices::create_note::FsNoteRepository;

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
/// into the use case's `U: UndoStack` slot. Forwarding wrapper to side-step
/// the orphan rule and keep the use case generic by-value.
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
pub struct DeletedNoteDto {
    pub id: String,
    pub original_path: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeleteNoteErrorDto {
    /// Client-side parse failure of `note_id` (mirrors restore-deleted-note
    /// MED-4: silent UNIX_EPOCH fallback masked parse errors as NoteNotFound).
    InvalidNoteId {
        raw: String,
    },
    NoteNotFound {
        id: String,
    },
    TrashError {
        path: String,
        variant: String,
        reason: String,
    },
}

impl From<DeleteNoteError> for DeleteNoteErrorDto {
    fn from(e: DeleteNoteError) -> Self {
        match e {
            DeleteNoteError::NoteNotFound { id } => Self::NoteNotFound {
                id: id.as_str().to_string(),
            },
            DeleteNoteError::TrashError { path, cause } => {
                let (variant, reason) = match cause {
                    TrashErrorKind::PermissionDenied => {
                        ("permission_denied".into(), "permission denied".into())
                    }
                    TrashErrorKind::Io(msg) => ("io".into(), msg),
                    TrashErrorKind::Unsupported => (
                        "unsupported".into(),
                        "trash not supported on this platform".into(),
                    ),
                };
                Self::TrashError {
                    path: path.display().to_string(),
                    variant,
                    reason,
                }
            }
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
pub async fn delete_note<R: Runtime>(
    app: AppHandle<R>,
    undo: State<'_, InMemoryUndoStack>,
    note_id: String,
) -> Result<DeletedNoteDto, DeleteNoteErrorDto> {
    let parsed = match Timestamp::parse_yyyymmddhhmmss(&note_id) {
        Ok(ts) => NoteId::from_timestamp(ts),
        Err(_) => return Err(DeleteNoteErrorDto::InvalidNoteId { raw: note_id }),
    };

    let storage_dir = resolve_storage_dir(&app);
    let repo = FsNoteRepository::new(storage_dir.clone());
    let trash = FsTrashService::new(storage_dir);
    let uc = DeleteNoteUseCase::new(
        repo,
        trash,
        UndoStackRef(undo.inner()),
        SystemClock,
        NoOpBus,
    );

    uc.execute(DeleteNoteCommand { note_id: parsed })
        .map(|d| DeletedNoteDto {
            id: d.id().as_str().to_string(),
            original_path: d.original_path().display().to_string(),
        })
        .map_err(Into::into)
}
