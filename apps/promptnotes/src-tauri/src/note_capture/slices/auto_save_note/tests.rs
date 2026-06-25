//! Tests for slice `auto-save-note`.
//!
//! Spec: `.ori/slices/auto-save-note/spec.md#test-perspectives`.
//!
//! RED phase: `AutoSaveNoteUseCase::execute` is `unimplemented!()`. Tests
//! that exercise behaviour panic; the compile-time signature pin (TP-AS1)
//! is allowed to pass — it only verifies the public surface, not behaviour.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use time::macros::datetime;
use time::OffsetDateTime;

use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, EventBus, NoteRepository};
use crate::note_capture::shared::types::{Note, NoteBody, NoteId, TagSet, Timestamp};

use super::application::AutoSaveNoteUseCase;
use super::domain::{AutoSaveError, AutoSaveNoteCommand};

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
    notes: RefCell<HashMap<String, Note>>,
    writes: RefCell<Vec<Note>>,
    fail_write_with: Cell<Option<io::ErrorKind>>,
    fail_load_with: Cell<Option<io::ErrorKind>>,
    storage_dir: PathBuf,
}
impl FakeRepo {
    fn new() -> Self {
        Self {
            notes: RefCell::new(HashMap::new()),
            writes: RefCell::new(Vec::new()),
            fail_write_with: Cell::new(None),
            fail_load_with: Cell::new(None),
            storage_dir: PathBuf::from("/tmp/promptnotes-test"),
        }
    }
    fn seed(&self, note: Note) {
        self.notes
            .borrow_mut()
            .insert(note.id().as_str().to_string(), note);
    }
    fn fail_next_write(&self, kind: io::ErrorKind) {
        self.fail_write_with.set(Some(kind));
    }
    fn fail_next_load(&self, kind: io::ErrorKind) {
        self.fail_load_with.set(Some(kind));
    }
    fn write_count(&self) -> usize {
        self.writes.borrow().len()
    }
}
impl NoteRepository for FakeRepo {
    fn write(&self, note: &Note) -> io::Result<()> {
        if let Some(kind) = self.fail_write_with.take() {
            return Err(io::Error::new(kind, "fake repo failure"));
        }
        self.notes
            .borrow_mut()
            .insert(note.id().as_str().to_string(), note.clone());
        self.writes.borrow_mut().push(note.clone());
        Ok(())
    }
    fn storage_dir(&self) -> &Path {
        &self.storage_dir
    }
    fn load_by_id(&self, id: &NoteId) -> io::Result<Option<Note>> {
        if let Some(kind) = self.fail_load_with.take() {
            return Err(io::Error::new(kind, "fake load failure"));
        }
        Ok(self.notes.borrow().get(id.as_str()).cloned())
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

struct RcRepo(Rc<FakeRepo>);
impl RcRepo {
    fn fail_next_load(&self, kind: io::ErrorKind) {
        self.0.fail_next_load(kind);
    }
}
impl NoteRepository for RcRepo {
    fn write(&self, n: &Note) -> io::Result<()> {
        self.0.write(n)
    }
    fn storage_dir(&self) -> &Path {
        self.0.storage_dir()
    }
    fn load_by_id(&self, id: &NoteId) -> io::Result<Option<Note>> {
        self.0.load_by_id(id)
    }
}

struct RcBus(Rc<FakeBus>);
impl EventBus for RcBus {
    fn publish(&self, e: DomainEvent) {
        self.0.publish(e);
    }
}

type Rig = (
    AutoSaveNoteUseCase<RcRepo, FixedClock, RcBus>,
    Rc<FakeRepo>,
    Rc<FakeBus>,
);

fn rig(now: OffsetDateTime) -> Rig {
    let repo = Rc::new(FakeRepo::new());
    let bus = Rc::new(FakeBus::new());
    let uc = AutoSaveNoteUseCase::new(
        RcRepo(repo.clone()),
        FixedClock::new(now),
        RcBus(bus.clone()),
    );
    (uc, repo, bus)
}

fn fixture_note(body: &str, created_at: OffsetDateTime) -> Note {
    Note::create(
        NoteBody::new(body.into()).expect("test fixture body must be valid"),
        TagSet::empty(),
        Timestamp::from_offset_datetime(created_at),
    )
}

// ===== TP-H*: happy path =====

/// spec.md#tp-happy TP-H1
#[test]
fn tp_h1_body_changed_returns_updated_note_with_new_body_and_updated_at() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let updated = uc
        .execute(AutoSaveNoteCommand {
            note_id: id.clone(),
            new_body: "hello world".into(),
        })
        .expect("body changed must succeed")
        .expect("body changed must return Some");

    assert_eq!(updated.id(), &id, "I-N1: id is immutable");
    assert_eq!(updated.body().as_str(), "hello world");
    assert_eq!(
        updated.updated_at(),
        Timestamp::from_offset_datetime(now),
        "I-N4: updated_at == clock.now()"
    );
}

/// spec.md#tp-happy TP-H2
#[test]
fn tp_h2_body_changed_calls_write_exactly_once() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let _ = uc.execute(AutoSaveNoteCommand {
        note_id: id,
        new_body: "hello world".into(),
    });

