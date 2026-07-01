//! Tests for detect-external-changes slice.
//! Phase 3 → Phase 4 transition: tests that exercises real implementation paths.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use time::OffsetDateTime;

use crate::note_capture::shared::ports::NoteRepository;
use crate::note_capture::shared::types::{BodyHash, Note, NoteBody, NoteId, TagSet, Timestamp};
use crate::note_capture::shared::events::DomainEvent;
use crate::note_feed::slices::detect_external_changes::application::DetectExternalChangesUseCase;
use crate::note_feed::slices::detect_external_changes::domain::{
    DetectExternalChangesCommand, DetectExternalChangesError, RawFileEvent, WatcherHandle,
};
use crate::note_feed::slices::detect_external_changes::infrastructure::FsWatcher;
use crate::user_preferences::shared::types::StorageDir;

fn make_timestamp(unix: i64) -> Timestamp {
    Timestamp::from_offset_datetime(
        OffsetDateTime::from_unix_timestamp(unix).expect("valid unix timestamp"),
    )
}

// ---------------------------------------------------------------------------
// Infrastructure: FsWatcher (file filtering utilities)
// ---------------------------------------------------------------------------

#[test]
fn fs_watcher_is_md_file_recognizes_dot_md() {
    assert!(FsWatcher::is_md_file(Path::new("note.md")));
    assert!(FsWatcher::is_md_file(Path::new("NOTE.MD")));
    assert!(FsWatcher::is_md_file(Path::new("20250630120000.md")));
}

#[test]
fn fs_watcher_is_md_file_rejects_non_md() {
    assert!(!FsWatcher::is_md_file(Path::new("note.txt")));
    assert!(!FsWatcher::is_md_file(Path::new("note")));
    assert!(!FsWatcher::is_md_file(Path::new("note.tmp")));
}

#[test]
fn fs_watcher_is_tmp_file_detects_dot_tmp() {
    assert!(FsWatcher::is_tmp_file(Path::new(".syncthing.xxx.tmp")));
    assert!(FsWatcher::is_tmp_file(Path::new("file.TMP")));
}

#[test]
fn fs_watcher_is_tmp_file_rejects_non_tmp() {
    assert!(!FsWatcher::is_tmp_file(Path::new("note.md")));
}

#[test]
fn fs_watcher_debounce_window_is_500ms() {
    assert_eq!(FsWatcher::debounce_window(), Duration::from_millis(500));
}

// ---------------------------------------------------------------------------
// Domain: WatcherHandle RAII
// ---------------------------------------------------------------------------

#[test]
fn watcher_handle_drop_stops_watcher() {
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = std::thread::spawn(move || {
        let _ = rx.recv();
    });
    let wh = WatcherHandle::new(tx, handle);
    drop(wh);
}

#[test]
fn detect_external_changes_error_displays_correctly() {
    let err = DetectExternalChangesError::WatcherStartFailed {
        path: PathBuf::from("/nonexistent"),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "no such directory"),
    };
    let msg = err.to_string();
    assert!(msg.contains("/nonexistent"));
    assert!(msg.contains("failed to start"));
}

// ---------------------------------------------------------------------------
// Domain: Note body_hash and is_stale (I-N9)
// ---------------------------------------------------------------------------

#[test]
fn body_hash_is_deterministic() {
    let body1 = NoteBody::new("hello world".to_string()).unwrap();
    let body2 = NoteBody::new("hello world".to_string()).unwrap();
    let hash1 = BodyHash::from_body(body1.as_str());
    let hash2 = BodyHash::from_body(body2.as_str());
    assert_eq!(hash1, hash2);
}

#[test]
fn body_hash_differs_for_different_content() {
    let hash1 = BodyHash::from_body("hello");
    let hash2 = BodyHash::from_body("world");
    assert_ne!(hash1, hash2);
}

#[test]
fn note_is_stale_detects_mismatch() {
    let now = make_timestamp(1719705600);
    let body = NoteBody::new("original".to_string()).unwrap();
    let note = Note::create(body, TagSet::empty(), now);
    let disk_hash = BodyHash::from_body("modified");
    assert!(note.is_stale(&disk_hash));
}

#[test]
fn note_is_stale_returns_false_for_match() {
    let now = make_timestamp(1719705600);
    let body = NoteBody::new("same".to_string()).unwrap();
    let note = Note::create(body, TagSet::empty(), now);
    let disk_hash = BodyHash::from_body("same");
    assert!(!note.is_stale(&disk_hash));
}

