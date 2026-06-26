use super::domain::{DeleteNoteCommand, DeleteNoteError};
use super::ports::{TrashService, UndoStack};
use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, EventBus, NoteRepository};
use crate::note_capture::shared::types::DeletedNote;

/// Orchestrates the delete-note pipeline (spec.md#impl-layout):
///   1. load_note via `NoteRepository::load_by_id`
///      - io::Err and Ok(None) both collapse to `NoteNotFound` (I-DN6,
///        mirrors copy-note-body I-CNB5)
///   2. resolve_path: `NoteRepository::storage_dir() / <note_id>.md` (I-DN1)
///      - reuses the existing `NoteRepository::storage_dir()` (no new
///        `SettingsReader` port — review Pass 1 M-2)
///   3. move_to_trash via `TrashService::move_to_trash` (I-DN2)
///      - failure short-circuits before push / event (I-DN4)
///   4. consume the loaded Note via `Note::delete_to_trash(original_path)`
///      to mint the `DeletedNote` Undo handle (review Pass 1 H-1: the
///      aggregate is the only construction site, so I-DN7 is enforced
///      at the type system level — `DeletedNote::new` is `pub(crate)`)
///   5. push to UndoStack (I-N7, I-DN8: accumulate, never destroy)
///   6. publish `NoteDeletedToTrash` event (I-DN5: trash + push 成功後)
///
/// **Double-delete in memory** is prevented by `Note::delete_to_trash(self)`
/// taking ownership. A second `execute()` call against the same `note_id`
/// after a successful first call observes `Ok(None)` from `load_by_id`
/// (the `.md` file is gone from `storage_dir`) and returns `NoteNotFound` —
/// behavior consistent with spec.md#tp-not-found (review Pass 1 M-3).
pub struct DeleteNoteUseCase<R, T, U, C, B>
where
    R: NoteRepository,
    T: TrashService,
    U: UndoStack,
    C: Clock,
    B: EventBus,
{
    repo: R,
    trash: T,
    undo: U,
    clock: C,
    bus: B,
}

impl<R, T, U, C, B> DeleteNoteUseCase<R, T, U, C, B>
where
    R: NoteRepository,
    T: TrashService,
    U: UndoStack,
    C: Clock,
    B: EventBus,
{
    pub fn new(repo: R, trash: T, undo: U, clock: C, bus: B) -> Self {
        Self {
            repo,
            trash,
            undo,
            clock,
            bus,
        }
    }

    pub fn execute(&self, cmd: DeleteNoteCommand) -> Result<DeletedNote, DeleteNoteError> {
        // Step 1 — load. io::Err and Ok(None) both collapse to NoteNotFound (I-DN6).
        // I-DN3 ordering: failure here short-circuits all subsequent side effects.
        let note = self
            .repo
            .load_by_id(&cmd.note_id)
            .ok()
            .flatten()
            .ok_or_else(|| DeleteNoteError::NoteNotFound {
                id: cmd.note_id.clone(),
            })?;

        // Step 2 — resolve_path (I-DN1: storage_dir / <id>.md, deterministic).
        let original_path = self
            .repo
            .storage_dir()
            .join(format!("{}.md", cmd.note_id.as_str()));

        // Step 3 — move_to_trash (I-DN2: trash 経由のみ).
        // I-DN4: failure here short-circuits push and event.
        self.trash
            .move_to_trash(&original_path)
            .map_err(|cause| DeleteNoteError::TrashError {
                path: original_path.clone(),
                cause,
            })?;

        // Step 4 — consume the aggregate to mint the DeletedNote Undo handle
        // (review Pass 1 H-1, I-DN7: aggregate-only construction path).
        let deleted = note.delete_to_trash(original_path.clone());

        // Step 5 — push to undo stack (I-N7, I-DN8: accumulate).
        self.undo.push(deleted.clone());

        // Step 6 — publish event (I-DN5: only after trash + push succeed).
        self.bus.publish(DomainEvent::NoteDeletedToTrash {
            note_id: cmd.note_id,
            original_path,
            deleted_at: self.clock.now(),
        });

        Ok(deleted)
    }
}
