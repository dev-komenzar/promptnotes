use super::Version;

/// `UpdateChannel::check_at_startup` 成功 + 新版検出時に発行される domain event
/// (`domain-events.md#new-version-detected`)。
///
/// 失敗時 / UpToDate / OlderVersion では発行しない (I-U2 / S14)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewVersionDetected {
    pub current_version: Version,
    pub latest_version: Version,
    pub release_url: String,
    pub release_notes: String,
}
