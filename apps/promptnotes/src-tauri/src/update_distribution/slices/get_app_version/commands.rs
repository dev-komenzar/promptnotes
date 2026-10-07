//! Tauri command for `get-app-version` slice.

use super::application::GetAppVersionUseCase;
use super::domain::GetAppVersionQuery;
use crate::update_distribution::shared::build_info::build_version;

/// 設定モーダル表示用に現在のアプリバージョンを返す。
///
/// **C-GAV1**: `Result` を返さない。**C-GAV5**: `check_for_updates` と同じ
/// `build_version()` を情報源とする。
#[tauri::command]
pub fn get_app_version() -> String {
    GetAppVersionUseCase::execute(GetAppVersionQuery, build_version()).into_string()
}
