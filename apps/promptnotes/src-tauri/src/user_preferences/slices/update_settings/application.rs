//! Pure application layer for `update-settings`. All I/O / event publish goes through
//! `SettingsRepository` / `EventBus` ports.

use std::path::{Path, PathBuf};

use crate::user_preferences::shared::ports::{EventBus, SettingsRepository};
use crate::user_preferences::shared::types::{Settings, SettingsEvent, StorageDir};

use super::domain::{
    PathErrorReason, SettingsDiff, UpdateSettingsCommand, UpdateSettingsError,
};

/// `update-settings` の use case (`workflows/update-settings.md#steps`)。
///
/// pipeline:
/// 1. `loadCurrent` (`SettingsRepository::load`)
/// 2. `validateStorageDir` (I-S1 + I-S2)
/// 3. `applyChanges` → `(Settings, SettingsDiff)`
/// 4. `persist` (no-op skip if `SettingsDiff::is_noop()`)
/// 5. `emitConditional` (順序: `StorageDirChanged` → `ThemeChanged`)
pub struct UpdateSettingsUseCase<R: SettingsRepository, B: EventBus> {
    repo: R,
    bus: B,
    config_path: PathBuf,
}

impl<R: SettingsRepository, B: EventBus> UpdateSettingsUseCase<R, B> {
    pub fn new(repo: R, bus: B, config_path: PathBuf) -> Self {
        Self {
            repo,
            bus,
            config_path,
        }
    }

    pub fn execute(
        &self,
        cmd: UpdateSettingsCommand,
    ) -> Result<Settings, UpdateSettingsError> {
        // 1. load
        let current = self.repo.load();

        // 2. validate storage_dir (I-S1 + I-S2)
        let validated_storage_dir =
            validate_storage_dir(cmd.new_storage_dir.as_deref(), &self.config_path)?;

        // 3. apply changes + diff
        let (updated, diff) = apply_changes(current.clone(), validated_storage_dir, cmd.new_theme);

        // 4. persist (skip if no-op, C-US1/C-US2)
        if diff.is_noop() {
            return Ok(current);
        }
        self.repo
            .save(&updated)
            .map_err(|cause| UpdateSettingsError::PersistError {
                path: self.config_path.clone(),
                cause,
            })?;

        // 5. emit conditional (order: storage_dir → theme, C-US5)
        if diff.storage_dir_changed {
            self.bus.publish(SettingsEvent::StorageDirChanged {
                old_dir: current.storage_dir().as_path().to_path_buf(),
                new_dir: updated.storage_dir().as_path().to_path_buf(),
            });
        }
        if diff.theme_changed {
            self.bus.publish(SettingsEvent::ThemeChanged {
                new_theme: updated.theme(),
            });
        }

        Ok(updated)
    }
}

/// I-S1 (絶対パス) + I-S2 (`config_path` を子孫として含まない) の二重検証。
///
/// `None` 入力はそのまま `None` を返す。
fn validate_storage_dir(
    candidate: Option<&Path>,
    config_path: &Path,
) -> Result<Option<StorageDir>, UpdateSettingsError> {
    let Some(path) = candidate else {
        return Ok(None);
    };
    let buf = path.to_path_buf();
    let storage_dir =
        StorageDir::try_from(buf.clone()).map_err(|_| UpdateSettingsError::InvalidPath {
            path: buf.clone(),
            reason: PathErrorReason::NotAbsolute,
        })?;
    // I-S2: config_path が storage_dir 配下にあれば違反 (load-settings の violates_i_s2 と同方向)。
    if config_path.starts_with(storage_dir.as_path()) {
        return Err(UpdateSettingsError::InvalidPath {
            path: buf,
            reason: PathErrorReason::ContainsConfigPath,
        });
    }
    Ok(Some(storage_dir))
}

/// 差分検出 + 適用 (`workflows/update-settings.md#steps` step 3)。
///
/// 個別 field が現在値と等しければ「変更なし」扱い (C-US2)。
fn apply_changes(
    current: Settings,
    new_storage_dir: Option<StorageDir>,
    new_theme: Option<crate::user_preferences::shared::types::Theme>,
) -> (Settings, SettingsDiff) {
    let storage_dir_change = new_storage_dir
        .filter(|nd| nd.as_path() != current.storage_dir().as_path());
    let theme_change = new_theme.filter(|nt| *nt != current.theme());

    let diff = SettingsDiff {
        storage_dir_changed: storage_dir_change.is_some(),
        theme_changed: theme_change.is_some(),
    };

    let mut updated = current;
    if let Some(nd) = storage_dir_change {
        updated = updated.change_storage_dir(nd);
    }
    if let Some(nt) = theme_change {
        updated = updated.change_theme(nt);
    }

    (updated, diff)
}
