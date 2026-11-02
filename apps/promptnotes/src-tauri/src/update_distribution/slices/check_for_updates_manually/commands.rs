//! Tauri command for `check-for-updates-manually` slice
//! (`.ori/slices/check-for-updates-manually/spec.md#boundary-contract`)。
//!
//! C-CFUM3: check-for-updates の揮発 state (`InMemoryUpdateCheckState`) は参照しない。
//! 手動確認は前回確認時刻 (I-U3) も通知済みバージョン (I-U4) も変えない (I-U9)。

use serde::Serialize;

use super::application::CheckForUpdatesManuallyUseCase;
use super::domain::CheckForUpdatesManuallyCommand;
use crate::update_distribution::shared::build_info::build_version;
use crate::update_distribution::shared::response::UpdateChannelResponse;
use crate::update_distribution::shared::types::{UpdateError, Version};
use crate::update_distribution::shared::updater::TauriUpdaterPort;

/// TS へのエラー DTO。`kind` は `UpdateError` の variant 名 (TP-B2)。
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UpdateCheckErrorResponse {
    pub kind: String,
    pub message: String,
}

impl From<UpdateError> for UpdateCheckErrorResponse {
    fn from(e: UpdateError) -> Self {
        let kind = match e {
            UpdateError::NetworkError => "NetworkError",
            UpdateError::ParseError => "ParseError",
            UpdateError::RateLimited => "RateLimited",
        };
        Self {
            kind: kind.to_string(),
            message: e.to_string(),
        }
    }
}

/// 設定モーダルの「Check for updates」から呼ばれる手動確認 (I-U9)。
/// 新しいリリースあり / 最新 は `Ok`、確認失敗は `Err` で返す。
#[tauri::command]
pub async fn check_for_updates_manually(
    app_handle: tauri::AppHandle,
) -> Result<UpdateChannelResponse, UpdateCheckErrorResponse> {
    let current_version = Version::from_str(build_version()).map_err(|_| {
        log::warn!("failed to parse CARGO_PKG_VERSION: {}", build_version());
        UpdateCheckErrorResponse::from(UpdateError::ParseError)
    })?;
    let usecase = CheckForUpdatesManuallyUseCase::new(TauriUpdaterPort { app_handle });
    usecase
        .execute(CheckForUpdatesManuallyCommand { current_version })
        .map(UpdateChannelResponse::from)
        .map_err(|e| {
            log::warn!("manual update check failed: {e:?}");
            UpdateCheckErrorResponse::from(e)
        })
}
