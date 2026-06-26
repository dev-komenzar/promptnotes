//! Domain types for `check-for-updates` slice (pure).

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
