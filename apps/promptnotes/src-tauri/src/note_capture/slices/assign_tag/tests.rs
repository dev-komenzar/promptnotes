//! Tests for slice `assign-tag`.
//!
//! Spec: `.ori/slices/assign-tag/spec.md#test-perspectives`.
//!
//! RED phase: `AssignTagUseCase::execute` is `unimplemented!()`. Tests that
//! exercise behaviour panic; the compile-time signature pin (TP-AS1) is
//! allowed to pass — it only verifies the public surface, not behaviour.

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
    Note, NoteBody, NoteId, Tag, TagError, TagSet, Timestamp,
};

use super::application::AssignTagUseCase;
use super::domain::{AssignTagCommand, AssignTagError};

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
    load_calls: Cell<usize>,
    storage_dir: PathBuf,
}
impl FakeRepo {
    fn new() -> Self {
        Self {
            notes: RefCell::new(HashMap::new()),
            writes: RefCell::new(Vec::new()),
            fail_write_with: Cell::new(None),
            fail_load_with: Cell::new(None),
            load_calls: Cell::new(0),
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
    fn load_count(&self) -> usize {
        self.load_calls.get()
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
        self.load_calls.set(self.load_calls.get() + 1);
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

struct RcBus(Rc<FakeBus>);
impl EventBus for RcBus {
    fn publish(&self, e: DomainEvent) {
        self.0.publish(e);
    }
}

type Rig = (
    AssignTagUseCase<RcRepo, FixedClock, RcBus>,
    Rc<FakeRepo>,
    Rc<FakeBus>,
);

fn rig(now: OffsetDateTime) -> Rig {
    let repo = Rc::new(FakeRepo::new());
    let bus = Rc::new(FakeBus::new());
    let uc = AssignTagUseCase::new(
        RcRepo(repo.clone()),
        FixedClock::new(now),
        RcBus(bus.clone()),
    );
    (uc, repo, bus)
}

fn fixture_note_with_tags(body: &str, created_at: OffsetDateTime, tags: TagSet) -> Note {
    Note::create(
        NoteBody::new(body.into()).expect("test fixture body must be valid"),
        tags,
        Timestamp::from_offset_datetime(created_at),
    )
}

fn tagset(names: &[&str]) -> TagSet {
    TagSet::from_tags(
        names
            .iter()
            .map(|n| Tag::new(n).expect("test fixture tag must normalize")),
    )
}

// ===== TP-H*: happy path =====

/// spec.md#tp-happy TP-H1
#[test]
fn tp_h1_added_tag_returns_updated_note_with_appended_tag_and_updated_at() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus) = rig(now);
    let seed = fixture_note_with_tags("hello", created, tagset(&["gpt"]));
    let id = seed.id().clone();
    repo.seed(seed);

    let updated = uc
        .execute(AssignTagCommand {
            note_id: id.clone(),
            raw_tag: "coding".into(),
        })
        .expect("added tag must succeed")
        .expect("added tag must return Some");

    assert_eq!(updated.id(), &id, "I-N1: id is immutable");
    assert_eq!(
        updated
            .tags()
            .as_slice()
            .iter()
            .map(|t| t.name().to_string())
            .collect::<Vec<_>>(),
        vec!["gpt".to_string(), "coding".to_string()],
        "TagSet preserves insertion order with the new tag appended"
    );
    assert_eq!(
        updated.updated_at(),
        Timestamp::from_offset_datetime(now),
        "I-N4 corollary: updated_at == clock.now()"
    );
}

/// spec.md#tp-happy TP-H2
#[test]
fn tp_h2_added_tag_calls_write_exactly_once() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus) = rig(now);
    let seed = fixture_note_with_tags("hello", created, tagset(&["gpt"]));
    let id = seed.id().clone();
    repo.seed(seed);

    let _ = uc.execute(AssignTagCommand {
        note_id: id,
        raw_tag: "coding".into(),
    });

    assert_eq!(repo.write_count(), 1);
}

