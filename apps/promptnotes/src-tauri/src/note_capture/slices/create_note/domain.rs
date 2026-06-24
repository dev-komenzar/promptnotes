use std::path::PathBuf;

use crate::note_capture::shared::types::{NoteBodyError, TagError};

#[derive(Debug, Clone)]
pub struct CreateNoteCommand {
    pub raw_body: String,
    pub raw_tags: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum CreateNoteError {
    #[error("invalid tag '{raw}': {source}")]
    InvalidTag {
        raw: String,
        #[source]
        source: TagError,
    },
    #[error("invalid note body: {source}")]
    InvalidBody {
        #[source]
        source: NoteBodyError,
    },
    #[error("failed to persist note at {path}: {source}")]
    PersistError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
