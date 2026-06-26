use std::path::PathBuf;

use super::domain::{AssignTagCommand, AssignTagError};
use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, EventBus, NoteRepository};
use crate::note_capture::shared::types::{Note, NoteId, Tag};

/// Orchestrates the assign-tag pipeline (spec.md#impl-pipeline):
///   1. parse_tag  → InvalidTag on Tag::new failure (runs before load_note)
///   2. load_note  → NoteNotFound / LoadError
///   3. compute_diff → TagDiff (Unchanged | Added)
///   4. branch_on_diff: Unchanged → Ok(None) (S4 dedupe, C-AT3)
///   5. apply_assign via Note::assign_tag (I-N5, I-N6 preserved)
///   6. persist (C-AT5: failure blocks event)
///   7. emit NoteTagsChanged (C-AT6)
pub struct AssignTagUseCase<R: NoteRepository, C: Clock, E: EventBus> {
    repo: R,
    clock: C,
    bus: E,
}

enum TagDiff {
    Unchanged,
    Added(Tag),
}

fn compute_diff(existing: &Note, candidate: Tag) -> TagDiff {
    if existing
        .tags()
        .as_slice()
        .iter()
        .any(|t| t.name() == candidate.name())
    {
        TagDiff::Unchanged
    } else {
        TagDiff::Added(candidate)
    }
}

impl<R: NoteRepository, C: Clock, E: EventBus> AssignTagUseCase<R, C, E> {
    pub fn new(repo: R, clock: C, bus: E) -> Self {
        Self { repo, clock, bus }
    }

    pub fn execute(&self, cmd: AssignTagCommand) -> Result<Option<Note>, AssignTagError> {
        let AssignTagCommand { note_id, raw_tag } = cmd;

        // Step 1 — parse_tag. Runs before load_note so invalid input fails
        // fast without touching the filesystem (C-AT1, workflow#notes).
        let tag = Tag::new(&raw_tag).map_err(|reason| AssignTagError::InvalidTag {
            name: raw_tag,
            reason,
        })?;

        // Step 2 — load_note. Read I/O failure is reported as LoadError
        // (semantically distinct from PersistError, spec.md#io-errors).
        let existing = self
            .repo
            .load_by_id(&note_id)
            .map_err(|source| self.load_error(&note_id, source))?
            .ok_or_else(|| AssignTagError::NoteNotFound {
                id: note_id.clone(),
            })?;

        // Step 3 + 4 — compute diff and branch (S4, C-AT3).
        let added = match compute_diff(&existing, tag) {
            TagDiff::Unchanged => return Ok(None),
            TagDiff::Added(t) => t,
        };

        // Step 5 — apply_assign.
        let now = self.clock.now();
        let updated = existing.assign_tag(added, now);

        // Step 6 — persist (C-AT5).
        self.repo
            .write(&updated)
            .map_err(|source| self.persist_error(updated.id(), source))?;

        // Step 7 — emit NoteTagsChanged (C-AT6).
        self.bus.publish(DomainEvent::NoteTagsChanged {
            note_id: updated.id().clone(),
            tags: updated.tags().clone(),
            updated_at: updated.updated_at(),
        });

        Ok(Some(updated))
    }

    fn persist_error(&self, id: &NoteId, source: std::io::Error) -> AssignTagError {
        AssignTagError::PersistError {
            path: self.note_md_path(id),
            source,
        }
    }

    fn load_error(&self, id: &NoteId, source: std::io::Error) -> AssignTagError {
        AssignTagError::LoadError {
            path: self.note_md_path(id),
            source,
        }
    }

    fn note_md_path(&self, id: &NoteId) -> PathBuf {
        self.repo.storage_dir().join(format!("{}.md", id.as_str()))
    }
}