/// spec.md#tp-happy TP-H3
#[test]
fn tp_h3_added_tag_publishes_note_tags_changed_once_with_correct_payload() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let expected_updated_at = Timestamp::from_offset_datetime(now);
    let (uc, repo, bus) = rig(now);
    let seed = fixture_note_with_tags("hello", created, tagset(&["gpt"]));
    let id = seed.id().clone();
    repo.seed(seed);

    let _ = uc.execute(AssignTagCommand {
        note_id: id.clone(),
        raw_tag: "coding".into(),
    });

    assert_eq!(bus.event_count(), 1, "C-AT6: exactly one event");
    match bus.last().expect("one event") {
        DomainEvent::NoteTagsChanged {
            note_id,
            tags,
            updated_at,
        } => {
            assert_eq!(note_id, id);
            assert_eq!(updated_at, expected_updated_at);
            assert_eq!(
                tags.as_slice()
                    .iter()
                    .map(|t| t.name().to_string())
                    .collect::<Vec<_>>(),
                vec!["gpt".to_string(), "coding".to_string()],
                "event payload carries the post-update TagSet"
            );
        }
        other => panic!("assign-tag must publish NoteTagsChanged, got {other:?}"),
    }
}

/// spec.md#tp-happy TP-H4
#[test]
fn tp_h4_added_tag_preserves_body_created_at_and_id() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus) = rig(now);
    let seed = fixture_note_with_tags("hello", created, tagset(&["gpt"]));
    let id = seed.id().clone();
    let body_before = seed.body().clone();
    let created_before = seed.created_at();
    repo.seed(seed);

    let updated = uc
        .execute(AssignTagCommand {
            note_id: id.clone(),
            raw_tag: "coding".into(),
        })
        .expect("must succeed")
        .expect("must be Some");

    assert_eq!(updated.id(), &id);
    assert_eq!(updated.body(), &body_before, "C-AT8: body unchanged");
    assert_eq!(updated.created_at(), created_before, "I-N1 corollary");
}

/// spec.md#tp-happy TP-H5 — empty TagSet receives its first tag.
#[test]
fn tp_h5_first_tag_on_empty_tagset() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus) = rig(now);
    let seed = fixture_note_with_tags("hello", created, TagSet::empty());
    let id = seed.id().clone();
    repo.seed(seed);

    let updated = uc
        .execute(AssignTagCommand {
            note_id: id,
            raw_tag: "gpt".into(),
        })
        .expect("must succeed")
        .expect("must be Some");

    assert_eq!(
        updated
            .tags()
            .as_slice()
            .iter()
            .map(|t| t.name().to_string())
            .collect::<Vec<_>>(),
        vec!["gpt".to_string()]
    );
    assert_eq!(bus.event_count(), 1);
}

// ===== TP-N*: S4 normalization + dedupe (no-op) =====

/// spec.md#tp-normalize-dedupe TP-N1 — "  GPT  " normalizes to "gpt"
/// which already exists in the TagSet → no-op.
#[test]
fn tp_n1_normalized_duplicate_returns_none() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus) = rig(now);
    let seed = fixture_note_with_tags("hello", created, tagset(&["gpt"]));
    let id = seed.id().clone();
    repo.seed(seed);

    let result = uc
        .execute(AssignTagCommand {
            note_id: id,
            raw_tag: "  GPT  ".into(),
        })
        .expect("unchanged tagset is not an error");

    assert!(result.is_none(), "S4: dedupe returns Ok(None)");
}

/// spec.md#tp-normalize-dedupe TP-N2
#[test]
fn tp_n2_normalized_duplicate_skips_write() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus) = rig(now);
    let seed = fixture_note_with_tags("hello", created, tagset(&["gpt"]));
    let id = seed.id().clone();
    repo.seed(seed);

    let _ = uc.execute(AssignTagCommand {
        note_id: id,
        raw_tag: "  GPT  ".into(),
    });

    assert_eq!(repo.write_count(), 0, "C-AT3: no write when tagset unchanged");
}

