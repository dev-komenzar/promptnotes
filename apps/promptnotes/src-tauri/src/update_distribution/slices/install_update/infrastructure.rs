//! Infrastructure adapters for `install-update` slice.
//!
//! `UpdateInstaller` / `PendingUpdate` を Tauri v2 updater plugin に橋渡しする。plugin の
//! version 比較は permissive にし、domain (`ensure_newer`) を唯一の比較者にする (C-IU2)。

use std::sync::{Mutex, PoisonError};

use tauri::utils::config::BundleType;
use tauri::utils::platform::bundle_type;
use tauri_plugin_updater::UpdaterExt;

use super::domain::InstallPhase;
use crate::update_distribution::shared::ports::{InstallGuard, PendingUpdate, UpdateInstaller};
use crate::update_distribution::shared::types::{
    BundleFormat, InstallUpdateError, InstallationKind,
};
#[cfg(debug_assertions)]
use crate::update_distribution::shared::updater::with_test_endpoint;

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
///
/// 開発ビルド (`debug_assertions`) は `Unbundled`。macOS では bundler 未経由でも
/// `bundle_type()` が `Some(App)` を返すため、そのままだと dev でもボタンが出てしまう。
/// ただし E2E (s24 / s25) は debug build で install 経路を通すため、`TAURI_TEST_INSTALLATION_KIND`
/// で配布形式を上書きできる (debug build 限定 seam。release build では compile out)。
pub fn current_installation_kind() -> InstallationKind {
    if cfg!(debug_assertions) {
        return debug_installation_kind();
    }
    installation_kind_from(bundle_type())
}

/// debug build の配布形式。`TAURI_TEST_INSTALLATION_KIND` があればそれ、無ければ `Unbundled`。
#[cfg(debug_assertions)]
fn debug_installation_kind() -> InstallationKind {
    std::env::var("TAURI_TEST_INSTALLATION_KIND")
        .ok()
        .and_then(|v| parse_test_installation_kind(&v))
        .unwrap_or(InstallationKind::Unbundled)
}

/// release build では呼ばれない (`cfg!(debug_assertions)` が false)。seam の env 読み出しを含めない。
#[cfg(not(debug_assertions))]
fn debug_installation_kind() -> InstallationKind {
    InstallationKind::Unbundled
}

/// `TAURI_TEST_INSTALLATION_KIND` の値 (`appimage` / `deb` / `rpm` / `unbundled`) を解釈する。
/// 未知の値は `None` (= debug 既定の `Unbundled`)。
#[cfg(debug_assertions)]
pub(crate) fn parse_test_installation_kind(value: &str) -> Option<InstallationKind> {
    let kind = match value.trim().to_ascii_lowercase().as_str() {
        "appimage" => InstallationKind::Bundled(BundleFormat::AppImage),
        "deb" => InstallationKind::Bundled(BundleFormat::Deb),
        "rpm" => InstallationKind::Bundled(BundleFormat::Rpm),
        "unbundled" => InstallationKind::Unbundled,
        other => {
            log::warn!("unknown TAURI_TEST_INSTALLATION_KIND: {other}");
            return None;
        }
    };
    Some(kind)
}

/// install-update の揮発な実行状態 (I-U10)。`lib.rs` で `Arc` にして manage し、
/// toast / 設定モーダルのどちらから invoke されても同じ instance を共有する (C-IU6)。
pub struct InMemoryInstallState {
    phase: Mutex<InstallPhase>,
}

impl InMemoryInstallState {
    pub fn new() -> Self {
        Self {
            phase: Mutex::new(InstallPhase::Idle),
        }
    }

    pub fn phase(&self) -> InstallPhase {
        *self.phase.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Default for InMemoryInstallState {
    fn default() -> Self {
        Self::new()
    }
}

impl InstallGuard for InMemoryInstallState {
    fn try_begin(&self) -> bool {
        let mut phase = self.phase.lock().unwrap_or_else(PoisonError::into_inner);
        match phase.begin() {
            Some(next) => {
                *phase = next;
                true
            }
            None => false,
        }
    }

    fn finish(&self, succeeded: bool) {
        let mut phase = self.phase.lock().unwrap_or_else(PoisonError::into_inner);
        *phase = phase.finish(succeeded);
    }
}

pub struct TauriUpdateInstaller {
    pub app_handle: tauri::AppHandle,
}

impl UpdateInstaller for TauriUpdateInstaller {
    fn fetch_pending(&self) -> Result<Box<dyn PendingUpdate>, InstallUpdateError> {
        let builder = self
            .app_handle
            .updater_builder()
            .version_comparator(|_current, _remote| true);

        // E2E test seam (s24 / s25): debug build では取得先と署名検証の公開鍵を差し替えられる。
        // 公開鍵は E2E が実行時に生成した鍵で署名したテストパッケージを検証するため。
        // release build では分岐ごと compile out され、本番の endpoint / pubkey (tauri.conf.json) は不変。
        #[cfg(debug_assertions)]
        let builder = {
            let builder = with_test_endpoint(builder).map_err(|e| {
                InstallUpdateError::CheckFailed(format!("test endpoint override failed: {e:?}"))
            })?;
            match std::env::var("TAURI_TEST_UPDATER_PUBKEY") {
                Ok(pubkey) => builder.pubkey(pubkey),
                Err(_) => builder,
            }
        };

        let updater = builder
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

pub(crate) fn map_download_error(e: tauri_plugin_updater::Error) -> InstallUpdateError {
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
