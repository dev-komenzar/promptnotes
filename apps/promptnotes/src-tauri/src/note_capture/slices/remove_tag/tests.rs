//! Tests for slice `remove-tag`.
//!
//! Spec: `.ori/slices/remove-tag/spec.md#test-perspectives`.
//!
//! RED phase: `RemoveTagUseCase::execute` is `unimplemented!()`. Behavioral
//! tests panic; the compile-time signature pin (TP-SIG) passes.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use time::macros::datetime;
use time::OffsetDateTime;

use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, EventBus, NoteRepository};
use crate::note_capture::shared::types::{Note, NoteBody, NoteId, Tag, TagSet, Timestamp};

use super::application::RemoveTagUseCase;
use super::domain::{RemoveTagCommand, RemoveTagError};

const STORAGE_DIR: &str = "/tmp/promptnotes-test";

/// Order log shared between repo + bus spies. Records "write" / "publish".
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
    writes: RefCell<Vec<Note>>,
    fail_write_with: Cell<Option<io::ErrorKind>>,
    fail_load_with: Cell<Option<io::ErrorKind>>,
    storage_dir: PathBuf,
    order_log: OrderLog,
}
impl FakeRepo {
    fn new(order_log: OrderLog) -> Self {
        Self {
            notes: RefCell::new(HashMap::new()),
            writes: RefCell::new(Vec::new()),
            fail_write_with: Cell::new(None),
            fail_load_with: Cell::new(None),
            storage_dir: PathBuf::from(STORAGE_DIR),
            order_log,
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
    fn last_written(&self) -> Option<Note> {
        self.writes.borrow().last().cloned()
    }
}
impl NoteRepository for FakeRepo {
    fn write(&self, note: &Note) -> io::Result<()> {
        if let Some(kind) = self.fail_write_with.take() {
            return Err(io::Error::new(kind, "fake repo failure"));
        }
        self.order_log.borrow_mut().push("write");
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
        self.order_log.borrow_mut().push("publish");
        self.events.borrow_mut().push(event);
    }
}

type Rig = (
    RemoveTagUseCase<Rc<FakeRepo>, FixedClock, Rc<FakeBus>>,
    Rc<FakeRepo>,
    Rc<FakeBus>,
    OrderLog,
);

fn rig(now: OffsetDateTime) -> Rig {
    let order_log: OrderLog = Rc::new(RefCell::new(Vec::new()));
    let repo = Rc::new(FakeRepo::new(order_log.clone()));
    let bus = Rc::new(FakeBus::new(order_log.clone()));
    let uc = RemoveTagUseCase::new(repo.clone(), FixedClock::new(now), bus.clone());
    (uc, repo, bus, order_log)
}

fn fixture_note(body: &str, tags: &[&str], created_at: OffsetDateTime) -> Note {
    let tag_set = TagSet::from_tags(
        tags.iter()
            .map(|n| Tag::new(n).expect("fixture tag must be valid")),
    );
    Note::create(
        NoteBody::new(body.into()).expect("fixture body must be valid"),
        tag_set,
        Timestamp::from_offset_datetime(created_at),
    )
}

fn tag_names(set: &TagSet) -> Vec<String> {
    set.as_slice()
        .iter()
        .map(|t| t.name().to_string())
        .collect()
}

// ===== TP-H*: happy path (spec.md#tp-happy) =====

/// spec.md#tp-happy TP-H1
#[test]
fn tp_h1_existing_tag_removed_with_write_and_publish() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, bus, _log) = rig(now);
    let seed = fixture_note("body", &["rust", "memo"], created);
    let id = seed.id().clone();
    repo.seed(seed);

    let updated = uc
        .execute(RemoveTagCommand {
            note_id: id.clone(),
            tag_name: "rust".to_string(),
        })
        .expect("happy path must succeed")
        .expect("happy path returns Some(Note)");

    assert_eq!(tag_names(updated.tags()), vec!["memo"]);
    assert_eq!(repo.write_count(), 1);
    assert_eq!(bus.event_count(), 1);
    assert_eq!(updated.updated_at(), Timestamp::from_offset_datetime(now));
}

