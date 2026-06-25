use super::ports::ClipboardErrorKind;
use crate::note_capture::shared::types::NoteId;

#[derive(Debug, Clone)]
pub struct CopyNoteBodyCommand {
    pub note_id: NoteId,
}

#[derive(Debug, thiserror::Error)]
pub enum CopyNoteBodyError {
    #[error("note not found: {id:?}")]
    NoteNotFound { id: NoteId },
    /// `ClipboardService::write_text` failed (spec.md#io-errors, I-CNB3).
    /// `#[source]` so the root cause is reachable via `std::error::Error::source()`
    /// for structured logging adapters.
    #[error("clipboard write failed: {cause}")]
    ClipboardError {
        #[source]
        cause: ClipboardErrorKind,
    },
}
