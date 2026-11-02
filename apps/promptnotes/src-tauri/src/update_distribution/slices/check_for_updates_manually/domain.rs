//! Domain types for `check-for-updates-manually` slice (pure)。

use crate::update_distribution::shared::types::Version;

/// `check-for-updates-manually` slice の input
/// (`workflows/check-for-updates-manually.md#input`)。
#[derive(Debug, Clone)]
pub struct CheckForUpdatesManuallyCommand {
    pub current_version: Version,
}
