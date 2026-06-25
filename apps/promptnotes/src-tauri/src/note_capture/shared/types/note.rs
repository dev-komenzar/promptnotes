use super::note_body::NoteBody;
use super::note_id::NoteId;
use super::tag_set::TagSet;
use super::timestamp::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    id: NoteId,
    body: NoteBody,
    tags: TagSet,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl Note {
    /// Sole constructor for a fresh Note (workflow: create-note).
    /// `id` is derived from `now`; `created_at == updated_at == now` (C-CN1).
    pub fn create(body: NoteBody, tags: TagSet, now: Timestamp) -> Self {
        Self {
            id: NoteId::from_timestamp(now),
            body,
            tags,
            created_at: now,
            updated_at: now,
        }
    }

    /// Reconstruct a Note from already-persisted state (used by repository
    /// implementations when reading `.md` files). `id` is computed from
    /// `created_at` to keep I-N2 enforced by construction.
    pub fn from_persisted(
        body: NoteBody,
        tags: TagSet,
        created_at: Timestamp,
        updated_at: Timestamp,
    ) -> Self {
        Self {
            id: NoteId::from_timestamp(created_at),
            body,
            tags,
            created_at,
            updated_at,
        }
    }

    /// Replace the body and stamp `updated_at = now` (workflow: auto-save-note,
    /// flush-note). The aggregate is consumed and returned to make in-place
    /// mutation aliasing-free; callers persist the returned value.
    pub fn edit_body(self, new_body: NoteBody, now: Timestamp) -> Self {
        Self {
            body: new_body,
            updated_at: now,
            ..self
        }
    }

    pub fn id(&self) -> &NoteId {
        &self.id
    }

    pub fn body(&self) -> &NoteBody {
        &self.body
    }

    pub fn tags(&self) -> &TagSet {
        &self.tags
    }

    pub fn created_at(&self) -> Timestamp {
        self.created_at
    }

    pub fn updated_at(&self) -> Timestamp {
        self.updated_at
    }
}
