use super::domain::{RecreateNoteCommand, RecreateNoteError};
use crate::note_capture::shared::ports::{Clock, NoteRepository};
use crate::note_capture::shared::types::{Note, NoteBody, Tag, TagSet};

/// Orchestrates the `recreate-note` pipeline (S20):
///   1. parseBody (NoteBody::new — I-N4, I-N8)
///   2. parseTags (first-error short-circuit)
///   3. Note::from_persisted(body, tags, created_at, now)
///      → preserves the original NoteId because id is derived from created_at
///   4. NoteRepository::write → recreates `<created_at>.md` on disk
///
/// No domain event is emitted: this is a persistence repair for a file that
/// the OS watcher observed as externally deleted. The NoteFeed / frontend are
/// updated by the caller (frontend patches the in-memory feed and the watcher
/// independently re-observes the recreated file as `NoteFileCreatedExternally`).
pub struct RecreateNoteUseCase<R: NoteRepository, C: Clock> {
    repo: R,
    clock: C,
}

impl<R: NoteRepository, C: Clock> RecreateNoteUseCase<R, C> {
    pub fn new(repo: R, clock: C) -> Self {
        Self { repo, clock }
    }

    pub fn execute(&self, cmd: RecreateNoteCommand) -> Result<Note, RecreateNoteError> {
        // Step 1 — parseBody.
        let body = NoteBody::new(cmd.raw_body)
            .map_err(|source| RecreateNoteError::InvalidBody { source })?;

        // Step 2 — parseTags (first-error short-circuit).
        let mut parsed: Vec<Tag> = Vec::with_capacity(cmd.raw_tags.len());
        for raw in &cmd.raw_tags {
            let tag = Tag::new(raw).map_err(|source| RecreateNoteError::InvalidTag {
                raw: raw.clone(),
                source,
            })?;
            parsed.push(tag);
        }
        let tags = TagSet::from_tags(parsed);

        // Step 3 — reconstruct with the original created_at (id preserved).
        let now = self.clock.now();
        let note = Note::from_persisted(body, tags, cmd.created_at, now);

        // Step 4 — persist. Failure blocks the Ok result (no partial success).
        self.repo
            .write(&note)
            .map_err(|source| RecreateNoteError::PersistError {
                path: self
                    .repo
                    .storage_dir()
                    .join(format!("{}.md", note.id().as_str())),
                source,
            })?;

        Ok(note)
    }
}
