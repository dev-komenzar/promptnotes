//! Tests for slice `delete-note`.
//!
//! Spec: `.ori/slices/delete-note/spec.md#test-perspectives`.
//!
//! Review Pass 1 follow-ups:
//! - H-1 / M-1 / M-2 解消後の構成。aggregate `Note::delete_to_trash` を経由し、
//!   `NoteRepository::storage_dir()` で path を解決する形に揃える。
//! - M-4: TP-TE2 に `path == expected` 検証を追加し I-DN1 を pin。

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
    DeletedNote, Note, NoteBody, NoteId, Tag, TagSet, Timestamp,
};

use super::application::DeleteNoteUseCase;
use super::domain::{DeleteNoteCommand, DeleteNoteError};
use super::ports::{TrashErrorKind, TrashService, UndoStack};

const STORAGE_DIR: &str = "/tmp/promptnotes-test";

// ===== shared spy: side-effect order =====

/// 副作用順序を spy するための共通 log。値は "trash" / "push" / "event" のいずれか。
type OrderLog = Rc<RefCell<Vec<&'static str>>>;

// ===== test doubles =====

#[derive(Default)]
struct FakeRepo {
    notes: RefCell<HashMap<String, Note>>,
    writes: RefCell<Vec<Note>>,
    fail_load_with: Cell<Option<io::ErrorKind>>,
    storage_dir: PathBuf,
}
impl FakeRepo {
    fn new() -> Self {
        Self {
            notes: RefCell::new(HashMap::new()),
            writes: RefCell::new(Vec::new()),
            fail_load_with: Cell::new(None),
            storage_dir: PathBuf::from(STORAGE_DIR),
        }
    }
    fn seed(&self, note: Note) {
        self.notes
            .borrow_mut()
            .insert(note.id().as_str().to_string(), note);
    }
    fn write_count(&self) -> usize {
        self.writes.borrow().len()
    }
}
impl NoteRepository for FakeRepo {
    fn write(&self, note: &Note) -> io::Result<()> {
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
    moves: RefCell<Vec<PathBuf>>,
    fail_next_with: Cell<Option<TrashErrorKind>>,
    order_log: OrderLog,
}
impl FakeTrash {
    fn new(order_log: OrderLog) -> Self {
        Self {
            moves: RefCell::new(Vec::new()),
            fail_next_with: Cell::new(None),
            order_log,
        }
    }
    fn fail_next_with(&self, kind: TrashErrorKind) {
        self.fail_next_with.set(Some(kind));
    }
    fn move_count(&self) -> usize {
        self.moves.borrow().len()
    }
    fn last_path(&self) -> Option<PathBuf> {
        self.moves.borrow().last().cloned()
    }
}
impl TrashService for FakeTrash {
    fn move_to_trash(&self, path: &Path) -> Result<(), TrashErrorKind> {
        if let Some(kind) = self.fail_next_with.take() {
            return Err(kind);
        }
        self.order_log.borrow_mut().push("trash");
        self.moves.borrow_mut().push(path.to_path_buf());
        Ok(())
    }
    fn restore_from_trash(&self, _path: &Path) -> Result<(), TrashErrorKind> {
        // restore-deleted-note slice の責務。delete-note tests では未使用なので
        // panic させて誤呼出を検出する。
        unreachable!("delete-note slice must not call restore_from_trash")
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
    order_log: OrderLog,
}
impl FakeUndo {
    fn new(order_log: OrderLog) -> Self {
        Self {
            stack: RefCell::new(Vec::new()),
            order_log,
        }
    }
    fn seed(&self, deleted: DeletedNote) {
        self.stack.borrow_mut().push(deleted);
    }
    fn snapshot(&self) -> Vec<DeletedNote> {
        self.stack.borrow().clone()
    }
}
impl UndoStack for FakeUndo {
    fn push(&self, deleted: DeletedNote) {
        self.order_log.borrow_mut().push("push");
        self.stack.borrow_mut().push(deleted);
    }
    fn find_by_id(&self, id: &NoteId) -> Option<DeletedNote> {
        self.stack
            .borrow()
            .iter()
            .find(|d| d.id() == id)
            .cloned()
    }
    fn remove_by_id(&self, _id: &NoteId) -> Option<DeletedNote> {
        unreachable!("delete-note slice must not call remove_by_id")
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
    DeleteNoteUseCase<RcRepo, RcTrash, RcUndo, FixedClock, RcBus>,
    Rc<FakeRepo>,
    Rc<FakeTrash>,
    Rc<FakeUndo>,
    Rc<FakeBus>,
    OrderLog,
);

fn rig(now: OffsetDateTime) -> Rig {
    let order_log: OrderLog = Rc::new(RefCell::new(Vec::new()));
    let repo = Rc::new(FakeRepo::new());
    let trash = Rc::new(FakeTrash::new(order_log.clone()));
    let undo = Rc::new(FakeUndo::new(order_log.clone()));
    let clock = FixedClock::new(now);
    let bus = Rc::new(FakeBus::new(order_log.clone()));
    let uc = DeleteNoteUseCase::new(
        RcRepo(repo.clone()),
        RcTrash(trash.clone()),
        RcUndo(undo.clone()),
        clock,
        RcBus(bus.clone()),
    );
    (uc, repo, trash, undo, bus, order_log)
}

fn fixture_note(body: &str, created_at: OffsetDateTime) -> Note {
    Note::create(
        NoteBody::new(body.into()).expect("test fixture body must be valid"),
        TagSet::empty(),
        Timestamp::from_offset_datetime(created_at),
    )
}

fn fixture_note_with_tags(body: &str, tag_names: &[&str], created_at: OffsetDateTime) -> Note {
    let tags = TagSet::from_tags(
        tag_names
            .iter()
            .map(|n| Tag::new(n).expect("test fixture tag must be valid")),
    );
    Note::create(
        NoteBody::new(body.into()).expect("test fixture body must be valid"),
        tags,
        Timestamp::from_offset_datetime(created_at),
    )
}

fn expected_path(id: &NoteId) -> PathBuf {
    PathBuf::from(STORAGE_DIR).join(format!("{}.md", id.as_str()))
}

// ===== TP-H*: happy path (spec.md#tp-happy) =====

/// spec.md#tp-happy TP-H1 — 3 副作用 + Ok(DeletedNote)
#[test]
fn tp_h1_happy_path_trash_push_event() {
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, trash, undo, bus, _log) = rig(now);
    let seed = fixture_note("hello world", now);
    let id = seed.id().clone();
    let expected = expected_path(&id);
    repo.seed(seed);