/// spec.md#tp-last-tag TP-LT1
#[test]
fn tp_lt1_removing_last_tag_empties_tagset() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, bus, _log) = rig(now);
    let seed = fixture_note("body", &["rust"], created);
    let id = seed.id().clone();
    repo.seed(seed);

    let updated = uc
        .execute(RemoveTagCommand {
            note_id: id,
            tag_name: "rust".to_string(),
        })
        .expect("must succeed")
        .expect("must be Some");

    assert!(updated.tags().as_slice().is_empty());
    match bus.last().expect("event must be published") {
        DomainEvent::NoteTagsChanged { tags, .. } => {
            assert!(tags.as_slice().is_empty(), "event payload tags also empty");
        }
        other => panic!("expected NoteTagsChanged, got {other:?}"),
    }
}

/// spec.md#tp-order-preserved TP-OP1
#[test]
fn tp_op1_middle_tag_removed_preserves_order() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, _bus, _log) = rig(now);
    let seed = fixture_note("body", &["a", "b", "c"], created);
    let id = seed.id().clone();
    repo.seed(seed);

    let updated = uc
        .execute(RemoveTagCommand {
            note_id: id,
            tag_name: "b".to_string(),
        })
        .expect("must succeed")
        .expect("must be Some");

    assert_eq!(tag_names(updated.tags()), vec!["a", "c"]);
}

// ===== TP-NM*: no-op (missing) (spec.md#tp-noop-missing) =====

/// spec.md#tp-noop-missing TP-NM1
#[test]
fn tp_nm1_missing_tag_is_noop_no_write_no_event() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, bus, _log) = rig(now);
    let seed = fixture_note("body", &["rust"], created);
    let id = seed.id().clone();
    repo.seed(seed);

    let result = uc
        .execute(RemoveTagCommand {
            note_id: id,
            tag_name: "python".to_string(),
        })
        .expect("must succeed");

    assert!(result.is_none(), "I-RT2: missing tag returns Ok(None)");
    assert_eq!(repo.write_count(), 0, "I-RT2: no write");
    assert_eq!(bus.event_count(), 0, "I-RT2: no event");
}

// ===== TP-NE*: no-op (empty tagset) (spec.md#tp-noop-empty-tagset) =====

/// spec.md#tp-noop-empty-tagset TP-NE1
#[test]
fn tp_ne1_empty_tagset_yields_noop() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, bus, _log) = rig(now);
    let seed = fixture_note("body", &[], created);
    let id = seed.id().clone();
    repo.seed(seed);

    let result = uc
        .execute(RemoveTagCommand {
            note_id: id,
            tag_name: "anything".to_string(),
        })
        .expect("must succeed");

    assert!(result.is_none());
    assert_eq!(repo.write_count(), 0);
    assert_eq!(bus.event_count(), 0);
}

// ===== TP-NU*: no-op (unnormalized input) (spec.md#tp-noop-unnormalized) =====

/// spec.md#tp-noop-unnormalized TP-NU1
#[test]
fn tp_nu1_whitespace_and_case_diff_yields_noop() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, bus, _log) = rig(now);
    let seed = fixture_note("body", &["rust"], created);
    let id = seed.id().clone();
    repo.seed(seed);

    let result = uc
        .execute(RemoveTagCommand {
            note_id: id,
            tag_name: " RUST ".to_string(),
        })
        .expect("must succeed");

    assert!(
        result.is_none(),
        "I-RT1: slice does not re-normalize, so unnormalized input is treated as missing"
    );
    assert_eq!(repo.write_count(), 0);
    assert_eq!(bus.event_count(), 0);
}

/// spec.md#tp-noop-missing TP-NM2 — pure empty string `tag_name` (review Pass 1 MED-F)
///
/// `tag_name = ""` は I-RT1 の「正規化済み前提」契約の境界ケース。
/// `Tag::name` は空文字を含み得ない (I-N6) ため必ず no-op になる事を pin する。
#[test]
fn tp_nm2_empty_string_tag_name_yields_noop() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, bus, _log) = rig(now);
    let seed = fixture_note("body", &["rust"], created);
    let id = seed.id().clone();
    repo.seed(seed);

    let result = uc
        .execute(RemoveTagCommand {
            note_id: id,
            tag_name: String::new(),
        })
        .expect("must succeed");

    assert!(
        result.is_none(),
        "I-RT1 + I-N6: empty tag_name can never match a Tag::name"
    );
    assert_eq!(repo.write_count(), 0);
    assert_eq!(bus.event_count(), 0);
}

// ===== TP-NC*: no-op (case-sensitive) (spec.md#tp-noop-case-sensitive) =====

