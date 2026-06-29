//! Tests for slice `restore-deleted-note`.
//!
//! Spec: `.ori/slices/restore-deleted-note/spec.md#test-perspectives`.
//!
//! RED phase: `RestoreDeletedNoteUseCase::execute` is `unimplemented!()`.
//! Behavioural tests panic; the compile-time signature pin (TP-SIG) passes.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use time::macros::datetime;
use time::OffsetDateTime;

use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, EventBus, NoteRepository};
use crate::note_capture::shared::types::{
    DeletedNote, Note, NoteBody, NoteId, TagSet, Timestamp,
};
use crate::note_capture::slices::delete_note::{TrashErrorKind, TrashService, UndoStack};

use super::application::RestoreDeletedNoteUseCase;
use super::domain::{RestoreDeletedNoteCommand, RestoreDeletedNoteError};

const STORAGE_DIR: &str = "/tmp/promptnotes-test";

type OrderLog = Rc<RefCell<Vec<&'static str>>>;

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
    fail_load_with: Cell<Option<io::ErrorKind>>,
    load_count: Cell<usize>,
    storage_dir: PathBuf,
    order_log: OrderLog,
}
impl FakeRepo {
    fn new(order_log: OrderLog) -> Self {
        Self {
            notes: RefCell::new(HashMap::new()),
            fail_load_with: Cell::new(None),
            load_count: Cell::new(0),
            storage_dir: PathBuf::from(STORAGE_DIR),
            order_log,
        }
    }
    fn seed(&self, note: Note) {
        self.notes
            .borrow_mut()
            .insert(note.id().as_str().to_string(), note);
    }
    fn load_count(&self) -> usize {
        self.load_count.get()
    }
}
impl NoteRepository for FakeRepo {
    fn write(&self, _note: &Note) -> io::Result<()> {
        unreachable!("restore-deleted-note must not call write")
    }
    fn storage_dir(&self) -> &Path {
        &self.storage_dir
    }
    fn load_by_id(&self, id: &NoteId) -> io::Result<Option<Note>> {
        self.load_count.set(self.load_count.get() + 1);
        // Always log: the failure path also needs "load" to appear so we can
        // assert that load ran between trash and remove (review Pass 1 MED-7).
        self.order_log.borrow_mut().push("load");
        if let Some(kind) = self.fail_load_with.take() {
            return Err(io::Error::new(kind, "fake load failure"));
        }
        Ok(self.notes.borrow().get(id.as_str()).cloned())
    }
}

struct RcRepo(Rc<FakeRepo>);
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

#[derive(Default)]
struct FakeTrash {
    restores: RefCell<Vec<PathBuf>>,
    fail_restore_with: Cell<Option<TrashErrorKind>>,
    order_log: OrderLog,
}
impl FakeTrash {
    fn new(order_log: OrderLog) -> Self {
        Self {
            restores: RefCell::new(Vec::new()),
            fail_restore_with: Cell::new(None),
            order_log,
        }
    }
    fn fail_next_restore(&self, kind: TrashErrorKind) {
        self.fail_restore_with.set(Some(kind));
    }
    fn restore_count(&self) -> usize {
        self.restores.borrow().len()
    }
    fn last_restored(&self) -> Option<PathBuf> {
        self.restores.borrow().last().cloned()
    }
}
impl TrashService for FakeTrash {
    fn move_to_trash(&self, _p: &Path) -> Result<(), TrashErrorKind> {
        unreachable!("restore-deleted-note must not call move_to_trash")
    }
    fn restore_from_trash(&self, path: &Path) -> Result<(), TrashErrorKind> {
        // Always log: the failure path also needs "trash" to appear so we can
        // assert that trash ran after find (review Pass 2 LOW-B — symmetry with
        // FakeUndo::find_by_id / FakeRepo::load_by_id which both log
        // unconditionally per Pass 1 MED-6/MED-7).
        self.order_log.borrow_mut().push("trash");
        if let Some(kind) = self.fail_restore_with.take() {
            return Err(kind);
        }
        self.restores.borrow_mut().push(path.to_path_buf());
        Ok(())
    }
}

struct RcTrash(Rc<FakeTrash>);
impl TrashService for RcTrash {
    fn move_to_trash(&self, p: &Path) -> Result<(), TrashErrorKind> {
        self.0.move_to_trash(p)
    }
    fn restore_from_trash(&self, p: &Path) -> Result<(), TrashErrorKind> {
        self.0.restore_from_trash(p)
    }
}

