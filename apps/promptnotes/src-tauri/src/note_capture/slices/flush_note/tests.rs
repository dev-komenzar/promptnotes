//! Tests for slice `flush-note`.
//!
//! Spec: `.ori/slices/flush-note/spec.md#test-perspectives`.
//!
//! RED phase: `FlushNoteUseCase::execute` is `unimplemented!()`. Tests that
//! exercise behaviour panic; the compile-time signature pin (TP-AS1) and
//! the trivial "1-Note responsibility" pin (TP-S13-2) are allowed to pass
//! — they only verify public surface, not pipeline behaviour.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use time::macros::datetime;
use time::OffsetDateTime;

use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, DebounceTimer, EventBus, NoteRepository};
use crate::note_capture::shared::types::{Note, NoteBody, NoteId, TagSet, Timestamp};

use super::application::FlushNoteUseCase;
use super::domain::{FlushError, FlushNoteCommand, FlushTrigger};

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

#[derive(Default)]
struct FakeTimer {
    cancel_calls: RefCell<Vec<NoteId>>,
    /// Order events: "cancel" or "write" — proves cancel precedes write.
    seq: RefCell<Vec<&'static str>>,
}
impl FakeTimer {
    fn new() -> Self {
        Self::default()
    }
    fn cancel_count(&self) -> usize {
        self.cancel_calls.borrow().len()
    }
    fn last_cancelled(&self) -> Option<NoteId> {
        self.cancel_calls.borrow().last().cloned()
    }
}
impl DebounceTimer for FakeTimer {
    fn cancel(&self, note_id: &NoteId) {
        self.cancel_calls.borrow_mut().push(note_id.clone());
        self.seq.borrow_mut().push("cancel");
    }
}

// ===== Rc wrappers so we can observe state after `execute` consumes the rig =====

struct RcRepo {
    inner: Rc<FakeRepo>,
    timer: Rc<FakeTimer>,
}
impl NoteRepository for RcRepo {
    fn write(&self, n: &Note) -> io::Result<()> {
        let r = self.inner.write(n);
        if r.is_ok() {
            // Mirror order into the timer's seq so a single Vec captures both events.
            self.timer.seq.borrow_mut().push("write");
        }
        r
    }
    fn storage_dir(&self) -> &Path {
        self.inner.storage_dir()
    }
    fn load_by_id(&self, id: &NoteId) -> io::Result<Option<Note>> {
        // Mirror the load attempt (success or failure) into the timer's seq
        // so tests can pin the cancel → load ordering on every code path.
        self.timer.seq.borrow_mut().push("load");
        self.inner.load_by_id(id)
    }
}

struct RcBus(Rc<FakeBus>);
impl EventBus for RcBus {
    fn publish(&self, e: DomainEvent) {
        self.0.publish(e);
    }
}

struct RcTimer(Rc<FakeTimer>);
impl DebounceTimer for RcTimer {
    fn cancel(&self, id: &NoteId) {
        self.0.cancel(id);
    }
}

type Rig = (
    FlushNoteUseCase<RcRepo, FixedClock, RcBus, RcTimer>,
    Rc<FakeRepo>,
    Rc<FakeBus>,
    Rc<FakeTimer>,
);

fn rig(now: OffsetDateTime) -> Rig {
    let repo = Rc::new(FakeRepo::new());
    let bus = Rc::new(FakeBus::new());
    let timer = Rc::new(FakeTimer::new());
    let uc = FlushNoteUseCase::new(
        RcRepo {
            inner: repo.clone(),
            timer: timer.clone(),
        },
        FixedClock::new(now),
        RcBus(bus.clone()),
        RcTimer(timer.clone()),
    );
    (uc, repo, bus, timer)
}

fn fixture_note(body: &str, created_at: OffsetDateTime) -> Note {
    Note::create(
        NoteBody::new(body.into()).expect("test fixture body must be valid"),
        TagSet::empty(),
        Timestamp::from_offset_datetime(created_at),
    )
}

fn cmd(note_id: NoteId, pending_body: &str, trigger: FlushTrigger) -> FlushNoteCommand {
    FlushNoteCommand {
        note_id,
        pending_body: pending_body.into(),
        trigger,
    }
}

// ===== TP-H*: happy path (body changed) =====

