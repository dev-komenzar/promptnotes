// I/O adapters for the `tasks` feature.
//
// The in-memory `MemoryRepo` is intentionally trivial — swap it for a
// persistent adapter (SQLite via sqlx, etc.) when your project needs one.
// Only this layer is allowed to depend on side-effectful crates.

use super::domain::{Task, TaskId};
use std::sync::Mutex;

pub trait TaskRepository: Send + Sync {
    fn find(&self, id: &TaskId) -> Option<Task>;
    fn save(&self, task: Task);
}

pub struct MemoryRepo {
    inner: Mutex<Vec<Task>>,
}

impl MemoryRepo {
    pub fn new() -> Self {
        Self { inner: Mutex::new(Vec::new()) }
    }
}

impl Default for MemoryRepo {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskRepository for MemoryRepo {
    fn find(&self, id: &TaskId) -> Option<Task> {
        let guard = self.inner.lock().ok()?;
        guard.iter().find(|t| &t.id == id).cloned()
    }

    fn save(&self, task: Task) {
        let Ok(mut guard) = self.inner.lock() else { return };
        if let Some(slot) = guard.iter_mut().find(|t| t.id == task.id) {
            *slot = task;
        } else {
            guard.push(task);
        }
    }
}