    let deleted = uc
        .execute(DeleteNoteCommand {
            note_id: id.clone(),
        })
        .expect("happy path must succeed");

    assert_eq!(trash.move_count(), 1, "trash called exactly once");
    assert_eq!(trash.last_path().as_deref(), Some(expected.as_path()));
    assert_eq!(undo.snapshot().len(), 1, "undo stack push exactly once");
    assert_eq!(bus.event_count(), 1, "event published exactly once");
    assert_eq!(deleted.id(), &id);
    assert_eq!(deleted.original_path(), expected.as_path());
}

// ===== TP-WT*: tagged note (spec.md#tp-with-tags) =====

/// spec.md#tp-with-tags TP-WT1 — tags の有無は副作用順序に影響しない
#[test]
fn tp_wt1_tagged_note_same_side_effect_shape() {
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, trash, undo, bus, _log) = rig(now);
    let seed = fixture_note_with_tags("body", &["rust", "memo"], now);
    let id = seed.id().clone();
    let expected = expected_path(&id);
    repo.seed(seed);

    uc.execute(DeleteNoteCommand {
        note_id: id.clone(),
    })
    .expect("tagged note delete must succeed");

    assert_eq!(trash.move_count(), 1);
    assert_eq!(trash.last_path(), Some(expected.clone()));
    assert_eq!(undo.snapshot().len(), 1);
    assert_eq!(bus.event_count(), 1);
}

// ===== TP-NF*: NoteNotFound (spec.md#tp-not-found, I-DN3) =====

/// spec.md#tp-not-found TP-NF1
#[test]
fn tp_nf1_missing_note_id_yields_note_not_found() {
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, _repo, _trash, _undo, _bus, _log) = rig(now);
    let missing = NoteId::from_timestamp(Timestamp::from_offset_datetime(now));

    let err = uc
        .execute(DeleteNoteCommand {
            note_id: missing.clone(),
        })
        .expect_err("missing id must be an error");

    match err {
        DeleteNoteError::NoteNotFound { id } => assert_eq!(id, missing),
        other => panic!("expected NoteNotFound, got {other:?}"),
    }
}

/// spec.md#tp-not-found TP-NF2 — I-DN3: 3 副作用いずれも未呼出
#[test]
fn tp_nf2_missing_note_touches_no_side_effects() {
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, _repo, trash, undo, bus, _log) = rig(now);
    let missing = NoteId::from_timestamp(Timestamp::from_offset_datetime(now));

    let _ = uc.execute(DeleteNoteCommand { note_id: missing });

    assert_eq!(trash.move_count(), 0, "I-DN3: trash untouched");
    assert_eq!(undo.snapshot().len(), 0, "I-DN3: undo untouched");
    assert_eq!(bus.event_count(), 0, "I-DN3: event bus untouched");
}

