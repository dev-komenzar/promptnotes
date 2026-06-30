//! Tauri command surface for `detect-external-changes` slice.
//!
//! Production wiring: resolves storage_dir from settings, starts/stops the
//! notify-based file watcher, and stores the WatcherHandle in Tauri managed state.

use std::sync::Mutex;

use tauri::{AppHandle, Manager, Runtime, State};
use time::OffsetDateTime;

use super::application::DetectExternalChangesUseCase;
use super::domain::{DetectExternalChangesCommand, WatcherHandle};
use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, EventBus};
use crate::note_capture::shared::storage;
use crate::note_capture::shared::types::Timestamp;
use crate::user_preferences::shared::types::StorageDir;

struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_offset_datetime(OffsetDateTime::now_utc())
    }
}

struct NoOpBus;
impl EventBus for NoOpBus {
    fn publish(&self, _event: DomainEvent) {}
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
) -> Result<(), String> {
    let storage_dir_path = storage::resolve_storage_dir(&app);
    let storage_dir = StorageDir::try_from(storage_dir_path)
        .map_err(|e| format!("invalid storage dir: {e}"))?;

    let clock = SystemClock;
    let event_bus = NoOpBus;
    let use_case = DetectExternalChangesUseCase::new(clock, event_bus);

    let cmd = DetectExternalChangesCommand { storage_dir };

    let handle = use_case
        .start_watcher(cmd)
        .map_err(|e| e.to_string())?;

    let mut state = watcher_state
        .lock()
        .map_err(|e| format!("watcher state lock: {e}"))?;
    state.handle = Some(handle);

    log::info!("detect-external-changes: file watcher started");
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
