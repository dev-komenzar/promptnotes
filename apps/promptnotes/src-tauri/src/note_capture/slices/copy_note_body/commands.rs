//! Tauri command surface for the `copy-note-body` slice.
//!
//! Only this file depends on `tauri::*`. The frontend invokes
//! `copy_note_body` from the hover button. The use case (`application.rs`)
//! stays pure — adapters are wired here.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};
use time::OffsetDateTime;

use super::application::CopyNoteBodyUseCase;
use super::domain::{CopyNoteBodyCommand, CopyNoteBodyError};
use super::ports::ClipboardErrorKind;
use crate::note_capture::shared::adapters::clipboard::TauriClipboardService;
use crate::note_capture::shared::types::{NoteId, Timestamp};
use crate::note_capture::slices::create_note::FsNoteRepository;

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CopyNoteBodyErrorDto {
    NoteNotFound { id: String },
    ClipboardError { variant: String, reason: String },
}

impl From<CopyNoteBodyError> for CopyNoteBodyErrorDto {
    fn from(e: CopyNoteBodyError) -> Self {
        match e {
            CopyNoteBodyError::NoteNotFound { id } => Self::NoteNotFound {
                id: id.as_str().to_string(),
            },
            CopyNoteBodyError::ClipboardError { cause } => match cause {
                ClipboardErrorKind::Unavailable => Self::ClipboardError {
                    variant: "unavailable".into(),
                    reason: "OS clipboard unavailable".into(),
                },
                ClipboardErrorKind::Io(msg) => Self::ClipboardError {
                    variant: "io".into(),
                    reason: msg,
                },
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
pub async fn copy_note_body<R: Runtime>(
    app: AppHandle<R>,
    note_id: String,
) -> Result<(), CopyNoteBodyErrorDto> {
    let storage_dir = resolve_storage_dir(&app);
    let repo = FsNoteRepository::new(storage_dir);
    let clipboard = TauriClipboardService::new(&app);
    let uc = CopyNoteBodyUseCase::new(repo, clipboard);

    uc.execute(CopyNoteBodyCommand {
        note_id: parse_note_id(&note_id),
    })
    .map_err(Into::into)
}

/// Mirror of `auto_save_note::commands::parse_note_id`: there is no public
/// validating constructor on `NoteId`, so an unparseable id is collapsed into
/// a sentinel that the repository will miss — surfacing as `NoteNotFound`,
/// which matches spec.md#oq-invalid-note-id.
fn parse_note_id(raw: &str) -> NoteId {
    match Timestamp::parse_yyyymmddhhmmss(raw) {
        Ok(ts) => NoteId::from_timestamp(ts),
        Err(_) => {
            NoteId::from_timestamp(Timestamp::from_offset_datetime(OffsetDateTime::UNIX_EPOCH))
        }
    }
}
