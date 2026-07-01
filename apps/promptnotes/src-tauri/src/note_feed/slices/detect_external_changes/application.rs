use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::{Clock, EventBus, NoteRepository};
use crate::note_capture::shared::types::NoteId;

use super::domain::{
    DetectExternalChangesCommand, DetectExternalChangesError, RawFileEvent, WatcherHandle,
};
use super::infrastructure::FsWatcher;

pub struct DetectExternalChangesUseCase {
    clock: Arc<dyn Clock + Send + Sync>,
    event_bus: Arc<dyn EventBus + Send + Sync>,
}

impl DetectExternalChangesUseCase {
    pub fn new(
        clock: Arc<dyn Clock + Send + Sync>,
        event_bus: Arc<dyn EventBus + Send + Sync>,
    ) -> Self {
        Self { clock, event_bus }
    }

    pub fn start_watcher(
        &self,
        cmd: DetectExternalChangesCommand,
        note_repo: Arc<dyn NoteRepository + Send + Sync>,
    ) -> Result<WatcherHandle, DetectExternalChangesError> {
        let watch_dir = cmd.storage_dir.as_path().to_path_buf();
        let fs_watcher = FsWatcher::new(&watch_dir).map_err(|source| {
            DetectExternalChangesError::WatcherStartFailed {
                path: watch_dir.clone(),
                source,
            }
        })?;

        let (stop_tx, stop_rx) = mpsc::channel();
        let clock = Arc::clone(&self.clock);
        let event_bus = Arc::clone(&self.event_bus);

        let handle = thread::spawn(move || {
            let mut fs_watcher = fs_watcher;
            let rx = fs_watcher.take_receiver();
            FsWatcher::run_event_loop(rx, stop_rx, move |raw| {
                match raw {
                    RawFileEvent::Created(path) => {
                        if let Some(note_id_str) = Self::resolve_note_id(&path) {
                            let note_id = NoteId::new(note_id_str);
                            match note_repo.load_by_id(&note_id) {
                                Ok(Some(note)) => {
                                    event_bus.publish(DomainEvent::NoteFileCreatedExternally {
                                        note_id: note.id().clone(),
                                        note,
                                        file_path: path,
                                        detected_at: clock.now(),
                                    });
                                }
                                Ok(None) => {
                                    log::warn!(
                                        "detect-external-changes: created file not loadable: {:?}",
                                        path
                                    );
                                }
                                Err(e) => {
                                    log::warn!(
                                        "detect-external-changes: load_by_id failed for {:?}: {e}",
                                        path
                                    );
                                }
                            }
                        }
                    }
                    RawFileEvent::Modified(path) => {
                        if let Some(note_id_str) = Self::resolve_note_id(&path) {
                            let note_id = NoteId::new(note_id_str);
                            match note_repo.load_by_id(&note_id) {
                                Ok(Some(note)) => {
                                    let disk_body_hash = note.body_hash().clone();
                                    event_bus.publish(DomainEvent::NoteFileModifiedExternally {
                                        note_id: note.id().clone(),
                                        disk_body_hash,
                                        note,
                                        file_path: path,
                                        detected_at: clock.now(),
                                    });
                                }
                                Ok(None) => {
                                    log::warn!(
                                        "detect-external-changes: modified file not loadable: {:?}",
                                        path
                                    );
                                }
                                Err(e) => {
                                    log::warn!(
                                        "detect-external-changes: load_by_id failed for {:?}: {e}",
                                        path
                                    );
                                }
                            }
                        }
                    }
                    RawFileEvent::Deleted(path) => {
                        if let Some(note_id_str) = Self::resolve_note_id(&path) {
                            let note_id = NoteId::new(note_id_str);
                            event_bus.publish(DomainEvent::NoteFileDeletedExternally {
                                note_id,
                                file_path: path,
                                detected_at: clock.now(),
                            });
                        }
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
