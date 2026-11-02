//! Application layer for `install-update`。副作用は `UpdateInstaller` 越しのみ。

use crate::update_distribution::shared::ports::{InstallGuard, UpdateInstaller};
use crate::update_distribution::shared::types::{InstallUpdateError, Version};

use super::domain::{ensure_installable, ensure_newer, InstallUpdateCommand};

pub struct InstallUpdateUseCase<I: UpdateInstaller, G: InstallGuard> {
    installer: I,
    guard: G,
}

impl<I: UpdateInstaller, G: InstallGuard> InstallUpdateUseCase<I, G> {
    pub fn new(installer: I, guard: G) -> Self {
        Self { installer, guard }
    }

    /// `workflows/install-update.md#steps`。成功時はインストールしたバージョンを返す。
    /// 再起動はしない (I-U8: 呼出側が Flush 後に `restart_app` を呼ぶ)。
    pub fn execute(&self, cmd: InstallUpdateCommand) -> Result<Version, InstallUpdateError> {
        // step 1: Unbundled は updater に触れずに拒否 (I-U6 / C-IU3)
        ensure_installable(cmd.installation)?;
        // step 2: 同時実行は 1 本だけ。実行中 / 再起動待ちなら updater に触れずに拒否 (I-U10 / C-IU6)
        if !self.guard.try_begin() {
            return Err(InstallUpdateError::InstallInProgress);
        }
        // panic で unwind しても slot を解放する (Drop で `finish(false)`、C-IU7)
        let slot = InstallSlot {
            guard: &self.guard,
            succeeded: false,
        };
        let result = self.install(cmd);
        // 成功なら再起動まで Installed を保持、失敗なら Idle に戻して再試行可能にする (C-IU7)
        slot.finish(result.is_ok());
        result
    }

    fn install(&self, cmd: InstallUpdateCommand) -> Result<Version, InstallUpdateError> {
        // step 3: 最新リリースを取得し直す (C-IU1: fetch 1 回)
        let pending = self.installer.fetch_pending()?;
        let version = Version::from_str(pending.version_string()).map_err(|_| {
            InstallUpdateError::CheckFailed(format!(
                "unparseable release version: {}",
                pending.version_string()
            ))
        })?;
        // step 4: 新しくなければ download しない (I-U7 / C-IU3)
        ensure_newer(&cmd.current_version, &version)?;
        // step 5: download + 署名検証 + install
        pending.download_and_install()?;
        Ok(version)
    }
}

/// 取得した install slot。`finish` で結果を記録し、`finish` されずに drop された場合 (panic による
/// unwind を含む) は失敗として `Idle` に戻す。slot が `Installing` のまま残って以後の install が
/// すべて `InstallInProgress` になるのを防ぐ (C-IU7)。
struct InstallSlot<'a, G: InstallGuard> {
    guard: &'a G,
    succeeded: bool,
}

impl<G: InstallGuard> InstallSlot<'_, G> {
    fn finish(mut self, succeeded: bool) {
        self.succeeded = succeeded;
    }
}

impl<G: InstallGuard> Drop for InstallSlot<'_, G> {
    fn drop(&mut self) {
        self.guard.finish(self.succeeded);
    }
}
