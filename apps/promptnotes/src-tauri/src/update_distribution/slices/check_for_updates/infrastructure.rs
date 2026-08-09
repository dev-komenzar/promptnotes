//! Infrastructure adapters for `check-for-updates` slice.
//!
//! Bridges the pure domain layer (`UpdaterPort`, `EventBus`) to the Tauri v2
//! updater plugin runtime. The plugin's built-in version comparison is bypassed
//! via a permissive `version_comparator` — our domain layer
//! (`CheckForUpdatesUseCase`) is the authoritative comparator (I-U2).

use serde::Serialize;
use tauri::Emitter;
use tauri_plugin_updater::UpdaterExt;

use crate::update_distribution::shared::ports::{EventBus, RawRelease, UpdaterPort};
use crate::update_distribution::shared::types::{NewVersionDetected, UpdateError};

/// Tauri v2 updater plugin adapter implementing `UpdaterPort`.
///
/// Wraps `tauri::AppHandle` and uses `tauri_plugin_updater::UpdaterExt` to
/// fetch the latest available release.
pub struct TauriUpdaterPort {
    pub app_handle: tauri::AppHandle,
}

impl UpdaterPort for TauriUpdaterPort {
    fn fetch_latest_release(&self) -> Result<RawRelease, UpdateError> {
        // Build updater with permissive comparator so the plugin always returns
        // the remote release regardless of version. Our domain layer does the
        // real comparison (I-U2).
        let updater = self
            .app_handle
            .updater_builder()
            .version_comparator(|_current, _remote| true)
            .build()
            .map_err(|e| {
                log::warn!("failed to build updater: {e:?}");
                UpdateError::NetworkError
            })?;

        // Bridge async -> sync: we're inside a Tauri async command (tokio runtime),
        // so block_in_place allows us to run the async check() synchronously.
        let check_result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(updater.check())
        });

        match check_result {
            Ok(Some(update)) => Ok(RawRelease {
                version_string: update.version,
                url: update.download_url.to_string(),
                notes: update.body.unwrap_or_default(),
            }),
            Ok(None) => {
                // 204 No Content: no release available on the remote.
                log::warn!("updater returned no release (204 No Content)");
                Err(UpdateError::NetworkError)
            }
            Err(e) => {
                log::warn!("updater check failed: {e:?}");
                Err(map_updater_error(e))
            }
        }
    }
}

/// Maps `tauri_plugin_updater::Error` to our domain `UpdateError`.
fn map_updater_error(e: tauri_plugin_updater::Error) -> UpdateError {
    use tauri_plugin_updater::Error as E;

    match &e {
        // Network/IO/connection errors
        E::Reqwest(_) | E::Network(_) | E::Io(_) | E::Http(_) | E::EmptyEndpoints => {
            UpdateError::NetworkError
        }
        // Server said no release
        E::ReleaseNotFound => UpdateError::NetworkError,
        // Parse/format errors
        E::Semver(_) | E::Serialization(_) | E::UrlParse(_) | E::FormatDate => {
            UpdateError::ParseError
        }
        // Platform/target errors
        E::TargetNotFound(_) | E::TargetsNotFound(_) | E::UnsupportedArch | E::UnsupportedOs => {
            UpdateError::ParseError
        }
        // Everything else — log and default to NetworkError
        _ => {
            log::warn!("unmapped updater error: {e:?}");
            UpdateError::NetworkError
        }
    }
}

/// Payload struct for the `new_version_detected` Tauri event.
///
/// Field names match the frontend's `NewVersionDetectedPayload` type
/// (`ui-widget/update-toast/store.svelte.ts`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
struct NewVersionDetectedPayload {
    current_version: String,
    latest_version: String,
    release_url: String,
    release_notes: String,
}

impl From<NewVersionDetected> for NewVersionDetectedPayload {
    fn from(event: NewVersionDetected) -> Self {
        Self {
            current_version: event.current_version.to_string(),
            latest_version: event.latest_version.to_string(),
            release_url: event.release_url,
            release_notes: event.release_notes,
        }
    }
}

/// Tauri event bus adapter implementing `EventBus`.
///
/// Publishes `NewVersionDetected` domain events as Tauri frontend events
/// via `app_handle.emit("new_version_detected", payload)`.
pub struct TauriEventBus {
    pub app_handle: tauri::AppHandle,
}

impl EventBus for TauriEventBus {
    fn publish(&self, event: NewVersionDetected) {
        let payload = NewVersionDetectedPayload::from(event);
        if let Err(e) = self.app_handle.emit("new_version_detected", &payload) {
            log::warn!("failed to emit new_version_detected event: {e:?}");
        }
    }
}
