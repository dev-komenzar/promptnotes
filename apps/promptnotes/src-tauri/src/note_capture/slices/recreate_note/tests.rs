//! Tests for slice `recreate-note` (scenario S20).
//!
//! Spec: `.ori/scenarios/s20-external-delete-while-editing/spec.md`.

use std::cell::{Cell, RefCell};
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use time::macros::datetime;
use time::OffsetDateTime;

use crate::note_capture::shared::ports::{Clock, NoteRepository};
use crate::note_capture::shared::types::{Note, Timestamp};

use super::application::RecreateNoteUseCase;
use super::domain::{RecreateNoteCommand, RecreateNoteError};

struct FixedClock {
    now: Timestamp,
}

impl FixedClock {
    fn new(dt: OffsetDateTime) -> Self {
        Self {
            now: Timestamp::from_offset_datetime(dt),
        }
    }
}

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        self.now
    }
}

#[derive(Default)]
struct FakeRepo {
    writes: RefCell<Vec<Note>>,
    fail_with: Cell<Option<io::ErrorKind>>,
    storage_dir: PathBuf,
}

impl FakeRepo {
    fn new() -> Self {
        Self {
            writes: RefCell::new(Vec::new()),
            fail_with: Cell::new(None),
            storage_dir: PathBuf::from("/tmp/promptnotes-test"),
        }
    }

    fn fail_next(&self, kind: io::ErrorKind) {
        self.fail_with.set(Some(kind));
    }

    fn write_count(&self) -> usize {
        self.writes.borrow().len()
    }

    fn last(&self) -> Option<Note> {
        self.writes.borrow().last().cloned()
    }
}

impl NoteRepository for FakeRepo {
    fn write(&self, note: &Note) -> io::Result<()> {
        if let Some(kind) = self.fail_with.take() {
            return Err(io::Error::new(kind, "fake repo failure"));
        }
        self.writes.borrow_mut().push(note.clone());
        Ok(())
    }

    fn storage_dir(&self) -> &Path {
        &self.storage_dir
    }
}

fn created_at_2026() -> Timestamp {
    Timestamp::from_offset_datetime(datetime!(2026-06-20 12:00:00 UTC))
}

fn use_case(repo: Rc<FakeRepo>) -> RecreateNoteUseCase<Rc<FakeRepo>, FixedClock> {
    RecreateNoteUseCase::new(repo, FixedClock::new(datetime!(2026-09-21 00:00:00 UTC)))
}

#[test]
fn recreates_file_at_original_id_preserving_created_at() {
    let repo = Rc::new(FakeRepo::new());
    let uc = use_case(Rc::clone(&repo));
    let created_at = created_at_2026();

    let note = uc
        .execute(RecreateNoteCommand {
            created_at,
            raw_body: "hello local".to_string(),
            raw_tags: vec!["Work".to_string()],
        })
        .expect("recreate should succeed");

    // id is derived from created_at → original identity is preserved (I-N2).
    assert_eq!(note.id().as_str(), "20260620120000");
    assert_eq!(note.created_at(), created_at);
    assert_eq!(note.updated_at().format_rfc3339(), "2026-09-21T00:00:00Z");
    assert_eq!(note.body().as_str(), "hello local");
    assert_eq!(note.tags().as_slice().len(), 1);
    assert_eq!(note.tags().as_slice()[0].name(), "work");

    // Persisted exactly once, with the same id.
    assert_eq!(repo.write_count(), 1);
    assert_eq!(repo.last().expect("a write").id().as_str(), "20260620120000");
}

#[test]
fn invalid_body_is_rejected_without_writing() {
    let repo = Rc::new(FakeRepo::new());
    // NoteBody rejects a body containing a frontmatter delimiter line (I-N8).
    let result = use_case(Rc::clone(&repo)).execute(RecreateNoteCommand {
        created_at: created_at_2026(),
        raw_body: "line\n---\nline".to_string(),
        raw_tags: vec![],
    });

    assert!(matches!(result, Err(RecreateNoteError::InvalidBody { .. })));
    assert_eq!(repo.write_count(), 0);
}

#[test]
fn invalid_tag_is_rejected_without_writing() {
    let repo = Rc::new(FakeRepo::new());
    let result = use_case(Rc::clone(&repo)).execute(RecreateNoteCommand {
        created_at: created_at_2026(),
        raw_body: "body".to_string(),
        raw_tags: vec!["bad tag".to_string()],
    });

    match result {
        Err(RecreateNoteError::InvalidTag { raw, .. }) => assert_eq!(raw, "bad tag"),
        other => panic!("expected InvalidTag, got {other:?}"),
    }
    assert_eq!(repo.write_count(), 0);
}

#[test]
fn persist_failure_surfaces_as_persist_error() {
    let repo = Rc::new(FakeRepo::new());
    repo.fail_next(io::ErrorKind::PermissionDenied);
    let result = use_case(Rc::clone(&repo)).execute(RecreateNoteCommand {
        created_at: created_at_2026(),
        raw_body: "body".to_string(),
        raw_tags: vec![],
    });

    assert!(matches!(
        result,
        Err(RecreateNoteError::PersistError { .. })
    ));
}
