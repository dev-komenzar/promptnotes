//! Tauri command for `get-default-storage-dir` slice.

use tauri::{AppHandle, Runtime};

use super::application::GetDefaultStorageDirUseCase;
use super::domain::GetDefaultStorageDirQuery;
use crate::user_preferences::shared::default_storage_dir::resolve_default_storage_dir;
use crate::user_preferences::shared::types::StorageDir;
use crate::user_preferences::slices::load_settings::infrastructure::FixedOsDirs;

/// 設定モーダルの「デフォルトに戻す」用に OS 慣習の default `storage_dir` を返す。
///
/// **C-GDSD1**: `Result` を返さない。**C-GDSD3**: load-settings / update-settings と
/// 同じ `resolve_default_storage_dir` を情報源とする。
#[tauri::command]
pub fn get_default_storage_dir<R: Runtime>(app: AppHandle<R>) -> StorageDir {
    let uc = GetDefaultStorageDirUseCase::new(FixedOsDirs::new(resolve_default_storage_dir(&app)));
    uc.execute(GetDefaultStorageDirQuery)
}
