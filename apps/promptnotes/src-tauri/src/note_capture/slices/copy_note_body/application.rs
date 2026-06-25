use super::domain::{CopyNoteBodyCommand, CopyNoteBodyError};
use super::ports::ClipboardService;
use crate::note_capture::shared::ports::NoteRepository;

/// Orchestrates the copy-note-body pipeline (spec.md#impl-layout):
///   1. load_note via `NoteRepository::load_by_id` (read only — aggregate
///      state and `updatedAt` remain unchanged; see TP-NM1)
///   2. extract_body via `Note::body_for_clipboard` (I-CNB1: body only,
///      no frontmatter/tags — guaranteed by I-N8 + aggregate field layout)
///   3. write_to_clipboard via `ClipboardService::write_text`
///
/// Deliberately does **not** take an `EventBus`: I-CNB4 (no domain event)
/// is enforced structurally by leaving the bus port out of the constructor
/// (TP-NE1 pins this at compile time).
///
/// **Note on load I/O failure**: `spec.md#io-errors` only enumerates
/// `NoteNotFound` and `ClipboardError`. There is no `LoadError` variant.
/// A read I/O failure on `load_by_id` is collapsed into `NoteNotFound`
/// — the user-observable effect is the same (no clipboard side effect,
/// the note isn't reachable) and adding a third variant would widen the
/// public surface beyond spec. Phase 6 review may revisit this.
pub struct CopyNoteBodyUseCase<R: NoteRepository, C: ClipboardService> {
    repo: R,
    clipboard: C,
}

impl<R: NoteRepository, C: ClipboardService> CopyNoteBodyUseCase<R, C> {
    pub fn new(repo: R, clipboard: C) -> Self {
        Self { repo, clipboard }
    }

    pub fn execute(&self, cmd: CopyNoteBodyCommand) -> Result<(), CopyNoteBodyError> {
        // Step 1 — load_note. Read I/O failure and `Ok(None)` both collapse
        // to `NoteNotFound` (see module doc). I-CNB3 ordering: if this step
        // fails the clipboard write below is unreachable.
        let note = self
            .repo
            .load_by_id(&cmd.note_id)
            .ok()
            .flatten()
            .ok_or(CopyNoteBodyError::NoteNotFound { id: cmd.note_id })?;

        // Step 2 — extract body (I-CNB1: body-only via aggregate query).
        let body = note.body_for_clipboard();

        // Step 3 — write to clipboard.
        self.clipboard
            .write_text(&body)
            .map_err(|cause| CopyNoteBodyError::ClipboardError { cause })?;

        Ok(())
    }
}