// ---------------------------------------------------------------------------
// Domain: RawFileEvent variants
// ---------------------------------------------------------------------------

#[test]
fn raw_file_event_created_holds_path() {
    let path = PathBuf::from("/tmp/notes/20250630120000.md");
    let event = RawFileEvent::Created(path.clone());
    match event {
        RawFileEvent::Created(p) => assert_eq!(p, path),
        _ => panic!("expected Created"),
    }
}

#[test]
fn raw_file_event_modified_holds_path() {
    let path = PathBuf::from("/tmp/notes/20250630120000.md");
    let event = RawFileEvent::Modified(path.clone());
    match event {
        RawFileEvent::Modified(p) => assert_eq!(p, path),
        _ => panic!("expected Modified"),
    }
}

#[test]
fn raw_file_event_deleted_holds_path() {
    let path = PathBuf::from("/tmp/notes/20250630120000.md");
    let event = RawFileEvent::Deleted(path.clone());
    match event {
        RawFileEvent::Deleted(p) => assert_eq!(p, path),
        _ => panic!("expected Deleted"),
    }
}

// ---------------------------------------------------------------------------
// Application: resolve_note_id
// ---------------------------------------------------------------------------

#[test]
fn resolve_note_id_parses_valid_timestamp_filename() {
    let path = PathBuf::from("/tmp/notes/20250630120000.md");
    assert_eq!(
        DetectExternalChangesUseCase::resolve_note_id(&path),
        Some("20250630120000".to_string())
    );
}

#[test]
fn resolve_note_id_rejects_non_numeric_stem() {
    let path = PathBuf::from("/tmp/notes/README.md");
    assert_eq!(
        DetectExternalChangesUseCase::resolve_note_id(&path),
        None
    );
}

#[test]
fn resolve_note_id_rejects_wrong_length() {
    let path = PathBuf::from("/tmp/notes/123.md");
    assert_eq!(
        DetectExternalChangesUseCase::resolve_note_id(&path),
        None
    );
}

// ---------------------------------------------------------------------------
// Application: start_watcher with a valid directory
// ---------------------------------------------------------------------------

struct FakeClock;
impl crate::note_capture::shared::ports::Clock for FakeClock {
    fn now(&self) -> Timestamp {
        make_timestamp(1719705600)
    }
}

struct FakeEventBus;
impl crate::note_capture::shared::ports::EventBus for FakeEventBus {
    fn publish(&self, _event: DomainEvent) {}
}

struct FakeNoteRepo;
impl NoteRepository for FakeNoteRepo {
    fn write(&self, _note: &Note) -> io::Result<()> { Ok(()) }
    fn storage_dir(&self) -> &Path { Path::new("/fake") }
    fn load_by_id(&self, _id: &NoteId) -> io::Result<Option<Note>> { Ok(None) }
    fn list_all(&self) -> io::Result<Vec<Note>> { Ok(Vec::new()) }
}

#[test]
fn start_watcher_succeeds_with_temp_dir() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let uc = DetectExternalChangesUseCase::new(
        Arc::new(FakeClock),
        Arc::new(FakeEventBus),
    );
    let note_repo: Arc<dyn NoteRepository + Send + Sync> = Arc::new(FakeNoteRepo);
    let cmd = DetectExternalChangesCommand {
        storage_dir: StorageDir::try_from(tmp.path().to_path_buf()).unwrap(),
    };
    let result = uc.start_watcher(cmd, note_repo);
    assert!(result.is_ok(), "watcher should start on a valid directory");
}