#[derive(Default)]
struct FakeUndo {
    stack: RefCell<Vec<DeletedNote>>,
    find_count: Cell<usize>,
    remove_count: Cell<usize>,
    order_log: OrderLog,
}
impl FakeUndo {
    fn new(order_log: OrderLog) -> Self {
        Self {
            stack: RefCell::new(Vec::new()),
            find_count: Cell::new(0),
            remove_count: Cell::new(0),
            order_log,
        }
    }
    fn seed(&self, deleted: DeletedNote) {
        self.stack.borrow_mut().push(deleted);
    }
    fn snapshot(&self) -> Vec<DeletedNote> {
        self.stack.borrow().clone()
    }
    fn find_count(&self) -> usize {
        self.find_count.get()
    }
    fn remove_count(&self) -> usize {
        self.remove_count.get()
    }
}
impl UndoStack for FakeUndo {
    fn push(&self, _d: DeletedNote) {
        unreachable!("restore-deleted-note must not call push")
    }
    fn find_by_id(&self, id: &NoteId) -> Option<DeletedNote> {
        self.find_count.set(self.find_count.get() + 1);
        // Always log: NoUndoAvailable paths still need positive proof that
        // find_by_id was the gating step (review Pass 1 MED-6).
        self.order_log.borrow_mut().push("find");
        self.stack
            .borrow()
            .iter()
            .find(|d| d.id() == id)
            .cloned()
    }
    fn remove_by_id(&self, id: &NoteId) -> Option<DeletedNote> {
        self.remove_count.set(self.remove_count.get() + 1);
        let mut stack = self.stack.borrow_mut();
        if let Some(pos) = stack.iter().position(|d| d.id() == id) {
            let removed = stack.remove(pos);
            self.order_log.borrow_mut().push("remove");
            Some(removed)
        } else {
            None
        }
    }
}

struct RcUndo(Rc<FakeUndo>);
impl UndoStack for RcUndo {
    fn push(&self, d: DeletedNote) {
        self.0.push(d);
    }
    fn find_by_id(&self, id: &NoteId) -> Option<DeletedNote> {
        self.0.find_by_id(id)
    }
    fn remove_by_id(&self, id: &NoteId) -> Option<DeletedNote> {
        self.0.remove_by_id(id)
    }
}

#[derive(Default)]
struct FakeBus {
    events: RefCell<Vec<DomainEvent>>,
    order_log: OrderLog,
}
impl FakeBus {
    fn new(order_log: OrderLog) -> Self {
        Self {
            events: RefCell::new(Vec::new()),
            order_log,
        }
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
        self.order_log.borrow_mut().push("event");
        self.events.borrow_mut().push(event);
    }
}

struct RcBus(Rc<FakeBus>);
impl EventBus for RcBus {
    fn publish(&self, e: DomainEvent) {
        self.0.publish(e);
    }
}

type Rig = (
    RestoreDeletedNoteUseCase<RcRepo, RcTrash, RcUndo, FixedClock, RcBus>,
    Rc<FakeRepo>,
    Rc<FakeTrash>,
    Rc<FakeUndo>,
    Rc<FakeBus>,
    OrderLog,
);

fn rig(now: OffsetDateTime) -> Rig {
    let order_log: OrderLog = Rc::new(RefCell::new(Vec::new()));
    let repo = Rc::new(FakeRepo::new(order_log.clone()));
    let trash = Rc::new(FakeTrash::new(order_log.clone()));
    let undo = Rc::new(FakeUndo::new(order_log.clone()));
    let bus = Rc::new(FakeBus::new(order_log.clone()));
    let uc = RestoreDeletedNoteUseCase::new(
        RcRepo(repo.clone()),
        RcTrash(trash.clone()),
        RcUndo(undo.clone()),
        FixedClock::new(now),
        RcBus(bus.clone()),
    );
    (uc, repo, trash, undo, bus, order_log)
}

fn fixture_note(body: &str, created_at: OffsetDateTime) -> Note {
    Note::create(
        NoteBody::new(body.into()).expect("fixture body must be valid"),
        TagSet::empty(),
        Timestamp::from_offset_datetime(created_at),
    )
}

fn expected_path(id: &NoteId) -> PathBuf {
    PathBuf::from(STORAGE_DIR).join(format!("{}.md", id.as_str()))
}

/// production 経路と同じ aggregate command で DeletedNote を造る。
fn make_deleted(note: Note, path: PathBuf) -> DeletedNote {
    note.delete_to_trash(path)
}