    assert_eq!(repo.write_count(), 1);
}

/// spec.md#tp-happy TP-H3
#[test]
fn tp_h3_body_changed_publishes_note_body_edited_once_with_correct_payload() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let expected_updated_at = Timestamp::from_offset_datetime(now);
    let (uc, repo, bus) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let _ = uc.execute(AutoSaveNoteCommand {
        note_id: id.clone(),
        new_body: "hello world".into(),
    });

    assert_eq!(bus.event_count(), 1, "C-AS6: exactly one event");
    match bus.last().expect("one event") {
        DomainEvent::NoteBodyEdited {
            note_id,
            updated_at,
        } => {
            assert_eq!(note_id, id);
            assert_eq!(updated_at, expected_updated_at);
        }
        other => panic!("auto-save-note must publish NoteBodyEdited, got {other:?}"),
    }
}

/// spec.md#tp-happy TP-H4
#[test]
fn tp_h4_body_changed_preserves_tags_created_at_and_id() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    let tags_before = seed.tags().clone();
    let created_before = seed.created_at();
    repo.seed(seed);

    let updated = uc
        .execute(AutoSaveNoteCommand {
            note_id: id.clone(),
            new_body: "hello world".into(),
        })
        .expect("must succeed")
        .expect("must be Some");

    assert_eq!(updated.id(), &id);
    assert_eq!(updated.tags(), &tags_before, "C-AS8: tags unchanged");
    assert_eq!(updated.created_at(), created_before, "I-N1 corollary");
}

// ===== TP-I*: S9 idempotency guard =====

/// spec.md#tp-idempotent TP-I1
#[test]
fn tp_i1_body_unchanged_returns_none() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let result = uc
        .execute(AutoSaveNoteCommand {
            note_id: id,
            new_body: "hello".into(),
        })
        .expect("unchanged body is not an error");

    assert!(result.is_none(), "S9: idempotent guard returns Ok(None)");
}

/// spec.md#tp-idempotent TP-I2
#[test]
fn tp_i2_body_unchanged_skips_write() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let _ = uc.execute(AutoSaveNoteCommand {
        note_id: id,
        new_body: "hello".into(),
    });

    assert_eq!(repo.write_count(), 0, "C-AS3: no write when body unchanged");
}

/// spec.md#tp-idempotent TP-I3
#[test]
fn tp_i3_body_unchanged_skips_publish() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let _ = uc.execute(AutoSaveNoteCommand {
        note_id: id,
        new_body: "hello".into(),
    });

    assert_eq!(bus.event_count(), 0, "S9: no event when body unchanged");
}

// ===== TP-NF*: NoteNotFound =====

