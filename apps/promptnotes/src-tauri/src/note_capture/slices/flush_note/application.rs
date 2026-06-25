use std::path::PathBuf;

use super::domain::{FlushError, FlushNoteCommand};
use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, DebounceTimer, EventBus, NoteRepository};
use crate::note_capture::shared::types::{Note, NoteBody, NoteId};

/// Orchestrates the flush pipeline (spec.md#impl-pipeline):
///   1. cancel_debounce  (C-FL1: cancel must precede load)
///   2. load_note
///   3. parse_body
///   4. compare_body  → BodyDiff
///   5. branch_on_diff: Unchanged → Ok(None) (C-FL4)
///   6. update_body via Note::edit_body (I-N4)
///   7. persist (C-FL6: failure blocks event)
///   8. emit NoteBodyEdited (C-FL7)
pub struct FlushNoteUseCase<R: NoteRepository, C: Clock, E: EventBus, D: DebounceTimer> {
    repo: R,
    clock: C,
    bus: E,
    timer: D,
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

impl<R: NoteRepository, C: Clock, E: EventBus, D: DebounceTimer> FlushNoteUseCase<R, C, E, D> {
    pub fn new(repo: R, clock: C, bus: E, timer: D) -> Self {
        Self {
            repo,
            clock,
            bus,
            timer,
        }
    }

    pub fn execute(&self, cmd: FlushNoteCommand) -> Result<Option<Note>, FlushError> {
        // Step 1 — cancel_debounce (C-FL1). Idempotent. Must precede every
        // other side effect so an AutoSave racing in parallel cannot land a
        // second write after we persist.
        self.timer.cancel(&cmd.note_id);

        // Step 2 — load_note. Read I/O failure is reported as LoadError,
        // semantically distinct from PersistError (C-FL2).
        let existing = self
            .repo
            .load_by_id(&cmd.note_id)
            .map_err(|source| self.load_error(&cmd.note_id, source))?
            .ok_or_else(|| FlushError::NoteNotFound {
                id: cmd.note_id.clone(),
            })?;

        // Step 3 — parse_body (C-FL3, I-N8).
        let parsed =
            NoteBody::new(cmd.pending_body).map_err(|source| FlushError::InvalidBody { source })?;

        // Step 4 + 5 — compare and branch (C-FL4).
        let new_body = match compare_body(existing.body(), parsed) {
            BodyDiff::Unchanged => return Ok(None),
            BodyDiff::Changed(b) => b,
        };

        // Step 6 — update_body (I-N4).
        let now = self.clock.now();
        let updated = existing.edit_body(new_body, now);

        // Step 7 — persist (C-FL6).
        self.repo
            .write(&updated)
            .map_err(|source| self.persist_error(updated.id(), source))?;

        // Step 8 — emit NoteBodyEdited (C-FL7).
        self.bus.publish(DomainEvent::NoteBodyEdited {
            note_id: updated.id().clone(),
            updated_at: updated.updated_at(),
        });

        Ok(Some(updated))
    }

    fn persist_error(&self, id: &NoteId, source: std::io::Error) -> FlushError {
        FlushError::PersistError {
            path: self.note_md_path(id),
            source,
        }
    }

    fn load_error(&self, id: &NoteId, source: std::io::Error) -> FlushError {
        FlushError::LoadError {
            path: self.note_md_path(id),
            source,
        }
    }

    fn note_md_path(&self, id: &NoteId) -> PathBuf {
        self.repo.storage_dir().join(format!("{}.md", id.as_str()))
    }
}
