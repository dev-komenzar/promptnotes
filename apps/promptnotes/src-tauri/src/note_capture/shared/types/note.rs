use std::path::PathBuf;

use super::deleted_note::DeletedNote;
use super::note_body::NoteBody;
use super::note_id::NoteId;
use super::tag::Tag;
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

    /// Add a normalized Tag to the TagSet (workflow: assign-tag). Idempotent
    /// against I-N5: when a tag with the same `name` already exists the
    /// aggregate is returned unchanged. Callers (the application service)
    /// pre-decide whether to invoke this by computing a TagDiff, so the
    /// no-op branch here only protects the invariant — event-emission control
    /// stays in the use case.
    ///
    /// `now` is passed in explicitly to keep `Note` pure; the open question
    /// about extending the domain doc to declare this signature is tracked
    /// in spec.md#oq-assign-tag-now-injection.
    pub fn assign_tag(self, tag: Tag, now: Timestamp) -> Self {
        if self.tags.as_slice().iter().any(|t| t.name() == tag.name()) {
            // I-N5: same name already present → no-op, including updated_at.
            return self;
        }
        let appended = self
            .tags
            .as_slice()
            .iter()
            .cloned()
            .chain(std::iter::once(tag))
            .collect::<Vec<_>>();
        Self {
            tags: TagSet::from_tags(appended),
            updated_at: now,
            ..self
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

    /// Returns the body string suited for OS clipboard output (workflow:
    /// copy-note-body, slice's I-CNB1 differentiator). `NoteBody` already
    /// excludes frontmatter delimiter lines by I-N8, and `tags` /
    /// timestamps are stored on separate fields — so "body only" is
    /// `self.body.as_str()` by construction.
    pub fn body_for_clipboard(&self) -> String {
        self.body.as_str().to_string()
    }

    /// Consume the Note and produce an Undo handle (workflow: delete-note,
    /// `aggregates.md#note-aggregate-commands` Note::delete_to_trash).
    ///
    /// `original_path` is supplied by the application service from the
    /// `NoteRepository::storage_dir()` value (`storage_dir / <id>.md`,
    /// slice spec.md#impl-notes I-DN1). Taking `self` provides compile-time
    /// guarantee that a Note instance cannot be deleted twice in memory.
    /// OS trash I/O remains an application-service responsibility
    /// (aggregate stays pure); this method's role is to mint the
    /// `DeletedNote` Undo handle from the privileged aggregate boundary,
    /// keeping `DeletedNote::new` `pub(crate)` per spec I-DN7.
    pub fn delete_to_trash(self, original_path: PathBuf) -> DeletedNote {
        DeletedNote::new(self.id, original_path)
    }
}
