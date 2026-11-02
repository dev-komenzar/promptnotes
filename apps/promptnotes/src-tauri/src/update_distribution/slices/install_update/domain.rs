//! Domain layer for `install-update` (純粋関数のみ)。

use crate::update_distribution::shared::types::{InstallUpdateError, InstallationKind, Version};

/// `workflows/install-update.md#input`。
#[derive(Debug, Clone)]
pub struct InstallUpdateCommand {
    pub current_version: Version,
    pub installation: InstallationKind,
}

/// install-update の揮発な実行状態 (I-U10)。プロセス内のみ、再起動で `Idle` に戻る。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallPhase {
    Idle,
    Installing,
    /// インストール成功後、再起動待ち。再インストールは受け付けない。
    Installed,
}

impl InstallPhase {
    /// step 2 `acquireInstallSlot`: `Idle` のときだけ `Installing` に進める。
    pub fn begin(self) -> Option<InstallPhase> {
        match self {
            InstallPhase::Idle => Some(InstallPhase::Installing),
            InstallPhase::Installing | InstallPhase::Installed => None,
        }
    }

    /// `Installing` の終了。成功なら `Installed`、失敗なら `Idle` (再試行可能、I-U7)。
    pub fn finish(self, succeeded: bool) -> InstallPhase {
        match self {
            InstallPhase::Installing if succeeded => InstallPhase::Installed,
            InstallPhase::Installing => InstallPhase::Idle,
            other => other,
        }
    }
}

/// step 1 (I-U6): bundle 種別を持たないビルドは install できない。
pub fn ensure_installable(installation: InstallationKind) -> Result<(), InstallUpdateError> {
    if installation.is_installable() {
        Ok(())
    } else {
        Err(InstallUpdateError::UnsupportedInstallation)
    }
}

/// step 4 (I-U7 / I-U2): 取得し直した版が現在版より新しいときだけ install する。
pub fn ensure_newer(current: &Version, candidate: &Version) -> Result<(), InstallUpdateError> {
    if candidate > current {
        Ok(())
    } else {
        Err(InstallUpdateError::NoUpdateAvailable)
    }
}
