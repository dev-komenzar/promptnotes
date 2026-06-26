//! In-app trash adapter that moves note files into `<storage_dir>/trash/`.
//!
//! The OS-native trash bridge (macOS NSWorkspace / Linux XDG / Windows
//! SHFileOperation, e.g. the `trash` crate) is deferred — the restore path
//! must locate the trashed file deterministically by basename, which an
//! in-app subdir gives for free while OS trash bins do not. Confining the
//! move to `settings.storage_dir` keeps the data lifetime aligned with the
//! user's chosen notes directory.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::note_capture::slices::delete_note::{TrashErrorKind, TrashService};

pub struct FsTrashService {
    trash_dir: PathBuf,
}

impl FsTrashService {
    pub fn new(storage_dir: PathBuf) -> Self {
        Self {
            trash_dir: storage_dir.join("trash"),
        }
    }

    fn target_for(&self, src: &Path) -> Result<PathBuf, TrashErrorKind> {
        src.file_name()
            .map(|name| self.trash_dir.join(name))
            .ok_or_else(|| TrashErrorKind::Io(format!("no file name in {}", src.display())))
    }
}

fn map_io_err(e: io::Error) -> TrashErrorKind {
    match e.kind() {
        io::ErrorKind::PermissionDenied => TrashErrorKind::PermissionDenied,
        _ => TrashErrorKind::Io(e.to_string()),
    }
}

impl TrashService for FsTrashService {
    fn move_to_trash(&self, path: &Path) -> Result<(), TrashErrorKind> {
        fs::create_dir_all(&self.trash_dir).map_err(map_io_err)?;
        let target = self.target_for(path)?;
        fs::rename(path, &target).map_err(map_io_err)
    }

    fn restore_from_trash(&self, path: &Path) -> Result<(), TrashErrorKind> {
        let source = self.target_for(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(map_io_err)?;
        }
        fs::rename(&source, path).map_err(map_io_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_tmp(prefix: &str) -> PathBuf {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("promptnotes-{prefix}-{ts}-{seq}"))
    }

    #[test]
    fn move_then_restore_round_trips_through_trash_subdir() {
        let storage_dir = unique_tmp("trash-round-trip");
        fs::create_dir_all(&storage_dir).unwrap();
        let note_path = storage_dir.join("20260101000000.md");
        fs::write(&note_path, b"hello").unwrap();

        let svc = FsTrashService::new(storage_dir.clone());
        svc.move_to_trash(&note_path).unwrap();

        assert!(!note_path.exists());
        let trashed = storage_dir.join("trash").join("20260101000000.md");
        assert!(trashed.exists());

        svc.restore_from_trash(&note_path).unwrap();
        assert!(note_path.exists());
        assert!(!trashed.exists());

        fs::remove_dir_all(&storage_dir).ok();
    }

    #[test]
    fn move_missing_source_returns_io_error() {
        let storage_dir = unique_tmp("trash-missing");
        let svc = FsTrashService::new(storage_dir.clone());
        let missing = storage_dir.join("nope.md");
        match svc.move_to_trash(&missing) {
            Err(TrashErrorKind::Io(_)) => {}
            other => panic!("expected Io error, got {other:?}"),
        }
        fs::remove_dir_all(&storage_dir).ok();
    }
}
