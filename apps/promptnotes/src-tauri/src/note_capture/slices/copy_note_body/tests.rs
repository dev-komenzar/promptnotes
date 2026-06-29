//! Tests for slice `copy-note-body`.
//!
//! Spec: `.ori/slices/copy-note-body/spec.md#test-perspectives`.
//!
//! RED phase: `CopyNoteBodyUseCase::execute` is `unimplemented!()`. Tests
//! that exercise behaviour panic; the compile-time signature pin (TP-AS1
//! / TP-NE1) is allowed to pass — it only verifies the public surface and
//! the structural absence of `EventBus`, not behaviour.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use time::macros::datetime;
use time::OffsetDateTime;

use crate::note_capture::shared::ports::NoteRepository;
use crate::note_capture::shared::types::{Note, NoteBody, NoteId, Tag, TagSet, Timestamp};

use super::application::CopyNoteBodyUseCase;
use super::domain::{CopyNoteBodyCommand, CopyNoteBodyError};
use super::ports::{ClipboardErrorKind, ClipboardService};

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
            storage_dir: PathBuf::from("/tmp/promptnotes-test"),
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

#[derive(Default)]
struct FakeClipboard {
    writes: RefCell<Vec<String>>,
    fail_next_with: Cell<Option<ClipboardErrorKind>>,
}
impl FakeClipboard {
    fn new() -> Self {
        Self::default()
    }
    fn fail_next_with(&self, kind: ClipboardErrorKind) {
        self.fail_next_with.set(Some(kind));
    }
    fn write_count(&self) -> usize {
        self.writes.borrow().len()
    }
    fn last_written(&self) -> Option<String> {
        self.writes.borrow().last().cloned()
    }
}
impl ClipboardService for FakeClipboard {
    fn write_text(&self, text: &str) -> Result<(), ClipboardErrorKind> {
        if let Some(kind) = self.fail_next_with.take() {
            return Err(kind);
        }
        self.writes.borrow_mut().push(text.to_string());
        Ok(())
    }
}

type Rig = (
    CopyNoteBodyUseCase<Rc<FakeRepo>, Rc<FakeClipboard>>,
    Rc<FakeRepo>,
    Rc<FakeClipboard>,
);

fn rig() -> Rig {
    let repo = Rc::new(FakeRepo::new());
    let clipboard = Rc::new(FakeClipboard::new());
    let uc = CopyNoteBodyUseCase::new(repo.clone(), clipboard.clone());
    (uc, repo, clipboard)
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

// ===== TP-H*: happy path (spec.md#tp-happy) =====

/// spec.md#tp-happy TP-H1
#[test]
fn tp_h1_normal_body_is_written_to_clipboard() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let (uc, repo, clipboard) = rig();
    let seed = fixture_note("hello world", created);
    let id = seed.id().clone();
    repo.seed(seed);

    uc.execute(CopyNoteBodyCommand {
        note_id: id.clone(),
    })
    .expect("happy path must succeed");

    assert_eq!(clipboard.write_count(), 1, "exactly one clipboard write");
    assert_eq!(
        clipboard.last_written().as_deref(),
        Some("hello world"),
        "body string written verbatim"
    );
}

// ===== TP-EX*: frontmatter / tag exclusion (spec.md#tp-exclude-frontmatter, I-CNB1) =====

/// spec.md#tp-exclude-frontmatter TP-EX1
///
/// 差別化 invariant: tags 付き Note でも clipboard には body のみが書かれ、
/// frontmatter delimiter / tag 文字列 / 日付文字列を一切含まない。
#[test]
fn tp_ex1_tagged_note_clipboard_contains_only_body() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let body = "line1\nline2";
    let (uc, repo, clipboard) = rig();
    let seed = fixture_note_with_tags(body, &["rust", "memo"], created);
    let id = seed.id().clone();
    repo.seed(seed);

    uc.execute(CopyNoteBodyCommand { note_id: id })
        .expect("tagged note copy must succeed");

    let written = clipboard
        .last_written()
        .expect("clipboard must have been written exactly once");
    assert_eq!(written, body, "I-CNB1: body verbatim");
    assert!(!written.contains("---"), "no frontmatter delimiter");
    assert!(!written.contains("tags:"), "no tags: key");
    assert!(!written.contains("rust"), "no tag name 'rust'");
    assert!(!written.contains("memo"), "no tag name 'memo'");
    assert!(!written.contains("createdAt"), "no createdAt key");
    assert!(!written.contains("2026"), "no year string from timestamp");
}

// ===== TP-EB*: empty body (spec.md#tp-empty-body, I-CNB2) =====