/// spec.md#tp-normalize-dedupe TP-N3
#[test]
fn tp_n3_normalized_duplicate_skips_publish() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus) = rig(now);
    let seed = fixture_note_with_tags("hello", created, tagset(&["gpt"]));
    let id = seed.id().clone();
    repo.seed(seed);

    let _ = uc.execute(AssignTagCommand {
        note_id: id,
        raw_tag: "  GPT  ".into(),
    });

    assert_eq!(bus.event_count(), 0, "S4: no event when tagset unchanged");
}

/// spec.md#tp-normalize-dedupe TP-N4 — case-only difference normalizes to dedupe.
#[test]
fn tp_n4_case_only_duplicate_returns_none() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus) = rig(now);
    let seed = fixture_note_with_tags("hello", created, tagset(&["gpt"]));
    let id = seed.id().clone();
    repo.seed(seed);

    let result = uc
        .execute(AssignTagCommand {
            note_id: id,
            raw_tag: "GPT".into(),
        })
        .expect("must succeed");

    assert!(result.is_none());
}

/// spec.md#tp-normalize-dedupe TP-N5 — whitespace-only difference normalizes to dedupe.
#[test]
fn tp_n5_whitespace_only_duplicate_returns_none() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus) = rig(now);
    let seed = fixture_note_with_tags("hello", created, tagset(&["gpt"]));
    let id = seed.id().clone();
    repo.seed(seed);

    let result = uc
        .execute(AssignTagCommand {
            note_id: id,
            raw_tag: " gpt ".into(),
        })
        .expect("must succeed");

    assert!(result.is_none());
}

// ===== TP-IC*: S10 invalid char =====

/// spec.md#tp-invalid-char TP-IC1 — comma rejected with InvalidChar.
#[test]
fn tp_ic1_comma_in_raw_tag_yields_invalid_tag_invalid_char() {
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, _repo, _bus) = rig(now);
    let id = NoteId::from_timestamp(Timestamp::from_offset_datetime(datetime!(
        2026-06-20 09:00:00 UTC
    )));

    let err = uc
        .execute(AssignTagCommand {
            note_id: id,
            raw_tag: "foo,bar".into(),
        })
        .expect_err("forbidden char must error");

    match err {
        AssignTagError::InvalidTag { name, reason } => {
            assert_eq!(name, "foo,bar");
            assert!(
                matches!(reason, TagError::InvalidChar { .. }),
                "expected InvalidChar, got {reason:?}"
            );
        }
        other => panic!("expected InvalidTag, got {other:?}"),
    }
}

/// spec.md#tp-invalid-char TP-IC2 — early reject means no load/write/publish.
#[test]
fn tp_ic2_invalid_tag_skips_load_write_and_publish() {
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus) = rig(now);
    let id = NoteId::from_timestamp(Timestamp::from_offset_datetime(datetime!(
        2026-06-20 09:00:00 UTC
    )));

    let _ = uc.execute(AssignTagCommand {
        note_id: id,
        raw_tag: "foo,bar".into(),
    });

    assert_eq!(repo.load_count(), 0, "C-AT1: parse_tag runs before load_note");
    assert_eq!(repo.write_count(), 0);
    assert_eq!(bus.event_count(), 0);
}

/// spec.md#tp-invalid-char TP-IC3 — interior whitespace and brackets also rejected.
#[test]
fn tp_ic3_other_forbidden_chars_reject() {
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, _repo, _bus) = rig(now);
    let id = NoteId::from_timestamp(Timestamp::from_offset_datetime(datetime!(
        2026-06-20 09:00:00 UTC
    )));

    for raw in ["foo bar", "foo\nbar", "[gpt", "gpt]", "fo\to"] {
        let err = uc
            .execute(AssignTagCommand {
                note_id: id.clone(),
                raw_tag: raw.into(),
            })
            .expect_err("forbidden char must error");

        match err {
            AssignTagError::InvalidTag { reason, .. } => assert!(
                matches!(reason, TagError::InvalidChar { .. }),
                "expected InvalidChar for {raw:?}, got {reason:?}"
            ),
            other => panic!("expected InvalidTag for {raw:?}, got {other:?}"),
        }
    }
}

