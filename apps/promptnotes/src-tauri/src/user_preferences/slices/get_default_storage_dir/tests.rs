//! Tests for slice `get-default-storage-dir`.
//!
//! Spec: `.ori/slices/get-default-storage-dir/spec.md#test-points`.

use std::cell::Cell;
use std::env;
use std::rc::Rc;

use crate::user_preferences::shared::ports::OsDirs;
use crate::user_preferences::shared::types::StorageDir;

use super::{GetDefaultStorageDirQuery, GetDefaultStorageDirUseCase};

struct CountingOsDirs {
    default: StorageDir,
    calls: Cell<usize>,
}

impl OsDirs for CountingOsDirs {
    fn default_storage_dir(&self) -> StorageDir {
        self.calls.set(self.calls.get() + 1);
        self.default.clone()
    }
}

fn os_dirs() -> CountingOsDirs {
    CountingOsDirs {
        default: StorageDir::try_from(env::temp_dir().join("promptnotes-app-data/notes"))
            .expect("temp_dir is absolute"),
        calls: Cell::new(0),
    }
}

// ===== spec.md#tp-returns-os-default =====

/// TP-D1: `OsDirs::default_storage_dir` の値をそのまま返す。
#[test]
fn tp_d1_returns_os_dirs_default() {
    let dirs = os_dirs();
    let expected = dirs.default.clone();
    let uc = GetDefaultStorageDirUseCase::new(dirs);
    assert_eq!(uc.execute(GetDefaultStorageDirQuery), expected);
}

/// TP-D2 (C-GDSD2): 1 回の execute で port を 1 回だけ参照し、冪等。
#[test]
fn tp_d2_idempotent_and_single_port_call() {
    let dirs = Rc::new(os_dirs());
    let uc = GetDefaultStorageDirUseCase::new(Rc::clone(&dirs));
    let first = uc.execute(GetDefaultStorageDirQuery);
    assert_eq!(dirs.calls.get(), 1);
    assert_eq!(uc.execute(GetDefaultStorageDirQuery), first);
}

// ===== spec.md#tp-type-level =====

/// TP-T1 (C-GDSD1): command の戻り値が `StorageDir` (`Result` を含まない) を compile-time pin。
#[test]
fn tp_t1_signature_has_no_result() {
    let f: fn(tauri::AppHandle<tauri::Wry>) -> StorageDir = super::get_default_storage_dir;
    let _ = f;
}
