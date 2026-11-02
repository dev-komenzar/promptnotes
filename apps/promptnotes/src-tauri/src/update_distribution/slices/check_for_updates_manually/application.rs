//! Application layer for `check-for-updates-manually`。副作用は `UpdaterPort` 越しのみ。
//!
//! I-U9: 失敗は `Err` で要求元 UI に返す (S14 の silent 方針は適用しない)。
//! `EventBus` / `NotifiedVersionLedger` に依存しないので、event 発行も通知済み state の更新も
//! 型レベルで起こらない (spec.md#invariants-slice-specific C-CFUM2)。

use crate::update_distribution::shared::ports::UpdaterPort;
use crate::update_distribution::shared::release_check::{compare, parse_release, Comparison};
use crate::update_distribution::shared::types::{UpdateChannel, UpdateError};

use super::domain::CheckForUpdatesManuallyCommand;

pub struct CheckForUpdatesManuallyUseCase<U: UpdaterPort> {
    updater: U,
}

impl<U: UpdaterPort> CheckForUpdatesManuallyUseCase<U> {
    pub fn new(updater: U) -> Self {
        Self { updater }
    }

    /// `workflows/check-for-updates-manually.md#steps`。fetch は 1 回のみ (C-CFUM1)。
    pub fn execute(
        &self,
        cmd: CheckForUpdatesManuallyCommand,
    ) -> Result<UpdateChannel, UpdateError> {
        let raw = self.updater.fetch_latest_release()?;
        let release = parse_release(raw)?;
        Ok(match compare(&cmd.current_version, release) {
            Comparison::NewVersion(release) => {
                UpdateChannel::with_release(cmd.current_version, release)
            }
            // I-U2: 同一 / 古い → None 正規化
            Comparison::UpToDate | Comparison::OlderVersion => {
                UpdateChannel::without_release(cmd.current_version)
            }
        })
    }
}