/// spec.md#tp-not-found TP-NF1
#[test]
fn tp_nf1_missing_note_id_yields_note_not_found() {
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, _repo, _bus) = rig(now);
    let missing = NoteId::from_timestamp(Timestamp::from_offset_datetime(datetime!(
        2026-06-20 09:00:00 UTC
    )));

    let err = uc
        .execute(AutoSaveNoteCommand {
            note_id: missing.clone(),
            new_body: "anything".into(),
        })
        .expect_err("missing id must be an error");

    match err {
        AutoSaveError::NoteNotFound { id } => assert_eq!(id, missing),
        other => panic!("expected NoteNotFound, got {other:?}"),
    }
}

/// spec.md#tp-not-found TP-NF2
#[test]
fn tp_nf2_missing_note_does_not_touch_write_or_bus() {
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus) = rig(now);
    let missing = NoteId::from_timestamp(Timestamp::from_offset_datetime(datetime!(
        2026-06-20 09:00:00 UTC
    )));

    let _ = uc.execute(AutoSaveNoteCommand {
        note_id: missing,
        new_body: "anything".into(),
    });

    assert_eq!(repo.write_count(), 0);
    assert_eq!(bus.event_count(), 0);
}

// ===== TP-PE*: PersistError =====

/// spec.md#tp-persist-err TP-PE1 + TP-PE2 (path + cause.kind() の両方を確認)
#[test]
fn tp_pe1_pe2_write_failure_surfaces_as_persist_error_with_kind() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);
    repo.fail_next_write(io::ErrorKind::PermissionDenied);

    let err = uc
        .execute(AutoSaveNoteCommand {
            note_id: id.clone(),
            new_body: "hello world".into(),
        })
        .expect_err("write failure must surface");

    match err {
        AutoSaveError::PersistError { path, source } => {
            assert_eq!(
                path,
                PathBuf::from(format!("/tmp/promptnotes-test/{}.md", id.as_str())),
            );
            assert_eq!(source.kind(), io::ErrorKind::PermissionDenied);
        }
        other => panic!("expected PersistError, got {other:?}"),
    }
}

/// spec.md#tp-persist-err TP-PE3
#[test]
fn tp_pe3_persist_failure_does_not_emit_event() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);
    repo.fail_next_write(io::ErrorKind::Other);

    let _ = uc.execute(AutoSaveNoteCommand {
        note_id: id,
        new_body: "hello world".into(),
    });

    assert_eq!(bus.event_count(), 0, "C-AS5: persist fail blocks event");
}

/// spec.md#tp-persist-err TP-PE4 — use case is stateless: a retry after the
/// transient I/O failure recovers without leftover state in the use case.
#[test]
fn tp_pe4_retry_after_transient_persist_failure_succeeds() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    repo.fail_next_write(io::ErrorKind::Other);
    let _ = uc.execute(AutoSaveNoteCommand {
        note_id: id.clone(),
        new_body: "hello world".into(),
    });

    let second = uc
        .execute(AutoSaveNoteCommand {
            note_id: id,
            new_body: "hello world".into(),
        })
        .expect("second call (fs healthy) must succeed")
        .expect("must be Some");
    assert_eq!(second.body().as_str(), "hello world");
    assert_eq!(bus.event_count(), 1, "only the successful run emits");
}

// ===== TP-LE*: LoadError (read I/O failure) — review HIGH-2 反映 =====

/// spec.md#io-errors LoadError: load_by_id が Err を返した場合
#[test]
fn tp_le1_load_failure_surfaces_as_load_error_not_persist_error() {
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus) = rig(now);
    let id = NoteId::from_timestamp(Timestamp::from_offset_datetime(datetime!(
        2026-06-20 09:00:00 UTC
    )));
    RcRepo(repo.clone()).fail_next_load(io::ErrorKind::PermissionDenied);

    let err = uc
        .execute(AutoSaveNoteCommand {
            note_id: id.clone(),
            new_body: "anything".into(),
        })
        .expect_err("load failure must surface");

    match err {
        AutoSaveError::LoadError { path, source } => {
            assert_eq!(
                path,
                PathBuf::from(format!("/tmp/promptnotes-test/{}.md", id.as_str()))
            );
            assert_eq!(source.kind(), io::ErrorKind::PermissionDenied);
        }
        AutoSaveError::PersistError { .. } => {
            panic!("read failure must NOT collapse into PersistError (review HIGH-2)")
        }
        other => panic!("expected LoadError, got {other:?}"),
    }
    assert_eq!(repo.write_count(), 0);
    assert_eq!(bus.event_count(), 0);
}

