//! In-memory Undo stack adapter — `Mutex<Vec<DeletedNote>>`.
//!
//! Registered as a `tauri::State` so `delete_note` (push) and the
//! follow-up `restore_deleted_note` wiring share the same instance for
//! the lifetime of the Tauri app. State is intentionally process-local:
//! Undo handles do not survive a restart (matches Toast半永久 ではない の
//! UX 契約)。

use std::sync::Mutex;

use crate::note_capture::shared::types::{DeletedNote, NoteId};
use crate::note_capture::slices::delete_note::UndoStack;

#[derive(Default)]
pub struct InMemoryUndoStack {
    inner: Mutex<Vec<DeletedNote>>,
}

impl InMemoryUndoStack {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.inner.lock().expect("undo stack mutex poisoned").len()
    }
}

impl UndoStack for InMemoryUndoStack {
    fn push(&self, deleted: DeletedNote) {
        let mut guard = self.inner.lock().expect("undo stack mutex poisoned");
        guard.push(deleted);
    }

    fn find_by_id(&self, id: &NoteId) -> Option<DeletedNote> {
        let guard = self.inner.lock().expect("undo stack mutex poisoned");
        guard.iter().rev().find(|d| d.id() == id).cloned()
    }

    fn remove_by_id(&self, id: &NoteId) -> Option<DeletedNote> {
        let mut guard = self.inner.lock().expect("undo stack mutex poisoned");
        let pos = guard.iter().rposition(|d| d.id() == id)?;
        Some(guard.remove(pos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::note_capture::shared::types::{Note, NoteBody, TagSet, Timestamp};
    use std::path::PathBuf;
    use time::macros::datetime;

    fn deleted_note(id_str: &str) -> DeletedNote {
        let ts = Timestamp::from_offset_datetime(datetime!(2026-01-01 00:00:00 UTC));
        let note = Note::from_persisted(
            NoteBody::new("body".into()).unwrap(),
            TagSet::default(),
            ts,
            ts,
        );
        note.delete_to_trash(PathBuf::from(format!("/tmp/{id_str}.md")))
    }

    #[test]
    fn push_then_find_recovers_handle() {
        let stack = InMemoryUndoStack::new();
        let d = deleted_note("a");
        let id = d.id().clone();
        stack.push(d.clone());
        assert_eq!(stack.find_by_id(&id), Some(d));
    }

    #[test]
    fn remove_by_id_deletes_one_entry() {
        let stack = InMemoryUndoStack::new();
        let d = deleted_note("a");
        let id = d.id().clone();
        stack.push(d);
        assert!(stack.remove_by_id(&id).is_some());
        assert_eq!(stack.len(), 0);
        assert!(stack.find_by_id(&id).is_none());
    }
}
