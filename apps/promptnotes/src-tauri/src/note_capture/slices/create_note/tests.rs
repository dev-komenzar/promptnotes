//! Tests for slice `create-note`.
//!
//! Spec: `.ori/slices/create-note/spec.md#test-perspectives`.

use std::cell::{Cell, RefCell};
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use proptest::prelude::*;
use time::macros::datetime;
use time::OffsetDateTime;

use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, EventBus, NoteRepository};
use crate::note_capture::shared::types::{Note, Timestamp};

use super::application::CreateNoteUseCase;
use super::domain::{CreateNoteCommand, CreateNoteError};
use super::infrastructure::FsNoteRepository;

// ===== test doubles =====

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

#[derive(Default)]
struct FakeBus {
    events: RefCell<Vec<DomainEvent>>,
}
impl FakeBus {
    fn new() -> Self {
        Self::default()
    }
    fn event_count(&self) -> usize {
        self.events.borrow().len()
    }
    fn last(&self) -> Option<DomainEvent> {
        self.events.borrow().last().cloned()
    }
}
impl EventBus for FakeBus {
    fn publish(&self, event: DomainEvent) {
        self.events.borrow_mut().push(event);
    }
}

// `Rc`-friendly adapters: a test wants to keep an owning handle to the fake
// while also moving an impl into the use case. Without an adapter the fake
// would be moved away and writes/events would be unobservable.

struct RcRepo(Rc<FakeRepo>);
impl NoteRepository for RcRepo {
    fn write(&self, n: &Note) -> io::Result<()> {
        self.0.write(n)
    }
    fn storage_dir(&self) -> &Path {
        self.0.storage_dir()
    }
}

struct RcBus(Rc<FakeBus>);
impl EventBus for RcBus {
    fn publish(&self, e: DomainEvent) {
        self.0.publish(e);
    }
}

type Rig = (
    CreateNoteUseCase<RcRepo, FixedClock, RcBus>,
    Rc<FakeRepo>,
    Rc<FakeBus>,
);

/// Build an observable use case: caller keeps `Rc<FakeRepo>` / `Rc<FakeBus>`
/// for post-call assertions, the use case owns a thin `RcRepo` / `RcBus`.
fn rig(now: OffsetDateTime) -> Rig {
    let repo = Rc::new(FakeRepo::new());
    let bus = Rc::new(FakeBus::new());
    let uc = CreateNoteUseCase::new(
        RcRepo(repo.clone()),
        FixedClock::new(now),
        RcBus(bus.clone()),
    );
    (uc, repo, bus)
}

// ===== TP-H1: happy path =====

#[test]
fn tp_h1_happy_path_creates_note_writes_md_and_emits_event() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, repo, bus) = rig(now);

    let note = uc
        .execute(CreateNoteCommand {
            raw_body: "hello".into(),
            raw_tags: vec![],
        })
        .expect("create-note must succeed")
        .expect("happy path must return Some(note)");

    assert_eq!(note.id().as_str(), "20260624100000");
    assert_eq!(note.body().as_str(), "hello");
    assert!(note.tags().is_empty());
    assert_eq!(note.created_at(), note.updated_at());
    assert_eq!(repo.write_count(), 1, "TP-H1: write called exactly once");
    assert_eq!(bus.event_count(), 1);
}

#[test]
fn tp_h1_emitted_event_payload_matches_aggregate() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, _repo, bus) = rig(now);

    let note = uc
        .execute(CreateNoteCommand {
            raw_body: "hello".into(),
            raw_tags: vec![],
        })
        .expect("must succeed")
        .expect("must be Some");

    match bus.last().expect("one event") {
        DomainEvent::NoteCreated {
            note_id,
            created_at,
            initial_tags,
        } => {
            assert_eq!(note_id.as_str(), note.id().as_str());
            assert_eq!(created_at, note.created_at());
            assert!(initial_tags.is_empty());
        }
        other => panic!("create-note must publish NoteCreated, got {other:?}"),
    }
}

// ===== TP-E*: empty / whitespace body is no-op =====

#[test]
fn tp_e1_empty_body_returns_none_and_skips_persist_and_event() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, repo, bus) = rig(now);

    let result = uc
        .execute(CreateNoteCommand {
            raw_body: String::new(),
            raw_tags: vec![],
        })
        .expect("empty body must not error");

    assert!(result.is_none(), "C-CN3: empty body is a no-op");
    assert_eq!(repo.write_count(), 0);
    assert_eq!(bus.event_count(), 0);
}

