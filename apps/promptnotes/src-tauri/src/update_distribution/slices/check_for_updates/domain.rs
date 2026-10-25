//! Domain types for `check-for-updates` slice (pure).

use std::time::{Duration, SystemTime};

use crate::update_distribution::shared::types::{Release, Version};

/// `check-for-updates` slice の input (`workflows/check-for-updates.md#input`)。
#[derive(Debug, Clone)]
pub struct CheckForUpdatesCommand {
    pub current_version: Version,
}

/// `compareVersions` step の結果 (`workflows/check-for-updates.md#steps`)。
/// `NewVersion(Release)` のみが event 発行 path、それ以外は I-U2 によって `None` 正規化。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comparison {
    NewVersion(Release),
    UpToDate,
    OlderVersion,
}

// ===== I-U3: 24 時間周期 (spec.md#invariants-slice-specific C-CFU10 / C-CFU11) =====

/// 前回確認からこの時間が経過したら再確認する (I-U3。定数、設定 UI なし)。
pub const UPDATE_CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// 定期確認 task が due 判定を行う間隔 (C-CFU11)。
///
/// 経過時間は wall clock で測るため、sleep / 休止で tick が遅れても復帰後最初の tick で due になる。
pub const PERIODIC_CHECK_TICK: Duration = Duration::from_secs(60 * 60);

/// `last_checked_at` から `now` までに `UPDATE_CHECK_INTERVAL` 以上経過していれば due (C-CFU10)。
///
/// wall clock が `last_checked_at` より過去に戻った場合も due とみなす (C-CFU11)。
/// 1 回確認すれば基準が張り直されるので、確認が連続することはない。
pub fn is_check_due(last_checked_at: SystemTime, now: SystemTime) -> bool {
    match now.duration_since(last_checked_at) {
        Ok(elapsed) => elapsed >= UPDATE_CHECK_INTERVAL,
        Err(_) => true,
    }
}

// ===== I-U4: 通知済みバージョンの再通知抑止 (C-CFU8) =====

/// `latest` を通知すべきか。通知済みが無い、または `latest > notified` のときのみ true。
pub fn should_notify(latest: &Version, notified: Option<&Version>) -> bool {
    match notified {
        None => true,
        Some(notified) => latest > notified,
    }
}
