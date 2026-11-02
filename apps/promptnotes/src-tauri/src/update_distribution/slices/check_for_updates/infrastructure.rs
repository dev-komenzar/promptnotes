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

use super::domain::{is_check_due, should_notify};
use crate::update_distribution::shared::ports::{EventBus, NotifiedVersionLedger};
use crate::update_distribution::shared::types::{NewVersionDetected, Version};

// updater adapter は手動確認 slice と共有するため shared に置く (再 export して既存の参照を保つ)。
#[cfg(test)]
pub(crate) use crate::update_distribution::shared::updater::release_page_url;
pub use crate::update_distribution::shared::updater::TauriUpdaterPort;

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
