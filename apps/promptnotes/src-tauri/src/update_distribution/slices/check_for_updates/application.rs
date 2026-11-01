//! Application layer for `check-for-updates`. 副作用は `UpdaterPort` + `EventBus` 越しのみ。
//!
//! `execute` シグネチャは `(cmd) -> UpdateChannel` (Result 露出なし、C-CFU1)。
//! 失敗 / UpToDate / OlderVersion は全て `UpdateChannel::without_release` に正規化される
//! (S14 silent failure + I-U2)。同一プロセス内で通知済みのバージョン以下は event を発行しない
//! (I-U4。`NotifiedVersionLedger` が揮発 state を持つ)。

use std::cmp::Ordering;

use crate::update_distribution::shared::ports::{
    EventBus, NotifiedVersionLedger, RawRelease, UpdaterPort,
};
use crate::update_distribution::shared::types::{
    NewVersionDetected, Release, UpdateChannel, UpdateError, Version,
};

use super::domain::{CheckForUpdatesCommand, Comparison};

pub struct CheckForUpdatesUseCase<U: UpdaterPort, B: EventBus, L: NotifiedVersionLedger> {
    updater: U,
    bus: B,
    ledger: L,
}

impl<U: UpdaterPort, B: EventBus, L: NotifiedVersionLedger> CheckForUpdatesUseCase<U, B, L> {
    pub fn new(updater: U, bus: B, ledger: L) -> Self {
        Self {
            updater,
            bus,
            ledger,
        }
    }

    /// pure entry point: `Result` を返さない (C-CFU1)。
    /// `try_execute` が `Err` を返したら `UpdateChannel::without_release` に降格 (S14)。
    pub fn execute(&self, cmd: CheckForUpdatesCommand) -> UpdateChannel {
        let current_version = cmd.current_version.clone();
        match self.try_execute(cmd) {
            Ok(channel) => channel,
            Err(e) => {
                // S14 / C-CFU3: silent failure。UI には伝搬せず、ログ出力のみ。
                log::warn!("update check failed: {e:?}");
                UpdateChannel::without_release(current_version)
            }
        }
    }

    /// 内部 pipeline (`workflows/check-for-updates.md#steps`)。`?` で early return。
    /// 1 回の確認で fetch は **1 回のみ** (即時リトライなし、I-U3 / C-CFU4)。
    fn try_execute(&self, cmd: CheckForUpdatesCommand) -> Result<UpdateChannel, UpdateError> {
        let raw = self.updater.fetch_latest_release()?;
        let release = parse_release(raw)?;
        match compare(&cmd.current_version, release) {
            Comparison::NewVersion(release) => {
                let channel =
                    UpdateChannel::with_release(cmd.current_version.clone(), release.clone());
                // step 6 suppressNotified (I-U4 / C-CFU8): 通知済み以下なら event 非発行。
                // claim は判定と記録を 1 操作で行う。確認結果 (Some) はそのまま返す。
                if !self.ledger.claim(release.version()) {
                    return Ok(channel);
                }
                // step 7 emit (C-CFU5): 未通知の新版のみ event 1 件 publish
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
