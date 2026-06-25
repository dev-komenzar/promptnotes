use std::path::PathBuf;

use crate::note_capture::shared::types::{NoteBodyError, NoteId};

/// 3 種のトリガー (domain/workflows/flush-note.md#input より):
/// (1) ブロック focus 喪失, (2) ウィンドウ blur, (3) アプリ quit。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushTrigger {
    BlockBlur,
    WindowBlur,
    AppQuit,
}

#[derive(Debug, Clone)]
pub struct FlushNoteCommand {
    pub note_id: NoteId,
    pub pending_body: String,
    pub trigger: FlushTrigger,
}

/// spec.md#io-errors の 4 variant 形 (auto-save-note と同形)。
/// domain/workflows/flush-note.md は 2 variant のみだが、本 slice は
/// spec で先取り (phase 7 で upstream proposal 化予定、
/// spec.md#oq-error-variant-alignment)。
#[derive(Debug, thiserror::Error)]
pub enum FlushError {
    #[error("note not found: {id:?}")]
    NoteNotFound { id: NoteId },
    /// `NoteBody::new(pending_body)` failed (I-N8 violation).
    #[error("invalid note body: {source}")]
    InvalidBody {
        #[source]
        source: NoteBodyError,
    },
    /// Read side I/O failure (`load_by_id`). Distinct from `PersistError`.
    #[error("failed to load note at {path}: {source}")]
    LoadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// Write side I/O failure (`write`).
    #[error("failed to persist note at {path}: {source}")]
    PersistError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