/// spec.md#tp-repo-io-err-collapse TP-NF3 — I-DN6 意図的選択 pin
#[test]
fn tp_nf3_repo_io_error_collapses_to_note_not_found_with_no_side_effects() {
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, trash, undo, bus, _log) = rig(now);
    let id = NoteId::from_timestamp(Timestamp::from_offset_datetime(now));
    repo.fail_load_with
        .set(Some(io::ErrorKind::PermissionDenied));

    let err = uc
        .execute(DeleteNoteCommand {
            note_id: id.clone(),
        })
        .expect_err("load io error must surface as an error");

    match err {
        DeleteNoteError::NoteNotFound { id: returned } => assert_eq!(returned, id),
        other => panic!("I-DN6: io::Err must collapse to NoteNotFound, got {other:?}"),
    }
    assert_eq!(trash.move_count(), 0, "I-DN3 + I-DN6: trash untouched");
    assert_eq!(undo.snapshot().len(), 0, "I-DN3 + I-DN6: undo untouched");
    assert_eq!(bus.event_count(), 0, "I-DN3 + I-DN6: bus untouched");
}

// ===== TP-TE*: TrashError (spec.md#tp-trash-err, I-DN4) =====

/// spec.md#tp-trash-err TP-TE1 — TrashError 伝播 + push/event 未呼出
#[test]
fn tp_te1_trash_error_propagates_and_blocks_push_event() {
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, trash, undo, bus, _log) = rig(now);
    let seed = fixture_note("hello", now);
    let id = seed.id().clone();
    let expected = expected_path(&id);
    repo.seed(seed);
    trash.fail_next_with(TrashErrorKind::PermissionDenied);

    let err = uc
        .execute(DeleteNoteCommand {
            note_id: id.clone(),
        })
        .expect_err("trash failure must surface");

    match err {
        DeleteNoteError::TrashError {
            path,
            cause: TrashErrorKind::PermissionDenied,
        } => assert_eq!(path, expected),
        other => panic!("expected TrashError::PermissionDenied, got {other:?}"),
    }
    assert_eq!(undo.snapshot().len(), 0, "I-DN4: undo not touched");
    assert_eq!(bus.event_count(), 0, "I-DN4: event not published");
}

/// spec.md#tp-trash-err TP-TE2 — Io variant も同様の路線 + I-DN1 path pin (review M-4)
#[test]
fn tp_te2_trash_io_error_preserves_cause_and_path() {
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, trash, _undo, _bus, _log) = rig(now);
    let seed = fixture_note("hello", now);
    let id = seed.id().clone();
    let expected = expected_path(&id);
    repo.seed(seed);
    trash.fail_next_with(TrashErrorKind::Io("nfs unavailable".into()));

    let err = uc
        .execute(DeleteNoteCommand { note_id: id })
        .expect_err("trash io failure must surface");

    match err {
        DeleteNoteError::TrashError {
            path,
            cause: TrashErrorKind::Io(msg),
        } => {
            assert_eq!(path, expected, "I-DN1: error carries derived path");
            assert_eq!(msg, "nfs unavailable");
        }
        other => panic!("expected TrashError::Io, got {other:?}"),
    }
}

// ===== TP-SO*: side-effect order (spec.md#tp-side-effect-order, I-DN5) =====

/// spec.md#tp-side-effect-order TP-SO1 — trash → push → event の順序
#[test]
fn tp_so1_side_effect_order_is_trash_then_push_then_event() {
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, _trash, _undo, _bus, log) = rig(now);
    let seed = fixture_note("hello", now);
    let id = seed.id().clone();
    repo.seed(seed);

    uc.execute(DeleteNoteCommand { note_id: id })
        .expect("happy path must succeed");

    let observed: Vec<&'static str> = log.borrow().iter().copied().collect();
    assert_eq!(
        observed,
        vec!["trash", "push", "event"],
        "I-DN5: order must be trash → push → event"
    );
}

// ===== TP-EP*: event payload (spec.md#tp-event-payload) =====

/// spec.md#tp-event-payload TP-EP1
#[test]
fn tp_ep1_event_payload_matches_input_path_and_clock() {
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, _trash, _undo, bus, _log) = rig(now);
    let seed = fixture_note("hello", now);
    let id = seed.id().clone();
    let expected_p = expected_path(&id);
    let expected_ts = Timestamp::from_offset_datetime(now);
    repo.seed(seed);

    uc.execute(DeleteNoteCommand {
        note_id: id.clone(),
    })
    .expect("happy path must succeed");

    match bus.last().expect("an event must be published") {
        DomainEvent::NoteDeletedToTrash {
            note_id,
            original_path,
            deleted_at,
        } => {
            assert_eq!(note_id, id);
            assert_eq!(original_path, expected_p);
            assert_eq!(deleted_at, expected_ts);
        }
        other => panic!("expected NoteDeletedToTrash, got {other:?}"),
    }
}

// ===== TP-SA*: stack accumulate (spec.md#tp-stack-accumulate, I-DN8) =====