/// spec.md#tp-happy TP-H1
#[test]
fn tp_h1_body_changed_returns_updated_note_with_new_body_and_updated_at() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus, _timer) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let updated = uc
        .execute(cmd(id.clone(), "hello world", FlushTrigger::BlockBlur))
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

/// spec.md#tp-happy TP-H2 — cancel(note_id) then write, in that order
#[test]
fn tp_h2_body_changed_calls_cancel_then_write_in_order() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus, timer) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let _ = uc.execute(cmd(id.clone(), "hello world", FlushTrigger::BlockBlur));

    assert_eq!(repo.write_count(), 1, "exactly one write");
    assert_eq!(timer.cancel_count(), 1, "exactly one cancel");
    assert_eq!(
        timer.last_cancelled().unwrap(),
        id,
        "cancel is keyed by the command's note_id"
    );
    assert_eq!(
        *timer.seq.borrow(),
        vec!["cancel", "load", "write"],
        "C-FL1: cancel must run before load and write"
    );
}

/// spec.md#tp-happy TP-H3
#[test]
fn tp_h3_body_changed_publishes_note_body_edited_once_with_correct_payload() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let expected_updated_at = Timestamp::from_offset_datetime(now);
    let (uc, repo, bus, _timer) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let _ = uc.execute(cmd(id.clone(), "hello world", FlushTrigger::BlockBlur));

    assert_eq!(bus.event_count(), 1, "C-FL7: exactly one event");
    match bus.last().expect("one event") {
        DomainEvent::NoteBodyEdited {
            note_id,
            updated_at,
        } => {
            assert_eq!(note_id, id);
            assert_eq!(updated_at, expected_updated_at);
        }
        other => panic!("flush-note must publish NoteBodyEdited, got {other:?}"),
    }
}

/// spec.md#tp-happy TP-H4
#[test]
fn tp_h4_body_changed_preserves_tags_created_at_and_id() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus, _timer) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    let tags_before = seed.tags().clone();
    let created_before = seed.created_at();
    repo.seed(seed);

    let updated = uc
        .execute(cmd(id.clone(), "hello world", FlushTrigger::BlockBlur))
        .expect("must succeed")
        .expect("must be Some");

    assert_eq!(updated.id(), &id);
    assert_eq!(updated.tags(), &tags_before, "C-FL9: tags unchanged");
    assert_eq!(updated.created_at(), created_before, "I-N1 corollary");
}

/// spec.md#tp-happy TP-H5 — WindowBlur と AppQuit でも同じ振る舞い (C-FL8)
#[test]
fn tp_h5_all_three_triggers_yield_identical_pipeline_behaviour() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);

    for trigger in [
        FlushTrigger::BlockBlur,
        FlushTrigger::WindowBlur,
        FlushTrigger::AppQuit,
    ] {
        let (uc, repo, bus, timer) = rig(now);
        let seed = fixture_note("hello", created);
        let id = seed.id().clone();
        repo.seed(seed);

        let updated = uc
            .execute(cmd(id.clone(), "hello world", trigger))
            .expect("must succeed")
            .expect("must be Some");

        assert_eq!(updated.body().as_str(), "hello world", "{trigger:?}");
        assert_eq!(repo.write_count(), 1, "{trigger:?}");
        assert_eq!(bus.event_count(), 1, "{trigger:?}");
        assert_eq!(timer.cancel_count(), 1, "{trigger:?}");
    }
}

// ===== TP-S3-*: S3 scenario =====

/// spec.md#tp-s3-blur TP-S3-1 + TP-S3-2 — focus loss scenario invokes
/// cancel, then publishes NoteBodyEdited (domain/validation.md#s3-then).
#[test]
fn tp_s3_focus_loss_cancels_then_publishes_event() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus, timer) = rig(now);
    let seed = fixture_note("draft", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let result = uc
        .execute(cmd(id.clone(), "draft+", FlushTrigger::BlockBlur))
        .expect("focus-loss flush must succeed")
        .expect("must be Some");

    assert_eq!(result.body().as_str(), "draft+");
    assert_eq!(timer.cancel_count(), 1, "S3: debounce timer was cancelled");
    assert_eq!(bus.event_count(), 1, "S3: NoteBodyEdited emitted");
    match bus.last().unwrap() {
        DomainEvent::NoteBodyEdited { updated_at, .. } => {
            assert_eq!(updated_at, Timestamp::from_offset_datetime(now))
        }
        other => panic!("expected NoteBodyEdited, got {other:?}"),
    }
}

