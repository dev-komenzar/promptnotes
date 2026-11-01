//! Application layer for `install-update`。副作用は `UpdateInstaller` 越しのみ。

use crate::update_distribution::shared::ports::UpdateInstaller;
use crate::update_distribution::shared::types::{InstallUpdateError, Version};

use super::domain::{ensure_installable, ensure_newer, InstallUpdateCommand};

pub struct InstallUpdateUseCase<I: UpdateInstaller> {
    installer: I,
}

impl<I: UpdateInstaller> InstallUpdateUseCase<I> {
    pub fn new(installer: I) -> Self {
        Self { installer }
    }

    /// `workflows/install-update.md#steps`。成功時はインストールしたバージョンを返す。
    /// 再起動はしない (I-U8: 呼出側が Flush 後に `restart_app` を呼ぶ)。
    pub fn execute(&self, cmd: InstallUpdateCommand) -> Result<Version, InstallUpdateError> {
        // step 1: Unbundled は updater に触れずに拒否 (I-U6 / C-IU3)
        ensure_installable(cmd.installation)?;
        // step 2: 最新リリースを取得し直す (C-IU1: fetch 1 回)
        let pending = self.installer.fetch_pending()?;
        let version = Version::from_str(pending.version_string()).map_err(|_| {
            InstallUpdateError::CheckFailed(format!(
                "unparseable release version: {}",
                pending.version_string()
            ))
        })?;
        // step 3: 新しくなければ download しない (I-U7 / C-IU3)
        ensure_newer(&cmd.current_version, &version)?;
        // step 4: download + 署名検証 + install
        pending.download_and_install()?;
        Ok(version)
    }
}