/// spec.md#tp-stack-accumulate TP-SA1
///
/// 事前に積まれた `DeletedNote(A)` を破壊せず、B の delete で stack が
/// `[A, B]` に伸びる事を確認する (I-DN8)。事前 seed は `Note::delete_to_trash`
/// 経由で生成し、production 経路と同じ construction site を使う。
#[test]
fn tp_sa1_existing_undo_stack_is_not_destroyed() {
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, _trash, undo, _bus, _log) = rig(now);

    // 事前に Note A を aggregate 経由で DeletedNote 化して seed (production と同じ経路)
    let now_a = datetime!(2026-06-25 09:00:00 UTC);
    let note_a = fixture_note("body A", now_a);
    let expected_a = expected_path(note_a.id());
    let deleted_a = note_a.delete_to_trash(expected_a.clone());
    let deleted_a_clone = deleted_a.clone();
    undo.seed(deleted_a);

    // B を本 slice 経由で削除
    let seed = fixture_note("body B", now);
    let id_b = seed.id().clone();
    let expected_b = expected_path(&id_b);
    repo.seed(seed);

    uc.execute(DeleteNoteCommand {
        note_id: id_b.clone(),
    })
    .expect("delete B must succeed");

    let snapshot = undo.snapshot();
    assert_eq!(snapshot.len(), 2, "I-DN8: stack accumulates, does not pop");
    assert_eq!(snapshot[0], deleted_a_clone, "A must remain at bottom unchanged");
    assert_eq!(snapshot[1].id(), &id_b, "B pushed on top with matching id");
    assert_eq!(snapshot[1].original_path(), expected_b.as_path());
}

// ===== TP-DS*: DeletedNote shape (spec.md#tp-deleted-note-shape, I-DN7) =====

/// spec.md#tp-deleted-note-shape TP-DS1 — id / path 一致 (review M-1)
///
/// I-DN7 は「DeletedNote.id は loaded Note の id と一致する」事を要求する。
/// `DeletedNote::new` を `pub(crate)` に固定し `Note::delete_to_trash` のみ
/// 構築可能としているため、id ↔ loaded Note の対応は型システムで担保されている。
/// 本 test は表面値が input と一致する事を確認する (production wiring 確認の意味)。
#[test]
fn tp_ds1_deleted_note_id_and_path_match_input_and_storage_dir() {
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, _trash, undo, _bus, _log) = rig(now);
    let seed = fixture_note("hello", now);
    let id = seed.id().clone();
    let expected_p = expected_path(&id);
    repo.seed(seed);

    let deleted = uc
        .execute(DeleteNoteCommand {
            note_id: id.clone(),
        })
        .expect("must succeed");

    assert_eq!(deleted.id(), &id, "I-DN7: returned id matches input");
    assert_eq!(
        deleted.original_path(),
        expected_p.as_path(),
        "I-DN7: returned path == storage_dir/<id>.md"
    );

    let pushed = undo.snapshot();
    assert_eq!(pushed.len(), 1);
    assert_eq!(pushed[0].id(), &id, "I-DN7: pushed id matches");
    assert_eq!(
        pushed[0].original_path(),
        expected_p.as_path(),
        "I-DN7: pushed path matches"
    );
}

// ===== TP-TO*: trash-only path (spec.md#tp-trash-only, I-DN2) =====

/// spec.md#tp-trash-only TP-TO1 — repository write 系が呼ばれない
#[test]
fn tp_to1_repo_write_not_called_during_delete() {
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, _trash, _undo, _bus, _log) = rig(now);
    let seed = fixture_note("hello", now);
    let id = seed.id().clone();
    repo.seed(seed);

    uc.execute(DeleteNoteCommand { note_id: id })
        .expect("must succeed");

    assert_eq!(
        repo.write_count(),
        0,
        "I-DN2 + read-only: NoteRepository::write must NOT be called"
    );
}

// ===== TP-AS*: type-level API surface =====

/// spec.md#io-output TP-AS1 — execute returns `Result<DeletedNote, DeleteNoteError>`.
#[test]
fn tp_as1_execute_signature_returns_result_deleted_note() {
    type ExecuteFn<R, T, U, C, B> = fn(
        &DeleteNoteUseCase<R, T, U, C, B>,
        DeleteNoteCommand,
    ) -> Result<DeletedNote, DeleteNoteError>;
    fn assert_signature<
        R: NoteRepository,
        T: TrashService,
        U: UndoStack,
        C: Clock,
        B: EventBus,
    >() {
        let _: ExecuteFn<R, T, U, C, B> = DeleteNoteUseCase::<R, T, U, C, B>::execute;
    }
    assert_signature::<FakeRepo, FakeTrash, FakeUndo, FixedClock, FakeBus>();
}
