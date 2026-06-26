use std::path::PathBuf;

use super::types::{NoteId, TagSet, Timestamp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    NoteCreated {
        note_id: NoteId,
        created_at: Timestamp,
        initial_tags: TagSet,
    },
    NoteBodyEdited {
        note_id: NoteId,
        updated_at: Timestamp,
    },
    NoteTagsChanged {
        note_id: NoteId,
        tags: TagSet,
        updated_at: Timestamp,
    },
    /// Emitted by slice `delete-note` after `TrashService::move_to_trash`
    /// succeeds and the `DeletedNote` has been pushed onto the application
    /// service's Undo stack (spec: domain/domain-events.md#note-deleted-to-trash,
    /// slice spec.md#io-output, I-DN5 order contract).
    NoteDeletedToTrash {
        note_id: NoteId,
        original_path: PathBuf,
        deleted_at: Timestamp,
    },
    /// Emitted by slice `restore-deleted-note` after the 4 preceding side
    /// effects (find_by_id / restore_from_trash / load_by_id / remove_by_id)
    /// have all succeeded (spec: domain/domain-events.md#note-restored-from-trash,
    /// slice spec.md#io-output, I-RDN5 / I-RDN6 order contract).
    NoteRestoredFromTrash {
        note_id: NoteId,
        restored_at: Timestamp,
    },
}
