use std::path::Path;

use super::events::DomainEvent;
use super::types::{Note, NoteId, Timestamp};

/// Persistence port for Note Aggregates. Implementations write `.md` files
/// (frontmatter + body) to a filesystem location.
pub trait NoteRepository {
    fn write(&self, note: &Note) -> std::io::Result<()>;
    fn storage_dir(&self) -> &Path;

    /// Load a Note by its identifier. Returns `Ok(None)` when no `.md` file
    /// matches. Phase 4 of slice `auto-save-note` will provide the concrete
    /// `FsNoteRepository` implementation; intermediate stubs panic.
    fn load_by_id(&self, _id: &NoteId) -> std::io::Result<Option<Note>> {
        unimplemented!("NoteRepository::load_by_id is required by slice auto-save-note (phase 4)")
    }
}

/// Injectable clock — production reads system time, tests use a fixed Timestamp.
pub trait Clock {
    fn now(&self) -> Timestamp;
}

/// In-process synchronous event bus.
pub trait EventBus {
    fn publish(&self, event: DomainEvent);
}