// ===== TP-S13-*: quit scenario =====

/// spec.md#tp-s13-quit TP-S13-1 — single-Note flush under AppQuit succeeds
/// and follows cancel→write order. (Multi-Note ordering is the caller's
/// responsibility, see TP-S13-2.)
#[test]
fn tp_s13_app_quit_flush_single_note_succeeds() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus, timer) = rig(now);
    let seed = fixture_note("a", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let _ = uc
        .execute(cmd(id, "a+b", FlushTrigger::AppQuit))
        .expect("AppQuit flush must succeed");

    assert_eq!(repo.write_count(), 1);
    assert_eq!(bus.event_count(), 1);
    assert_eq!(*timer.seq.borrow(), vec!["cancel", "load", "write"]);
}

/// spec.md#tp-s13-quit TP-S13-2 — use case responsibility is exactly one
/// FlushNoteCommand. Type-level pin: the signature accepts one command and
/// returns one Result, never a collection.
#[test]
fn tp_s13_2_use_case_signature_takes_one_command_not_many() {
    fn _pin<R: NoteRepository, C: Clock, E: EventBus, D: DebounceTimer>(
        uc: &FlushNoteUseCase<R, C, E, D>,
    ) {
        let _f: fn(
            &FlushNoteUseCase<R, C, E, D>,
            FlushNoteCommand,
        ) -> Result<Option<Note>, FlushError> = FlushNoteUseCase::<R, C, E, D>::execute;
        let _ = uc;
    }
    let (uc, _r, _b, _t) = rig(datetime!(2026-06-25 12:00:00 UTC));
    _pin(&uc);
}

// ===== TP-I*: idempotency guard (body unchanged) =====

/// spec.md#tp-idempotent TP-I1
#[test]
fn tp_i1_body_unchanged_returns_none() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus, _timer) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let result = uc
        .execute(cmd(id, "hello", FlushTrigger::BlockBlur))
        .expect("unchanged body is not an error");

    assert!(result.is_none(), "idempotent guard returns Ok(None)");
}

/// spec.md#tp-idempotent TP-I2 — cancel is called, write is skipped
#[test]
fn tp_i2_body_unchanged_still_cancels_but_skips_write() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus, timer) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let _ = uc.execute(cmd(id, "hello", FlushTrigger::BlockBlur));

    assert_eq!(
        timer.cancel_count(),
        1,
        "C-FL1: cancel runs before body comparison"
    );
    assert_eq!(repo.write_count(), 0, "C-FL4: no write when body unchanged");
}

/// spec.md#tp-idempotent TP-I3
#[test]
fn tp_i3_body_unchanged_skips_publish() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus, _timer) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let _ = uc.execute(cmd(id, "hello", FlushTrigger::BlockBlur));

    assert_eq!(bus.event_count(), 0, "no event when body unchanged");
}

// ===== TP-NF*: NoteNotFound =====

/// spec.md#tp-not-found TP-NF1
#[test]
fn tp_nf1_missing_note_id_yields_note_not_found() {
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, _repo, _bus, _timer) = rig(now);
    let missing = NoteId::from_timestamp(Timestamp::from_offset_datetime(datetime!(
        2026-06-20 09:00:00 UTC
    )));

    let err = uc
        .execute(cmd(missing.clone(), "anything", FlushTrigger::WindowBlur))
        .expect_err("missing id must be an error");

    match err {
        FlushError::NoteNotFound { id } => assert_eq!(id, missing),
        other => panic!("expected NoteNotFound, got {other:?}"),
    }
}

/// spec.md#tp-not-found TP-NF2 — cancel is still called (C-FL1),
/// write / publish are not.
#[test]
fn tp_nf2_missing_note_cancels_but_does_not_write_or_publish() {
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus, timer) = rig(now);
    let missing = NoteId::from_timestamp(Timestamp::from_offset_datetime(datetime!(
        2026-06-20 09:00:00 UTC
    )));

    let _ = uc.execute(cmd(missing.clone(), "anything", FlushTrigger::WindowBlur));

    assert_eq!(timer.cancel_count(), 1, "C-FL1: cancel precedes load");
    assert_eq!(
        timer.last_cancelled().unwrap(),
        missing,
        "cancel keyed by command's note_id, even on NoteNotFound"
    );
    assert_eq!(
        *timer.seq.borrow(),
        vec!["cancel", "load"],
        "C-FL1: cancel before load even when load returns Ok(None)"
    );
    assert_eq!(repo.write_count(), 0);
    assert_eq!(bus.event_count(), 0);
}

