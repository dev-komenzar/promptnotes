use thiserror::Error;

/// `install-update` の失敗 (`workflows/install-update.md#errors`)。
///
/// check-for-updates の `UpdateError` と違い、ユーザ操作の結果なので UI に返す (I-U7)。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InstallUpdateError {
    #[error("this installation does not support in-app updates")]
    UnsupportedInstallation,
    #[error("no newer release is available")]
    NoUpdateAvailable,
    #[error("failed to fetch the latest release: {0}")]
    CheckFailed(String),
    #[error("failed to download the update: {0}")]
    DownloadFailed(String),
    #[error("failed to install the update: {0}")]
    InstallFailed(String),
}

impl InstallUpdateError {
    /// variant 名。TS へのエラー DTO の `kind` に使う (spec.md#tp-boundary TP-B3)。
    pub fn kind(&self) -> &'static str {
        match self {
            Self::UnsupportedInstallation => "UnsupportedInstallation",
            Self::NoUpdateAvailable => "NoUpdateAvailable",
            Self::CheckFailed(_) => "CheckFailed",
            Self::DownloadFailed(_) => "DownloadFailed",
            Self::InstallFailed(_) => "InstallFailed",
        }
    }
}