// ===== TP-H*: happy path (spec.md#tp-happy) =====

#[test]
fn tp_h1_undo_in_window_succeeds_with_5_side_effects() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, trash, undo, bus, _log) = rig(now);

    let original = fixture_note("hello world", created);
    let id = original.id().clone();
    let path = expected_path(&id);
    repo.seed(original.clone());
    undo.seed(make_deleted(original.clone(), path.clone()));

    let restored = uc
        .execute(RestoreDeletedNoteCommand {
            note_id: id.clone(),
        })
        .expect("happy path must succeed");

    assert_eq!(trash.restore_count(), 1);
    assert_eq!(trash.last_restored().as_deref(), Some(path.as_path()));
    assert_eq!(repo.load_count(), 1);
    assert_eq!(undo.find_count(), 1);
    assert_eq!(undo.remove_count(), 1);
    assert_eq!(undo.snapshot().len(), 0, "I-RDN7: popped");
    assert_eq!(bus.event_count(), 1);
    assert_eq!(restored.id(), &id);
    assert_eq!(restored.body().as_str(), "hello world");
}

// ===== TP-NU*: NoUndoAvailable (spec.md#tp-no-undo-empty, #tp-no-undo-different-id) =====

#[test]
fn tp_nu1_empty_stack_yields_no_undo_available_with_zero_side_effects() {
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, trash, undo, bus, log) = rig(now);
    let missing = NoteId::from_timestamp(Timestamp::from_offset_datetime(now));

    let err = uc
        .execute(RestoreDeletedNoteCommand {
            note_id: missing.clone(),
        })
        .expect_err("empty stack must error");

    match err {
        RestoreDeletedNoteError::NoUndoAvailable { id } => assert_eq!(id, missing),
        other => panic!("expected NoUndoAvailable, got {other:?}"),
    }
    assert_eq!(trash.restore_count(), 0);
    assert_eq!(repo.load_count(), 0);
    assert_eq!(undo.remove_count(), 0);
    assert_eq!(bus.event_count(), 0);
    // I-RDN1: find_by_id was the gating step (review Pass 1 MED-6 強化)
    let observed: Vec<&'static str> = log.borrow().iter().copied().collect();
    assert_eq!(
        observed,
        vec!["find"],
        "I-RDN1: NoUndoAvailable は find_by_id 後すぐ短絡する"
    );
}

#[test]
fn tp_nu2_different_id_leaves_other_deleted_intact() {
    let created_a = datetime!(2026-06-26 08:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, _trash, undo, bus, _log) = rig(now);

    let note_a = fixture_note("body A", created_a);
    let id_a = note_a.id().clone();
    let path_a = expected_path(&id_a);
    undo.seed(make_deleted(note_a, path_a));

    // Note B never seeded
    let id_b = NoteId::from_timestamp(Timestamp::from_offset_datetime(
        datetime!(2026-06-26 09:00:00 UTC),
    ));

    let err = uc
        .execute(RestoreDeletedNoteCommand {
            note_id: id_b.clone(),
        })
        .expect_err("missing B must error");

    match err {
        RestoreDeletedNoteError::NoUndoAvailable { id } => assert_eq!(id, id_b),
        other => panic!("expected NoUndoAvailable, got {other:?}"),
    }
    let snapshot = undo.snapshot();
    assert_eq!(snapshot.len(), 1, "A must remain (per-toast 独立)");
    assert_eq!(snapshot[0].id(), &id_a);
    assert_eq!(repo.load_count(), 0);
    assert_eq!(bus.event_count(), 0);
}

// ===== TP-TP*: targeted pop (spec.md#tp-stack-targeted-pop, I-RDN7) =====

#[test]
fn tp_tp1_middle_element_pop_preserves_others_in_order() {
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, _trash, undo, _bus, _log) = rig(now);

    let na = fixture_note("a", datetime!(2026-06-26 09:00:00 UTC));
    let nb = fixture_note("b", datetime!(2026-06-26 09:00:01 UTC));
    let nc = fixture_note("c", datetime!(2026-06-26 09:00:02 UTC));
    let id_a = na.id().clone();
    let id_b = nb.id().clone();
    let id_c = nc.id().clone();
    let pa = expected_path(&id_a);
    let pb = expected_path(&id_b);
    let pc = expected_path(&id_c);
    repo.seed(nb.clone()); // B will be reloaded after restore
    undo.seed(make_deleted(na, pa));
    undo.seed(make_deleted(nb, pb));
    undo.seed(make_deleted(nc, pc));

    uc.execute(RestoreDeletedNoteCommand { note_id: id_b.clone() })
        .expect("must succeed");

    let snapshot = undo.snapshot();
    assert_eq!(snapshot.len(), 2);
    assert_eq!(snapshot[0].id(), &id_a, "A retained at index 0");
    assert_eq!(snapshot[1].id(), &id_c, "C retained at index 1");
}

