use std::path::PathBuf;

use crate::note_capture::shared::types::NoteId;
use crate::note_capture::slices::delete_note::TrashErrorKind;

#[derive(Debug, Clone)]
pub struct RestoreDeletedNoteCommand {
    pub note_id: NoteId,
}

#[derive(Debug, thiserror::Error)]
pub enum RestoreDeletedNoteError {
    /// Undo スタックに対応する DeletedNote が存在しない (S7 二重防御)。
    #[error("no undo available for note: {id:?}")]
    NoUndoAvailable { id: NoteId },
    /// `TrashService::restore_from_trash` 失敗 (I-RDN3)。
    #[error("trash restore failed for {path:?}: {cause}")]
    TrashRestoreError {
        path: PathBuf,
        #[source]
        cause: TrashErrorKind,
    },
    /// `NoteRepository::load_by_id` の io::Err、または復帰直後の `Ok(None)`
    /// (I-RDN4 / oq-read-error-ok-none-policy)。
    #[error("failed to reload note at {path}: {source}")]
    ReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
