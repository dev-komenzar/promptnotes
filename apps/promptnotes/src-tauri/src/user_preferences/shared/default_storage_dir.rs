//! OS 慣習に基づく default `storage_dir` (I-S3) の解決。
//!
//! `<app_data_dir>/notes` を default とする。各 Tauri command の composition root
//! (load-settings / update-settings / get-default-storage-dir / note_feed / note_capture)
//! が同じ規則で解決するよう、ここに 1 箇所だけ置く。

use std::env;
use std::path::PathBuf;

use tauri::{AppHandle, Manager, Runtime};

use super::types::StorageDir;

/// Tauri の `app_data_dir()` 解決結果から default `storage_dir` を組み立てる (pure)。
///
/// - `Some(dir)` → `dir/notes`
/// - `None` (解決失敗) → `<temp_dir>/promptnotes/notes`
/// - 上記が絶対パスでない場合の最終 fallback: `std::env::temp_dir()`
///   (POSIX/Windows ともに OS 契約上 **絶対パス** を返すため `StorageDir::try_from` は必ず成功する)
pub fn default_storage_dir_from(app_data_dir: Option<PathBuf>) -> StorageDir {
    let candidate = app_data_dir
        .map(|p| p.join("notes"))
        .unwrap_or_else(|| env::temp_dir().join("promptnotes/notes"));
    StorageDir::try_from(candidate).unwrap_or_else(|_| {
        StorageDir::try_from(env::temp_dir())
            .expect("std::env::temp_dir() is absolute by OS contract")
    })
}

/// `AppHandle` から default `storage_dir` を解決する。
pub fn resolve_default_storage_dir<R: Runtime>(app: &AppHandle<R>) -> StorageDir {
    default_storage_dir_from(app.path().app_data_dir().ok())
}

#[cfg(test)]
mod tests {
    //! Spec: `.ori/slices/get-default-storage-dir/spec.md#tp-shared-resolve` (TP-R1..R3).

    use super::*;

    #[test]
    fn tp_r1_app_data_dir_resolved_appends_notes() {
        let base = env::temp_dir().join("promptnotes-app-data");
        assert_eq!(
            default_storage_dir_from(Some(base.clone())).as_path(),
            base.join("notes")
        );
    }

    #[test]
    fn tp_r2_app_data_dir_unresolved_falls_back_to_temp_dir() {
        assert_eq!(
            default_storage_dir_from(None).as_path(),
            env::temp_dir().join("promptnotes/notes")
        );
    }

    #[test]
    fn tp_r3_relative_app_data_dir_falls_back_to_temp_dir() {
        assert_eq!(
            default_storage_dir_from(Some(PathBuf::from("relative"))).as_path(),
            env::temp_dir()
        );
    }
}