// ===== TP-TR*: TrashRestoreError (spec.md#tp-trash-restore-err, I-RDN3) =====

#[test]
fn tp_tr1_restore_failure_short_circuits_load_remove_event() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, trash, undo, bus, log) = rig(now);

    let original = fixture_note("hi", created);
    let id = original.id().clone();
    let path = expected_path(&id);
    undo.seed(make_deleted(original.clone(), path.clone()));
    repo.seed(original);
    trash.fail_next_restore(TrashErrorKind::PermissionDenied);

    let err = uc
        .execute(RestoreDeletedNoteCommand {
            note_id: id.clone(),
        })
        .expect_err("trash failure must error");

    match err {
        RestoreDeletedNoteError::TrashRestoreError {
            path: p,
            cause: TrashErrorKind::PermissionDenied,
        } => assert_eq!(p, path),
        other => panic!("expected TrashRestoreError::PermissionDenied, got {other:?}"),
    }
    assert_eq!(repo.load_count(), 0, "I-RDN3: load not called");
    assert_eq!(undo.remove_count(), 0, "I-RDN3: remove not called");
    assert_eq!(bus.event_count(), 0, "I-RDN3: event not published");
    let snapshot = undo.snapshot();
    assert_eq!(snapshot.len(), 1, "DeletedNote remains for retry");
    // review Pass 1 MED-5: 同じ DeletedNote (id, path) が残っている事を確認
    assert_eq!(snapshot[0].id(), &id, "retry: same id remains");
    assert_eq!(
        snapshot[0].original_path(),
        path.as_path(),
        "retry: same path remains"
    );
    // review Pass 2 LOW-B: FakeTrash も常時 log 化されたので trash 失敗 path の
    // 順序も pin できる。find → trash(fail) で短絡し load/remove/event 未到達。
    let observed: Vec<&'static str> = log.borrow().iter().copied().collect();
    assert_eq!(
        observed,
        vec!["find", "trash"],
        "I-RDN3: find → trash(fail) で短絡し load/remove/event 未到達"
    );
}

// ===== TP-RE*: ReadError (spec.md#tp-read-err-io, #tp-read-err-ok-none, I-RDN4) =====

#[test]
fn tp_re1_load_io_error_yields_read_error_and_keeps_undo() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, _trash, undo, bus, log) = rig(now);

    let original = fixture_note("hi", created);
    let id = original.id().clone();
    let path = expected_path(&id);
    undo.seed(make_deleted(original, path.clone()));
    repo.fail_load_with.set(Some(io::ErrorKind::InvalidData));

    let err = uc
        .execute(RestoreDeletedNoteCommand {
            note_id: id.clone(),
        })
        .expect_err("io failure must error");

    match err {
        RestoreDeletedNoteError::ReadError { path: p, source } => {
            assert_eq!(p, path);
            assert_eq!(source.kind(), io::ErrorKind::InvalidData);
        }
        other => panic!("expected ReadError, got {other:?}"),
    }
    assert_eq!(undo.remove_count(), 0, "I-RDN4: remove not called");
    assert_eq!(bus.event_count(), 0);
    let snapshot = undo.snapshot();
    assert_eq!(snapshot.len(), 1, "DeletedNote remains for retry");
    assert_eq!(snapshot[0].id(), &id, "retry: same id remains (MED-5)");
    assert_eq!(snapshot[0].original_path(), path.as_path());
    // review Pass 2 LOW-A: MED-7 で load が常時 log 化されたので failure path の
    // 副作用順序も pin できる。find → trash (成功) → load (失敗) で短絡。
    let observed: Vec<&'static str> = log.borrow().iter().copied().collect();
    assert_eq!(
        observed,
        vec!["find", "trash", "load"],
        "I-RDN4: find → trash → load(fail) で短絡し remove/event 未到達"
    );
}

