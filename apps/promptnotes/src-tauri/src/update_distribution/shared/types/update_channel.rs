use super::{Release, Version};

/// Update Distribution BC の唯一の集約 root (`aggregates.md#update-channel-aggregate`)。揮発。
///
/// `latest_release` の値は I-U2 によって正規化されている:
/// - `Some(_)` ならば `latest_release.version > current_version`
/// - 同一 / 古いリリース / 失敗時はすべて `None`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateChannel {
    current_version: Version,
    latest_release: Option<Release>,
}

impl UpdateChannel {
    /// 新版検出時: `Some(release)` 付き UpdateChannel を構築。
    /// I-U2 の比較は呼出側 (use case の compare_versions) で保証される責務。
    pub fn with_release(current_version: Version, release: Release) -> Self {
        Self {
            current_version,
            latest_release: Some(release),
        }
    }

    /// UpToDate / OlderVersion / 失敗時: `None` の UpdateChannel を構築 (S14 silent + I-U2)。
    pub fn without_release(current_version: Version) -> Self {
        Self {
            current_version,
            latest_release: None,
        }
    }

    pub fn current_version(&self) -> &Version {
        &self.current_version
    }

    pub fn latest_release(&self) -> Option<&Release> {
        self.latest_release.as_ref()
    }

    pub fn has_new_version(&self) -> bool {
        self.latest_release.is_some()
    }
}
