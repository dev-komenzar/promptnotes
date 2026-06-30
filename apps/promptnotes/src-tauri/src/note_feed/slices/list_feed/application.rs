//! Application layer of slice `list-feed`.
//!
//! Spec: `.ori/slices/list-feed/spec.md#impl-tauri`.
//!
//! Pipeline (workflows/list-feed.md#steps):
//!   1. NoteRepository::list_all → Vec<Note>
//!   2. NoteFeed::hydrate(notes)
//!   3. NoteFeed::visible_notes() → projection
//!
//! TP-SE1: `Repository` のみ inject、`EventBus` は inject しない (C-LF6 を type-level に固定)。

use std::io;

use crate::note_capture::shared::ports::NoteRepository;
use crate::note_feed::shared::types::NoteFeed;

use super::domain::ListFeedCommand;

pub struct ListFeedUseCase<R: NoteRepository> {
    repo: R,
}

impl<R: NoteRepository> ListFeedUseCase<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Hydrate `feed.source` from `NoteRepository::list_all` and return the
    /// resulting `NoteFeed` (sort / filter is left to the caller; `visible_notes`
    /// is read off the returned aggregate when needed).
    pub fn execute(&self, feed: NoteFeed, _cmd: ListFeedCommand) -> io::Result<NoteFeed> {
        let notes = self.repo.list_all()?;
        Ok(feed.hydrate(notes))
    }
}

/// Pure read projection extracted from a hydrated `NoteFeed`.
/// `visible_notes()` no longer takes `now` — uses `OffsetDateTime::now_utc()` internally
/// (aggregates.md 改訂により `now` パラメータ削除)。
pub fn visible_notes_snapshot(
    feed: &NoteFeed,
) -> Vec<crate::note_capture::shared::types::Note> {
    feed.visible_notes().into_iter().cloned().collect()
}