#[test]
fn tp_e2_whitespace_only_spaces_is_noop() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, _, _) = rig(now);

    let result = uc
        .execute(CreateNoteCommand {
            raw_body: "   ".into(),
            raw_tags: vec![],
        })
        .expect("whitespace body must not error");
    assert!(result.is_none());
}

#[test]
fn tp_e3_whitespace_only_mixed_is_noop() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, _, _) = rig(now);

    let result = uc
        .execute(CreateNoteCommand {
            raw_body: "\n\t  \n".into(),
            raw_tags: vec![],
        })
        .expect("mixed whitespace must not error");
    assert!(result.is_none());
}

#[test]
fn tp_e4_single_char_body_is_not_noop() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, _, _) = rig(now);

    let result = uc
        .execute(CreateNoteCommand {
            raw_body: "a".into(),
            raw_tags: vec![],
        })
        .expect("non-empty body must succeed");
    assert!(result.is_some());
}

/// spec.md#tp-empty-body TP-E5 — zenkaku/CJK whitespace
#[test]
fn tp_e5_zenkaku_only_body_is_noop() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, _, _) = rig(now);

    let result = uc
        .execute(CreateNoteCommand {
            raw_body: "\u{3000}\u{3000}".into(), // U+3000 IDEOGRAPHIC SPACE
            raw_tags: vec![],
        })
        .expect("zenkaku-only body must not error");
    assert!(result.is_none(), "str::trim covers Unicode White_Space");
}

// ===== TP-T*: tag assignment =====

#[test]
fn tp_t1_tags_are_normalized_and_order_preserved() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, _, _) = rig(now);

    let note = uc
        .execute(CreateNoteCommand {
            raw_body: "x".into(),
            raw_tags: vec!["GPT".into(), "Coding".into()],
        })
        .expect("must succeed")
        .expect("must be Some");

    let names: Vec<&str> = note.tags().as_slice().iter().map(|t| t.name()).collect();
    assert_eq!(names, vec!["gpt", "coding"]);
}

#[test]
fn tp_t2_duplicate_tags_are_deduped_first_wins() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, _, _) = rig(now);

    let note = uc
        .execute(CreateNoteCommand {
            raw_body: "x".into(),
            raw_tags: vec!["gpt".into(), "gpt".into()],
        })
        .expect("must succeed")
        .expect("must be Some");

    assert_eq!(note.tags().len(), 1);
    assert_eq!(note.tags().as_slice()[0].name(), "gpt");
}

/// spec.md#tp-with-tags TP-T3 — cross-case dedupe
#[test]
fn tp_t3_cross_case_tags_dedupe_after_normalization() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, _, _) = rig(now);

    let note = uc
        .execute(CreateNoteCommand {
            raw_body: "x".into(),
            raw_tags: vec!["GPT".into(), "gpt".into(), "Gpt".into()],
        })
        .expect("must succeed")
        .expect("must be Some");

    assert_eq!(note.tags().len(), 1, "all collapse via lowercase normalize");
    assert_eq!(note.tags().as_slice()[0].name(), "gpt");
}

// ===== TP-IT*: invalid tag =====

#[test]
fn tp_it1_tag_with_comma_is_rejected() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, _, _) = rig(now);

    let err = uc
        .execute(CreateNoteCommand {
            raw_body: "x".into(),
            raw_tags: vec!["bad,tag".into()],
        })
        .expect_err("must error");

    assert!(matches!(err, CreateNoteError::InvalidTag { raw, .. } if raw == "bad,tag"));
}

#[test]
fn tp_it2_tag_with_internal_space_is_rejected() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, _, _) = rig(now);

    let err = uc
        .execute(CreateNoteCommand {
            raw_body: "x".into(),
            raw_tags: vec!["a b".into()],
        })
        .expect_err("must error");
    assert!(matches!(err, CreateNoteError::InvalidTag { .. }));
}

#[test]
fn tp_it3_invalid_tag_does_not_persist() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, repo, _) = rig(now);

    let _ = uc.execute(CreateNoteCommand {
        raw_body: "x".into(),
        raw_tags: vec!["bad,tag".into()],
    });

    assert_eq!(repo.write_count(), 0, "C-CN4: no write on InvalidTag");
}

