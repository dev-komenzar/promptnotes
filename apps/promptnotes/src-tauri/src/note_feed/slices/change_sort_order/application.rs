//! Pure application layer for `change-sort-order`. NoteFeed と Settings を
//! **1 トランザクション** で更新する唯一の slice (`aggregates.md#notes-sort-side-effect`)。

use std::path::PathBuf;

use crate::note_feed::shared::types::NoteFeed;
use crate::user_preferences::shared::ports::{EventBus, SettingsRepository};
use crate::user_preferences::shared::types::{PersistError, SettingsEvent};

use super::domain::{ChangeSortOrderCommand, ChangeSortOrderError};

/// `change-sort-order` の use case (`workflows/change-sort-order.md#steps`)。
///
/// pipeline:
/// 1. `load Settings` (`SettingsRepository::load`)
/// 2. 差分判定 (C-CSO1): 同値なら早期 return
/// 3. `apply_to_feed` (`NoteFeed::change_sort`)
/// 4. `apply_to_settings` (`Settings::change_sort_preference`)
/// 5. `persist` (`SettingsRepository::save`)
/// 6. `emit` (`EventBus::publish(SortPreferenceChanged)`)
///
/// PersistError は `shared::types::PersistError` を利用 (ori-hpo.8 / C-CSO6)。
pub struct ChangeSortOrderUseCase<R: SettingsRepository, B: EventBus> {
    repo: R,
    bus: B,
    config_path: PathBuf,
}

impl<R: SettingsRepository, B: EventBus> ChangeSortOrderUseCase<R, B> {
    pub fn new(repo: R, bus: B, config_path: PathBuf) -> Self {
        Self {
            repo,
            bus,
            config_path,
        }
    }

    pub fn execute(
        &self,
        feed: NoteFeed,
        cmd: ChangeSortOrderCommand,
    ) -> Result<NoteFeed, ChangeSortOrderError> {
        // 1. load
        let current_settings = self.repo.load();

        // 2. 差分判定 (C-CSO1)
        if current_settings.sort_preference() == cmd.new_sort {
            return Ok(feed);
        }

        // 3. NoteFeed 更新 (in-memory)
        let updated_feed = feed.change_sort(cmd.new_sort);

        // 4. Settings 更新 (in-memory)
        let updated_settings = current_settings.change_sort_preference(cmd.new_sort);

        // 5. persist
        self.repo
            .save(&updated_settings)
            .map_err(|cause| PersistError {
                path: self.config_path.clone(),
                cause,
            })?;

        // 6. emit (C-CSO3: persist 成功後のみ)
        self.bus.publish(SettingsEvent::SortPreferenceChanged {
            new_sort: cmd.new_sort,
        });

        Ok(updated_feed)
    }
}
