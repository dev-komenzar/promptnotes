//! Tauri command for `check-for-updates` slice.
//!
//! I-U3: 起動時の確認は frontend (page-main) が `new_version_detected` の listen 確立後に
//! この command を 1 回 invoke する。以降 24 時間ごとの定期確認は Rust 側の背景 task
//! (`spawn_periodic_update_check`) が `execute_update_check` を呼ぶ。
//! 両経路は同じ `InMemoryUpdateCheckState` (前回確認時刻 / 通知済みバージョン) を共有する
//! (spec.md#invariants-slice-specific C-CFU12)。

use std::sync::Arc;
use std::time::SystemTime;

use serde::Serialize;
use tauri::Manager;

use super::application::CheckForUpdatesUseCase;
use super::domain::{CheckForUpdatesCommand, PERIODIC_CHECK_TICK};
use super::infrastructure::{InMemoryUpdateCheckState, TauriEventBus, TauriUpdaterPort};
use crate::update_distribution::shared::build_info::build_version;
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
/// **I-U3**: frontend が起動時に 1 回 invoke する。呼ばれた時刻を前回確認時刻として記録し
/// (C-CFU10)、以降の定期確認はその 24 時間後に due になる。
/// **I-U4**: 通知済みバージョン以下なら `latest_release` は返すが event は発行しない。
///
/// `State<'_>` を引数に取る async command は `Result` を返す必要があるため、state は
/// `AppHandle` から取り出す (C-CFU1 を保つため)。
#[tauri::command]
pub async fn check_for_updates(app_handle: tauri::AppHandle) -> UpdateChannelResponse {
    let state = app_handle
        .state::<Arc<InMemoryUpdateCheckState>>()
        .inner()
        .clone();
    state.begin_check(SystemTime::now());
    execute_update_check(&app_handle, state)
}

/// 1 回分の確認 (use case 実行) を行う。前回確認時刻の記録は呼出側の責務
/// (command は `begin_check`、定期 task は `begin_due_check`)。
pub(crate) fn execute_update_check(
    app_handle: &tauri::AppHandle,
    state: Arc<InMemoryUpdateCheckState>,
) -> UpdateChannelResponse {
    let current_version_str = build_version();
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

    let usecase = CheckForUpdatesUseCase::new(updater, bus, state);
    let channel = usecase.execute(CheckForUpdatesCommand { current_version });

    UpdateChannelResponse::from(channel)
}

/// `PERIODIC_CHECK_TICK` ごとに due を判定し、due なら確認する背景 task を起動する
/// (spec.md#impl-periodic)。`lib.rs` の `setup` で 1 回だけ呼ぶ。
///
/// 起動時の確認は page-main が listen 確立後に invoke する (ori-m2ra.1)。この task は
/// 前回確認から 24h 以上経ってからしか確認しないので、listen 前に emit することはない。
pub fn spawn_periodic_update_check(app_handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(PERIODIC_CHECK_TICK).await;
            let state = app_handle
                .state::<Arc<InMemoryUpdateCheckState>>()
                .inner()
                .clone();
            if state.begin_due_check(SystemTime::now()) {
                let _ = execute_update_check(&app_handle, state);
            }
        }
    });
}
