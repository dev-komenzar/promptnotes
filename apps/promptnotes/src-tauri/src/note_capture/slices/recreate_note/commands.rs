//! Tauri command surface for the `recreate-note` slice.
//!
//! Only this file depends on `tauri::*`. Scenario S20 calls `recreate_note`
//! when the user chooses "新規ファイルとして保存" after an external program
//! deleted the file while its Block was EDITING.

use serde::Serialize;
use tauri::{AppHandle, Runtime};
use time::OffsetDateTime;

use super::application::RecreateNoteUseCase;
use super::domain::{RecreateNoteCommand, RecreateNoteError};
use crate::note_capture::shared::ports::Clock;
use crate::note_capture::shared::storage::resolve_storage_dir;
use crate::note_capture::shared::types::Timestamp;
use crate::note_capture::slices::create_note::FsNoteRepository;

struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_offset_datetime(OffsetDateTime::now_utc())
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum RecreateNoteOutcome {
    Recreated {
        id: String,
        body: String,
        tags: Vec<String>,
        created_at: String,
        updated_at: String,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RecreateNoteErrorDto {
    InvalidNoteId {
        raw: String,
    },
    InvalidBody {
        reason: String,
    },
    InvalidTag {
        raw: String,
        reason: String,
    },
    PersistError {
        path: String,
        reason: String,
    },
}

impl From<RecreateNoteError> for RecreateNoteErrorDto {
    fn from(e: RecreateNoteError) -> Self {
        match e {
            RecreateNoteError::InvalidBody { source } => Self::InvalidBody {
                reason: source.to_string(),
            },
            RecreateNoteError::InvalidTag { raw, source } => Self::InvalidTag {
                raw,
                reason: source.to_string(),
            },
            RecreateNoteError::PersistError { path, source } => Self::PersistError {
                path: path.display().to_string(),
                reason: source.to_string(),
            },
        }
    }
}

/// Recreate `storage_dir/<note_id>.md` with the supplied body / tags.
///
/// `note_id` must be the canonical `YYYYMMDDhhmmss` form; a malformed id is
/// surfaced distinctly from body / persist failures so the frontend can react
/// (mirrors `restore_deleted_note`'s `InvalidNoteId` handling).
#[tauri::command]
pub async fn recreate_note<R: Runtime>(
    app: AppHandle<R>,
    note_id: String,
    raw_body: String,
    raw_tags: Vec<String>,
) -> Result<RecreateNoteOutcome, RecreateNoteErrorDto> {
    let created_at = match Timestamp::parse_yyyymmddhhmmss(&note_id) {
        Ok(ts) => ts,
        Err(_) => return Err(RecreateNoteErrorDto::InvalidNoteId { raw: note_id }),
    };

    let storage_dir = resolve_storage_dir(&app);
    let uc = RecreateNoteUseCase::new(FsNoteRepository::new(storage_dir), SystemClock);

    let cmd = RecreateNoteCommand {
        created_at,
        raw_body,
        raw_tags,
    };

    match uc.execute(cmd) {
        Ok(note) => Ok(RecreateNoteOutcome::Recreated {
            id: note.id().as_str().to_string(),
            body: note.body_for_clipboard(),
            tags: note
                .tags()
                .as_slice()
                .iter()
                .map(|t| t.name().to_string())
                .collect(),
            created_at: note.created_at().format_rfc3339(),
            updated_at: note.updated_at().format_rfc3339(),
        }),
        Err(e) => Err(e.into()),
    }
}
