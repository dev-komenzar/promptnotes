use std::path::PathBuf;

use super::domain::{RemoveTagCommand, RemoveTagError};
use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, EventBus, NoteRepository};
use crate::note_capture::shared::types::{Note, NoteId, Tag};

/// Orchestrates the remove-tag pipeline (spec.md#impl-layout):
///   1. load_note  → NoteNotFound / LoadError (I-RT5, I-RT8)
///   2. compute_diff → RemoveTagDiff (Unchanged | Removed)
///   3. branch_on_diff: Unchanged → Ok(None) (I-RT2, workflow#steps step 3)
///   4. apply_remove via Note::remove_tag (I-N5/I-N6 preserved by aggregate)
///   5. persist (I-RT4: failure blocks event)
///   6. emit NoteTagsChanged with the post-removal TagSet (I-RT7)
pub struct RemoveTagUseCase<R, C, E>
where
    R: NoteRepository,
    C: Clock,
    E: EventBus,
{
    repo: R,
    clock: C,
    bus: E,
}

/// Result of comparing the current TagSet against the requested `tag_name`.
/// Mirrors `TagDiff = Unchanged | Removed(Tag)` declared in
/// `domain/workflows/remove-tag.md#steps`: the `Removed` variant carries
/// the actual `Tag` that will be dropped, so downstream layers do not have
/// to re-search the TagSet (review Pass 1 HIGH-A).
enum RemoveTagDiff {
    Unchanged,
    Removed(Tag),
}

fn compute_diff(existing: &Note, tag_name: &str) -> RemoveTagDiff {
    match existing
        .tags()
        .as_slice()
        .iter()
        .find(|t| t.name() == tag_name)
    {
        Some(t) => RemoveTagDiff::Removed(t.clone()),
        None => RemoveTagDiff::Unchanged,
    }
}

impl<R, C, E> RemoveTagUseCase<R, C, E>
where
    R: NoteRepository,
    C: Clock,
    E: EventBus,
{
    pub fn new(repo: R, clock: C, bus: E) -> Self {
        Self { repo, clock, bus }
    }

    pub fn execute(
        &self,
        cmd: RemoveTagCommand,
    ) -> Result<Option<Note>, RemoveTagError> {
        let RemoveTagCommand { note_id, tag_name } = cmd;

        // Step 1 — load_note. Read I/O failure is reported as LoadError
        // (semantically distinct from PersistError, I-RT8).
        let existing = self
            .repo
            .load_by_id(&note_id)
            .map_err(|source| self.load_error(&note_id, source))?
            .ok_or_else(|| RemoveTagError::NoteNotFound {
                id: note_id.clone(),
            })?;

        // Step 2 + 3 — compute diff and branch on Unchanged (I-RT2).
        // `Removed(Tag)` carries the matched aggregate Tag for symmetry with
        // the domain workflow contract (review Pass 1 HIGH-A).
        let _matched = match compute_diff(&existing, &tag_name) {
            RemoveTagDiff::Unchanged => return Ok(None),
            RemoveTagDiff::Removed(t) => t,
        };

        // Step 4 — apply_remove via aggregate.
        let now = self.clock.now();
        let updated = existing.remove_tag(&tag_name, now);

        // Step 5 — persist (I-RT4: failure blocks event).
        self.repo
            .write(&updated)
            .map_err(|source| self.persist_error(updated.id(), source))?;

        // Step 6 — emit NoteTagsChanged with post-removal TagSet (I-RT7).
        self.bus.publish(DomainEvent::NoteTagsChanged {
            note_id: updated.id().clone(),
            tags: updated.tags().clone(),
            updated_at: updated.updated_at(),
        });

        Ok(Some(updated))
    }

    fn load_error(&self, id: &NoteId, source: std::io::Error) -> RemoveTagError {
        RemoveTagError::LoadError {
            path: self.note_md_path(id),
            source,
        }
    }

    fn persist_error(&self, id: &NoteId, source: std::io::Error) -> RemoveTagError {
        RemoveTagError::PersistError {
            path: self.note_md_path(id),
            source,
        }
    }

    fn note_md_path(&self, id: &NoteId) -> PathBuf {
        self.repo.storage_dir().join(format!("{}.md", id.as_str()))
    }
}
