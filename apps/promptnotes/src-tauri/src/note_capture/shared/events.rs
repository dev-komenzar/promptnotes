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
}
