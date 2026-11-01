//! Tauri commands for `install-update` slice (`.ori/slices/install-update/spec.md#boundary-contract`)。
//!
//! I-U8: `install_update` は再起動しない。page-main が全 pending Note を Flush してから
//! `restart_app` を呼ぶ (quit 時の S13 と同じ順序保証を composition 側に寄せる)。

use serde::Serialize;

use super::application::InstallUpdateUseCase;
use super::domain::InstallUpdateCommand;
use super::infrastructure::{current_installation_kind, TauriUpdateInstaller};
use crate::update_distribution::shared::build_info::build_version;
use crate::update_distribution::shared::types::{InstallUpdateError, Version};

/// TS へのエラー DTO。`kind` は `InstallUpdateError` の variant 名 (TP-B3)。
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct InstallUpdateErrorResponse {
    pub kind: String,
    pub message: String,
}

impl From<InstallUpdateError> for InstallUpdateErrorResponse {
    fn from(e: InstallUpdateError) -> Self {
        Self {
            kind: e.kind().to_string(),
            message: e.to_string(),
        }
    }
}

/// 最新リリースを取得し直して download + 署名検証 + install する (I-U5〜I-U7)。
/// 成功時はインストールしたバージョン文字列を返す。
#[tauri::command]
pub async fn install_update(
    app_handle: tauri::AppHandle,
) -> Result<String, InstallUpdateErrorResponse> {
    let current_version = Version::from_str(build_version()).map_err(|_| {
        InstallUpdateErrorResponse::from(InstallUpdateError::CheckFailed(format!(
            "unparseable current version: {}",
            build_version()
        )))
    })?;
    let usecase = InstallUpdateUseCase::new(TauriUpdateInstaller { app_handle });
    usecase
        .execute(InstallUpdateCommand {
            current_version,
            installation: current_installation_kind(),
        })
        .map(|v| v.to_string())
        .map_err(|e| {
            log::warn!("install-update failed: {e:?}");
            InstallUpdateErrorResponse::from(e)
        })
}

/// 通知 UI が「Update & restart」を出してよいか (I-U6)。ネットワークには触れない。
#[tauri::command]
pub fn is_update_installable() -> bool {
    current_installation_kind().is_installable()
}

/// アプリを再起動する。install 成功 + Flush 完了後にだけ呼ばれる (I-U8)。
#[tauri::command]
pub fn restart_app(app_handle: tauri::AppHandle) {
    app_handle.restart();
}
