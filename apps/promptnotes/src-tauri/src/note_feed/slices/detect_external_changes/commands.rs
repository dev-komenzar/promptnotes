//! Tauri command surface for `detect-external-changes` slice.
//!
//! Production wiring: resolves storage_dir from settings, starts/stops the
//! notify-based file watcher, wires a subscriber that updates NoteFeed on
//! domain events, and stores the WatcherHandle in Tauri managed state.
//!
//! ## `StorageDirChanged` subscriber (C-DEC11 / TP-WL4)
//!
//! `update-settings` (User Preferences BC) publishes `SettingsEvent::StorageDirChanged`,
//! which its `TauriEventBus` adapter surfaces to the outside world as the Tauri event
//! `settings:storage_dir_changed`. Note Feed cannot import that BC's domain event type
//! directly (cross-BC boundary), so the composition root (`lib.rs::setup`) registers
//! [`register_storage_dir_changed_subscriber`] against **that Tauri event** — the
//! cross-BC contract surface emitted by the publisher. On receipt this slice drops the
//! old `WatcherHandle` (RAII stop, C-DEC7) and starts a new watcher for the freshly
//! persisted `storage_dir`, retrying up to 3 times at 1 second intervals on failure
//! (`workflows/detect-external-changes.md#notes`).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Listener, Manager, Runtime, State};
use time::OffsetDateTime;

use super::application::DetectExternalChangesUseCase;
use super::domain::{DetectExternalChangesCommand, WatcherHandle};
use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, EventBus, NoteRepository};
use crate::note_capture::shared::storage;
use crate::note_capture::shared::types::Timestamp;
use crate::note_capture::slices::create_note::infrastructure::FsNoteRepository;
use crate::note_feed::shared::adapters::InMemoryNoteFeedState;
use crate::user_preferences::shared::adapters::event_bus::STORAGE_DIR_CHANGED_EVENT;
use crate::user_preferences::shared::types::StorageDir;

/// Retry policy for watcher restart triggered by `StorageDirChanged`.
/// (`workflows/detect-external-changes.md#notes`: 最大 3 回、1 秒間隔)
const WATCHER_RESTART_MAX_ATTEMPTS: u32 = 3;
const WATCHER_RESTART_RETRY_DELAY: Duration = Duration::from_secs(1);

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

/// Process-local state holding the currently-running `WatcherHandle`.
///
/// Replacing `handle` drops the previous watcher (RAII stop) before the new one
/// is stored, so the "stop old → start new" ordering required by C-DEC7 is
/// guaranteed by construction.
pub struct WatcherState {
    pub handle: Option<WatcherHandle>,
}

impl WatcherState {
    pub fn new() -> Self {
        Self { handle: None }
    }
}

/// Starts (or restarts) the file watcher for the `storage_dir` currently persisted
/// in `settings.json`.
///
/// Also wires the NoteFeed subscriber (`upsert_one` / `remove_one` + Tauri event
/// `notes-changed`) onto a fresh in-process `EventBus` for the new watcher.
pub fn start_watcher_for_current_settings<R: Runtime>(
    app: &AppHandle<R>,
    watcher_state: &Mutex<WatcherState>,
    feed_state: &Arc<InMemoryNoteFeedState>,
) -> Result<(), String> {
    let storage_dir_path = storage::resolve_storage_dir(app);
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
        let feed_state = Arc::clone(feed_state);
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
    // Dropping the previous WatcherHandle here stops the old watcher before the
    // new handle is installed (C-DEC7 "旧 watcher 停止 → 新 watcher 起動").
    state.handle = Some(handle);

    Ok(())
}

/// Restarts the watcher for the current `storage_dir`, retrying up to
/// `WATCHER_RESTART_MAX_ATTEMPTS` times at `WATCHER_RESTART_RETRY_DELAY`.
///
/// On total failure we log only: the UI already shows the S11 / I-S4
/// "restart required" prompt on every `StorageDirChanged`, which doubles as the
/// user-facing signal (`workflows/detect-external-changes.md#errors`).
fn restart_watcher_with_retry<R: Runtime>(app: &AppHandle<R>) {
    for attempt in 1..=WATCHER_RESTART_MAX_ATTEMPTS {
        let watcher_state = app.state::<Mutex<WatcherState>>();
        let feed_state = app.state::<Arc<InMemoryNoteFeedState>>();
        match start_watcher_for_current_settings(app, watcher_state.inner(), feed_state.inner()) {
            Ok(()) => {
                log::info!(
                    "detect-external-changes: watcher restarted on new storage_dir (attempt {attempt})"
                );
                return;
            }
            Err(e) => {
                log::warn!(
                    "detect-external-changes: watcher restart attempt {attempt}/{WATCHER_RESTART_MAX_ATTEMPTS} failed: {e}"
                );
                if attempt < WATCHER_RESTART_MAX_ATTEMPTS {
                    std::thread::sleep(WATCHER_RESTART_RETRY_DELAY);
                }
            }
        }
    }
    log::error!(
        "detect-external-changes: watcher restart failed after {WATCHER_RESTART_MAX_ATTEMPTS} attempts; user must restart the app (I-S4)"
    );
}

/// Registers the Infrastructure-layer subscriber for `StorageDirChanged` (C-DEC11).
///
/// Called once from the composition root (`lib.rs::setup`). The handler runs
/// synchronously inside `Emitter::emit`, i.e. during `update_settings`'s event
/// publish, so the watcher switch happens before the command returns.
pub fn register_storage_dir_changed_subscriber<R: Runtime>(app: AppHandle<R>) {
    let listener_app = app.clone();
    app.listen(STORAGE_DIR_CHANGED_EVENT, move |_event| {
        log::info!("detect-external-changes: StorageDirChanged received; restarting file watcher");
        restart_watcher_with_retry(&listener_app);
    });
}

#[tauri::command]
pub async fn start_file_watcher<R: Runtime>(
    app: AppHandle<R>,
    watcher_state: State<'_, Mutex<WatcherState>>,
    feed_state: State<'_, Arc<InMemoryNoteFeedState>>,
) -> Result<(), String> {
    start_watcher_for_current_settings(&app, watcher_state.inner(), feed_state.inner())?;

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
