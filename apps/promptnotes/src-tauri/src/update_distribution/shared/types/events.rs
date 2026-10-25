use super::Version;

/// `UpdateChannel::check` 成功 + 同一プロセス内で未通知の新版検出時に発行される domain event
/// (`domain-events.md#new-version-detected`)。起動時と 24 時間ごとの定期確認で発行されうる (I-U3)。
///
/// 失敗時 / UpToDate / OlderVersion / 通知済みバージョン以下では発行しない (I-U2 / S14 / I-U4)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewVersionDetected {
    pub current_version: Version,
    pub latest_version: Version,
    pub release_url: String,
    pub release_notes: String,
}
