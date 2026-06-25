use std::path::PathBuf;

use super::domain::{AutoSaveNoteCommand, AutoSaveError};
use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, EventBus, NoteRepository};
use crate::note_capture::shared::types::{Note, NoteBody, NoteId};

/// Orchestrates the auto-save pipeline (spec.md#impl-pipeline):
///   1. load_note
///   2. parse_body
///   3. compare_body  → BodyDiff
///   4. branch_on_diff: Unchanged → Ok(None) (S9 idempotency guard, C-AS3)
///   5. update_body via Note::edit_body (I-N4)
///   6. persist (C-AS5: failure blocks event)
///   7. emit NoteBodyEdited (C-AS6)
pub struct AutoSaveNoteUseCase<R: NoteRepository, C: Clock, E: EventBus> {
    repo: R,
    clock: C,
    bus: E,
}

enum BodyDiff {
    Unchanged,
    Changed(NoteBody),
}

fn compare_body(existing: &NoteBody, candidate: NoteBody) -> BodyDiff {
    if existing == &candidate {
        BodyDiff::Unchanged
    } else {
        BodyDiff::Changed(candidate)
    }
}

impl<R: NoteRepository, C: Clock, E: EventBus> AutoSaveNoteUseCase<R, C, E> {
    pub fn new(repo: R, clock: C, bus: E) -> Self {
        Self { repo, clock, bus }
    }

    pub fn execute(&self, cmd: AutoSaveNoteCommand) -> Result<Option<Note>, AutoSaveError> {
        // Step 1 — load_note.
        let existing = self
            .repo
            .load_by_id(&cmd.note_id)
            .map_err(|source| self.persist_error(&cmd.note_id, source))?
            .ok_or_else(|| AutoSaveError::NoteNotFound {
                id: cmd.note_id.clone(),
            })?;

        // Step 2 — parse_body.
        let parsed = NoteBody::new(cmd.new_body)
            .map_err(|source| AutoSaveError::InvalidBody { source })?;

        // Step 3 + 4 — compare and branch (S9, C-AS3).
        let new_body = match compare_body(existing.body(), parsed) {
            BodyDiff::Unchanged => return Ok(None),
            BodyDiff::Changed(b) => b,
        };

        // Step 5 — update_body (I-N4).
        let now = self.clock.now();
        let updated = existing.edit_body(new_body, now);

        // Step 6 — persist (C-AS5).
        self.repo
            .write(&updated)
            .map_err(|source| self.persist_error(updated.id(), source))?;

        // Step 7 — emit NoteBodyEdited (C-AS6).
        self.bus.publish(DomainEvent::NoteBodyEdited {
            note_id: updated.id().clone(),
            updated_at: updated.updated_at(),
        });

        Ok(Some(updated))
    }

    fn persist_error(&self, id: &NoteId, source: std::io::Error) -> AutoSaveError {
        AutoSaveError::PersistError {
            path: self.note_md_path(id),
            source,
        }
    }

    fn note_md_path(&self, id: &NoteId) -> PathBuf {
        self.repo.storage_dir().join(format!("{}.md", id.as_str()))
    }
}
