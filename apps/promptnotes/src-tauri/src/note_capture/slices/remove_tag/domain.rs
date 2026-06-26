use std::path::PathBuf;

use crate::note_capture::shared::types::NoteId;

#[derive(Debug, Clone)]
pub struct RemoveTagCommand {
    pub note_id: NoteId,
    /// Tag name to remove. Treated as already-normalized per I-RT1
    /// (UI passes the `Tag::name` string verbatim from a tag chip).
    pub tag_name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum RemoveTagError {
    #[error("note not found: {id:?}")]
    NoteNotFound { id: NoteId },
    /// I/O failure on the read side of the pipeline (load_by_id).
    /// Semantically distinct from `PersistError` (write side, I-RT8,
    /// matches assign-tag / auto-save-note variant decomposition).
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
