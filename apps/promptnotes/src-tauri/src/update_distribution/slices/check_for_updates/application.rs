//! Application layer for `check-for-updates`. 副作用は `UpdaterPort` + `EventBus` 越しのみ。
//!
//! `execute` シグネチャは `(cmd) -> UpdateChannel` (Result 露出なし、C-CFU1)。
//! 失敗 / UpToDate / OlderVersion は全て `UpdateChannel::without_release` に正規化される
//! (S14 silent failure + I-U2)。

use std::cmp::Ordering;

use crate::update_distribution::shared::ports::{EventBus, RawRelease, UpdaterPort};
use crate::update_distribution::shared::types::{
    NewVersionDetected, Release, UpdateChannel, UpdateError, Version,
};

use super::domain::{CheckForUpdatesCommand, Comparison};

pub struct CheckForUpdatesUseCase<U: UpdaterPort, B: EventBus> {
    updater: U,
    bus: B,
}

impl<U: UpdaterPort, B: EventBus> CheckForUpdatesUseCase<U, B> {
    pub fn new(updater: U, bus: B) -> Self {
        Self { updater, bus }
    }

    /// pure entry point: `Result` を返さない (C-CFU1)。
    /// `try_execute` が `Err` を返したら `UpdateChannel::without_release` に降格 (S14)。
    pub fn execute(&self, cmd: CheckForUpdatesCommand) -> UpdateChannel {
        let current_version = cmd.current_version.clone();
        match self.try_execute(cmd) {
            Ok(channel) => channel,
            Err(_e) => {
                // S14 / C-CFU3: silent failure。実 wiring (ori-6l4) 後は `log::warn!("update check failed: {:?}", _e);` を有効化。
                UpdateChannel::without_release(current_version)
            }
        }
    }

    /// 内部 pipeline (`workflows/check-for-updates.md#steps`)。`?` で early return。
    /// I-U3 通り fetch は **1 回のみ** (リトライループなし、C-CFU4)。
    fn try_execute(&self, cmd: CheckForUpdatesCommand) -> Result<UpdateChannel, UpdateError> {
        let raw = self.updater.fetch_latest_release()?;
        let release = parse_release(raw)?;
        match compare(&cmd.current_version, release) {
            Comparison::NewVersion(release) => {
                let channel =
                    UpdateChannel::with_release(cmd.current_version.clone(), release.clone());
                // C-CFU5: 新版時のみ event 1 件 publish
                self.bus.publish(NewVersionDetected {
                    current_version: cmd.current_version,
                    latest_version: release.version().clone(),
                    release_url: release.url().to_string(),
                    release_notes: release.notes().to_string(),
                });
                Ok(channel)
            }
            // I-U2: 同一 / 古い → None 正規化、event 非発行 (C-CFU6)
            Comparison::UpToDate | Comparison::OlderVersion => {
                Ok(UpdateChannel::without_release(cmd.current_version))
            }
        }
    }
}

/// `RawRelease` を parse して `Release` を作る (step 2 + 5 をまとめる)。
fn parse_release(raw: RawRelease) -> Result<Release, UpdateError> {
    let version = Version::from_str(&raw.version_string)?;
    Ok(Release::new(version, raw.url, raw.notes))
}

/// `(Version, Release)` から `Comparison` を導出 (I-U2 の単一エントリポイント)。
fn compare(current: &Version, candidate: Release) -> Comparison {
    match candidate.version().cmp(current) {
        Ordering::Greater => Comparison::NewVersion(candidate),
        Ordering::Equal => Comparison::UpToDate,
        Ordering::Less => Comparison::OlderVersion,
    }
}