// ===== TP-IB*: InvalidBody (I-N8 violation, aggregate smart constructor 由来) =====

/// spec.md#tp-invalid-body TP-IB1
#[test]
fn tp_ib1_body_with_frontmatter_delimiter_line_yields_invalid_body() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let err = uc
        .execute(AutoSaveNoteCommand {
            note_id: id,
            new_body: "before\n---\nafter".into(),
        })
        .expect_err("body with `---` delimiter line must error");

    match err {
        AutoSaveError::InvalidBody { source } => {
            use crate::note_capture::shared::types::NoteBodyError;
            assert!(matches!(
                source,
                NoteBodyError::ContainsFrontmatterDelimiter
            ));
        }
        other => panic!("expected InvalidBody, got {other:?}"),
    }
}

/// spec.md#tp-invalid-body TP-IB2
#[test]
fn tp_ib2_invalid_body_skips_write_and_publish() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let _ = uc.execute(AutoSaveNoteCommand {
        note_id: id,
        new_body: "---".into(),
    });

    assert_eq!(repo.write_count(), 0);
    assert_eq!(bus.event_count(), 0);
}

// ===== TP-BC*: body comparison detail =====

/// spec.md#tp-body-compare TP-BC1
#[test]
fn tp_bc1_empty_body_unchanged_is_noop() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus) = rig(now);
    let seed = fixture_note("", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let result = uc
        .execute(AutoSaveNoteCommand {
            note_id: id,
            new_body: String::new(),
        })
        .expect("empty unchanged is not an error");

    assert!(result.is_none());
    assert_eq!(repo.write_count(), 0);
    assert_eq!(bus.event_count(), 0);
}

/// spec.md#tp-body-compare TP-BC2 — trailing whitespace counts as a change
#[test]
fn tp_bc2_trailing_space_is_a_change() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let result = uc
        .execute(AutoSaveNoteCommand {
            note_id: id,
            new_body: "hello ".into(),
        })
        .expect("must succeed")
        .expect("must be Some");

    assert_eq!(result.body().as_str(), "hello ");
}

/// spec.md#tp-body-compare TP-BC3 — case differences count as a change
#[test]
fn tp_bc3_case_difference_is_a_change() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let result = uc
        .execute(AutoSaveNoteCommand {
            note_id: id,
            new_body: "Hello".into(),
        })
        .expect("must succeed")
        .expect("must be Some");

    assert_eq!(result.body().as_str(), "Hello");
}

// ===== TP-INV*: invariants =====

/// spec.md#tp-invariants TP-INV1 / TP-INV2
#[test]
fn tp_inv1_inv2_id_immutable_and_updated_at_ge_created_at() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let updated = uc
        .execute(AutoSaveNoteCommand {
            note_id: id.clone(),
            new_body: "changed".into(),
        })
        .expect("must succeed")
        .expect("must be Some");

    assert_eq!(updated.id(), &id, "I-N1: id immutable");
    assert!(
        updated.updated_at() >= updated.created_at(),
        "I-N3: updated_at >= created_at"
    );
}

// ===== TP-AS*: type-level API surface =====

/// spec.md#tp-api-shape TP-AS1
///
/// Compile-time pin of the public signature. If the shape drifts the
/// project will fail to build, not at runtime.
#[test]
fn tp_as1_execute_signature_returns_result_option_note() {
    type ExecuteFn<R, C, E> = fn(
        &AutoSaveNoteUseCase<R, C, E>,
        AutoSaveNoteCommand,
    ) -> Result<Option<Note>, AutoSaveError>;
    fn assert_signature<R: NoteRepository, C: Clock, E: EventBus>() {
        let _: ExecuteFn<R, C, E> = AutoSaveNoteUseCase::<R, C, E>::execute;
    }
    assert_signature::<FakeRepo, FixedClock, FakeBus>();
}