#[test]
fn tp_it4_invalid_tag_does_not_emit_event() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, _, bus) = rig(now);

    let _ = uc.execute(CreateNoteCommand {
        raw_body: "x".into(),
        raw_tags: vec!["bad,tag".into()],
    });

    assert_eq!(bus.event_count(), 0);
}

/// spec.md#tp-invalid-tag TP-IT5 — empty raw_tag surfaces as InvalidTag::Empty
#[test]
fn tp_it5_empty_string_tag_is_rejected_as_empty() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, _, _) = rig(now);

    let err = uc
        .execute(CreateNoteCommand {
            raw_body: "x".into(),
            raw_tags: vec!["".into()],
        })
        .expect_err("empty raw_tag must error");

    match err {
        CreateNoteError::InvalidTag { raw, source } => {
            assert_eq!(raw, "");
            use crate::note_capture::shared::types::TagError;
            assert!(matches!(source, TagError::Empty));
        }
        other => panic!("expected InvalidTag {{ source: Empty }}, got {other:?}"),
    }
}

// ===== TP-IB*: NoteBody validation =====

#[test]
fn tp_ib1_standalone_dash_line_returns_invalid_body() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, _, _) = rig(now);

    let err = uc
        .execute(CreateNoteCommand {
            raw_body: "---".into(),
            raw_tags: vec![],
        })
        .expect_err("standalone '---' must error, not panic");
    assert!(matches!(err, CreateNoteError::InvalidBody { .. }));
}

#[test]
fn tp_ib2_dash_line_in_middle_returns_invalid_body() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, _, _) = rig(now);

    let err = uc
        .execute(CreateNoteCommand {
            raw_body: "hello\n---\nworld".into(),
            raw_tags: vec![],
        })
        .expect_err("interior '---' line must error");
    assert!(matches!(err, CreateNoteError::InvalidBody { .. }));
}

#[test]
fn tp_ib3_invalid_body_does_not_persist_or_emit() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, repo, bus) = rig(now);

    let _ = uc.execute(CreateNoteCommand {
        raw_body: "---".into(),
        raw_tags: vec![],
    });

    assert_eq!(repo.write_count(), 0);
    assert_eq!(bus.event_count(), 0);
}

// ===== TP-PE*: persist failure =====

#[test]
fn tp_pe1_write_io_error_becomes_persist_error_at_expected_path() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, repo, _) = rig(now);
    repo.fail_next(io::ErrorKind::PermissionDenied);

    let err = uc
        .execute(CreateNoteCommand {
            raw_body: "hello".into(),
            raw_tags: vec![],
        })
        .expect_err("write failure must surface as PersistError");

    match err {
        CreateNoteError::PersistError { path, source } => {
            assert_eq!(
                path,
                PathBuf::from("/tmp/promptnotes-test/20260624100000.md"),
                "C-CN5: filename is <id>.md under storage_dir"
            );
            assert_eq!(source.kind(), io::ErrorKind::PermissionDenied);
        }
        other => panic!("expected PersistError, got {other:?}"),
    }
}

#[test]
fn tp_pe2_persist_error_does_not_emit_event() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, repo, bus) = rig(now);
    repo.fail_next(io::ErrorKind::Other);

    let _ = uc.execute(CreateNoteCommand {
        raw_body: "hello".into(),
        raw_tags: vec![],
    });

    assert_eq!(bus.event_count(), 0, "C-CN4: persist fail blocks event");
}

// ===== TP-I*: property tests for invariants =====