/// spec.md#tp-empty-body TP-EB1
#[test]
fn tp_eb1_empty_body_is_copied_as_empty_string() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let (uc, repo, clipboard) = rig();
    let seed = fixture_note("", created);
    let id = seed.id().clone();
    repo.seed(seed);

    uc.execute(CopyNoteBodyCommand { note_id: id })
        .expect("empty body is Ok (I-CNB2)");

    assert_eq!(clipboard.write_count(), 1);
    assert_eq!(clipboard.last_written().as_deref(), Some(""));
}

// ===== TP-NF*: NoteNotFound (spec.md#tp-not-found, I-CNB3) =====

/// spec.md#tp-not-found TP-NF1
#[test]
fn tp_nf1_missing_note_id_yields_note_not_found() {
    let (uc, _repo, _clipboard) = rig();
    let missing = NoteId::from_timestamp(Timestamp::from_offset_datetime(datetime!(
        2026-06-20 09:00:00 UTC
    )));

    let err = uc
        .execute(CopyNoteBodyCommand {
            note_id: missing.clone(),
        })
        .expect_err("missing id must be an error");

    match err {
        CopyNoteBodyError::NoteNotFound { id } => assert_eq!(id, missing),
        other => panic!("expected NoteNotFound, got {other:?}"),
    }
}

/// spec.md#tp-not-found TP-NF2 — I-CNB3 副作用順序契約。
#[test]
fn tp_nf2_missing_note_does_not_touch_clipboard() {
    let (uc, _repo, clipboard) = rig();
    let missing = NoteId::from_timestamp(Timestamp::from_offset_datetime(datetime!(
        2026-06-20 09:00:00 UTC
    )));

    let _ = uc.execute(CopyNoteBodyCommand { note_id: missing });

    assert_eq!(
        clipboard.write_count(),
        0,
        "I-CNB3: clipboard untouched when load misses"
    );
}

/// spec.md#tp-repo-io-err-collapse TP-NF3 — I-CNB5 意図的選択 pin。
///
/// `NoteRepository::load_by_id` の `io::Err` は `NoteNotFound` に collapse
/// する設計判断を test で固定する。impl が将来 `LoadError` variant を導入
/// したらこの test は失敗 → spec.md#io-errors / spec.md#invariants-slice-specific
/// I-CNB5 を同時更新する必要がある（設計判断の SSoT を test に置く）。
#[test]
fn tp_nf3_repo_io_error_collapses_to_note_not_found_with_no_clipboard_write() {
    let (uc, repo, clipboard) = rig();
    let id = NoteId::from_timestamp(Timestamp::from_offset_datetime(datetime!(
        2026-06-20 09:00:00 UTC
    )));
    // 注意: seed しない。fail_load_with を仕掛けて io::Err 経路を強制する。
    repo.fail_load_with
        .set(Some(io::ErrorKind::PermissionDenied));

    let err = uc
        .execute(CopyNoteBodyCommand {
            note_id: id.clone(),
        })
        .expect_err("load io error must surface as an error");

    match err {
        CopyNoteBodyError::NoteNotFound { id: returned } => assert_eq!(returned, id),
        other => panic!("I-CNB5: io::Err must collapse to NoteNotFound, got {other:?}"),
    }
    assert_eq!(
        clipboard.write_count(),
        0,
        "I-CNB3 + I-CNB5: clipboard untouched on load io error"
    );
}

// ===== TP-CE*: ClipboardError (spec.md#tp-clipboard-err) =====

/// spec.md#tp-clipboard-err TP-CE1 — Unavailable variant
#[test]
fn tp_ce1_clipboard_unavailable_surfaces_as_clipboard_error() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let (uc, repo, clipboard) = rig();
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);
    clipboard.fail_next_with(ClipboardErrorKind::Unavailable);

    let err = uc
        .execute(CopyNoteBodyCommand { note_id: id })
        .expect_err("clipboard failure must surface");

    match err {
        CopyNoteBodyError::ClipboardError { cause } => {
            assert_eq!(cause, ClipboardErrorKind::Unavailable);
        }
        other => panic!("expected ClipboardError, got {other:?}"),
    }
}

/// spec.md#tp-clipboard-err TP-CE2 — Io variant preserves message.
#[test]
fn tp_ce2_clipboard_io_failure_preserves_cause() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let (uc, repo, clipboard) = rig();
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);
    clipboard.fail_next_with(ClipboardErrorKind::Io("xclip not found".into()));

    let err = uc
        .execute(CopyNoteBodyCommand { note_id: id })
        .expect_err("clipboard failure must surface");

    match err {
        CopyNoteBodyError::ClipboardError {
            cause: ClipboardErrorKind::Io(msg),
        } => assert_eq!(msg, "xclip not found"),
        other => panic!("expected ClipboardError::Io, got {other:?}"),
    }
}

