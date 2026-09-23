//! In-memory Undo stack adapter — `Mutex<Vec<Entry>>` with per-element TTL.
//!
//! Registered as a `tauri::State` so `delete_note` (push) and
//! `restore_deleted_note` (find / remove) share the same instance for the
//! lifetime of the Tauri app. State is intentionally process-local: Undo
//! handles do not survive a restart (matches Toast半永久 ではない の UX 契約)。
//!
//! S7 (`.ori/domain/validation.md#s7-undo-after-toast`): each entry carries an
//! independent TTL matching the Toast lifetime — `delete-note.md#dependencies`
//! は `UndoStack` を「TTL 管理付き、各要素ごとに個別タイマー」と規定する。
//! 期限切れエントリはアクセス時に lazy prune され、Toast 消失後の
//! `restore_deleted_note` は `NoUndoAvailable`（restore-deleted-note.md#errors
//! の二重防御）を返す。他のエントリ（別 Toast）には影響しない（per-toast 独立性）。

use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::note_capture::shared::types::{DeletedNote, NoteId};
use crate::note_capture::slices::delete_note::UndoStack;

/// Toast 有効期間（仮 5 秒）。frontend の `toastStore` timeout と同期する
/// (`screen-1.md#cross-toast-display` / `validation.md#s7-undo-after-toast`)。
pub const UNDO_TTL: Duration = Duration::from_secs(5);

struct Entry {
    deleted: DeletedNote,
    expires_at: Instant,
}

pub struct InMemoryUndoStack {
    inner: Mutex<Vec<Entry>>,
    ttl: Duration,
}

impl Default for InMemoryUndoStack {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryUndoStack {
    pub fn new() -> Self {
        Self::with_ttl(UNDO_TTL)
    }

    /// テスト / 埋め込み用: 任意 TTL で構築する。production は `new()`（= 5s）。
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            inner: Mutex::new(Vec::new()),
            ttl,
        }
    }

    /// 期限切れ (`expires_at <= now`) のエントリを除去する（lazy expiry）。
    fn prune_expired(entries: &mut Vec<Entry>) {
        let now = Instant::now();
        entries.retain(|e| e.expires_at > now);
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        let mut guard = self.inner.lock().expect("undo stack mutex poisoned");
        Self::prune_expired(&mut guard);
        guard.len()
    }
}

impl UndoStack for InMemoryUndoStack {
    fn push(&self, deleted: DeletedNote) {
        let mut guard = self.inner.lock().expect("undo stack mutex poisoned");
        Self::prune_expired(&mut guard);
        guard.push(Entry {
            deleted,
            expires_at: Instant::now() + self.ttl,
        });
    }

    fn find_by_id(&self, id: &NoteId) -> Option<DeletedNote> {
        let mut guard = self.inner.lock().expect("undo stack mutex poisoned");
        Self::prune_expired(&mut guard);
        guard
            .iter()
            .rev()
            .find(|e| e.deleted.id() == id)
            .map(|e| e.deleted.clone())
    }

    fn remove_by_id(&self, id: &NoteId) -> Option<DeletedNote> {
        let mut guard = self.inner.lock().expect("undo stack mutex poisoned");
        Self::prune_expired(&mut guard);
        let pos = guard.iter().rposition(|e| e.deleted.id() == id)?;
        Some(guard.remove(pos).deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::note_capture::shared::types::{Note, NoteBody, TagSet, Timestamp};
    use std::path::PathBuf;
    use time::macros::datetime;

    fn deleted_note(id_str: &str) -> DeletedNote {
        deleted_note_at(id_str, datetime!(2026-01-01 00:00:00 UTC))
    }

    fn deleted_note_at(id_str: &str, ts: time::OffsetDateTime) -> DeletedNote {
        let timestamp = Timestamp::from_offset_datetime(ts);
        let note = Note::from_persisted(
            NoteBody::new("body".into()).unwrap(),
            TagSet::default(),
            timestamp,
            timestamp,
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

    #[test]
    fn expired_entry_is_pruned_and_not_found() {
        // S7: TTL 経過後は find_by_id が None を返す (= NoUndoAvailable)。
        let stack = InMemoryUndoStack::with_ttl(Duration::from_millis(10));
        let d = deleted_note("a");
        let id = d.id().clone();
        stack.push(d);

        std::thread::sleep(Duration::from_millis(30));

        assert!(stack.find_by_id(&id).is_none());
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn expiry_is_per_element() {
        // per-toast 独立性: 一方の期限切れは他方のエントリに影響しない。
        let stack = InMemoryUndoStack::with_ttl(Duration::from_millis(200));
        let a = deleted_note_at("a", datetime!(2026-01-01 00:00:00 UTC));
        let id_a = a.id().clone();
        stack.push(a);
        std::thread::sleep(Duration::from_millis(250)); // A は期限切れ

        // B は直後に push するので、A の prune 後も生存している。
        let b = deleted_note_at("b", datetime!(2026-01-01 00:00:01 UTC));
        let id_b = b.id().clone();
        stack.push(b);

        assert!(stack.find_by_id(&id_a).is_none());
        assert!(stack.find_by_id(&id_b).is_some());
    }
}