proptest! {
    /// I-N2: createdAt formatted == id
    #[test]
    fn tp_i1_note_id_roundtrips_with_created_at(seed in 0i64..4_000_000_000) {
        let dt = OffsetDateTime::from_unix_timestamp(seed).unwrap();
        let now = Timestamp::from_offset_datetime(dt);
        let uc = CreateNoteUseCase::new(FakeRepo::new(), FixedClock { now }, FakeBus::new());

        let note = uc
            .execute(CreateNoteCommand {
                raw_body: "x".into(),
                raw_tags: vec![],
            })
            .expect("must succeed")
            .expect("must be Some");

        prop_assert_eq!(note.created_at().format_yyyymmddhhmmss(), note.id().as_str().to_string());
    }

    /// I-N3: updatedAt >= createdAt (equal on create)
    #[test]
    fn tp_i2_updated_at_ge_created_at_at_creation(seed in 0i64..4_000_000_000) {
        let dt = OffsetDateTime::from_unix_timestamp(seed).unwrap();
        let now = Timestamp::from_offset_datetime(dt);
        let uc = CreateNoteUseCase::new(FakeRepo::new(), FixedClock { now }, FakeBus::new());

        let note = uc
            .execute(CreateNoteCommand {
                raw_body: "x".into(),
                raw_tags: vec![],
            })
            .expect("must succeed")
            .expect("must be Some");

        prop_assert!(note.updated_at() >= note.created_at());
    }

    /// I-N6: interior forbidden char in trimmed Tag content is always rejected.
    /// Strategy widened: uppercase ASCII, CJK, and surrounding whitespace are mixed in.
    #[test]
    fn tp_i3_interior_forbidden_char_in_tag_is_always_rejected(
        leading_ws in "[ \\t]{0,3}",
        prefix in "[a-zA-Z\u{3040}-\u{309F}]{1,5}",
        forbidden in prop_oneof![Just(' '), Just('\t'), Just('\n'), Just(','), Just('['), Just(']')],
        suffix in "[a-zA-Z\u{3040}-\u{309F}]{1,5}",
        trailing_ws in "[ \\t]{0,3}",
    ) {
        let raw = format!("{leading_ws}{prefix}{forbidden}{suffix}{trailing_ws}");
        let now = datetime!(2026-06-24 10:00:00 UTC);
        let (uc, _, _) = rig(now);

        let err = uc.execute(CreateNoteCommand {
            raw_body: "x".into(),
            raw_tags: vec![raw.clone()],
        });

        let is_invalid_tag = matches!(err, Err(CreateNoteError::InvalidTag { .. }));
        prop_assert!(is_invalid_tag);
    }
}

// ===== TP-C*: structural collision avoidance =====

#[test]
fn tp_c1_same_now_first_non_empty_succeeds_then_empty_is_noop() {
    let now = datetime!(2026-06-24 10:00:00 UTC);
    let (uc, repo, _) = rig(now);

    let first = uc
        .execute(CreateNoteCommand {
            raw_body: "x".into(),
            raw_tags: vec![],
        })
        .expect("first must succeed");
    let second = uc
        .execute(CreateNoteCommand {
            raw_body: String::new(),
            raw_tags: vec![],
        })
        .expect("second (empty) must not error");

    assert!(first.is_some());
    assert!(second.is_none(), "C-CN6: second empty is no-op");
    assert_eq!(repo.write_count(), 1);
}

// ===== Infrastructure: FsNoteRepository file format =====

#[test]
fn fs_note_repo_writes_frontmatter_and_body_at_id_path() {
    use crate::note_capture::shared::types::{NoteBody, TagSet};

    let now = datetime!(2026-06-24 10:00:00 UTC);
    let tempdir = tempfile::tempdir().expect("tempdir");
    let repo = FsNoteRepository::new(tempdir.path().to_path_buf());
    let note = Note::create(
        NoteBody::new("hello world".into()).unwrap(),
        TagSet::empty(),
        Timestamp::from_offset_datetime(now),
    );

    repo.write(&note).expect("write must succeed");

    let path = tempdir.path().join("20260624100000.md");
    let content = std::fs::read_to_string(&path).expect("file must exist");
    assert_eq!(
        content,
        "---\n\
         createdAt: 20260624100000\n\
         updatedAt: 20260624100000\n\
         tags: []\n\
         ---\n\
         hello world",
        "frontmatter format is locked by spec.md#impl-frontmatter"
    );
}

#[test]
fn fs_note_repo_writes_tags_inline_in_insertion_order() {
    use crate::note_capture::shared::types::{NoteBody, Tag, TagSet};

    let now = datetime!(2026-06-24 10:00:00 UTC);
    let tempdir = tempfile::tempdir().expect("tempdir");
    let repo = FsNoteRepository::new(tempdir.path().to_path_buf());
    let tags = TagSet::from_tags([
        Tag::new("GPT").unwrap(),
        Tag::new("coding").unwrap(),
    ]);
    let note = Note::create(
        NoteBody::new("body".into()).unwrap(),
        tags,
        Timestamp::from_offset_datetime(now),
    );

    repo.write(&note).expect("write must succeed");

    let content = std::fs::read_to_string(tempdir.path().join("20260624100000.md")).unwrap();
    assert!(content.contains("tags: [gpt, coding]"), "got:\n{content}");
}

