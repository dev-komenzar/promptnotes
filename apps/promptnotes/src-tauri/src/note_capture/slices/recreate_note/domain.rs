use std::path::PathBuf;

use crate::note_capture::shared::types::{NoteBodyError, TagError, Timestamp};

/// Command for the `recreate-note` slice.
///
/// Scenario S20 (`.ori/domain/validation.md#s20-external-delete-while-editing`):
/// the user is EDITING Note A when an external program deletes
/// `storage_dir/<A.id>.md`. Choosing "新規ファイルとして保存" (save as new file)
/// recreates the `.md` at the **same NoteId** with the in-flight editing body.
///
/// `created_at` is parsed at the Tauri boundary from the `note_id` string
/// (`YYYYMMDDhhmmss`); `Note::from_persisted` recomputes the id from
/// `created_at`, so the recreated file keeps the original identity (I-N2).
#[derive(Debug, Clone)]
pub struct RecreateNoteCommand {
    pub created_at: Timestamp,
    pub raw_body: String,
    pub raw_tags: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RecreateNoteError {
    #[error("invalid note body: {source}")]
    InvalidBody {
        #[source]
        source: NoteBodyError,
    },
    #[error("invalid tag '{raw}': {source}")]
    InvalidTag {
        raw: String,
        #[source]
        source: TagError,
    },
    #[error("failed to persist note at {path}: {source}")]
    PersistError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
