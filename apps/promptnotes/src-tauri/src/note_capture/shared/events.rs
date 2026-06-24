use super::types::{NoteId, TagSet, Timestamp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    NoteCreated {
        note_id: NoteId,
        created_at: Timestamp,
        initial_tags: TagSet,
    },
}
