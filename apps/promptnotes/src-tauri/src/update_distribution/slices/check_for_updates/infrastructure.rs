//! Infrastructure adapters for `check-for-updates` slice.
//!
//! Bridges the pure domain layer (`UpdaterPort`, `EventBus`) to the Tauri v2
//! updater plugin runtime. The plugin's built-in version comparison is bypassed
//! via a permissive `version_comparator` — our domain layer
//! (`CheckForUpdatesUseCase`) is the authoritative comparator (I-U2).

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::SystemTime;

use serde::Serialize;
use tauri::Emitter;
use tauri_plugin_updater::UpdaterExt;

use super::domain::{is_check_due, should_notify};
use crate::update_distribution::shared::ports::{
    EventBus, NotifiedVersionLedger, RawRelease, UpdaterPort,
};
use crate::update_distribution::shared::types::{NewVersionDetected, UpdateError, Version};

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
        let builder = self
            .app_handle
            .updater_builder()
            .version_comparator(|_current, _remote| true);

        // S14 E2E test seam (`.ori/scenarios/s14-update-check-failure`):
        // debug builds may point the updater at a local endpoint via
        // `TAURI_TEST_UPDATER_ENDPOINT` to deterministically reproduce an HTTP
        // failure. Release builds compile this branch out entirely, so the
        // production endpoint (tauri.conf.json) is never overridden.
        #[cfg(debug_assertions)]
        let builder = match std::env::var("TAURI_TEST_UPDATER_ENDPOINT") {
            Ok(endpoint) => {
                let url = tauri::Url::parse(&endpoint).map_err(|e| {
                    log::warn!("invalid TAURI_TEST_UPDATER_ENDPOINT ({endpoint}): {e:?}");
                    UpdateError::ParseError
                })?;
                builder.endpoints(vec![url]).map_err(|e| {
                    log::warn!("failed to apply TAURI_TEST_UPDATER_ENDPOINT: {e:?}");
                    UpdateError::NetworkError
                })?
            }
            Err(_) => builder,
        };

        let updater = builder.build().map_err(|e| {
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
                // `update.download_url` は latest.json の platform 別アセット URL
                // (tauri-action が api.github.com/.../releases/assets/<id> を書く) で
                // ブラウザで開くとバイナリ / JSON になる。toast の「View release」は
                // GitHub Releases page を開く契約 (aggregates.md#update-channel-aggregate
                // `url: Url (GitHub Releases page)`) なので、version から組み立てる。
                url: release_page_url(&update.version),
                version_string: update.version,
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

/// GitHub Releases の tag page URL prefix。tag は release workflow が `v<version>` で切る。
const RELEASE_TAG_URL_PREFIX: &str = "https://github.com/dev-komenzar/promptnotes/releases/tag/v";

/// latest.json の `version` (例: `0.2.3` / `v0.2.3`) から GitHub Releases page の URL を組み立てる。
pub(crate) fn release_page_url(version: &str) -> String {
    let version = version.trim();
    let version = version.strip_prefix('v').unwrap_or(version);
    format!("{RELEASE_TAG_URL_PREFIX}{version}")
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

// ===== 揮発 state: 前回確認時刻 + 通知済みバージョン (I-U3 / I-U4) =====

/// `check-for-updates` の揮発 state (spec.md#impl-periodic)。プロセス内のみ、永続化しない。
///
/// - 前回確認時刻: 未確認ならプロセス起動時刻 (`new(started_at)`)。I-U3 の 24h 周期の基準
/// - 通知済みバージョン: `NotifiedVersionLedger` として I-U4 の再通知抑止に使う
///
/// frontend の起動時 invoke と定期確認 task の両方が同じ instance を共有する (C-CFU12)。
pub struct InMemoryUpdateCheckState {
    last_checked_at: Mutex<SystemTime>,
    notified: Mutex<Option<Version>>,
}

impl InMemoryUpdateCheckState {
    pub fn new(started_at: SystemTime) -> Self {
        Self {
            last_checked_at: Mutex::new(started_at),
            notified: Mutex::new(None),
        }
    }

    /// `now` 時点で定期確認が due か (state は変えない)。
    pub fn is_due(&self, now: SystemTime) -> bool {
        is_check_due(*lock(&self.last_checked_at), now)
    }

    /// 確認開始を記録する (frontend 起動時 invoke 経路。成否を問わず、C-CFU10)。
    pub fn begin_check(&self, now: SystemTime) {
        *lock(&self.last_checked_at) = now;
    }

    /// due なら確認開始を記録して true、not due なら state を変えず false (定期 task 経路)。
    pub fn begin_due_check(&self, now: SystemTime) -> bool {
        let mut last = lock(&self.last_checked_at);
        if !is_check_due(*last, now) {
            return false;
        }
        *last = now;
        true
    }
}

impl NotifiedVersionLedger for InMemoryUpdateCheckState {
    fn claim(&self, version: &Version) -> bool {
        let mut notified = lock(&self.notified);
        if !should_notify(version, notified.as_ref()) {
            return false;
        }
        *notified = Some(version.clone());
        true
    }
}

/// poison された Mutex でも中身 (時刻 / バージョン) は常に整合しているので、そのまま使う。
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
