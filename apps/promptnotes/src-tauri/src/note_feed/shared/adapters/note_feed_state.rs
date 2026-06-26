//! In-memory [`NoteFeed`] state adapter — `Mutex<NoteFeed>`.
//!
//! Registered as a `tauri::State` so `change-sort-order` and `update-feed-filter`
//! share the same process-local `NoteFeed`. NoteFeed は揮発 read model なので
//! restart 越しの永続化はしない (`aggregates.md#note-feed-aggregate`)。

use std::sync::Mutex;

use crate::note_feed::shared::types::NoteFeed;

#[derive(Default)]
pub struct InMemoryNoteFeedState {
    inner: Mutex<NoteFeed>,
}

impl InMemoryNoteFeedState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> NoteFeed {
        self.inner.lock().expect("note feed mutex poisoned").clone()
    }

    pub fn replace(&self, feed: NoteFeed) {
        let mut guard = self.inner.lock().expect("note feed mutex poisoned");
        *guard = feed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user_preferences::shared::types::{SortDirection, SortField, SortOrder};

    #[test]
    fn snapshot_returns_default_initially() {
        let state = InMemoryNoteFeedState::new();
        assert_eq!(state.snapshot(), NoteFeed::empty());
    }

    #[test]
    fn replace_updates_subsequent_snapshots() {
        let state = InMemoryNoteFeedState::new();
        let updated = NoteFeed::empty()
            .change_sort(SortOrder::new(SortField::UpdatedAt, SortDirection::Asc));
        state.replace(updated.clone());
        assert_eq!(state.snapshot(), updated);
    }
}
