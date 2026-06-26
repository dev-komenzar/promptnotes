use std::path::PathBuf;

use super::ports::TrashErrorKind;
use crate::note_capture::shared::types::NoteId;

#[derive(Debug, Clone)]
pub struct DeleteNoteCommand {
    pub note_id: NoteId,
}

#[derive(Debug, thiserror::Error)]
pub enum DeleteNoteError {
    #[error("note not found: {id:?}")]
    NoteNotFound { id: NoteId },
    /// `TrashService::move_to_trash` failed (spec.md#io-errors, I-DN4).
    #[error("trash move failed for {path:?}: {cause}")]
    TrashError {
        path: PathBuf,
        #[source]
        cause: TrashErrorKind,
    },
}