#[test]
fn start_watcher_fails_on_nonexistent_dir() {
    let uc = DetectExternalChangesUseCase::new(
        Arc::new(FakeClock),
        Arc::new(FakeEventBus),
    );
    let note_repo: Arc<dyn NoteRepository + Send + Sync> = Arc::new(FakeNoteRepo);
    let cmd = DetectExternalChangesCommand {
        storage_dir: StorageDir::try_from(PathBuf::from("/nonexistent/dir/path")).unwrap(),
    };
    let result = uc.start_watcher(cmd, note_repo);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Application: domain event publishing from watcher callback
// ---------------------------------------------------------------------------

use std::sync::Mutex;

struct SpyingEventBus {
    events: Mutex<Vec<DomainEvent>>,
}

impl SpyingEventBus {
    fn new() -> Self {
        Self { events: Mutex::new(Vec::new()) }
    }

    fn published_events(&self) -> Vec<DomainEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl crate::note_capture::shared::ports::EventBus for SpyingEventBus {
    fn publish(&self, event: DomainEvent) {
        self.events.lock().unwrap().push(event);
    }
}

struct FileSystemNoteRepo {
    dir: PathBuf,
}

impl FileSystemNoteRepo {
    fn new(dir: PathBuf) -> Self {
        Self { dir }
    }
}

impl NoteRepository for FileSystemNoteRepo {
    fn write(&self, note: &Note) -> io::Result<()> {
        let path = self.dir.join(format!("{}.md", note.id().as_str()));
        let content = format!(
            "---\ncreatedAt: {}\nupdatedAt: {}\ntags: []\n---\n{}",
            note.created_at().format_yyyymmddhhmmss(),
            note.updated_at().format_yyyymmddhhmmss(),
            note.body().as_str(),
        );
        std::fs::create_dir_all(&self.dir)?;
        std::fs::write(&path, content)
    }

    fn storage_dir(&self) -> &Path {
        &self.dir
    }

    fn load_by_id(&self, id: &NoteId) -> io::Result<Option<Note>> {
        let path = self.dir.join(format!("{}.md", id.as_str()));
        match std::fs::read_to_string(&path) {
            Ok(raw) => Ok(Some(parse_via_fs_repo(&raw)?)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn list_all(&self) -> io::Result<Vec<Note>> {
        // Not needed for these tests
        Ok(Vec::new())
    }
}

// Reuse the parse logic from FsNoteRepository
fn parse_via_fs_repo(raw: &str) -> io::Result<Note> {
    // Simplified parse for testing — mirrors FsNoteRepository logic
    let rest = raw.strip_prefix("---\n").ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "missing opening delimiter")
    })?;
    let (frontmatter, body) = rest.split_once("\n---\n").ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "missing closing delimiter")
    })?;

    let mut created_at: Option<Timestamp> = None;
    let mut updated_at: Option<Timestamp> = None;
    for line in frontmatter.lines() {
        if let Some(v) = line.strip_prefix("createdAt: ") {
            created_at = Some(Timestamp::parse_yyyymmddhhmmss(v).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("createdAt: {e}"))
            })?);
        } else if let Some(v) = line.strip_prefix("updatedAt: ") {
            updated_at = Some(Timestamp::parse_yyyymmddhhmmss(v).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("updatedAt: {e}"))
            })?);
        }
    }

    let created_at = created_at.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "missing createdAt")
    })?;
    let updated_at = updated_at.unwrap_or(created_at);
    let note_body = NoteBody::new(body.to_string()).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("body: {e}"))
    })?;

    Ok(Note::from_persisted(note_body, TagSet::empty(), created_at, updated_at))
}

#[test]
fn watcher_emits_note_file_created_externally() {
    let tmp = tempfile::tempdir().expect("tempdir");

    // Pre-create a valid .md file in the watched directory
    let note_path = tmp.path().join("20250630120000.md");
    std::fs::write(&note_path, "---\ncreatedAt: 20250630120000\nupdatedAt: 20250630120000\ntags: []\n---\nhello world").unwrap();

    let event_bus = Arc::new(SpyingEventBus::new());
    let uc = DetectExternalChangesUseCase::new(
        Arc::new(FakeClock),
        event_bus.clone(),
    );
    let note_repo: Arc<dyn NoteRepository + Send + Sync> =
        Arc::new(FileSystemNoteRepo::new(tmp.path().to_path_buf()));

    let cmd = DetectExternalChangesCommand {
        storage_dir: StorageDir::try_from(tmp.path().to_path_buf()).unwrap(),
    };
    let result = uc.start_watcher(cmd, note_repo);
    assert!(result.is_ok());

    // Create a new .md file to trigger the watcher
    let new_path = tmp.path().join("20250630130000.md");
    std::fs::write(&new_path, "---\ncreatedAt: 20250630130000\nupdatedAt: 20250630130000\ntags: []\n---\nnew note").unwrap();

    // Wait for the watcher to detect and debounce
    std::thread::sleep(Duration::from_millis(800));

    let events = event_bus.published_events();
    let created = events.iter().find(|e| matches!(e, DomainEvent::NoteFileCreatedExternally { .. }));
    assert!(created.is_some(), "expected NoteFileCreatedExternally event, got: {events:?}");

    if let Some(DomainEvent::NoteFileCreatedExternally { note_id, note, .. }) = created {
        assert_eq!(note_id.as_str(), "20250630130000");
        assert_eq!(note.body().as_str(), "new note");
    }
}