#[test]
fn fs_note_repo_creates_storage_dir_on_first_write() {
    use crate::note_capture::shared::types::{NoteBody, TagSet};

    let now = datetime!(2026-06-24 10:00:00 UTC);
    let tempdir = tempfile::tempdir().expect("tempdir");
    let nested = tempdir.path().join("a/b/c");
    assert!(!nested.exists());

    let repo = FsNoteRepository::new(nested.clone());
    let note = Note::create(
        NoteBody::new("body".into()).unwrap(),
        TagSet::empty(),
        Timestamp::from_offset_datetime(now),
    );
    repo.write(&note).expect("write must create parent dirs");

    assert!(nested.join("20260624100000.md").exists());
}

// ===== FsNoteRepository::load_by_id — roundtrip + edge cases =====
// (auto-save-note review MED-2 反映: load_by_id 経路の regression を本 slice で守る)

#[test]
fn fs_note_repo_load_by_id_roundtrips_a_freshly_written_note() {
    use crate::note_capture::shared::types::{NoteBody, Tag, TagSet};

    let created = datetime!(2026-06-20 09:00:00 UTC);
    let tempdir = tempfile::tempdir().expect("tempdir");
    let repo = FsNoteRepository::new(tempdir.path().to_path_buf());

    let tags = TagSet::from_tags([Tag::new("GPT").unwrap(), Tag::new("coding").unwrap()]);
    let note = Note::create(
        NoteBody::new("hello world".into()).unwrap(),
        tags.clone(),
        Timestamp::from_offset_datetime(created),
    );
    repo.write(&note).expect("write");

    let loaded = repo
        .load_by_id(note.id())
        .expect("load io ok")
        .expect("note must round-trip");

    assert_eq!(loaded.id(), note.id());
    assert_eq!(loaded.body().as_str(), "hello world");
    assert_eq!(loaded.tags(), &tags);
    assert_eq!(loaded.created_at(), note.created_at());
    assert_eq!(loaded.updated_at(), note.updated_at());
}

#[test]
fn fs_note_repo_load_by_id_returns_none_on_missing_file() {
    use crate::note_capture::shared::types::NoteId;

    let tempdir = tempfile::tempdir().expect("tempdir");
    let repo = FsNoteRepository::new(tempdir.path().to_path_buf());
    let id = NoteId::from_timestamp(Timestamp::from_offset_datetime(datetime!(
        2026-06-20 09:00:00 UTC
    )));

    let result = repo.load_by_id(&id).expect("missing file is not an error");
    assert!(result.is_none());
}

#[test]
fn fs_note_repo_load_by_id_yields_invalid_data_on_malformed_frontmatter() {
    use crate::note_capture::shared::types::NoteId;

    let tempdir = tempfile::tempdir().expect("tempdir");
    let repo = FsNoteRepository::new(tempdir.path().to_path_buf());
    let id_dt = datetime!(2026-06-20 09:00:00 UTC);
    let id = NoteId::from_timestamp(Timestamp::from_offset_datetime(id_dt));
    let path = tempdir.path().join(format!("{}.md", id.as_str()));
    // No leading `---` line.
    std::fs::write(&path, "not a frontmatter\nat all").expect("seed");

    let err = repo.load_by_id(&id).expect_err("malformed file must error");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn fs_note_repo_load_by_id_handles_empty_tags_inline() {
    use crate::note_capture::shared::types::{NoteBody, TagSet};

    let created = datetime!(2026-06-20 09:00:00 UTC);
    let tempdir = tempfile::tempdir().expect("tempdir");
    let repo = FsNoteRepository::new(tempdir.path().to_path_buf());

    let note = Note::create(
        NoteBody::new("body".into()).unwrap(),
        TagSet::empty(),
        Timestamp::from_offset_datetime(created),
    );
    repo.write(&note).expect("write");

    let loaded = repo
        .load_by_id(note.id())
        .expect("load io ok")
        .expect("must roundtrip empty-tags note");
    assert!(loaded.tags().is_empty());
}