/// spec.md#tp-trash-restore-err + #tp-stack-targeted-pop combined
/// (review Pass 1 MED-8: I-RDN4 ∧ I-RDN7 — 失敗パスでも sibling DeletedNote は影響を受けない)
#[test]
fn tp_tr2_trash_failure_keeps_sibling_deleted_intact() {
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, trash, undo, bus, _log) = rig(now);

    let na = fixture_note("a", datetime!(2026-06-26 09:00:00 UTC));
    let nb = fixture_note("b", datetime!(2026-06-26 09:00:01 UTC));
    let id_a = na.id().clone();
    let id_b = nb.id().clone();
    let pa = expected_path(&id_a);
    let pb = expected_path(&id_b);
    undo.seed(make_deleted(na, pa.clone()));
    undo.seed(make_deleted(nb.clone(), pb.clone()));
    repo.seed(nb);
    trash.fail_next_restore(TrashErrorKind::Io("disk full".into()));

    // Restore B fails → A の sibling は無影響、B は retry 可能で残存
    let _ = uc.execute(RestoreDeletedNoteCommand {
        note_id: id_b.clone(),
    });

    let snapshot = undo.snapshot();
    assert_eq!(snapshot.len(), 2, "both DeletedNotes retained");
    assert_eq!(snapshot[0].id(), &id_a, "A sibling intact at index 0");
    assert_eq!(snapshot[0].original_path(), pa.as_path());
    assert_eq!(snapshot[1].id(), &id_b, "B retained for retry at index 1");
    assert_eq!(snapshot[1].original_path(), pb.as_path());
    assert_eq!(bus.event_count(), 0, "no event on failure");
}

#[test]
fn tp_re2_ok_none_after_restore_collapses_to_read_error() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, _repo, _trash, undo, bus, _log) = rig(now);

    let original = fixture_note("hi", created);
    let id = original.id().clone();
    let path = expected_path(&id);
    undo.seed(make_deleted(original, path.clone()));
    // repo not seeded → load_by_id returns Ok(None)

    let err = uc
        .execute(RestoreDeletedNoteCommand {
            note_id: id.clone(),
        })
        .expect_err("missing reload must error");

    match err {
        RestoreDeletedNoteError::ReadError { path: p, source } => {
            assert_eq!(p, path);
            assert_eq!(
                source.kind(),
                io::ErrorKind::NotFound,
                "Ok(None) collapse → NotFound (oq-read-error-ok-none-policy)"
            );
        }
        other => panic!("expected ReadError, got {other:?}"),
    }
    assert_eq!(undo.remove_count(), 0);
    assert_eq!(bus.event_count(), 0);
}

// ===== TP-SO*: side-effect order (spec.md#tp-side-effect-order, I-RDN5) =====

#[test]
fn tp_so1_order_is_find_trash_load_remove_event() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, _trash, undo, _bus, log) = rig(now);

    let original = fixture_note("hi", created);
    let id = original.id().clone();
    let path = expected_path(&id);
    repo.seed(original.clone());
    undo.seed(make_deleted(original, path));

    uc.execute(RestoreDeletedNoteCommand { note_id: id })
        .expect("must succeed");

    let observed: Vec<&'static str> = log.borrow().iter().copied().collect();
    assert_eq!(
        observed,
        vec!["find", "trash", "load", "remove", "event"],
        "I-RDN5: find → trash → load → remove → event"
    );
}

// ===== TP-EP*: event payload (spec.md#tp-event-payload, I-RDN8) =====

#[test]
fn tp_ep1_event_payload_carries_note_id_and_clock_time() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, _trash, undo, bus, _log) = rig(now);

    let original = fixture_note("hi", created);
    let id = original.id().clone();
    let path = expected_path(&id);
    repo.seed(original.clone());
    undo.seed(make_deleted(original, path));

    uc.execute(RestoreDeletedNoteCommand {
        note_id: id.clone(),
    })
    .expect("must succeed");

    match bus.last().expect("event must be published") {
        DomainEvent::NoteRestoredFromTrash {
            note_id,
            restored_at,
        } => {
            assert_eq!(note_id, id);
            assert_eq!(
                restored_at,
                Timestamp::from_offset_datetime(now),
                "I-RDN8: restored_at from Clock"
            );
        }
        other => panic!("expected NoteRestoredFromTrash, got {other:?}"),
    }
}

// ===== TP-RS*: restored note shape (spec.md#tp-restored-note-shape) =====