/// spec.md#tp-invalid-char TP-IC4 — whitespace-only trims to empty → Empty.
#[test]
fn tp_ic4_empty_after_trim_yields_invalid_tag_empty() {
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, _repo, _bus) = rig(now);
    let id = NoteId::from_timestamp(Timestamp::from_offset_datetime(datetime!(
        2026-06-20 09:00:00 UTC
    )));

    let err = uc
        .execute(AssignTagCommand {
            note_id: id,
            raw_tag: "   ".into(),
        })
        .expect_err("whitespace-only must error");

    match err {
        AssignTagError::InvalidTag { name, reason } => {
            assert_eq!(name, "   ");
            assert!(
                matches!(reason, TagError::Empty),
                "expected Empty, got {reason:?}"
            );
        }
        other => panic!("expected InvalidTag, got {other:?}"),
    }
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
        .execute(AssignTagCommand {
            note_id: missing.clone(),
            raw_tag: "gpt".into(),
        })
        .expect_err("missing id must be an error");

    match err {
        AssignTagError::NoteNotFound { id } => assert_eq!(id, missing),
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

    let _ = uc.execute(AssignTagCommand {
        note_id: missing,
        raw_tag: "gpt".into(),
    });

    assert_eq!(repo.write_count(), 0);
    assert_eq!(bus.event_count(), 0);
}

// ===== TP-LE*: LoadError =====

/// spec.md#tp-load-err TP-LE1 + TP-LE3 (LoadError does not collapse to PersistError)
#[test]
fn tp_le1_load_failure_surfaces_as_load_error_not_persist_error() {
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus) = rig(now);
    let id = NoteId::from_timestamp(Timestamp::from_offset_datetime(datetime!(
        2026-06-20 09:00:00 UTC
    )));
    repo.fail_next_load(io::ErrorKind::PermissionDenied);

    let err = uc
        .execute(AssignTagCommand {
            note_id: id.clone(),
            raw_tag: "gpt".into(),
        })
        .expect_err("load failure must surface");

    match err {
        AssignTagError::LoadError { path, source } => {
            assert_eq!(
                path,
                PathBuf::from(format!("/tmp/promptnotes-test/{}.md", id.as_str()))
            );
            assert_eq!(source.kind(), io::ErrorKind::PermissionDenied);
        }
        AssignTagError::PersistError { .. } => {
            panic!("read failure must NOT collapse into PersistError (spec C-AT2)")
        }
        other => panic!("expected LoadError, got {other:?}"),
    }
    assert_eq!(repo.write_count(), 0);
    assert_eq!(bus.event_count(), 0);
}

// ===== TP-PE*: PersistError =====

/// spec.md#tp-persist-err TP-PE1 + TP-PE2
#[test]
fn tp_pe1_pe2_write_failure_surfaces_as_persist_error_with_kind() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus) = rig(now);
    let seed = fixture_note_with_tags("hello", created, tagset(&["gpt"]));
    let id = seed.id().clone();
    repo.seed(seed);
    repo.fail_next_write(io::ErrorKind::PermissionDenied);

    let err = uc
        .execute(AssignTagCommand {
            note_id: id.clone(),
            raw_tag: "coding".into(),
        })
        .expect_err("write failure must surface");

    match err {
        AssignTagError::PersistError { path, source } => {
            assert_eq!(
                path,
                PathBuf::from(format!("/tmp/promptnotes-test/{}.md", id.as_str()))
            );
            assert_eq!(source.kind(), io::ErrorKind::PermissionDenied);
        }
        other => panic!("expected PersistError, got {other:?}"),
    }
}

