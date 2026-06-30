use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use crate::note_capture::shared::ports::{Clock, EventBus};

use super::domain::{
    DetectExternalChangesCommand, DetectExternalChangesError, RawFileEvent, WatcherHandle,
};
use super::infrastructure::FsWatcher;

pub struct DetectExternalChangesUseCase<C: Clock, B: EventBus> {
    clock: C,
    event_bus: B,
}

impl<C: Clock + Send + 'static, B: EventBus + Send + 'static> DetectExternalChangesUseCase<C, B> {
    pub fn new(clock: C, event_bus: B) -> Self {
        Self { clock, event_bus }
    }

    pub fn start_watcher(
        &self,
        cmd: DetectExternalChangesCommand,
    ) -> Result<WatcherHandle, DetectExternalChangesError> {
        let watch_dir = cmd.storage_dir.as_path().to_path_buf();
        let fs_watcher = FsWatcher::new(&watch_dir).map_err(|source| {
            DetectExternalChangesError::WatcherStartFailed {
                path: watch_dir.clone(),
                source,
            }
        })?;

        let (stop_tx, stop_rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            let rx = fs_watcher.into_receiver();
            FsWatcher::run_event_loop(rx, stop_rx, move |raw| {
                match raw {
                    RawFileEvent::Created(path) => {
                        log::info!("detect-external-changes: file created: {:?}", path);
                    }
                    RawFileEvent::Modified(path) => {
                        log::info!("detect-external-changes: file modified: {:?}", path);
                    }
                    RawFileEvent::Deleted(path) => {
                        log::info!("detect-external-changes: file deleted: {:?}", path);
                    }
                }
            });
        });

        Ok(WatcherHandle::new(stop_tx, handle))
    }

    pub fn resolve_note_id(path: &PathBuf) -> Option<String> {
        let stem = path.file_stem()?.to_str()?;
        if stem.len() == 14 && stem.chars().all(|c| c.is_ascii_digit()) {
            Some(stem.to_string())
        } else {
            None
        }
    }
}
