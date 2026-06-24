use std::path::Path;

use super::events::DomainEvent;
use super::types::{Note, Timestamp};

/// Persistence port for Note Aggregates. Implementations write `.md` files
/// (frontmatter + body) to a filesystem location.
pub trait NoteRepository {
    fn write(&self, note: &Note) -> std::io::Result<()>;
    fn storage_dir(&self) -> &Path;
}

/// Injectable clock — production reads system time, tests use a fixed Timestamp.
pub trait Clock {
    fn now(&self) -> Timestamp;
}

/// In-process synchronous event bus.
pub trait EventBus {
    fn publish(&self, event: DomainEvent);
}
