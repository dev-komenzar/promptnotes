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
    /// Defensive variant for the case where `new_body` contains a `---`
    /// delimiter line (NoteBody invariant from create-note BC). spec.md
    /// asserts the path is infallible because the aggregate definition reads
    /// "任意の UTF-8 文字列"; in practice the BC's NoteBody constructor still
    /// rejects frontmatter delimiters. See open question
    /// `oq-invalid-body-variant` in spec.md.
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
