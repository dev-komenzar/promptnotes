//! Infrastructure adapters for `install-update` slice.
//!
//! `UpdateInstaller` / `PendingUpdate` を Tauri v2 updater plugin に橋渡しする。plugin の
//! version 比較は permissive にし、domain (`ensure_newer`) を唯一の比較者にする (C-IU2)。

use tauri::utils::config::BundleType;
use tauri::utils::platform::bundle_type;
use tauri_plugin_updater::UpdaterExt;

use crate::update_distribution::shared::ports::{PendingUpdate, UpdateInstaller};
use crate::update_distribution::shared::types::{
    BundleFormat, InstallUpdateError, InstallationKind,
};

/// `BundleType` → `InstallationKind` (C-IU4)。`None` は bundle 種別なし (nix 等)。
pub(crate) fn installation_kind_from(bundle: Option<BundleType>) -> InstallationKind {
    let format = match bundle {
        Some(BundleType::App) | Some(BundleType::Dmg) => BundleFormat::MacApp,
        Some(BundleType::AppImage) => BundleFormat::AppImage,
        Some(BundleType::Deb) => BundleFormat::Deb,
        Some(BundleType::Rpm) => BundleFormat::Rpm,
        Some(BundleType::Msi) => BundleFormat::Msi,
        Some(BundleType::Nsis) => BundleFormat::Nsis,
        _ => return InstallationKind::Unbundled,
    };
    InstallationKind::Bundled(format)
}

/// 実行中アプリの配布形式。bundler が binary に埋め込んだ値を読む。
pub fn current_installation_kind() -> InstallationKind {
    installation_kind_from(bundle_type())
}

pub struct TauriUpdateInstaller {
    pub app_handle: tauri::AppHandle,
}

impl UpdateInstaller for TauriUpdateInstaller {
    fn fetch_pending(&self) -> Result<Box<dyn PendingUpdate>, InstallUpdateError> {
        let updater = self
            .app_handle
            .updater_builder()
            .version_comparator(|_current, _remote| true)
            .build()
            .map_err(|e| InstallUpdateError::CheckFailed(e.to_string()))?;

        let result = block_on(updater.check());
        match result {
            Ok(Some(update)) => Ok(Box::new(TauriPendingUpdate { update })),
            Ok(None) => Err(InstallUpdateError::NoUpdateAvailable),
            Err(e) => {
                log::warn!("install-update: updater check failed: {e:?}");
                Err(InstallUpdateError::CheckFailed(e.to_string()))
            }
        }
    }
}

struct TauriPendingUpdate {
    update: tauri_plugin_updater::Update,
}

impl PendingUpdate for TauriPendingUpdate {
    fn version_string(&self) -> &str {
        &self.update.version
    }

    fn download_and_install(self: Box<Self>) -> Result<(), InstallUpdateError> {
        // 署名検証は plugin の download 内で行われる。検証失敗は install 失敗として扱う (I-U7)。
        let bytes = block_on(self.update.download(|_, _| {}, || {})).map_err(|e| {
            log::warn!("install-update: download failed: {e:?}");
            map_download_error(e)
        })?;
        self.update.install(bytes).map_err(|e| {
            log::warn!("install-update: install failed: {e:?}");
            InstallUpdateError::InstallFailed(e.to_string())
        })
    }
}

fn map_download_error(e: tauri_plugin_updater::Error) -> InstallUpdateError {
    use tauri_plugin_updater::Error as E;
    match e {
        E::Minisign(_) | E::Base64(_) | E::SignatureUtf8(_) => {
            InstallUpdateError::InstallFailed(e.to_string())
        }
        _ => InstallUpdateError::DownloadFailed(e.to_string()),
    }
}

/// Tauri async command (tokio runtime) 内から updater の async API を同期で呼ぶ。
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}
