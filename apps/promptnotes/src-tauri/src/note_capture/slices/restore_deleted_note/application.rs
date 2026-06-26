use std::io;

use super::domain::{RestoreDeletedNoteCommand, RestoreDeletedNoteError};
use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, EventBus, NoteRepository};
use crate::note_capture::shared::types::{DeletedNote, Note};
use crate::note_capture::slices::delete_note::{TrashService, UndoStack};

/// Orchestrates the restore-deleted-note pipeline (spec.md#impl-layout):
///   1. find_by_id → NoUndoAvailable on miss (I-RDN1, S7 二重防御)
///   2. restore_from_trash → TrashRestoreError (I-RDN3)
///   3. load_by_id → ReadError (io::Err or Ok(None) collapse, I-RDN4 +
///      spec.md#oq-read-error-ok-none-policy)
///   4. remove_by_id (per-toast 独立性、I-RDN7)
///   5. publish NoteRestoredFromTrash (I-RDN6: 4 副作用全成功後)
pub struct RestoreDeletedNoteUseCase<R, T, U, C, B>
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

impl<R, T, U, C, B> RestoreDeletedNoteUseCase<R, T, U, C, B>
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

    pub fn execute(
        &self,
        cmd: RestoreDeletedNoteCommand,
    ) -> Result<Note, RestoreDeletedNoteError> {
        let RestoreDeletedNoteCommand { note_id } = cmd;

        // Step 1 — find (I-RDN1: NoUndoAvailable は 4 副作用未呼出)
        let deleted: DeletedNote = self
            .undo
            .find_by_id(&note_id)
            .ok_or_else(|| RestoreDeletedNoteError::NoUndoAvailable {
                id: note_id.clone(),
            })?;
        let original_path = deleted.original_path().to_path_buf();

        // Step 2 — restore_from_trash (I-RDN3: 失敗時 load/remove/event 未呼出)
        self.trash.restore_from_trash(&original_path).map_err(|cause| {
            RestoreDeletedNoteError::TrashRestoreError {
                path: original_path.clone(),
                cause,
            }
        })?;

        // Step 3 — reload (I-RDN4 + oq-read-error-ok-none-policy:
        // io::Err と Ok(None) の両方を ReadError へ collapse)
        let restored = self
            .repo
            .load_by_id(&note_id)
            .map_err(|source| RestoreDeletedNoteError::ReadError {
                path: original_path.clone(),
                source,
            })?
            .ok_or_else(|| RestoreDeletedNoteError::ReadError {
                path: original_path.clone(),
                source: io::Error::new(
                    io::ErrorKind::NotFound,
                    "restored .md not found by load_by_id (post-restore inconsistency)",
                ),
            })?;

        // Step 4 — remove (I-RDN7: per-toast 独立性、当該 NoteId 1 件のみ)
        let _popped: Option<DeletedNote> = self.undo.remove_by_id(&note_id);

        // Step 5 — emit (I-RDN6: 副作用全成功後).
        // event の note_id は input command の `note_id` を採用する
        // (review Pass 1 MED-1: reloaded Note の id 経由だと将来の repo refactor で
        // event payload と input id が乖離するリスクがあるため、入力 id を信頼源とする)。
        self.bus.publish(DomainEvent::NoteRestoredFromTrash {
            note_id: note_id.clone(),
            restored_at: self.clock.now(),
        });

        Ok(restored)
    }
}