// ===== TP-LE*: LoadError (read I/O failure) =====

/// spec.md#tp-load-err TP-LE1 + TP-LE3
#[test]
fn tp_le1_load_failure_surfaces_as_load_error_not_persist_error() {
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus, timer) = rig(now);
    let id = NoteId::from_timestamp(Timestamp::from_offset_datetime(datetime!(
        2026-06-20 09:00:00 UTC
    )));
    repo.fail_next_load(io::ErrorKind::PermissionDenied);

    let err = uc
        .execute(cmd(id.clone(), "anything", FlushTrigger::BlockBlur))
        .expect_err("load failure must surface");

    match err {
        FlushError::LoadError { path, source } => {
            assert_eq!(
                path,
                PathBuf::from(format!("/tmp/promptnotes-test/{}.md", id.as_str()))
            );
            assert_eq!(source.kind(), io::ErrorKind::PermissionDenied);
        }
        FlushError::PersistError { .. } => {
            panic!("read failure must NOT collapse into PersistError (C-FL2)")
        }
        other => panic!("expected LoadError, got {other:?}"),
    }
    assert_eq!(timer.cancel_count(), 1, "C-FL1: cancel runs even on LoadError");
    assert_eq!(timer.last_cancelled().unwrap(), id);
    assert_eq!(repo.write_count(), 0);
    assert_eq!(bus.event_count(), 0);
}

// ===== TP-IB*: InvalidBody (I-N8 violation) =====

/// spec.md#tp-invalid-body TP-IB1
#[test]
fn tp_ib1_body_with_frontmatter_delimiter_line_yields_invalid_body() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus, _timer) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let err = uc
        .execute(cmd(id, "before\n---\nafter", FlushTrigger::AppQuit))
        .expect_err("body with `---` delimiter line must error");

    match err {
        FlushError::InvalidBody { source } => {
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
    let (uc, repo, bus, timer) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let _ = uc.execute(cmd(id.clone(), "---", FlushTrigger::AppQuit));

    assert_eq!(
        timer.cancel_count(),
        1,
        "C-FL1: cancel runs even on InvalidBody"
    );
    assert_eq!(timer.last_cancelled().unwrap(), id);
    assert_eq!(repo.write_count(), 0);
    assert_eq!(bus.event_count(), 0);
}

// ===== TP-PE*: PersistError =====

/// spec.md#tp-persist-err TP-PE1 + TP-PE2
#[test]
fn tp_pe1_pe2_write_failure_surfaces_as_persist_error_with_kind() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus, _timer) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);
    repo.fail_next_write(io::ErrorKind::PermissionDenied);

    let err = uc
        .execute(cmd(id.clone(), "hello world", FlushTrigger::BlockBlur))
        .expect_err("write failure must surface");

    match err {
        FlushError::PersistError { path, source } => {
            assert_eq!(
                path,
                PathBuf::from(format!("/tmp/promptnotes-test/{}.md", id.as_str()))
            );
            assert_eq!(source.kind(), io::ErrorKind::PermissionDenied);
        }
        other => panic!("expected PersistError, got {other:?}"),
    }
}

/// spec.md#tp-persist-err TP-PE3 — write failure must not emit an event.
#[test]
fn tp_pe3_persist_failure_does_not_emit_event() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus, timer) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);
    repo.fail_next_write(io::ErrorKind::Other);

    let _ = uc.execute(cmd(id.clone(), "hello world", FlushTrigger::BlockBlur));

    assert_eq!(
        timer.cancel_count(),
        1,
        "C-FL1: cancel runs even on PersistError"
    );
    assert_eq!(timer.last_cancelled().unwrap(), id);
    assert_eq!(bus.event_count(), 0, "C-FL6: persist fail blocks event");
}

/// spec.md#tp-persist-err TP-PE4 — stateless retry succeeds when fs recovers.
#[test]
fn tp_pe4_retry_after_transient_persist_failure_succeeds() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus, _timer) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    repo.fail_next_write(io::ErrorKind::Other);
    let _ = uc.execute(cmd(id.clone(), "hello world", FlushTrigger::BlockBlur));

    let second = uc
        .execute(cmd(id, "hello world", FlushTrigger::BlockBlur))
        .expect("second call (fs healthy) must succeed")
        .expect("must be Some");
    assert_eq!(second.body().as_str(), "hello world");
    assert_eq!(bus.event_count(), 1, "only the successful run emits");
}