/// spec.md#tp-persist-err TP-PE3 — persist failure blocks event emission.
#[test]
fn tp_pe3_persist_failure_does_not_emit_event() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus) = rig(now);
    let seed = fixture_note_with_tags("hello", created, tagset(&["gpt"]));
    let id = seed.id().clone();
    repo.seed(seed);
    repo.fail_next_write(io::ErrorKind::Other);

    let _ = uc.execute(AssignTagCommand {
        note_id: id,
        raw_tag: "coding".into(),
    });

    assert_eq!(bus.event_count(), 0, "C-AT5: persist fail blocks event");
}

/// spec.md#tp-persist-err TP-PE4 — use case is stateless: retry recovers.
#[test]
fn tp_pe4_retry_after_transient_persist_failure_succeeds() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, bus) = rig(now);
    let seed = fixture_note_with_tags("hello", created, tagset(&["gpt"]));
    let id = seed.id().clone();
    repo.seed(seed);

    repo.fail_next_write(io::ErrorKind::Other);
    let _ = uc.execute(AssignTagCommand {
        note_id: id.clone(),
        raw_tag: "coding".into(),
    });

    let second = uc
        .execute(AssignTagCommand {
            note_id: id,
            raw_tag: "coding".into(),
        })
        .expect("second call (fs healthy) must succeed")
        .expect("must be Some");
    assert!(second
        .tags()
        .as_slice()
        .iter()
        .any(|t| t.name() == "coding"));
    assert_eq!(bus.event_count(), 1, "only the successful run emits");
}

// ===== TP-INV*: invariants =====

/// spec.md#tp-invariants TP-INV1 / TP-INV2
#[test]
fn tp_inv1_inv2_id_immutable_and_updated_at_ge_created_at() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus) = rig(now);
    let seed = fixture_note_with_tags("hello", created, tagset(&["gpt"]));
    let id = seed.id().clone();
    repo.seed(seed);

    let updated = uc
        .execute(AssignTagCommand {
            note_id: id.clone(),
            raw_tag: "coding".into(),
        })
        .expect("must succeed")
        .expect("must be Some");

    assert_eq!(updated.id(), &id, "I-N1: id immutable");
    assert!(
        updated.updated_at() >= updated.created_at(),
        "I-N3: updated_at >= created_at"
    );
}

/// spec.md#tp-invariants TP-INV3 + TP-INV4 — TagSet uniqueness + normalization.
#[test]
fn tp_inv3_inv4_tagset_unique_and_normalized() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let now = datetime!(2026-06-25 12:34:56 UTC);
    let (uc, repo, _bus) = rig(now);
    let seed = fixture_note_with_tags("hello", created, tagset(&["gpt"]));
    let id = seed.id().clone();
    repo.seed(seed);

    let updated = uc
        .execute(AssignTagCommand {
            note_id: id,
            raw_tag: "CODING".into(),
        })
        .expect("must succeed")
        .expect("must be Some");

    // I-N5: unique names.
    let names: Vec<String> = updated
        .tags()
        .as_slice()
        .iter()
        .map(|t| t.name().to_string())
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        names.len(),
        "I-N5: TagSet must contain no duplicate names"
    );
    // I-N6: normalized — lowercase + no forbidden chars (the latter is
    // guaranteed by Tag::new, but verify the lowercase post-condition).
    for name in &names {
        assert_eq!(name.as_str(), name.to_lowercase(), "I-N6: lowercase");
    }
}

// ===== TP-AS*: type-level API surface =====

/// spec.md#tp-api-shape TP-AS1
///
/// Compile-time pin of the public signature. If the shape drifts the project
/// will fail to build, not at runtime.
#[test]
fn tp_as1_execute_signature_returns_result_option_note() {
    type ExecuteFn<R, C, E> = fn(
        &AssignTagUseCase<R, C, E>,
        AssignTagCommand,
    ) -> Result<Option<Note>, AssignTagError>;
    fn assert_signature<R: NoteRepository, C: Clock, E: EventBus>() {
        let _: ExecuteFn<R, C, E> = AssignTagUseCase::<R, C, E>::execute;
    }
    assert_signature::<FakeRepo, FixedClock, FakeBus>();
}
