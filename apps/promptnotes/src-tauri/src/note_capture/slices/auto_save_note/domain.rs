use std::path::PathBuf;

use crate::note_capture::shared::types::{NoteBodyError, NoteId};

#[derive(Debug, Clone)]
pub struct AutoSaveNoteCommand {
    pub note_id: NoteId,
    pub new_body: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AutoSaveError {
    #[error("note not found: {id:?}")]
    NoteNotFound { id: NoteId },
    /// `NoteBody::new(new_body)` failed (aggregates.md#note-aggregate-invariants
    /// I-N8: body must not contain a frontmatter delimiter line).
    #[error("invalid note body: {source}")]
    InvalidBody {
        #[source]
        source: NoteBodyError,
    },
    /// I/O failure on the read side of the pipeline (load_by_id).
    /// Semantically distinct from `PersistError` (write side); the on-disk
    /// note exists but can't be parsed / read.
    #[error("failed to load note at {path}: {source}")]
    LoadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to persist note at {path}: {source}")]
    PersistError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
