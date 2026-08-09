//! Tauri command for `check-for-updates` slice.
//!
//! This command is designed to be called **once** on app startup (I-U3).
//! The frontend should invoke it on PageMain mount, not in a polling loop.

use serde::Serialize;

use super::application::CheckForUpdatesUseCase;
use super::domain::CheckForUpdatesCommand;
use super::infrastructure::{TauriEventBus, TauriUpdaterPort};
use crate::update_distribution::shared::types::{UpdateChannel, Version};

/// Serializable response for the frontend.
///
/// `UpdateChannel` is not `Serialize` (domain types are persistence-agnostic),
/// so we project it into this DTO at the command boundary.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateChannelResponse {
    pub current_version: String,
    pub latest_release: Option<ReleaseResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReleaseResponse {
    pub version: String,
    pub url: String,
    pub notes: String,
}

impl From<UpdateChannel> for UpdateChannelResponse {
    fn from(channel: UpdateChannel) -> Self {
        Self {
            current_version: channel.current_version().to_string(),
            latest_release: channel.latest_release().map(|r| ReleaseResponse {
                version: r.version().to_string(),
                url: r.url().to_string(),
                notes: r.notes().to_string(),
            }),
        }
    }
}

/// Check for available updates and return the result.
///
/// **C-CFU1 preserved**: this command never returns `Result`. All errors are
/// silently degraded to `UpdateChannel::without_release` inside the domain
/// use case (S14 silent failure).
///
/// **I-U3**: invoke once on app start. No polling.
#[tauri::command]
pub async fn check_for_updates(app_handle: tauri::AppHandle) -> UpdateChannelResponse {
    let current_version_str = env!("CARGO_PKG_VERSION");
    let current_version = match Version::from_str(current_version_str) {
        Ok(v) => v,
        Err(_) => {
            log::warn!("failed to parse CARGO_PKG_VERSION: {current_version_str}");
            return UpdateChannelResponse {
                current_version: current_version_str.to_string(),
                latest_release: None,
            };
        }
    };

    let updater = TauriUpdaterPort {
        app_handle: app_handle.clone(),
    };
    let bus = TauriEventBus {
        app_handle: app_handle.clone(),
    };

    let usecase = CheckForUpdatesUseCase::new(updater, bus);
    let channel = usecase.execute(CheckForUpdatesCommand { current_version });

    UpdateChannelResponse::from(channel)
}