// ===== TP-CO*: cancel ↔ persist ordering =====

/// spec.md#tp-cancel-order TP-CO1 — cancel runs **before** load (C-FL1).
/// The ordering is observed via `timer.seq`, which `RcRepo::load_by_id`
/// mirrors as `"load"` and `FakeTimer::cancel` mirrors as `"cancel"`. The
/// happy path's `["cancel", "load", "write"]` and the load-failure path's
/// `["cancel", "load"]` together pin the invariant: cancel cannot be
/// emitted after load.
#[test]
fn tp_co1_cancel_runs_before_load() {
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus, timer) = rig(now);
    let id = NoteId::from_timestamp(Timestamp::from_offset_datetime(datetime!(
        2026-06-20 09:00:00 UTC
    )));
    repo.fail_next_load(io::ErrorKind::Other);

    let _ = uc.execute(cmd(id, "anything", FlushTrigger::BlockBlur));

    assert_eq!(timer.cancel_count(), 1, "C-FL1: cancel even on load fail");
    assert_eq!(
        *timer.seq.borrow(),
        vec!["cancel", "load"],
        "C-FL1: cancel strictly precedes the load attempt"
    );
}

/// spec.md#tp-cancel-order TP-CO2 — cancel is idempotent (no Result).
/// `DebounceTimer::cancel` returns `()`; repeated calls must be safe.
#[test]
fn tp_co2_cancel_is_idempotent_signature() {
    let (uc, repo, _bus, timer) = rig(datetime!(2026-06-25 12:34:56 UTC));
    let seed = fixture_note("hello", datetime!(2026-06-20 09:00:00 UTC));
    let id = seed.id().clone();
    repo.seed(seed);

    let _ = uc.execute(cmd(id.clone(), "hello", FlushTrigger::BlockBlur));
    let _ = uc.execute(cmd(id, "hello", FlushTrigger::BlockBlur));

    assert_eq!(
        timer.cancel_count(),
        2,
        "cancel succeeds on the second call too (idempotent, no error)"
    );
}

// ===== TP-BC*: body comparison detail =====

/// spec.md#tp-body-compare TP-BC1
#[test]
fn tp_bc1_empty_body_unchanged_is_noop() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus, _timer) = rig(now);
    let seed = fixture_note("", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let result = uc
        .execute(cmd(id, "", FlushTrigger::BlockBlur))
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
    let (uc, repo, _bus, _timer) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let result = uc
        .execute(cmd(id, "hello ", FlushTrigger::BlockBlur))
        .expect("must succeed")
        .expect("must be Some");

    assert_eq!(result.body().as_str(), "hello ");
}

/// spec.md#tp-body-compare TP-BC3 — case difference counts as a change
#[test]
fn tp_bc3_case_difference_is_a_change() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus, _timer) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let result = uc
        .execute(cmd(id, "Hello", FlushTrigger::BlockBlur))
        .expect("must succeed")
        .expect("must be Some");

    assert_eq!(result.body().as_str(), "Hello");
}

// ===== TP-INV*: invariants =====

/// spec.md#tp-invariants TP-INV1 + TP-INV2
#[test]
fn tp_inv1_inv2_returned_note_preserves_id_and_monotonic_timestamps() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus, _timer) = rig(now);
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let updated = uc
        .execute(cmd(id.clone(), "hello world", FlushTrigger::BlockBlur))
        .unwrap()
        .unwrap();

    assert_eq!(updated.id(), &id, "I-N1");
    assert!(updated.updated_at() >= updated.created_at(), "I-N3");
}

// ===== TP-AS*: API shape =====

/// spec.md#tp-api-shape TP-AS1 — compile-time signature pin.
#[test]
fn tp_as1_use_case_signature_is_pinned() {
    fn _pin<R: NoteRepository, C: Clock, E: EventBus, D: DebounceTimer>() {
        let _f: fn(
            &FlushNoteUseCase<R, C, E, D>,
            FlushNoteCommand,
        ) -> Result<Option<Note>, FlushError> = FlushNoteUseCase::<R, C, E, D>::execute;
    }
    _pin::<RcRepo, FixedClock, RcBus, RcTimer>();
}