// ===== TP-NM*: no state mutation (spec.md#tp-no-mutation) =====

/// spec.md#tp-no-mutation TP-NM1 — read-only path 契約。
#[test]
fn tp_nm1_execute_does_not_call_write() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let (uc, repo, _clipboard) = rig();
    let seed = fixture_note("hello", created);
    let id = seed.id().clone();
    repo.seed(seed);

    let _ = uc.execute(CopyNoteBodyCommand { note_id: id });

    assert_eq!(
        repo.write_count(),
        0,
        "NoteRepository::write must NOT be called by copy-note-body"
    );
}

// ===== TP-NE*: no domain event (spec.md#tp-no-event, I-CNB4) =====

/// spec.md#tp-no-event TP-NE1
///
/// I-CNB4 is enforced **structurally**: `CopyNoteBodyUseCase::new` does not
/// accept an `EventBus`. If a future refactor adds bus dependency this test
/// will stop compiling, surfacing the regression at build time.
#[test]
fn tp_ne1_use_case_does_not_take_event_bus() {
    type NewFn<R, C> = fn(R, C) -> CopyNoteBodyUseCase<R, C>;
    fn assert_no_bus_parameter<R: NoteRepository, C: ClipboardService>() {
        let _: NewFn<R, C> = CopyNoteBodyUseCase::<R, C>::new;
    }
    assert_no_bus_parameter::<FakeRepo, FakeClipboard>();
}

// ===== TP-BC*: body-for-clipboard 経路 (spec.md#tp-uses-body-for-clipboard) =====

/// spec.md#tp-uses-body-for-clipboard TP-BC1
///
/// clipboard に書かれる文字列が seed body と byte-for-byte 一致することを確認。
/// I-CNB1 の enforcement は spec.md#invariants-slice-specific の通り test-time +
/// 構造で確定済み。TP-BC1 は「I-CNB1 の表面契約（seed 入力との一致）」を、
/// TP-BC2 は「`Note::body_for_clipboard()` 戻り値との一致」を別軸で pin する。
#[test]
fn tp_bc1_clipboard_content_equals_seed_body_byte_for_byte() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let body = "multi\nbyte\nバイト列も含む";
    let (uc, repo, clipboard) = rig();
    let seed = fixture_note(body, created);
    let id = seed.id().clone();
    repo.seed(seed);

    uc.execute(CopyNoteBodyCommand { note_id: id })
        .expect("must succeed");

    assert_eq!(clipboard.last_written().as_deref(), Some(body));
}

/// spec.md#tp-uses-body-for-clipboard TP-BC2 — I-CNB1 の経路 pin。
///
/// clipboard 内容が `Note::body_for_clipboard()` の戻り値と byte-for-byte
/// 一致することを assert する。TP-BC1 は seed 文字列と clipboard の一致、
/// TP-BC2 は aggregate query 戻り値と clipboard の一致を確認することで、
/// 将来 `body_for_clipboard()` に normalization (trailing newline 付与等)
/// が入った場合の regression を検出できる。slice が `note.body` 直接
/// アクセスへ退化した場合も TP-BC2 が失敗する。
#[test]
fn tp_bc2_clipboard_content_equals_body_for_clipboard_return() {
    let created = datetime!(2026-06-20 09:00:00 UTC);
    let body = "line1\nline2";
    let (uc, repo, clipboard) = rig();
    let seed = fixture_note_with_tags(body, &["rust"], created);
    let expected = seed.body_for_clipboard();
    let id = seed.id().clone();
    repo.seed(seed);

    uc.execute(CopyNoteBodyCommand { note_id: id })
        .expect("must succeed");

    assert_eq!(clipboard.last_written(), Some(expected));
}

// ===== TP-AS*: type-level API surface =====

/// spec.md#io-output TP-AS1 — execute returns `Result<(), CopyNoteBodyError>`.
///
/// Compile-time pin of the public signature. If the shape drifts the project
/// fails to build, not at runtime.
#[test]
fn tp_as1_execute_signature_returns_result_unit() {
    type ExecuteFn<R, C> =
        fn(&CopyNoteBodyUseCase<R, C>, CopyNoteBodyCommand) -> Result<(), CopyNoteBodyError>;
    fn assert_signature<R: NoteRepository, C: ClipboardService>() {
        let _: ExecuteFn<R, C> = CopyNoteBodyUseCase::<R, C>::execute;
    }
    assert_signature::<FakeRepo, FakeClipboard>();
}
