//! Domain layer for `install-update` (純粋関数のみ)。

use crate::update_distribution::shared::types::{InstallUpdateError, InstallationKind, Version};

/// `workflows/install-update.md#input`。
#[derive(Debug, Clone)]
pub struct InstallUpdateCommand {
    pub current_version: Version,
    pub installation: InstallationKind,
}

/// step 1 (I-U6): bundle 種別を持たないビルドは install できない。
pub fn ensure_installable(installation: InstallationKind) -> Result<(), InstallUpdateError> {
    if installation.is_installable() {
        Ok(())
    } else {
        Err(InstallUpdateError::UnsupportedInstallation)
    }
}

/// step 3 (I-U7 / I-U2): 取得し直した版が現在版より新しいときだけ install する。
pub fn ensure_newer(current: &Version, candidate: &Version) -> Result<(), InstallUpdateError> {
    if candidate > current {
        Ok(())
    } else {
        Err(InstallUpdateError::NoUpdateAvailable)
    }
}