/// spec.md#tp-noop-case-sensitive TP-NC1
#[test]
fn tp_nc1_case_mismatch_yields_noop() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, bus, _log) = rig(now);
    let seed = fixture_note("body", &["rust"], created);
    let id = seed.id().clone();
    repo.seed(seed);

    let result = uc
        .execute(RemoveTagCommand {
            note_id: id,
            tag_name: "Rust".to_string(),
        })
        .expect("must succeed");

    assert!(result.is_none(), "I-RT1: case-sensitive match");
    assert_eq!(repo.write_count(), 0);
    assert_eq!(bus.event_count(), 0);
}

// ===== TP-NF*: NoteNotFound (spec.md#tp-not-found, I-RT5) =====

/// spec.md#tp-not-found TP-NF1
#[test]
fn tp_nf1_missing_note_id_yields_note_not_found() {
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, bus, _log) = rig(now);
    let missing = NoteId::from_timestamp(Timestamp::from_offset_datetime(now));

    let err = uc
        .execute(RemoveTagCommand {
            note_id: missing.clone(),
            tag_name: "rust".to_string(),
        })
        .expect_err("missing note must be an error");

    match err {
        RemoveTagError::NoteNotFound { id } => assert_eq!(id, missing),
        other => panic!("expected NoteNotFound, got {other:?}"),
    }
    assert_eq!(repo.write_count(), 0, "I-RT5: no write");
    assert_eq!(bus.event_count(), 0, "I-RT5: no event");
}

// ===== TP-LE*: LoadError (spec.md#tp-load-err, I-RT5, I-RT8) =====

/// spec.md#tp-load-err TP-LE1
#[test]
fn tp_le1_load_io_error_yields_load_error() {
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, bus, _log) = rig(now);
    let id = NoteId::from_timestamp(Timestamp::from_offset_datetime(now));
    repo.fail_next_load(io::ErrorKind::PermissionDenied);

    let err = uc
        .execute(RemoveTagCommand {
            note_id: id.clone(),
            tag_name: "rust".to_string(),
        })
        .expect_err("load io error must surface");

    match err {
        RemoveTagError::LoadError { path, source } => {
            assert_eq!(
                path,
                PathBuf::from(STORAGE_DIR).join(format!("{}.md", id.as_str())),
                "I-RT8: path is derived from storage_dir + id.md"
            );
            assert_eq!(source.kind(), io::ErrorKind::PermissionDenied);
        }
        other => panic!("expected LoadError, got {other:?}"),
    }
    assert_eq!(repo.write_count(), 0);
    assert_eq!(bus.event_count(), 0);
}

// ===== TP-PE*: PersistError (spec.md#tp-persist-err, I-RT4) =====

/// spec.md#tp-persist-err TP-PE1
#[test]
fn tp_pe1_write_io_error_blocks_publish() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, bus, _log) = rig(now);
    let seed = fixture_note("body", &["rust"], created);
    let id = seed.id().clone();
    repo.seed(seed);
    repo.fail_next_write(io::ErrorKind::Other);

    let err = uc
        .execute(RemoveTagCommand {
            note_id: id.clone(),
            tag_name: "rust".to_string(),
        })
        .expect_err("persist failure must surface");

    match err {
        RemoveTagError::PersistError { path, source } => {
            assert_eq!(
                path,
                PathBuf::from(STORAGE_DIR).join(format!("{}.md", id.as_str()))
            );
            assert_eq!(source.kind(), io::ErrorKind::Other);
        }
        other => panic!("expected PersistError, got {other:?}"),
    }
    assert_eq!(
        bus.event_count(),
        0,
        "I-RT4: event must not be published on persist failure"
    );
}

// ===== TP-SO*: side-effect order (spec.md#tp-side-effect-order, I-RT3) =====

/// spec.md#tp-side-effect-order TP-SO1
#[test]
fn tp_so1_write_then_publish_order_on_happy_path() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, _bus, log) = rig(now);
    let seed = fixture_note("body", &["rust"], created);
    let id = seed.id().clone();
    repo.seed(seed);

    uc.execute(RemoveTagCommand {
        note_id: id,
        tag_name: "rust".to_string(),
    })
    .expect("must succeed");

    let observed: Vec<&'static str> = log.borrow().iter().copied().collect();
    assert_eq!(
        observed,
        vec!["write", "publish"],
        "I-RT3: write before publish"
    );
}

// ===== TP-EP*: event payload after removal (spec.md#tp-event-payload-after-remove) =====

