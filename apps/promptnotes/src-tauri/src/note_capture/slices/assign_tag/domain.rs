use std::path::PathBuf;

use crate::note_capture::shared::types::{NoteId, TagError};

#[derive(Debug, Clone)]
pub struct AssignTagCommand {
    pub note_id: NoteId,
    pub raw_tag: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AssignTagError {
    #[error("note not found: {id:?}")]
    NoteNotFound { id: NoteId },
    /// `Tag::new(raw_tag)` failed (aggregates.md#note-aggregate-invariants
    /// I-N6: tag must be non-empty after trim and free of forbidden chars).
    #[error("invalid tag '{name}': {reason}")]
    InvalidTag {
        name: String,
        #[source]
        reason: TagError,
    },
    /// I/O failure on the read side of the pipeline (load_by_id).
    /// Semantically distinct from `PersistError` (write side).
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
