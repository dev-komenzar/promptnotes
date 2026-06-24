use super::domain::{CreateNoteCommand, CreateNoteError};
use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, EventBus, NoteRepository};
use crate::note_capture::shared::types::{Note, NoteBody, Tag, TagSet};

/// Orchestrates the 7-step DMMF pipeline (spec.md#impl-pipeline):
///   0. empty-body guard  →  `Ok(None)` (C-CN3)
///   1. parseBody
///   2. parseTags (first-error short-circuit)
///   3. assignId via Clock
///   4. Note::create
///   5. NoteRepository::write
///   6. EventBus::publish(NoteCreated)
pub struct CreateNoteUseCase<R: NoteRepository, C: Clock, E: EventBus> {
    repo: R,
    clock: C,
    bus: E,
}

impl<R: NoteRepository, C: Clock, E: EventBus> CreateNoteUseCase<R, C, E> {
    pub fn new(repo: R, clock: C, bus: E) -> Self {
        Self { repo, clock, bus }
    }

    pub fn execute(&self, cmd: CreateNoteCommand) -> Result<Option<Note>, CreateNoteError> {
        // Step 0 — empty-body guard (C-CN3).
        if cmd.raw_body.trim().is_empty() {
            return Ok(None);
        }

        // Step 1 — parseBody. Validation failure surfaces as InvalidBody
        // (spec.md#io-errors, Pass 1 review hardening).
        let body = NoteBody::new(cmd.raw_body)
            .map_err(|source| CreateNoteError::InvalidBody { source })?;

        // Step 2 — parseTags (first-error short-circuit).
        let mut parsed: Vec<Tag> = Vec::with_capacity(cmd.raw_tags.len());
        for raw in &cmd.raw_tags {
            let tag = Tag::new(raw).map_err(|source| CreateNoteError::InvalidTag {
                raw: raw.clone(),
                source,
            })?;
            parsed.push(tag);
        }
        let tags = TagSet::from_tags(parsed);

        // Step 3 — assignId via clock.
        let now = self.clock.now();

        // Step 4 — Note::create.
        let note = Note::create(body, tags, now);

        // Step 5 — persist (C-CN4: failure blocks event emission).
        self.repo
            .write(&note)
            .map_err(|source| CreateNoteError::PersistError {
                path: self
                    .repo
                    .storage_dir()
                    .join(format!("{}.md", note.id().as_str())),
                source,
            })?;

        // Step 6 — emit NoteCreated.
        self.bus.publish(DomainEvent::NoteCreated {
            note_id: note.id().clone(),
            created_at: note.created_at(),
            initial_tags: note.tags().clone(),
        });

        Ok(Some(note))
    }
}
