//! frontend への確認結果 DTO。自動確認 (`check_for_updates`) と手動確認
//! (`check_for_updates_manually`) の command が同じ形で返す。

use serde::Serialize;

use super::types::UpdateChannel;

/// Serializable response for the frontend.
///
/// `UpdateChannel` is not `Serialize` (domain types are persistence-agnostic),
/// so we project it into this DTO at the command boundary.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateChannelResponse {
    pub current_version: String,
    pub latest_release: Option<ReleaseResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReleaseResponse {
    pub version: String,
    pub url: String,
    pub notes: String,
}

impl From<UpdateChannel> for UpdateChannelResponse {
    fn from(channel: UpdateChannel) -> Self {
        Self {
            current_version: channel.current_version().to_string(),
            latest_release: channel.latest_release().map(|r| ReleaseResponse {
                version: r.version().to_string(),
                url: r.url().to_string(),
                notes: r.notes().to_string(),
            }),
        }
    }
}