#[test]
fn tp_rs1_restored_note_preserves_persisted_shape() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, _trash, undo, _bus, _log) = rig(now);

    let original = fixture_note("preserved body", created);
    let id = original.id().clone();
    let path = expected_path(&id);
    let expected_body = original.body().clone();
    let expected_created = original.created_at();
    repo.seed(original.clone());
    undo.seed(make_deleted(original, path));

    let restored = uc
        .execute(RestoreDeletedNoteCommand {
            note_id: id.clone(),
        })
        .expect("must succeed");

    assert_eq!(restored.id(), &id);
    assert_eq!(restored.body(), &expected_body);
    assert_eq!(restored.created_at(), expected_created);
}

// ===== TP-PD*: path from DeletedNote (spec.md#tp-path-from-deleted-note, I-RDN2) =====

#[test]
fn tp_pd1_trash_restore_uses_deleted_note_original_path_not_storage_dir() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, trash, undo, _bus, _log) = rig(now);

    let original = fixture_note("hi", created);
    let id = original.id().clone();
    // 任意の path (storage_dir とは異なる場所)
    let custom_path = PathBuf::from("/custom/dir").join(format!("{}.md", id.as_str()));
    repo.seed(original.clone());
    undo.seed(make_deleted(original, custom_path.clone()));

    uc.execute(RestoreDeletedNoteCommand { note_id: id })
        .expect("must succeed");

    assert_eq!(
        trash.last_restored().as_deref(),
        Some(custom_path.as_path()),
        "I-RDN2: path from DeletedNote, not storage_dir"
    );
}

// ===== TP-NN*: NoUndoAvailable strictly read-only (spec.md#tp-no-undo-noop) =====

#[test]
fn tp_nn1_no_undo_path_does_not_touch_any_side_effect() {
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, trash, undo, bus, _log) = rig(now);
    let missing = NoteId::from_timestamp(Timestamp::from_offset_datetime(now));

    let _ = uc.execute(RestoreDeletedNoteCommand {
        note_id: missing,
    });

    assert_eq!(trash.restore_count(), 0);
    assert_eq!(repo.load_count(), 0);
    assert_eq!(
        undo.remove_count(),
        0,
        "I-RDN1: remove must not be called when find missed"
    );
    assert_eq!(bus.event_count(), 0);
}

// ===== OQ pinning: oq-duplicate-deleted-note-by-id (spec.md#oq-duplicate-deleted-note-by-id) =====
//
// spec I-RDN9 は「同 NoteId の DeletedNote は同時に存在しない」前提で first-match
// 挙動を未定義 (OQ) としている。本 test は current impl の "first-match (Vec front)"
// 挙動を文書化するもので、invariant 保証ではない (review Pass 1 LOW [9])。
#[test]
#[ignore = "OQ pinning: oq-duplicate-deleted-note-by-id (挙動は未保証、文書化目的)"]
fn tp_oq1_duplicate_note_id_first_match_semantics() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, trash, undo, _bus, _log) = rig(now);

    let original = fixture_note("hi", created);
    let id = original.id().clone();
    let path1 = PathBuf::from("/dir1").join(format!("{}.md", id.as_str()));
    let path2 = PathBuf::from("/dir2").join(format!("{}.md", id.as_str()));
    undo.seed(make_deleted(original.clone(), path1.clone()));
    undo.seed(make_deleted(original, path2.clone()));
    repo.seed(fixture_note("hi", created));

    uc.execute(RestoreDeletedNoteCommand {
        note_id: id.clone(),
    })
    .expect("first-match を restore 対象にする");

    assert_eq!(
        trash.last_restored().as_deref(),
        Some(path1.as_path()),
        "find_by_id は最初に match した DeletedNote (path1) を返す"
    );
    let snapshot = undo.snapshot();
    assert_eq!(snapshot.len(), 1, "remove_by_id は 1 件のみ除去");
    assert_eq!(
        snapshot[0].original_path(),
        path2.as_path(),
        "残存は path2 (後方の要素)"
    );
}

// ===== TP-SIG: signature pin =====

#[test]
fn tp_sig_execute_signature() {
    type ExecuteFn<R, T, U, C, B> = fn(
        &RestoreDeletedNoteUseCase<R, T, U, C, B>,
        RestoreDeletedNoteCommand,
    ) -> Result<Note, RestoreDeletedNoteError>;
    fn assert_signature<
        R: NoteRepository,
        T: TrashService,
        U: UndoStack,
        C: Clock,
        B: EventBus,
    >() {
        let _: ExecuteFn<R, T, U, C, B> = RestoreDeletedNoteUseCase::<R, T, U, C, B>::execute;
    }
    assert_signature::<FakeRepo, FakeTrash, FakeUndo, FixedClock, FakeBus>();
}