/// spec.md#tp-event-payload-after-remove TP-EP1
#[test]
fn tp_ep1_event_payload_is_post_removal_tagset() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, bus, _log) = rig(now);
    let seed = fixture_note("body", &["a", "b"], created);
    let id = seed.id().clone();
    repo.seed(seed);

    let updated = uc
        .execute(RemoveTagCommand {
            note_id: id.clone(),
            tag_name: "a".to_string(),
        })
        .expect("must succeed")
        .expect("must be Some");

    match bus.last().expect("event must be published") {
        DomainEvent::NoteTagsChanged {
            note_id,
            tags,
            updated_at,
        } => {
            assert_eq!(note_id, id);
            assert_eq!(tag_names(&tags), vec!["b"], "I-RT7: post-removal TagSet");
            assert_eq!(updated_at, updated.updated_at());
        }
        other => panic!("expected NoteTagsChanged, got {other:?}"),
    }
}

// ===== TP-IM*: immutability (spec.md#tp-immutability, I-RT6) =====

/// spec.md#tp-immutability TP-IM1
#[test]
fn tp_im1_id_body_created_at_unchanged() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, _bus, _log) = rig(now);
    let seed = fixture_note("body content", &["rust"], created);
    let original_id = seed.id().clone();
    let original_body = seed.body().clone();
    let original_created = seed.created_at();
    repo.seed(seed);

    uc.execute(RemoveTagCommand {
        note_id: original_id.clone(),
        tag_name: "rust".to_string(),
    })
    .expect("must succeed");

    let written = repo.last_written().expect("write must have happened");
    assert_eq!(written.id(), &original_id);
    assert_eq!(written.body(), &original_body);
    assert_eq!(written.created_at(), original_created);
}

// ===== TP-NU*: no event for unchanged (spec.md#tp-no-event-unchanged, I-RT2) =====

/// spec.md#tp-no-event-unchanged TP-EU1
#[test]
fn tp_eu1_unchanged_path_publishes_no_event() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, bus, _log) = rig(now);
    let seed = fixture_note("body", &["rust"], created);
    let id = seed.id().clone();
    repo.seed(seed);

    let _ = uc.execute(RemoveTagCommand {
        note_id: id,
        tag_name: "doesnotexist".to_string(),
    });

    assert_eq!(bus.event_count(), 0, "I-RT2: no event for no-op");
}

// ===== TP-AI*: aggregate invariants maintained (spec.md#tp-aggregate-invariants) =====

/// spec.md#tp-aggregate-invariants TP-AI1
#[test]
fn tp_ai1_post_removal_tagset_keeps_aggregate_invariants() {
    let created = datetime!(2026-06-26 09:00:00 UTC);
    let now = datetime!(2026-06-26 10:00:00 UTC);
    let (uc, repo, _bus, _log) = rig(now);
    let seed = fixture_note("body", &["rust", "memo", "draft"], created);
    let id = seed.id().clone();
    repo.seed(seed);

    let updated = uc
        .execute(RemoveTagCommand {
            note_id: id,
            tag_name: "memo".to_string(),
        })
        .expect("must succeed")
        .expect("must be Some");

    let names = tag_names(updated.tags());
    // I-N5: uniqueness
    let mut seen = std::collections::HashSet::new();
    for n in &names {
        assert!(seen.insert(n.clone()), "I-N5: duplicate detected: {n}");
    }
    // I-N6: each remaining name is non-empty and free of forbidden chars
    for n in &names {
        assert!(!n.is_empty(), "I-N6: empty tag name");
        assert!(
            !n.contains(' ')
                && !n.contains('\t')
                && !n.contains('\n')
                && !n.contains(',')
                && !n.contains('[')
                && !n.contains(']'),
            "I-N6: forbidden char in {n}"
        );
    }
}

// ===== TP-SIG: signature pin =====

/// spec.md#io-output TP-SIG — execute returns `Result<Option<Note>, RemoveTagError>`.
#[test]
fn tp_sig_execute_signature() {
    type ExecuteFn<R, C, E> =
        fn(&RemoveTagUseCase<R, C, E>, RemoveTagCommand) -> Result<Option<Note>, RemoveTagError>;
    fn assert_signature<R: NoteRepository, C: Clock, E: EventBus>() {
        let _: ExecuteFn<R, C, E> = RemoveTagUseCase::<R, C, E>::execute;
    }
    assert_signature::<FakeRepo, FixedClock, FakeBus>();
}
