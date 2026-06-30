use std::path::PathBuf;

use crate::user_preferences::shared::types::StorageDir;

/// Command to start the file watcher for a given storage directory.
/// (spec: domain/workflows/detect-external-changes.md#input)
#[derive(Debug, Clone)]
pub struct DetectExternalChangesCommand {
    /// Absolute path to the storage directory to watch.
    pub storage_dir: StorageDir,
}

/// Handle to a running file watcher. Dropping this stops the watcher.
/// (spec: C-DEC7: RAII guard)
pub struct WatcherHandle {
    /// Stop signal sender — Drop sends a stop signal to the watcher thread.
    stop_tx: Option<std::sync::mpsc::Sender<()>>,
    /// Join handle for the watcher event-loop thread.
    thread: Option<std::thread::JoinHandle<()>>,
}

impl WatcherHandle {
    pub fn new(
        stop_tx: std::sync::mpsc::Sender<()>,
        thread: std::thread::JoinHandle<()>,
    ) -> Self {
        Self {
            stop_tx: Some(stop_tx),
            thread: Some(thread),
        }
    }
}

impl Drop for WatcherHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

/// Error type for watcher operations.
#[derive(Debug, thiserror::Error)]
pub enum DetectExternalChangesError {
    #[error("failed to start file watcher on {path}: {source}")]
    WatcherStartFailed {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Represents a detected external file event before domain transformation.
#[derive(Debug, Clone)]
pub enum RawFileEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
}
