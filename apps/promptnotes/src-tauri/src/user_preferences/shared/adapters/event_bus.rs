//! Production `EventBus` adapter for the User Preferences BC.
//!
//! Tauri の `app.emit` 経由で UI 側へ `settings:*` イベントを通知する。
//! emit 失敗 (window がまだ無い等) は無視して握り潰す: domain event の
//! 永続化はあくまで `SettingsRepository::save` の責務であり、通知失敗で
//! ロールバックはしない (`domain-events.md#notes-sync-rationale` の方針)。

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};

use crate::user_preferences::shared::ports::EventBus;
use crate::user_preferences::shared::types::{SettingsEvent, SortOrder, Theme};

pub const STORAGE_DIR_CHANGED_EVENT: &str = "settings:storage_dir_changed";
pub const THEME_CHANGED_EVENT: &str = "settings:theme_changed";
pub const SORT_PREFERENCE_CHANGED_EVENT: &str = "settings:sort_preference_changed";

#[derive(Serialize, Clone)]
struct StorageDirChangedPayload {
    old_dir: PathBuf,
    new_dir: PathBuf,
}

#[derive(Serialize, Clone)]
struct ThemeChangedPayload {
    new_theme: Theme,
}

#[derive(Serialize, Clone)]
struct SortPreferenceChangedPayload {
    new_sort: SortOrder,
}

pub struct TauriEventBus<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriEventBus<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> EventBus for TauriEventBus<R> {
    fn publish(&self, event: SettingsEvent) {
        match event {
            SettingsEvent::StorageDirChanged { old_dir, new_dir } => {
                let _ = self.app.emit(
                    STORAGE_DIR_CHANGED_EVENT,
                    StorageDirChangedPayload { old_dir, new_dir },
                );
            }
            SettingsEvent::ThemeChanged { new_theme } => {
                let _ = self
                    .app
                    .emit(THEME_CHANGED_EVENT, ThemeChangedPayload { new_theme });
            }
            SettingsEvent::SortPreferenceChanged { new_sort } => {
                let _ = self.app.emit(
                    SORT_PREFERENCE_CHANGED_EVENT,
                    SortPreferenceChangedPayload { new_sort },
                );
            }
        }
    }
}
