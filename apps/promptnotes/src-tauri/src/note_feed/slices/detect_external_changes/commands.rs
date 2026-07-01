//! Tauri command surface for `detect-external-changes` slice.
//!
//! Production wiring: resolves storage_dir from settings, starts/stops the
//! notify-based file watcher, wires a subscriber that updates NoteFeed on
//! domain events, and stores the WatcherHandle in Tauri managed state.

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Runtime, State};
use time::OffsetDateTime;

use super::application::DetectExternalChangesUseCase;
use super::domain::{DetectExternalChangesCommand, WatcherHandle};
use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, EventBus, NoteRepository};
use crate::note_capture::shared::types::Timestamp;
use crate::note_capture::slices::create_note::infrastructure::FsNoteRepository;
use crate::note_capture::shared::storage;
use crate::note_feed::shared::adapters::InMemoryNoteFeedState;
use crate::user_preferences::shared::types::StorageDir;

struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_offset_datetime(OffsetDateTime::now_utc())
    }
}

struct AppEventBus {
    handlers: Mutex<Vec<Box<dyn Fn(DomainEvent) + Send>>>,
}

impl AppEventBus {
    fn new() -> Self {
        Self {
            handlers: Mutex::new(Vec::new()),
        }
    }

    fn subscribe(&self, handler: Box<dyn Fn(DomainEvent) + Send>) {
        self.handlers
            .lock()
            .expect("event bus poisoned")
            .push(handler);
    }
}

impl EventBus for AppEventBus {
    fn publish(&self, event: DomainEvent) {
        let handlers = self.handlers.lock().expect("event bus poisoned");
        for handler in handlers.iter() {
            handler(event.clone());
        }
    }
}

pub struct WatcherState {
    pub handle: Option<WatcherHandle>,
}

impl WatcherState {
    pub fn new() -> Self {
        Self { handle: None }
    }
}

#[tauri::command]
pub async fn start_file_watcher<R: Runtime>(
    app: AppHandle<R>,
    watcher_state: State<'_, Mutex<WatcherState>>,
    feed_state: State<'_, Arc<InMemoryNoteFeedState>>,
) -> Result<(), String> {
    let storage_dir_path = storage::resolve_storage_dir(&app);
    let storage_dir = StorageDir::try_from(storage_dir_path)
        .map_err(|e| format!("invalid storage dir: {e}"))?;

    let watch_dir = storage_dir.as_path().to_path_buf();
    let note_repo: Arc<dyn NoteRepository + Send + Sync> =
        Arc::new(FsNoteRepository::new(watch_dir));

    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let event_bus: Arc<AppEventBus> = Arc::new(AppEventBus::new());

    // Wire subscriber that updates NoteFeed on external change events.
    // Note: upsert/remove logic runs in the watcher thread via EventBus::publish().
    // InMemoryNoteFeedState uses internal Mutex for thread safety (C-DEC10 scope boundary).
    {
        let feed_state = Arc::clone(&feed_state);
        let app_handle = app.clone();
        event_bus.subscribe(Box::new(move |event: DomainEvent| {
            match &event {
                DomainEvent::NoteFileCreatedExternally { note, .. }
                | DomainEvent::NoteFileModifiedExternally { note, .. } => {
                    feed_state.upsert_one(note.clone());
                    let _ = app_handle.emit("notes-changed", ());
                }
                DomainEvent::NoteFileDeletedExternally { note_id, .. } => {
                    feed_state.remove_one(note_id);
                    let _ = app_handle.emit("notes-changed", ());
                }
                _ => {}
            }
        }));
    }

    let use_case = DetectExternalChangesUseCase::new(clock, event_bus);

    let cmd = DetectExternalChangesCommand { storage_dir };

    let handle = use_case
        .start_watcher(cmd, note_repo)
        .map_err(|e| e.to_string())?;

    let mut state = watcher_state
        .lock()
        .map_err(|e| format!("watcher state lock: {e}"))?;
    state.handle = Some(handle);

    log::info!("detect-external-changes: file watcher started with domain event pipeline");
    Ok(())
}

#[tauri::command]
pub async fn stop_file_watcher<R: Runtime>(
    _app: AppHandle<R>,
    watcher_state: State<'_, Mutex<WatcherState>>,
) -> Result<(), String> {
    let mut state = watcher_state
        .lock()
        .map_err(|e| format!("watcher state lock: {e}"))?;
    state.handle = None;
    log::info!("detect-external-changes: file watcher stopped");
    Ok(())
}
