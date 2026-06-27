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

    /// Read every `.md` file under `storage_dir()` and return the parsed Notes
    /// (slice list-feed, workflow `list-feed#steps`). Individual parse / I/O
    /// failures must be skipped (C-LF1: "読めるものだけ読む"); only top-level
    /// `read_dir` failures bubble up.
    fn list_all(&self) -> std::io::Result<Vec<Note>> {
        unimplemented!("NoteRepository::list_all is required by slice list-feed")
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

/// Handle to the UI-side debounce timer. The `flush-note` slice cancels any
/// pending AutoSave for a Note before it persists synchronously, preventing
/// a duplicate write race (spec.md#invariants-slice-specific C-FL1).
///
/// The cancellation is idempotent: calling `cancel` when no timer is armed
/// must be a successful no-op.
pub trait DebounceTimer {
    fn cancel(&self, note_id: &NoteId);
}
